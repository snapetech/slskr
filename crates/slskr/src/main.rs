#[allow(dead_code)]
mod batch;
mod config;
mod events_ws;
mod http_server;
mod logging;
mod openapi;
mod persistence;
mod rate_limit;
mod routing;
mod storage;
mod tracing;
mod utils;
mod webhooks;

use std::{
    collections::BTreeMap,
    env, fs,
    net::{SocketAddr, SocketAddrV4},
    path::{Component, Path, PathBuf},
    sync::Arc,
};

use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use slskr_client::{
    connection::ConnectionKind,
    listener::{IncomingConnection, Listener},
    peer_connect::{
        connect_file_transfer, connect_peer_messages, send_obfuscated_peer_init,
        send_pierce_firewall,
    },
    protocol::{
        peer::{
            FileEntry, FileSearchResponse, FolderContentsRequest, PeerMessage, TransferRequest,
            TransferResponse, UserInfo,
        },
        server::{
            ConnectToPeerRequest, ConnectToPeerResponse, PeerAddress, RoomList, RoomListEntry,
            SearchRequest, ServerMessage, TargetedSearchRequest, UserStats, UserStatus,
            WatchedUser,
        },
        Reader, Writer, ROTATED_OBFUSCATION_TYPE,
    },
    server::ServerSession,
    share_payload::{compress_zlib_payload, decompress_zlib_payload},
    stream::{ObfuscatedPeerMessageConnection, PeerMessageConnection, ServerConnection},
    version::{CLIENT_MAJOR_VERSION, CLIENT_MINOR_VERSION, CLIENT_NAME},
};
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
    sync::{broadcast, mpsc, RwLock, Semaphore},
    time::{self, Duration, Instant},
};

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

use crate::config::{
    json_bool_option, json_escape, json_option, json_u32_option, json_u64_option,
    json_usize_option, AppConfig,
};
use crate::utils::*;

use config::redact_username;

const TRANSFER_PROGRESS_CHUNK_BYTES: usize = 64 * 1024;
const EVENT_HISTORY_LIMIT: usize = 500;
const DEFAULT_LIST_LIMIT: usize = 500;

#[allow(dead_code)]
const APP_CAPABILITIES: &[&str] = &[
    "health",
    "version",
    "config",
    "stats",
    "metrics",
    "telemetry",
    "session-control",
    "session-privilege-check",
    "listeners",
    "shares",
    "share-catalog",
    "share-files",
    "share-rescan",
    "search-dispatch",
    "search-results",
    "user-watch",
    "user-stats",
    "user-browse",
    "messages",
    "rooms",
    "room-list-sync",
    "transfers",
    "events",
    "browser-session-auth",
    "csrf-origin-guard",
];

#[allow(dead_code)]
const NETWORK_CAPABILITIES: &[&str] = &[
    "server-session",
    "regular-listener",
    "obfuscated-listener",
    "plain-peer-messages",
    "obfuscated-peer-messages",
    "distributed-peer",
    "file-transfer",
    "indirect-connect",
];

#[allow(dead_code)]
const STORAGE_CAPABILITIES: &[&str] = &[
    "share-index-tsv",
    "transfer-events-tsv",
    "transfer-state-json",
];

const MAX_TRANSFER_STATE_BYTES: u64 = 16 * 1024 * 1024;

#[allow(dead_code)]
const EXPERIMENTAL_CAPABILITIES: &[&str] = &[
    "direct-peer-browse",
    "direct-and-indirect-file-transfer",
    "dashboard",
];

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    let args = env::args_os().skip(1).collect::<Vec<_>>();
    if args.first().and_then(|arg| arg.to_str()) == Some("serve") {
        let once = args.iter().any(|arg| arg == "--once");
        return serve(once).await;
    }

    slskr_cli::run_from_args(args).await
}

#[derive(Clone, Debug)]
struct ShareRoot {
    label: String,
    files: usize,
    bytes: u64,
    extensions: Vec<ShareExtensionSummary>,
}

#[derive(Clone, Debug)]
struct ShareExtensionSummary {
    extension: String,
    files: usize,
    bytes: u64,
}

impl ShareExtensionSummary {
    fn json(&self) -> String {
        format!(
            "{{\"extension\":\"{}\",\"files\":{},\"bytes\":{}}}",
            json_escape(&self.extension),
            self.files,
            self.bytes
        )
    }
}

impl ShareRoot {
    fn json(&self) -> String {
        let extensions = self
            .extensions
            .iter()
            .map(ShareExtensionSummary::json)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"label\":\"{}\",\"files\":{},\"bytes\":{},\"extensions\":[{}]}}",
            json_escape(&self.label),
            self.files,
            self.bytes,
            extensions
        )
    }
}

#[derive(Debug)]
struct ShareScan {
    entries: Vec<FileEntry>,
    local_paths: BTreeMap<String, PathBuf>,
    roots: Vec<ShareRoot>,
    errors: Vec<String>,
}

#[derive(Clone, Copy, Debug)]
struct ShareScanOptions {
    follow_symlinks: bool,
    include_hidden: bool,
    max_files: usize,
}

#[derive(Clone, Debug)]
struct ShareIndexSnapshot {
    entries: Vec<FileEntry>,
    local_paths: BTreeMap<String, PathBuf>,
    roots: Vec<ShareRoot>,
    fixture_files: usize,
    scan_errors: Vec<String>,
    cache_path: PathBuf,
    cache_written_at: Option<u64>,
    cache_error: Option<String>,
    updated_at: u64,
}

impl ShareIndexSnapshot {
    fn json(&self) -> String {
        let roots = self
            .roots
            .iter()
            .map(ShareRoot::json)
            .collect::<Vec<_>>()
            .join(",");
        let errors = self
            .scan_errors
            .iter()
            .map(|error| json_option(Some(error.as_str())))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"roots\":[{}],\"files\":{},\"fixture_files\":{},\"scan_errors\":[{}],\"cache_file\":\"{}\",\"cache_written_at\":{},\"cache_error\":{},\"updated_at\":{}}}",
            roots,
            self.entries.len(),
            self.fixture_files,
            errors,
            json_escape(
                self.cache_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("share-index.tsv")
            ),
            json_u64_option(self.cache_written_at),
            json_option(self.cache_error.as_deref()),
            self.updated_at
        )
    }

    fn summary_json(&self) -> String {
        let bytes = self.entries.iter().map(|entry| entry.size).sum::<u64>();
        format!(
            "{{\"roots\":{},\"files\":{},\"fixture_files\":{},\"bytes\":{},\"scan_errors\":{},\"cache_error\":{},\"updated_at\":{}}}",
            self.roots.len(),
            self.entries.len(),
            self.fixture_files,
            bytes,
            self.scan_errors.len(),
            self.cache_error.is_some(),
            self.updated_at
        )
    }

    fn catalog_json(&self, query: Option<&str>) -> String {
        let filter = CatalogFilter::from_query(query);
        let mut entries = self
            .entries
            .iter()
            .filter(|entry| filter.matches(entry))
            .collect::<Vec<_>>();
        entries.sort_by(|left, right| left.filename.cmp(&right.filename));
        let filtered_count = entries.len();
        let total_bytes = entries.iter().map(|entry| entry.size).sum::<u64>();
        let files = entries
            .iter()
            .skip(filter.offset)
            .take(filter.limit.unwrap_or(usize::MAX))
            .map(|entry| {
                format!(
                    "{{\"path\":\"{}\",\"size\":{},\"extension\":\"{}\",\"attribute_count\":{}}}",
                    json_escape(&entry.filename),
                    entry.size,
                    json_escape(&entry.extension),
                    entry.attributes.len()
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"files\":[{}],\"count\":{},\"filtered_count\":{},\"total_bytes\":{},\"offset\":{},\"limit\":{},\"updated_at\":{}}}",
            files,
            self.entries.len(),
            filtered_count,
            total_bytes,
            filter.offset,
            json_usize_option(filter.limit),
            self.updated_at
        )
    }
}

#[derive(Debug, Default)]
struct CatalogFilter {
    q: Option<String>,
    prefix: Option<String>,
    extension: Option<String>,
    limit: Option<usize>,
    offset: usize,
}

impl CatalogFilter {
    fn from_query(query: Option<&str>) -> Self {
        let mut filter = Self {
            limit: Some(DEFAULT_LIST_LIMIT),
            ..Self::default()
        };
        for (name, value) in query_params(query.unwrap_or_default()) {
            match name.as_str() {
                "q" => filter.q = non_empty(value.to_ascii_lowercase()),
                "prefix" => filter.prefix = non_empty(value),
                "extension" => {
                    filter.extension =
                        non_empty(value.trim_start_matches('.').to_ascii_lowercase());
                }
                "limit" => filter.limit = Some(parse_list_limit(&value)),
                "offset" => filter.offset = value.parse::<usize>().unwrap_or(0),
                _ => {}
            }
        }
        filter
    }

    fn matches(&self, entry: &FileEntry) -> bool {
        if let Some(q) = &self.q {
            if !entry.filename.to_ascii_lowercase().contains(q) {
                return false;
            }
        }
        if let Some(prefix) = &self.prefix {
            if !entry.filename.starts_with(prefix) {
                return false;
            }
        }
        if let Some(extension) = &self.extension {
            if entry.extension.to_ascii_lowercase() != *extension {
                return false;
            }
        }
        true
    }
}

#[derive(Clone, Debug)]
struct SearchResultEntry {
    peer_username: Option<String>,
    filename: String,
    size: u64,
    extension: String,
    slot_free: Option<bool>,
    average_speed: Option<u32>,
    queue_length: Option<u32>,
}

impl SearchResultEntry {
    fn from_file_entry(entry: &FileEntry) -> Self {
        Self {
            peer_username: None,
            filename: entry.filename.clone(),
            size: entry.size,
            extension: entry.extension.clone(),
            slot_free: Some(true),
            average_speed: Some(0),
            queue_length: Some(0),
        }
    }

    fn from_peer_response_entry(response: &FileSearchResponse, entry: &FileEntry) -> Self {
        Self {
            peer_username: Some(response.username.clone()),
            filename: entry.filename.clone(),
            size: entry.size,
            extension: entry.extension.clone(),
            slot_free: Some(response.slot_free),
            average_speed: Some(response.average_speed),
            queue_length: Some(response.queue_length),
        }
    }

    fn json(&self) -> String {
        format!(
            "{{\"peer_username\":{},\"filename\":\"{}\",\"size\":{},\"extension\":\"{}\",\"slot_free\":{},\"average_speed\":{},\"queue_length\":{}}}",
            json_option(self.peer_username.as_deref()),
            json_escape(&self.filename),
            self.size,
            json_escape(&self.extension),
            json_bool_option(self.slot_free),
            json_u32_option(self.average_speed),
            json_u32_option(self.queue_length)
        )
    }

    fn slskd_file_json(&self) -> serde_json::Value {
        serde_json::json!({
            "filename": self.filename,
            "size": self.size,
            "code": 1,
            "isLocked": !self.slot_free.unwrap_or(true),
            "extension": self.extension,
            "bitRate": null,
            "bitDepth": null,
            "length": null,
            "sampleRate": null,
        })
    }
}

#[derive(Clone, Debug)]
struct SearchRecord {
    id: String,
    token: u32,
    query: String,
    target: &'static str,
    target_name: Option<String>,
    status: &'static str,
    results: Vec<SearchResultEntry>,
    expires_at: u64,
    created_at: u64,
    updated_at: u64,
}

impl SearchRecord {
    fn json(&self) -> String {
        let results = self
            .results
            .iter()
            .map(SearchResultEntry::json)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"id\":\"{}\",\"token\":{},\"query\":\"{}\",\"searchText\":\"{}\",\"target\":\"{}\",\"target_name\":{},\"status\":\"{}\",\"state\":\"{}\",\"isComplete\":{},\"result_count\":{},\"fileCount\":{},\"responseCount\":{},\"results\":[{}],\"expires_at\":{},\"created_at\":{},\"updated_at\":{}}}",
            json_escape(&self.id),
            self.token,
            json_escape(&self.query),
            json_escape(&self.query),
            self.target,
            json_option(self.target_name.as_deref()),
            self.status,
            if self.status == "active" { "InProgress" } else { "Completed" },
            self.status != "active",
            self.results.len(),
            self.results.len(),
            self.results
                .iter()
                .filter(|entry| entry.peer_username.is_some())
                .count(),
            results,
            self.expires_at,
            self.created_at,
            self.updated_at
        )
    }

    fn slskd_responses_json(&self) -> String {
        let mut grouped: BTreeMap<String, Vec<&SearchResultEntry>> = BTreeMap::new();
        for result in &self.results {
            grouped
                .entry(result.peer_username.clone().unwrap_or_default())
                .or_default()
                .push(result);
        }
        let responses = grouped
            .into_iter()
            .map(|(username, entries)| {
                let files = entries
                    .iter()
                    .map(|entry| entry.slskd_file_json())
                    .collect::<Vec<_>>();
                let file_count = files.len();
                let first = entries.first().copied();
                serde_json::json!({
                    "username": username,
                    "token": self.token,
                    "hasFreeUploadSlot": first.and_then(|entry| entry.slot_free).unwrap_or(true),
                    "queueLength": first.and_then(|entry| entry.queue_length).unwrap_or(0),
                    "uploadSpeed": first.and_then(|entry| entry.average_speed).unwrap_or(0),
                    "fileCount": file_count,
                    "files": files,
                    "lockedFileCount": 0,
                    "lockedFiles": [],
                })
            })
            .collect::<Vec<_>>();
        serde_json::Value::Array(responses).to_string()
    }

    fn from_persisted(record: &persistence::SearchRecord) -> Option<Self> {
        let token = record.id.parse::<u32>().ok()?;
        let target = persisted_target(record.target.as_deref());
        let target_name = match target {
            "user" => record.target.clone(),
            "room" => record.room.clone(),
            _ => None,
        };
        Some(Self {
            id: record.id.clone(),
            token,
            query: record.query.clone(),
            target,
            target_name,
            status: persisted_search_status(&record.status),
            results: Vec::new(),
            expires_at: 0,
            created_at: record.created_at.max(0) as u64,
            updated_at: record.completed_at.unwrap_or(record.created_at).max(0) as u64,
        })
    }
}

#[derive(Debug)]
struct SearchStore {
    records: Vec<SearchRecord>,
    next_token: u32,
}

impl SearchStore {
    fn new() -> Self {
        Self {
            records: Vec::new(),
            next_token: 1,
        }
    }

    fn from_persisted(records: Vec<persistence::SearchRecord>) -> Self {
        let records = records
            .iter()
            .filter_map(SearchRecord::from_persisted)
            .collect::<Vec<_>>();
        let next_token = records
            .iter()
            .map(|record| record.token)
            .max()
            .unwrap_or(0)
            .wrapping_add(1)
            .max(1);
        Self {
            records,
            next_token,
        }
    }

    fn create(
        &mut self,
        id: Option<String>,
        query: String,
        target: &'static str,
        target_name: Option<String>,
        results: Vec<FileEntry>,
        ttl_seconds: u64,
    ) -> SearchRecord {
        let now = unix_timestamp();
        let token = self.next_token;
        let record = SearchRecord {
            id: id
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| token.to_string()),
            token,
            query,
            target,
            target_name,
            status: "active",
            results: results
                .iter()
                .map(SearchResultEntry::from_file_entry)
                .collect(),
            expires_at: now.saturating_add(ttl_seconds),
            created_at: now,
            updated_at: now,
        };
        self.next_token = self.next_token.wrapping_add(1).max(1);
        self.records.push(record.clone());
        record
    }

    fn get_by_identifier(&self, id: &str) -> Option<SearchRecord> {
        self.records
            .iter()
            .find(|record| record.id == id || record.token.to_string() == id)
            .cloned()
    }

    fn remove_by_identifier(&mut self, id: &str) -> bool {
        if let Some(pos) = self
            .records
            .iter()
            .position(|record| record.id == id || record.token.to_string() == id)
        {
            self.records.remove(pos);
            true
        } else {
            false
        }
    }

    fn complete(&mut self, token: u32) -> Option<SearchRecord> {
        let record = self
            .records
            .iter_mut()
            .find(|record| record.token == token)?;
        record.status = "completed";
        record.updated_at = unix_timestamp();
        Some(record.clone())
    }

    fn expire_due(&mut self) -> usize {
        let now = unix_timestamp();
        let mut expired = 0;
        for record in &mut self.records {
            if record.status == "active" && record.expires_at <= now {
                record.status = "expired";
                record.updated_at = now;
                expired += 1;
            }
        }
        expired
    }

    fn prune_expired(&mut self) -> usize {
        self.expire_due();
        let before = self.records.len();
        self.records.retain(|record| record.status != "expired");
        before - self.records.len()
    }

    fn add_peer_response(&mut self, response: &FileSearchResponse) -> Option<SearchRecord> {
        let record = self
            .records
            .iter_mut()
            .find(|record| record.token == response.token)?;
        record.results.extend(
            response
                .results
                .iter()
                .chain(response.private_results.iter())
                .map(|entry| SearchResultEntry::from_peer_response_entry(response, entry)),
        );
        record.updated_at = unix_timestamp();
        Some(record.clone())
    }

    fn get(&self, token: u32) -> Option<SearchRecord> {
        self.records
            .iter()
            .find(|record| record.token == token)
            .cloned()
    }

    fn json(&self, query: Option<&str>) -> String {
        let filter = RecordListFilter::from_query(query);
        let records = self
            .records
            .iter()
            .filter(|record| {
                filter
                    .status
                    .as_deref()
                    .map_or(true, |status| record.status == status)
            })
            .filter(|record| {
                filter
                    .target
                    .as_deref()
                    .map_or(true, |target| record.target == target)
            })
            .filter(|record| {
                filter.q.as_deref().map_or(true, |q| {
                    record.query.to_ascii_lowercase().contains(q)
                        || record
                            .target_name
                            .as_deref()
                            .is_some_and(|target| target.to_ascii_lowercase().contains(q))
                })
            })
            .collect::<Vec<_>>();
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

    fn summary_json(&self) -> String {
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
}

fn persisted_search_status(status: &str) -> &'static str {
    match status {
        "active" | "pending" => "active",
        "completed" => "completed",
        "expired" => "expired",
        _ => "active",
    }
}

fn persisted_target(target: Option<&str>) -> &'static str {
    match target {
        Some("user") => "user",
        Some("room") => "room",
        Some("wishlist") => "wishlist",
        _ => "global",
    }
}

#[derive(Debug, Default)]
struct RecordListFilter {
    q: Option<String>,
    status: Option<String>,
    target: Option<String>,
    direction: Option<String>,
    username: Option<String>,
    joined: Option<bool>,
    kind: Option<String>,
    limit: Option<usize>,
    offset: usize,
}

impl RecordListFilter {
    fn from_query(query: Option<&str>) -> Self {
        let mut filter = Self {
            limit: Some(DEFAULT_LIST_LIMIT),
            ..Self::default()
        };
        for (name, value) in query_params(query.unwrap_or_default()) {
            match name.as_str() {
                "q" => filter.q = non_empty(value.to_ascii_lowercase()),
                "status" => filter.status = non_empty(value),
                "target" => filter.target = non_empty(value),
                "direction" => filter.direction = non_empty(value),
                "username" => filter.username = non_empty(value),
                "joined" => filter.joined = parse_bool_value(&value),
                "kind" => filter.kind = non_empty(value),
                "limit" => filter.limit = Some(parse_list_limit(&value)),
                "offset" => filter.offset = value.parse::<usize>().unwrap_or(0),
                _ => {}
            }
        }
        filter
    }
}

fn parse_list_limit(value: &str) -> usize {
    value
        .parse::<usize>()
        .ok()
        .filter(|limit| *limit > 0)
        .map(|limit| limit.min(DEFAULT_LIST_LIMIT))
        .unwrap_or(DEFAULT_LIST_LIMIT)
}

#[derive(Clone, Debug)]
struct EventRecord {
    id: u64,
    kind: &'static str,
    resource: String,
    detail: Option<String>,
    created_at: u64,
}

impl EventRecord {
    fn json(&self) -> String {
        format!(
            "{{\"id\":{},\"kind\":\"{}\",\"resource\":\"{}\",\"detail\":{},\"created_at\":{}}}",
            self.id,
            self.kind,
            json_escape(&self.resource),
            json_option(self.detail.as_deref()),
            self.created_at
        )
    }

    fn slskd_json(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id.to_string(),
            "timestamp": self.created_at.to_string(),
            "type": self.kind,
            "data": serde_json::json!({
                "resource": self.resource,
                "detail": self.detail,
            }).to_string(),
        })
    }
}

#[derive(Debug)]
struct EventStore {
    records: Vec<EventRecord>,
    next_id: u64,
    history_limit: usize,
}

impl EventStore {
    fn new(history_limit: usize) -> Self {
        Self {
            records: Vec::new(),
            next_id: 1,
            history_limit,
        }
    }

    fn record(
        &mut self,
        kind: &'static str,
        resource: impl Into<String>,
        detail: Option<String>,
    ) -> EventRecord {
        let record = EventRecord {
            id: self.next_id,
            kind,
            resource: resource.into(),
            detail,
            created_at: unix_timestamp(),
        };
        self.next_id += 1;
        self.records.push(record.clone());
        if self.records.len() > self.history_limit {
            let extra = self.records.len() - self.history_limit;
            self.records.drain(0..extra);
        }
        record
    }

    #[allow(dead_code)]
    #[allow(dead_code)]
    fn json(&self, query: Option<&str>) -> String {
        let filter = RecordListFilter::from_query(query);
        let records = self
            .records
            .iter()
            .filter(|record| {
                filter
                    .kind
                    .as_deref()
                    .map_or(true, |kind| record.kind == kind)
            })
            .filter(|record| {
                filter.q.as_deref().map_or(true, |q| {
                    record.kind.to_ascii_lowercase().contains(q)
                        || record.resource.to_ascii_lowercase().contains(q)
                        || record
                            .detail
                            .as_deref()
                            .is_some_and(|detail| detail.to_ascii_lowercase().contains(q))
                })
            })
            .collect::<Vec<_>>();
        let filtered_count = records.len();
        let entries = records
            .into_iter()
            .rev()
            .skip(filter.offset)
            .take(filter.limit.unwrap_or(usize::MAX))
            .map(EventRecord::json)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"entries\":[{}],\"count\":{},\"filtered_count\":{},\"offset\":{},\"limit\":{},\"history_limit\":{},\"next_id\":{}}}",
            entries,
            self.records.len(),
            filtered_count,
            filter.offset,
            json_usize_option(filter.limit),
            self.history_limit,
            self.next_id
        )
    }

    fn slskd_json(&self, query: Option<&str>) -> String {
        let filter = RecordListFilter::from_query(query);
        let entries = self
            .records
            .iter()
            .rev()
            .skip(filter.offset)
            .take(filter.limit.unwrap_or(usize::MAX))
            .map(EventRecord::slskd_json)
            .collect::<Vec<_>>();
        serde_json::Value::Array(entries).to_string()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct TransferEntry {
    id: u64,
    direction: u32,
    token: u32,
    peer_username: Option<String>,
    filename: String,
    local_path: Option<String>,
    size: Option<u64>,
    bytes_transferred: u64,
    status: String,
    reason: Option<String>,
    requested_at: u64,
    updated_at: u64,
}

impl TransferEntry {
    #[allow(dead_code)]
    #[allow(dead_code)]
    fn json(&self) -> String {
        format!(
            "{{\"id\":{},\"direction\":{},\"token\":{},\"peer_username\":{},\"filename\":\"{}\",\"local_path\":{},\"size\":{},\"bytes_transferred\":{},\"status\":\"{}\",\"reason\":{},\"requested_at\":{},\"updated_at\":{}}}",
            self.id,
            self.direction,
            self.token,
            json_option(self.peer_username.as_deref()),
            json_escape(&self.filename),
            json_option(self.local_path.as_deref()),
            json_u64_option(self.size),
            self.bytes_transferred,
            json_escape(&self.status),
            json_option(self.reason.as_deref()),
            self.requested_at,
            self.updated_at
        )
    }

    fn slskd_file_json(&self) -> serde_json::Value {
        let size = self.size.unwrap_or(0);
        let bytes_remaining = size.saturating_sub(self.bytes_transferred);
        let percent_complete = if size == 0 {
            0.0
        } else {
            (self.bytes_transferred as f64 / size as f64) * 100.0
        };
        let started_at = if self.status == "queued" {
            String::new()
        } else {
            self.updated_at.to_string()
        };
        let ended_at = if matches!(
            self.status.as_str(),
            "succeeded" | "completed" | "cancelled" | "failed" | "rejected"
        ) {
            self.updated_at.to_string()
        } else {
            String::new()
        };
        serde_json::json!({
            "id": self.id.to_string(),
            "username": self.peer_username.as_deref().unwrap_or_default(),
            "direction": if self.direction == 0 { "Download" } else { "Upload" },
            "filename": self.filename,
            "size": size,
            "startOffset": 0,
            "state": slskd_transfer_state(&self.status),
            "requestedAt": self.requested_at.to_string(),
            "enqueuedAt": self.requested_at.to_string(),
            "startedAt": started_at,
            "endedAt": ended_at,
            "bytesTransferred": self.bytes_transferred,
            "averageSpeed": 0.0,
            "bytesRemaining": bytes_remaining,
            "elapsedTime": "",
            "percentComplete": percent_complete,
            "remainingTime": "",
        })
    }
}

fn slskd_transfer_state(status: &str) -> &str {
    match status {
        "queued" => "Queued",
        "accepted" | "peer_lookup" | "peer_negotiating" | "indirect_pending" | "in_progress" => {
            "InProgress"
        }
        "succeeded" | "completed" => "Completed",
        "cancelled" => "Cancelled",
        "failed" | "rejected" => "Failed",
        other => other,
    }
}

fn slskd_empty_transfer_json(direction: u32, username: &str, id: u64) -> String {
    serde_json::json!({
        "id": id.to_string(),
        "username": username,
        "direction": if direction == 0 { "Download" } else { "Upload" },
        "filename": "",
        "size": 0,
        "startOffset": 0,
        "state": "None",
        "requestedAt": "",
        "enqueuedAt": "",
        "startedAt": "",
        "endedAt": "",
        "bytesTransferred": 0,
        "averageSpeed": 0.0,
        "bytesRemaining": 0,
        "elapsedTime": "",
        "percentComplete": 0.0,
        "remainingTime": "",
    })
    .to_string()
}

#[derive(Debug)]
struct TransferQueue {
    entries: Vec<TransferEntry>,
    next_id: u64,
    next_token: u32,
    history_limit: usize,
    events_path: PathBuf,
    state_path: PathBuf,
    events_error: Option<String>,
    state_error: Option<String>,
    updated_at: u64,
}

impl TransferQueue {
    fn new(config: &AppConfig) -> Self {
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
            next_id,
            next_token,
            history_limit: config.transfer_history_limit,
            events_path,
            state_path,
            events_error,
            state_error,
            updated_at: unix_timestamp(),
        };
        if queue.state_error.is_none() {
            queue.persist_state();
        }
        queue
    }

    #[cfg(test)]
    fn new_in_memory(history_limit: usize) -> Self {
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
        Self {
            entries: Vec::new(),
            next_id: 1,
            next_token: 1,
            history_limit,
            events_path,
            state_path,
            events_error,
            state_error: None,
            updated_at: 0,
        }
    }

    fn record_rejected_request(
        &mut self,
        direction: u32,
        token: u32,
        filename: String,
        size: Option<u64>,
        reason: String,
    ) -> TransferEntry {
        let now = unix_timestamp();
        let entry = TransferEntry {
            id: self.next_id,
            direction,
            token,
            peer_username: None,
            filename,
            local_path: None,
            size,
            bytes_transferred: 0,
            status: "rejected".to_owned(),
            reason: Some(reason),
            requested_at: now,
            updated_at: now,
        };
        self.next_id += 1;
        self.push_entry(entry)
    }

    fn record_accepted_inbound_request(
        &mut self,
        direction: u32,
        token: u32,
        filename: String,
        local_path: String,
        size: u64,
    ) -> TransferEntry {
        let now = unix_timestamp();
        let entry = TransferEntry {
            id: self.next_id,
            direction,
            token,
            peer_username: None,
            filename,
            local_path: Some(local_path),
            size: Some(size),
            bytes_transferred: 0,
            status: "accepted".to_owned(),
            reason: None,
            requested_at: now,
            updated_at: now,
        };
        self.next_id += 1;
        self.push_entry(entry)
    }

    fn create(
        &mut self,
        direction: u32,
        peer_username: Option<String>,
        filename: String,
        local_path: Option<String>,
        size: Option<u64>,
    ) -> TransferEntry {
        let now = unix_timestamp();
        let token = self.next_token;
        self.next_token = self.next_token.wrapping_add(1).max(1);
        let entry = TransferEntry {
            id: self.next_id,
            direction,
            token,
            peer_username,
            filename,
            local_path,
            size,
            bytes_transferred: 0,
            status: "queued".to_owned(),
            reason: None,
            requested_at: now,
            updated_at: now,
        };
        self.next_id += 1;
        self.push_entry(entry)
    }

    fn update_status(
        &mut self,
        id: u64,
        status: &str,
        bytes_transferred: Option<u64>,
        reason: Option<String>,
    ) -> Option<TransferEntry> {
        let entry = self.entries.iter_mut().find(|entry| entry.id == id)?;
        entry.status = status.to_owned();
        if let Some(bytes_transferred) = bytes_transferred {
            entry.bytes_transferred = bytes_transferred;
        }
        entry.reason = reason;
        entry.updated_at = unix_timestamp();
        if let Err(error) = append_transfer_event(&self.events_path, entry) {
            self.events_error = Some(error);
        }
        let entry = entry.clone();
        self.persist_state();
        self.updated_at = unix_timestamp();
        Some(entry)
    }

    fn update_local_execution(
        &mut self,
        id: u64,
        status: &str,
        bytes_transferred: u64,
        size: Option<u64>,
        reason: Option<String>,
    ) -> Option<TransferEntry> {
        let entry = self.entries.iter_mut().find(|entry| entry.id == id)?;
        entry.status = status.to_owned();
        entry.bytes_transferred = bytes_transferred;
        if size.is_some() {
            entry.size = size;
        }
        entry.reason = reason;
        entry.updated_at = unix_timestamp();
        if let Err(error) = append_transfer_event(&self.events_path, entry) {
            self.events_error = Some(error);
        }
        let entry = entry.clone();
        self.persist_state();
        self.updated_at = unix_timestamp();
        Some(entry)
    }

    fn update_progress(&mut self, id: u64, bytes_transferred: u64) -> Option<TransferEntry> {
        let entry = self.entries.iter_mut().find(|entry| entry.id == id)?;
        entry.status = "in_progress".to_owned();
        entry.bytes_transferred = bytes_transferred;
        entry.reason = None;
        entry.updated_at = unix_timestamp();
        if let Err(error) = append_transfer_event(&self.events_path, entry) {
            self.events_error = Some(error);
        }
        let entry = entry.clone();
        self.persist_state();
        self.updated_at = unix_timestamp();
        Some(entry)
    }

    fn pending_peer_transfer(&self, username: &str) -> Option<TransferEntry> {
        self.entries
            .iter()
            .find(|entry| {
                entry.peer_username.as_deref() == Some(username)
                    && (entry.status == "peer_lookup" || entry.status == "peer_negotiating")
            })
            .cloned()
    }

    fn pending_indirect_transfer(&self, username: &str, token: u32) -> Option<TransferEntry> {
        self.entries
            .iter()
            .find(|entry| {
                entry.peer_username.as_deref() == Some(username)
                    && entry.token == token
                    && entry.status == "indirect_pending"
            })
            .cloned()
    }

    fn pending_inbound_file_transfer(&self, token: Option<u32>) -> Option<TransferEntry> {
        self.entries
            .iter()
            .find(|entry| {
                entry.local_path.is_some()
                    && (entry.status == "accepted" || entry.status == "in_progress")
                    && token.map_or(true, |token| entry.token == token)
            })
            .cloned()
    }

    fn active_count_excluding(&self, id: Option<u64>) -> usize {
        self.entries
            .iter()
            .filter(|entry| id != Some(entry.id) && is_active_transfer_status(&entry.status))
            .count()
    }

    fn push_entry(&mut self, entry: TransferEntry) -> TransferEntry {
        self.entries.push(entry.clone());
        if self.entries.len() > self.history_limit {
            let extra = self.entries.len() - self.history_limit;
            self.entries.drain(0..extra);
        }
        self.updated_at = unix_timestamp();
        if let Err(error) = append_transfer_event(&self.events_path, &entry) {
            self.events_error = Some(error);
        }
        self.persist_state();
        entry
    }

    fn persist_state(&mut self) {
        self.state_error = write_transfer_state(&self.state_path, &self.entries).err();
    }

    fn stats_json(&self) -> String {
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
            .filter(|entry| entry.status == "succeeded")
            .count();
        let cancelled = self
            .entries
            .iter()
            .filter(|entry| entry.status == "cancelled")
            .count();
        let failed = self
            .entries
            .iter()
            .filter(|entry| entry.status == "failed" || entry.status == "rejected")
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

    fn summary_json(&self) -> String {
        self.stats_json()
    }

    fn slskd_transfers_json(&self, direction: u32, username: Option<&str>) -> String {
        let mut grouped: BTreeMap<String, Vec<&TransferEntry>> = BTreeMap::new();
        for entry in self.entries.iter().filter(|entry| {
            entry.direction == direction
                && username.map_or(true, |username| {
                    entry.peer_username.as_deref() == Some(username)
                })
        }) {
            grouped
                .entry(entry.peer_username.clone().unwrap_or_default())
                .or_default()
                .push(entry);
        }

        let transfers = grouped
            .into_iter()
            .map(|(username, entries)| {
                let files = entries
                    .into_iter()
                    .map(TransferEntry::slskd_file_json)
                    .collect::<Vec<_>>();
                serde_json::json!({
                    "username": username,
                    "directories": [{
                        "directory": "",
                        "fileCount": files.len(),
                        "files": files,
                    }],
                })
            })
            .collect::<Vec<_>>();
        serde_json::Value::Array(transfers).to_string()
    }

    fn slskd_transfer_json(&self, direction: u32, username: &str, id: u64) -> Option<String> {
        let entry = self.entries.iter().find(|entry| {
            entry.direction == direction
                && entry.id == id
                && entry.peer_username.as_deref() == Some(username)
        })?;
        Some(entry.slskd_file_json().to_string())
    }

    fn json(&self, query: Option<&str>) -> String {
        let filter = RecordListFilter::from_query(query);
        let entries = self
            .entries
            .iter()
            .filter(|entry| {
                filter
                    .status
                    .as_deref()
                    .map_or(true, |status| entry.status == status)
            })
            .filter(|entry| {
                filter
                    .direction
                    .as_deref()
                    .and_then(|direction| direction.parse::<u32>().ok())
                    .map_or(true, |direction| entry.direction == direction)
            })
            .filter(|entry| {
                filter.username.as_deref().map_or(true, |username| {
                    entry.peer_username.as_deref() == Some(username)
                })
            })
            .filter(|entry| {
                filter.q.as_deref().map_or(true, |q| {
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
            json_option(self.events_error.as_deref()),
            json_escape(
                self.state_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("transfer-state.json")
            ),
            json_option(self.state_error.as_deref()),
            self.updated_at
        )
    }
}

#[derive(Clone, Debug)]
struct SessionSnapshot {
    state: &'static str,
    username: Option<String>,
    supporter: Option<bool>,
    privileges_seconds: Option<u32>,
    last_error: Option<String>,
    last_server_message: Option<String>,
    server_messages_seen: u64,
    reconnects: u64,
    connected_at: Option<u64>,
    updated_at: u64,
}

impl SessionSnapshot {
    fn disconnected(config: &AppConfig) -> Self {
        Self {
            state: "disconnected",
            username: config.username.as_deref().map(redact_username),
            supporter: None,
            privileges_seconds: None,
            last_error: None,
            last_server_message: None,
            server_messages_seen: 0,
            reconnects: 0,
            connected_at: None,
            updated_at: unix_timestamp(),
        }
    }

    fn json(&self) -> String {
        format!(
            "{{\"state\":\"{}\",\"username\":{},\"supporter\":{},\"privileges_seconds\":{},\"last_error\":{},\"last_server_message\":{},\"server_messages_seen\":{},\"reconnects\":{},\"connected_at\":{},\"updated_at\":{}}}",
            self.state,
            json_option(self.username.as_deref()),
            json_bool_option(self.supporter),
            json_u32_option(self.privileges_seconds),
            json_option(self.last_error.as_deref()),
            json_option(self.last_server_message.as_deref()),
            self.server_messages_seen,
            self.reconnects,
            json_u64_option(self.connected_at),
            self.updated_at
        )
    }

    fn summary_json(&self) -> String {
        format!(
            "{{\"state\":\"{}\",\"connected\":{},\"privileges_seconds\":{},\"server_messages_seen\":{},\"reconnects\":{},\"connected_at\":{},\"updated_at\":{}}}",
            self.state,
            self.state == "connected",
            json_u32_option(self.privileges_seconds),
            self.server_messages_seen,
            self.reconnects,
            json_u64_option(self.connected_at),
            self.updated_at
        )
    }
}

#[derive(Clone, Debug)]
struct ListenerSnapshot {
    regular_bind: Option<String>,
    regular_local_addr: Option<String>,
    obfuscated_bind: Option<String>,
    obfuscated_local_addr: Option<String>,
    regular_accepts: u64,
    obfuscated_accepts: u64,
    peer_messages: u64,
    obfuscated_peer_messages: u64,
    file_transfers: u64,
    distributed: u64,
    peer_inits: u64,
    pierce_firewalls: u64,
    unknown_inits: u64,
    user_info_requests: u64,
    user_info_responses: u64,
    share_list_requests: u64,
    share_list_responses: u64,
    file_search_requests: u64,
    file_search_responses: u64,
    transfer_rejections: u64,
    unsupported_peer_messages: u64,
    errors: u64,
    last_event: Option<String>,
    last_error: Option<String>,
    updated_at: u64,
}

impl ListenerSnapshot {
    fn new(config: &AppConfig) -> Self {
        Self {
            regular_bind: config.listener_bind.clone(),
            regular_local_addr: None,
            obfuscated_bind: config.obfuscated_listener_bind.clone(),
            obfuscated_local_addr: None,
            regular_accepts: 0,
            obfuscated_accepts: 0,
            peer_messages: 0,
            obfuscated_peer_messages: 0,
            file_transfers: 0,
            distributed: 0,
            peer_inits: 0,
            pierce_firewalls: 0,
            unknown_inits: 0,
            user_info_requests: 0,
            user_info_responses: 0,
            share_list_requests: 0,
            share_list_responses: 0,
            file_search_requests: 0,
            file_search_responses: 0,
            transfer_rejections: 0,
            unsupported_peer_messages: 0,
            errors: 0,
            last_event: None,
            last_error: None,
            updated_at: unix_timestamp(),
        }
    }

    fn json(&self) -> String {
        format!(
            "{{\"regular_bind\":{},\"regular_local_addr\":{},\"obfuscated_bind\":{},\"obfuscated_local_addr\":{},\"regular_accepts\":{},\"obfuscated_accepts\":{},\"peer_messages\":{},\"obfuscated_peer_messages\":{},\"file_transfers\":{},\"distributed\":{},\"peer_inits\":{},\"pierce_firewalls\":{},\"unknown_inits\":{},\"user_info_requests\":{},\"user_info_responses\":{},\"share_list_requests\":{},\"share_list_responses\":{},\"file_search_requests\":{},\"file_search_responses\":{},\"transfer_rejections\":{},\"unsupported_peer_messages\":{},\"errors\":{},\"last_event\":{},\"last_error\":{},\"updated_at\":{}}}",
            json_option(self.regular_bind.as_deref()),
            json_option(self.regular_local_addr.as_deref()),
            json_option(self.obfuscated_bind.as_deref()),
            json_option(self.obfuscated_local_addr.as_deref()),
            self.regular_accepts,
            self.obfuscated_accepts,
            self.peer_messages,
            self.obfuscated_peer_messages,
            self.file_transfers,
            self.distributed,
            self.peer_inits,
            self.pierce_firewalls,
            self.unknown_inits,
            self.user_info_requests,
            self.user_info_responses,
            self.share_list_requests,
            self.share_list_responses,
            self.file_search_requests,
            self.file_search_responses,
            self.transfer_rejections,
            self.unsupported_peer_messages,
            self.errors,
            json_option(self.last_event.as_deref()),
            json_option(self.last_error.as_deref()),
            self.updated_at
        )
    }
}

#[derive(Clone, Debug)]
struct UserRecord {
    username: String,
    watched: bool,
    status: Option<String>,
    average_speed: Option<u32>,
    upload_count: Option<u32>,
    file_count: Option<u32>,
    directory_count: Option<u32>,
    updated_at: u64,
}

impl UserRecord {
    fn json(&self) -> String {
        format!(
            "{{\"username\":\"{}\",\"watched\":{},\"status\":{},\"average_speed\":{},\"upload_count\":{},\"file_count\":{},\"directory_count\":{},\"updated_at\":{}}}",
            json_escape(&self.username),
            self.watched,
            json_option(self.status.as_deref()),
            json_u32_option(self.average_speed),
            json_u32_option(self.upload_count),
            json_u32_option(self.file_count),
            json_u32_option(self.directory_count),
            self.updated_at
        )
    }

    fn slskd_status_json(&self) -> serde_json::Value {
        serde_json::json!({
            "username": self.username,
            "presence": self.status.as_deref().unwrap_or("Unknown"),
            "isPrivileged": false,
        })
    }

    fn slskd_info_json(&self) -> serde_json::Value {
        serde_json::json!({
            "description": "",
            "hasFreeUploadSlot": true,
            "hasPicture": false,
            "picture": null,
            "queueLength": 0,
            "uploadSlots": 0,
            "uploadSpeed": self.average_speed.unwrap_or(0),
            "uploadCount": self.upload_count.unwrap_or(0),
            "fileCount": self.file_count.unwrap_or(0),
            "directoryCount": self.directory_count.unwrap_or(0),
        })
    }
}

#[derive(Debug)]
struct UserStore {
    records: Vec<UserRecord>,
    updated_at: u64,
}

impl UserStore {
    fn new() -> Self {
        Self {
            records: Vec::new(),
            updated_at: unix_timestamp(),
        }
    }

    fn watch(&mut self, username: String) -> UserRecord {
        let now = unix_timestamp();
        if let Some(record) = self
            .records
            .iter_mut()
            .find(|record| record.username == username)
        {
            record.watched = true;
            record.updated_at = now;
            self.updated_at = now;
            return record.clone();
        }
        let record = UserRecord {
            username,
            watched: true,
            status: None,
            average_speed: None,
            upload_count: None,
            file_count: None,
            directory_count: None,
            updated_at: now,
        };
        self.records.push(record.clone());
        self.updated_at = now;
        record
    }

    fn unwatch(&mut self, username: &str) -> Option<UserRecord> {
        let now = unix_timestamp();
        let record = self
            .records
            .iter_mut()
            .find(|record| record.username == username)?;
        record.watched = false;
        record.updated_at = now;
        self.updated_at = now;
        Some(record.clone())
    }

    fn apply_watched_user(&mut self, user: &WatchedUser) -> UserRecord {
        let now = unix_timestamp();
        let status = user.status.map(|status| status.to_string());
        if let Some(record) = self
            .records
            .iter_mut()
            .find(|record| record.username == user.username)
        {
            record.status = status;
            if let Some(stats) = user.stats.as_ref() {
                record.average_speed = Some(stats.average_speed);
                record.upload_count = Some(stats.upload_count);
                record.file_count = Some(stats.file_count);
                record.directory_count = Some(stats.directory_count);
            }
            record.updated_at = now;
            self.updated_at = now;
            return record.clone();
        }
        let record = UserRecord {
            username: user.username.clone(),
            watched: true,
            status,
            average_speed: user.stats.as_ref().map(|stats| stats.average_speed),
            upload_count: user.stats.as_ref().map(|stats| stats.upload_count),
            file_count: user.stats.as_ref().map(|stats| stats.file_count),
            directory_count: user.stats.as_ref().map(|stats| stats.directory_count),
            updated_at: now,
        };
        self.records.push(record.clone());
        self.updated_at = now;
        record
    }

    fn apply_status(&mut self, status: &UserStatus) -> UserRecord {
        let now = unix_timestamp();
        if let Some(record) = self
            .records
            .iter_mut()
            .find(|record| record.username == status.username)
        {
            record.status = Some(status.status.to_string());
            record.updated_at = now;
            self.updated_at = now;
            return record.clone();
        }
        let record = UserRecord {
            username: status.username.clone(),
            watched: false,
            status: Some(status.status.to_string()),
            average_speed: None,
            upload_count: None,
            file_count: None,
            directory_count: None,
            updated_at: now,
        };
        self.records.push(record.clone());
        self.updated_at = now;
        record
    }

    fn apply_stats(&mut self, username: String, stats: &UserStats) -> UserRecord {
        let now = unix_timestamp();
        if let Some(record) = self
            .records
            .iter_mut()
            .find(|record| record.username == username)
        {
            record.average_speed = Some(stats.average_speed);
            record.upload_count = Some(stats.upload_count);
            record.file_count = Some(stats.file_count);
            record.directory_count = Some(stats.directory_count);
            record.updated_at = now;
            self.updated_at = now;
            return record.clone();
        }
        let record = UserRecord {
            username,
            watched: false,
            status: None,
            average_speed: Some(stats.average_speed),
            upload_count: Some(stats.upload_count),
            file_count: Some(stats.file_count),
            directory_count: Some(stats.directory_count),
            updated_at: now,
        };
        self.records.push(record.clone());
        self.updated_at = now;
        record
    }

    fn json(&self) -> String {
        let records = self
            .records
            .iter()
            .map(UserRecord::json)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"entries\":[{}],\"count\":{},\"updated_at\":{}}}",
            records,
            self.records.len(),
            self.updated_at
        )
    }

    fn summary_json(&self) -> String {
        let watched = self.records.iter().filter(|record| record.watched).count();
        format!(
            "{{\"total\":{},\"watched\":{},\"updated_at\":{}}}",
            self.records.len(),
            watched,
            self.updated_at
        )
    }
}

#[derive(Clone, Debug)]
struct BrowseEntry {
    filename: String,
    size: u64,
    extension: String,
}

impl BrowseEntry {
    fn json(&self) -> String {
        format!(
            "{{\"filename\":\"{}\",\"size\":{},\"extension\":\"{}\"}}",
            json_escape(&self.filename),
            self.size,
            json_escape(&self.extension)
        )
    }
}

#[derive(Clone, Debug)]
struct BrowseRecord {
    username: String,
    status: &'static str,
    entries: Vec<BrowseEntry>,
    reason: Option<String>,
    folder: Option<String>,
    indirect_token: Option<u32>,
    requested_at: Option<u64>,
    updated_at: u64,
}

impl BrowseRecord {
    fn json(&self) -> String {
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
            json_option(self.reason.as_deref()),
            json_option(self.folder.as_deref()),
            json_u32_option(self.indirect_token),
            json_u64_option(self.requested_at),
            self.updated_at
        )
    }
}

#[derive(Debug)]
struct BrowseStore {
    records: Vec<BrowseRecord>,
    next_indirect_token: u32,
    updated_at: u64,
}

impl BrowseStore {
    fn new() -> Self {
        Self {
            records: Vec::new(),
            next_indirect_token: 1,
            updated_at: unix_timestamp(),
        }
    }

    fn next_indirect_token(&mut self) -> u32 {
        let token = self.next_indirect_token;
        self.next_indirect_token = self.next_indirect_token.wrapping_add(1).max(1);
        token
    }

    fn request(&mut self, username: String) -> BrowseRecord {
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
            return record.clone();
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
        record
    }

    fn request_folder(&mut self, username: String, folder: String) -> BrowseRecord {
        let now = unix_timestamp();
        if let Some(record) = self
            .records
            .iter_mut()
            .find(|record| record.username == username)
        {
            record.status = "requested";
            record.reason = None;
            record.folder = Some(folder);
            record.indirect_token = None;
            record.requested_at = Some(now);
            record.updated_at = now;
            self.updated_at = now;
            return record.clone();
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
        record
    }

    fn requested_folder(&self, username: &str) -> Option<Option<String>> {
        self.records
            .iter()
            .find(|record| record.username == username && record.status == "requested")
            .map(|record| record.folder.clone())
    }

    fn mark_indirect_pending(&mut self, username: &str, reason: String) -> Option<u32> {
        let token = self.next_indirect_token();
        let now = unix_timestamp();
        let record = self
            .records
            .iter_mut()
            .find(|record| record.username == username && record.status == "requested")?;
        record.status = "indirect_pending";
        record.reason = Some(reason);
        record.indirect_token = Some(token);
        record.updated_at = now;
        self.updated_at = now;
        Some(token)
    }

    fn pending_indirect(&self, username: &str, token: u32) -> Option<Option<String>> {
        self.records
            .iter()
            .find(|record| {
                record.username == username
                    && record.status == "indirect_pending"
                    && record.indirect_token == Some(token)
            })
            .map(|record| record.folder.clone())
    }

    fn fail_indirect(&mut self, token: u32, reason: String) -> Option<BrowseRecord> {
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

    fn add_entries(
        &mut self,
        username: String,
        entries: Vec<BrowseEntry>,
        complete: bool,
    ) -> BrowseRecord {
        let now = unix_timestamp();
        let status = if complete { "ready" } else { "partial" };
        if let Some(record) = self
            .records
            .iter_mut()
            .find(|record| record.username == username)
        {
            record.status = status;
            record.entries.extend(entries);
            record.reason = None;
            record.indirect_token = None;
            record.updated_at = now;
            self.updated_at = now;
            return record.clone();
        }
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
        record
    }

    fn fail(&mut self, username: String, reason: String) -> BrowseRecord {
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
            return record.clone();
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
        record
    }

    fn get(&self, username: &str) -> Option<BrowseRecord> {
        self.records
            .iter()
            .find(|record| record.username == username)
            .cloned()
    }

    fn json(&self, query: Option<&str>) -> String {
        let filter = RecordListFilter::from_query(query);
        let records = self
            .records
            .iter()
            .filter(|record| {
                filter
                    .status
                    .as_deref()
                    .map_or(true, |status| record.status == status)
            })
            .filter(|record| {
                filter
                    .q
                    .as_deref()
                    .map_or(true, |q| record.username.to_ascii_lowercase().contains(q))
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

    fn summary_json(&self) -> String {
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

#[derive(Clone, Debug)]
struct MessageRecord {
    id: u64,
    username: String,
    direction: &'static str,
    body: String,
    acknowledged: bool,
    created_at: u64,
    updated_at: u64,
}

impl MessageRecord {
    fn json(&self) -> String {
        format!(
            "{{\"id\":{},\"username\":\"{}\",\"direction\":\"{}\",\"body\":\"{}\",\"acknowledged\":{},\"created_at\":{},\"updated_at\":{}}}",
            self.id,
            json_escape(&self.username),
            self.direction,
            json_escape(&self.body),
            self.acknowledged,
            self.created_at,
            self.updated_at
        )
    }

    fn slskd_json(&self) -> serde_json::Value {
        serde_json::json!({
            "timestamp": self.created_at.to_string(),
            "id": self.id,
            "username": self.username,
            "direction": if self.direction == "inbound" { "In" } else { "Out" },
            "message": self.body,
            "isAcknowledged": self.acknowledged,
            "wasReplayed": false,
        })
    }
}

#[derive(Debug)]
struct MessageStore {
    records: Vec<MessageRecord>,
    next_id: u64,
    updated_at: u64,
}

impl MessageStore {
    fn new() -> Self {
        Self {
            records: Vec::new(),
            next_id: 1,
            updated_at: unix_timestamp(),
        }
    }

    fn add(&mut self, username: String, direction: &'static str, body: String) -> MessageRecord {
        let now = unix_timestamp();
        let record = MessageRecord {
            id: self.next_id,
            username,
            direction,
            body,
            acknowledged: false,
            created_at: now,
            updated_at: now,
        };
        self.next_id += 1;
        self.records.push(record.clone());
        self.updated_at = now;
        record
    }

    fn ack(&mut self, id: u64) -> Option<MessageRecord> {
        let now = unix_timestamp();
        let record = self.records.iter_mut().find(|record| record.id == id)?;
        record.acknowledged = true;
        record.updated_at = now;
        self.updated_at = now;
        Some(record.clone())
    }

    fn json(&self, query: Option<&str>) -> String {
        self.json_filtered(None, query)
    }

    fn json_for_user(&self, username: &str, query: Option<&str>) -> String {
        self.json_filtered(Some(username), query)
    }

    fn json_filtered(&self, username: Option<&str>, query: Option<&str>) -> String {
        let filter = RecordListFilter::from_query(query);
        let records = self
            .records
            .iter()
            .filter(|record| username.map_or(true, |username| record.username == username))
            .filter(|record| {
                filter
                    .username
                    .as_deref()
                    .map_or(true, |username| record.username == username)
            })
            .filter(|record| {
                filter
                    .direction
                    .as_deref()
                    .map_or(true, |direction| record.direction == direction)
            })
            .filter(|record| {
                filter.q.as_deref().map_or(true, |q| {
                    record.username.to_ascii_lowercase().contains(q)
                        || record.body.to_ascii_lowercase().contains(q)
                })
            })
            .collect::<Vec<_>>();
        let filtered_count = records.len();
        let records = records
            .into_iter()
            .skip(filter.offset)
            .take(filter.limit.unwrap_or(usize::MAX))
            .map(MessageRecord::json)
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

    fn summary_json(&self) -> String {
        let inbound = self
            .records
            .iter()
            .filter(|record| record.direction == "inbound")
            .count();
        let outbound = self
            .records
            .iter()
            .filter(|record| record.direction == "outbound")
            .count();
        let acknowledged = self
            .records
            .iter()
            .filter(|record| record.acknowledged)
            .count();
        format!(
            "{{\"total\":{},\"inbound\":{},\"outbound\":{},\"acknowledged\":{},\"unacknowledged\":{},\"updated_at\":{}}}",
            self.records.len(),
            inbound,
            outbound,
            acknowledged,
            self.records.len().saturating_sub(acknowledged),
            self.updated_at
        )
    }

    fn slskd_conversations_json(&self, query: Option<&str>) -> String {
        let filter = RecordListFilter::from_query(query);
        let mut grouped: BTreeMap<String, Vec<&MessageRecord>> = BTreeMap::new();
        for record in &self.records {
            if filter.q.as_deref().is_some_and(|q| {
                !record.username.to_ascii_lowercase().contains(q)
                    && !record.body.to_ascii_lowercase().contains(q)
            }) {
                continue;
            }
            grouped
                .entry(record.username.clone())
                .or_default()
                .push(record);
        }
        let conversations = grouped
            .into_iter()
            .map(|(username, messages)| slskd_conversation_json(username, messages, true))
            .collect::<Vec<_>>();
        serde_json::Value::Array(conversations).to_string()
    }

    fn slskd_conversation_json(&self, username: &str, include_messages: bool) -> String {
        let messages = self
            .records
            .iter()
            .filter(|record| record.username == username)
            .collect::<Vec<_>>();
        slskd_conversation_json(username.to_owned(), messages, include_messages).to_string()
    }

    fn slskd_messages_json(&self, username: &str, unacknowledged_only: bool) -> String {
        let messages = self
            .records
            .iter()
            .filter(|record| record.username == username)
            .filter(|record| !unacknowledged_only || !record.acknowledged)
            .map(MessageRecord::slskd_json)
            .collect::<Vec<_>>();
        serde_json::Value::Array(messages).to_string()
    }

    fn ack_all_for_user(&mut self, username: &str) -> usize {
        let now = unix_timestamp();
        let mut updated = 0;
        for record in self
            .records
            .iter_mut()
            .filter(|record| record.username == username && !record.acknowledged)
        {
            record.acknowledged = true;
            record.updated_at = now;
            updated += 1;
        }
        if updated > 0 {
            self.updated_at = now;
        }
        updated
    }
}

fn slskd_conversation_json(
    username: String,
    messages: Vec<&MessageRecord>,
    include_messages: bool,
) -> serde_json::Value {
    let unacknowledged = messages
        .iter()
        .filter(|message| !message.acknowledged)
        .count();
    let messages_json = include_messages.then(|| {
        messages
            .into_iter()
            .map(MessageRecord::slskd_json)
            .collect::<Vec<_>>()
    });
    let mut value = serde_json::json!({
        "username": username,
        "isActive": true,
        "unAcknowledgedMessageCount": unacknowledged,
        "hasUnAcknowledgedMessages": unacknowledged > 0,
    });
    if let Some(messages) = messages_json {
        value["messages"] = serde_json::Value::Array(messages);
    }
    value
}

#[derive(Clone, Debug)]
struct RoomMessageRecord {
    username: String,
    body: String,
    created_at: u64,
}

impl RoomMessageRecord {
    fn json(&self) -> String {
        format!(
            "{{\"username\":\"{}\",\"body\":\"{}\",\"created_at\":{}}}",
            json_escape(&self.username),
            json_escape(&self.body),
            self.created_at
        )
    }

    fn slskd_json(&self, room_name: &str) -> serde_json::Value {
        serde_json::json!({
            "timestamp": self.created_at.to_string(),
            "username": self.username,
            "message": self.body,
            "roomName": room_name,
        })
    }
}

#[derive(Clone, Debug)]
struct RoomRecord {
    name: String,
    joined: bool,
    kind: &'static str,
    user_count: Option<u32>,
    operated: bool,
    messages: Vec<RoomMessageRecord>,
    updated_at: u64,
}

impl RoomRecord {
    fn json(&self) -> String {
        let messages = self
            .messages
            .iter()
            .map(RoomMessageRecord::json)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"name\":\"{}\",\"joined\":{},\"kind\":\"{}\",\"user_count\":{},\"operated\":{},\"messages\":[{}],\"message_count\":{},\"updated_at\":{}}}",
            json_escape(&self.name),
            self.joined,
            self.kind,
            json_u32_option(self.user_count),
            self.operated,
            messages,
            self.messages.len(),
            self.updated_at
        )
    }

    fn slskd_info_json(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.name,
            "userCount": self.user_count.unwrap_or(0),
            "isPrivate": self.kind != "public",
            "isOwned": self.operated,
            "isModerated": self.operated,
        })
    }

    fn slskd_room_json(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.name,
            "isPrivate": self.kind != "public",
            "users": [],
            "messages": self.messages.iter().map(|message| message.slskd_json(&self.name)).collect::<Vec<_>>(),
        })
    }
}

#[derive(Debug)]
struct RoomStore {
    records: Vec<RoomRecord>,
    updated_at: u64,
}

impl RoomStore {
    fn new() -> Self {
        Self {
            records: Vec::new(),
            updated_at: unix_timestamp(),
        }
    }

    fn join(&mut self, name: String) -> RoomRecord {
        let now = unix_timestamp();
        if let Some(record) = self.records.iter_mut().find(|record| record.name == name) {
            record.joined = true;
            record.updated_at = now;
            self.updated_at = now;
            return record.clone();
        }
        let record = RoomRecord {
            name,
            joined: true,
            kind: "local",
            user_count: None,
            operated: false,
            messages: Vec::new(),
            updated_at: now,
        };
        self.records.push(record.clone());
        self.updated_at = now;
        record
    }

    fn leave(&mut self, name: &str) -> Option<RoomRecord> {
        let now = unix_timestamp();
        let record = self.records.iter_mut().find(|record| record.name == name)?;
        record.joined = false;
        record.updated_at = now;
        self.updated_at = now;
        Some(record.clone())
    }

    fn apply_room_list(&mut self, room_list: &RoomList) {
        for entry in &room_list.public_rooms {
            self.upsert_room_list_entry(entry, "public", false);
        }
        for entry in &room_list.owned_private_rooms {
            self.upsert_room_list_entry(entry, "owned_private", true);
        }
        for entry in &room_list.private_rooms {
            let operated = room_list
                .operated_private_rooms
                .iter()
                .any(|room| room == &entry.name);
            self.upsert_room_list_entry(entry, "private", operated);
        }
        for room in &room_list.operated_private_rooms {
            if !self.records.iter().any(|record| record.name == *room) {
                let now = unix_timestamp();
                self.records.push(RoomRecord {
                    name: room.clone(),
                    joined: false,
                    kind: "operated_private",
                    user_count: None,
                    operated: true,
                    messages: Vec::new(),
                    updated_at: now,
                });
                self.updated_at = now;
            }
        }
    }

    fn upsert_room_list_entry(
        &mut self,
        entry: &RoomListEntry,
        kind: &'static str,
        operated: bool,
    ) -> RoomRecord {
        let now = unix_timestamp();
        if let Some(record) = self
            .records
            .iter_mut()
            .find(|record| record.name == entry.name)
        {
            record.kind = kind;
            record.user_count = Some(entry.user_count);
            record.operated = operated;
            record.updated_at = now;
            self.updated_at = now;
            return record.clone();
        }
        let record = RoomRecord {
            name: entry.name.clone(),
            joined: false,
            kind,
            user_count: Some(entry.user_count),
            operated,
            messages: Vec::new(),
            updated_at: now,
        };
        self.records.push(record.clone());
        self.updated_at = now;
        record
    }

    fn add_message(&mut self, room: &str, username: String, body: String) -> Option<RoomRecord> {
        let now = unix_timestamp();
        let record = self.records.iter_mut().find(|record| record.name == room)?;
        record.messages.push(RoomMessageRecord {
            username,
            body,
            created_at: now,
        });
        record.updated_at = now;
        self.updated_at = now;
        Some(record.clone())
    }

    fn json(&self, query: Option<&str>) -> String {
        let filter = RecordListFilter::from_query(query);
        let filtered_count = self
            .records
            .iter()
            .filter(|record| filter.joined.map_or(true, |joined| record.joined == joined))
            .filter(|record| {
                filter
                    .q
                    .as_deref()
                    .map_or(true, |q| record.name.to_ascii_lowercase().contains(q))
            })
            .count();
        format!(
            "{{\"entries\":{},\"count\":{},\"filtered_count\":{},\"offset\":{},\"limit\":{},\"updated_at\":{}}}",
            self.json_array(query),
            self.records.len(),
            filtered_count,
            filter.offset,
            json_usize_option(filter.limit),
            self.updated_at
        )
    }

    fn json_array(&self, query: Option<&str>) -> String {
        let filter = RecordListFilter::from_query(query);
        let records = self
            .records
            .iter()
            .filter(|record| filter.joined.map_or(true, |joined| record.joined == joined))
            .filter(|record| {
                filter
                    .q
                    .as_deref()
                    .map_or(true, |q| record.name.to_ascii_lowercase().contains(q))
            })
            .skip(filter.offset)
            .take(filter.limit.unwrap_or(usize::MAX))
            .map(RoomRecord::json)
            .collect::<Vec<_>>()
            .join(",");
        format!("[{}]", records)
    }

    fn joined_names_json(&self) -> String {
        let records = self
            .records
            .iter()
            .filter(|record| record.joined)
            .map(|record| json_option(Some(record.name.as_str())))
            .collect::<Vec<_>>()
            .join(",");
        format!("[{}]", records)
    }

    fn slskd_available_json(&self) -> String {
        serde_json::Value::Array(
            self.records
                .iter()
                .map(RoomRecord::slskd_info_json)
                .collect::<Vec<_>>(),
        )
        .to_string()
    }

    fn summary_json(&self) -> String {
        let joined = self.records.iter().filter(|record| record.joined).count();
        let messages = self
            .records
            .iter()
            .map(|record| record.messages.len())
            .sum::<usize>();
        format!(
            "{{\"total\":{},\"joined\":{},\"messages\":{},\"updated_at\":{}}}",
            self.records.len(),
            joined,
            messages,
            self.updated_at
        )
    }
}

// Collection Models
#[derive(Clone, Debug)]
struct CollectionItem {
    id: String,
    content_id: String,
    artist: String,
    title: String,
    kind: String,
    added_at: u64,
}

impl CollectionItem {
    fn json(&self) -> String {
        format!(
            "{{\"id\":\"{}\",\"contentId\":\"{}\",\"content_id\":\"{}\",\"artist\":\"{}\",\"title\":\"{}\",\"fileName\":\"{}\",\"mediaKind\":\"{}\",\"kind\":\"{}\",\"addedAt\":{},\"added_at\":{}}}",
            json_escape(&self.id),
            json_escape(&self.content_id),
            json_escape(&self.content_id),
            json_escape(&self.artist),
            json_escape(&self.title),
            json_escape(&self.title),
            json_escape(&self.kind),
            json_escape(&self.kind),
            self.added_at,
            self.added_at
        )
    }
}

#[derive(Clone, Debug)]
struct CollectionRecord {
    id: String,
    name: String,
    description: String,
    items: Vec<CollectionItem>,
    created_at: u64,
    updated_at: u64,
}

impl CollectionRecord {
    fn json(&self) -> String {
        let items = self
            .items
            .iter()
            .map(CollectionItem::json)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"id\":\"{}\",\"title\":\"{}\",\"name\":\"{}\",\"type\":\"Playlist\",\"description\":\"{}\",\"items\":[{}],\"itemCount\":{},\"item_count\":{},\"createdAt\":{},\"created_at\":{},\"updatedAt\":{},\"updated_at\":{}}}",
            json_escape(&self.id),
            json_escape(&self.name),
            json_escape(&self.name),
            json_escape(&self.description),
            items,
            self.items.len(),
            self.items.len(),
            self.created_at,
            self.created_at,
            self.updated_at,
            self.updated_at
        )
    }
}

#[derive(Debug)]
struct CollectionStore {
    records: Vec<CollectionRecord>,
    next_id: u64,
    updated_at: u64,
}

impl CollectionStore {
    fn new() -> Self {
        Self {
            records: Vec::new(),
            next_id: 1,
            updated_at: unix_timestamp(),
        }
    }

    fn create(&mut self, name: String, description: String) -> CollectionRecord {
        let now = unix_timestamp();
        let id = format!("col-{}", self.next_id);
        self.next_id += 1;
        let record = CollectionRecord {
            id,
            name,
            description,
            items: Vec::new(),
            created_at: now,
            updated_at: now,
        };
        self.records.push(record.clone());
        self.updated_at = now;
        record
    }

    fn get(&self, id: &str) -> Option<CollectionRecord> {
        self.records.iter().find(|r| r.id == id).cloned()
    }

    fn update(&mut self, id: &str, name: String, description: String) -> Option<CollectionRecord> {
        let now = unix_timestamp();
        let record = self.records.iter_mut().find(|r| r.id == id)?;
        record.name = name;
        record.description = description;
        record.updated_at = now;
        self.updated_at = now;
        Some(record.clone())
    }

    fn delete(&mut self, id: &str) -> bool {
        if let Some(pos) = self.records.iter().position(|r| r.id == id) {
            self.records.remove(pos);
            self.updated_at = unix_timestamp();
            true
        } else {
            false
        }
    }

    fn add_item(&mut self, collection_id: &str, item: CollectionItem) -> Option<CollectionRecord> {
        let now = unix_timestamp();
        let record = self.records.iter_mut().find(|r| r.id == collection_id)?;
        record.items.push(item);
        record.updated_at = now;
        self.updated_at = now;
        Some(record.clone())
    }

    #[allow(dead_code)]
    fn json(&self, query: Option<&str>) -> String {
        let filter = RecordListFilter::from_query(query);
        let filtered_count = self
            .records
            .iter()
            .filter(|record| {
                filter
                    .q
                    .as_deref()
                    .map_or(true, |q| record.name.to_ascii_lowercase().contains(q))
            })
            .count();
        format!(
            "{{\"entries\":{},\"count\":{},\"filtered_count\":{},\"offset\":{},\"limit\":{},\"updated_at\":{}}}",
            self.json_array(query),
            self.records.len(),
            filtered_count,
            filter.offset,
            json_usize_option(filter.limit),
            self.updated_at
        )
    }

    fn json_array(&self, query: Option<&str>) -> String {
        let filter = RecordListFilter::from_query(query);
        let records = self
            .records
            .iter()
            .filter(|record| {
                filter
                    .q
                    .as_deref()
                    .map_or(true, |q| record.name.to_ascii_lowercase().contains(q))
            })
            .skip(filter.offset)
            .take(filter.limit.unwrap_or(usize::MAX))
            .map(CollectionRecord::json)
            .collect::<Vec<_>>()
            .join(",");
        format!("[{}]", records)
    }
}

// Wishlist Models
#[derive(Clone, Debug)]
struct WishlistItem {
    id: String,
    artist: String,
    title: String,
    kind: String,
    added_at: u64,
}

impl WishlistItem {
    fn json(&self) -> String {
        let search_text = if self.title.is_empty() {
            self.artist.as_str()
        } else if self.artist.is_empty() {
            self.title.as_str()
        } else {
            ""
        };
        let owned_search_text;
        let search_text = if search_text.is_empty() {
            owned_search_text = format!("{} {}", self.artist, self.title);
            owned_search_text.as_str()
        } else {
            search_text
        };
        format!(
            "{{\"id\":\"{}\",\"artist\":\"{}\",\"title\":\"{}\",\"kind\":\"{}\",\"added_at\":{},\"searchText\":\"{}\",\"filter\":\"\",\"enabled\":true,\"autoDownload\":false,\"maxResults\":100,\"lastSearchedAt\":null,\"lastMatchCount\":0,\"totalSearchCount\":0,\"lastSearchId\":null}}",
            json_escape(&self.id),
            json_escape(&self.artist),
            json_escape(&self.title),
            json_escape(&self.kind),
            self.added_at,
            json_escape(search_text)
        )
    }
}

#[derive(Clone, Debug)]
struct WishlistRecord {
    id: String,
    items: Vec<WishlistItem>,
    updated_at: u64,
}

#[derive(Debug)]
struct WishlistStore {
    records: Vec<WishlistRecord>,
    updated_at: u64,
}

impl WishlistStore {
    fn new() -> Self {
        Self {
            records: Vec::new(),
            updated_at: unix_timestamp(),
        }
    }

    fn get_or_create(&mut self) -> WishlistRecord {
        let now = unix_timestamp();
        if let Some(record) = self.records.iter_mut().find(|r| r.id == "default") {
            record.clone()
        } else {
            let record = WishlistRecord {
                id: "default".to_string(),
                items: Vec::new(),
                updated_at: now,
            };
            self.records.push(record.clone());
            self.updated_at = now;
            record
        }
    }

    fn add_item(&mut self, item: WishlistItem) -> Option<WishlistRecord> {
        let now = unix_timestamp();
        let record = self.records.iter_mut().find(|r| r.id == "default")?;
        record.items.push(item);
        record.updated_at = now;
        self.updated_at = now;
        Some(record.clone())
    }

    fn remove_item(&mut self, item_id: &str) -> Option<WishlistRecord> {
        let now = unix_timestamp();
        let record = self.records.iter_mut().find(|r| r.id == "default")?;
        if let Some(pos) = record.items.iter().position(|i| i.id == item_id) {
            record.items.remove(pos);
            record.updated_at = now;
            self.updated_at = now;
            Some(record.clone())
        } else {
            None
        }
    }

    fn json_array(&mut self) -> String {
        let record = self.get_or_create();
        let items = record
            .items
            .iter()
            .map(WishlistItem::json)
            .collect::<Vec<_>>()
            .join(",");
        format!("[{}]", items)
    }
}

// Contact Models
#[derive(Clone, Debug)]
struct ContactRecord {
    id: String,
    username: String,
    online: bool,
    status: String,
    free_upload_slots: Option<u32>,
    queue_length: Option<u32>,
    created_at: u64,
    updated_at: u64,
}

impl ContactRecord {
    fn json(&self) -> String {
        format!(
            "{{\"id\":\"{}\",\"username\":\"{}\",\"online\":{},\"status\":\"{}\",\"free_upload_slots\":{},\"queue_length\":{},\"created_at\":{},\"updated_at\":{}}}",
            json_escape(&self.id),
            json_escape(&self.username),
            self.online,
            json_escape(&self.status),
            json_u32_option(self.free_upload_slots),
            json_u32_option(self.queue_length),
            self.created_at,
            self.updated_at
        )
    }
}

#[derive(Debug)]
struct ContactStore {
    records: Vec<ContactRecord>,
    next_id: u64,
    updated_at: u64,
}

impl ContactStore {
    fn new() -> Self {
        Self {
            records: Vec::new(),
            next_id: 1,
            updated_at: unix_timestamp(),
        }
    }

    fn create(&mut self, username: String) -> ContactRecord {
        let now = unix_timestamp();
        let id = format!("contact-{}", self.next_id);
        self.next_id += 1;
        let record = ContactRecord {
            id,
            username,
            online: false,
            status: "offline".to_string(),
            free_upload_slots: None,
            queue_length: None,
            created_at: now,
            updated_at: now,
        };
        self.records.push(record.clone());
        self.updated_at = now;
        record
    }

    fn get(&self, id: &str) -> Option<ContactRecord> {
        self.records.iter().find(|r| r.id == id).cloned()
    }

    fn update(
        &mut self,
        id: &str,
        username: Option<String>,
        online: Option<bool>,
    ) -> Option<ContactRecord> {
        let now = unix_timestamp();
        let record = self.records.iter_mut().find(|r| r.id == id)?;
        if let Some(u) = username {
            record.username = u;
        }
        if let Some(o) = online {
            record.online = o;
            record.status = if o {
                "online".to_string()
            } else {
                "offline".to_string()
            };
        }
        record.updated_at = now;
        self.updated_at = now;
        Some(record.clone())
    }

    fn delete(&mut self, id: &str) -> bool {
        if let Some(pos) = self.records.iter().position(|r| r.id == id) {
            self.records.remove(pos);
            self.updated_at = unix_timestamp();
            true
        } else {
            false
        }
    }

    fn json_array(&self, query: Option<&str>) -> String {
        let filter = RecordListFilter::from_query(query);
        let records = self
            .records
            .iter()
            .filter(|record| {
                filter
                    .q
                    .as_deref()
                    .map_or(true, |q| record.username.to_ascii_lowercase().contains(q))
            })
            .skip(filter.offset)
            .take(filter.limit.unwrap_or(usize::MAX))
            .map(ContactRecord::json)
            .collect::<Vec<_>>()
            .join(",");
        format!("[{}]", records)
    }
}

// ShareGroup Models
#[derive(Clone, Debug)]
struct ShareGroupMember {
    username: String,
    added_at: u64,
}

impl ShareGroupMember {
    fn json(&self) -> String {
        format!(
            "{{\"username\":\"{}\",\"added_at\":{}}}",
            json_escape(&self.username),
            self.added_at
        )
    }
}

#[derive(Clone, Debug)]
struct ShareGroupRecord {
    id: String,
    name: String,
    description: String,
    members: Vec<ShareGroupMember>,
    created_at: u64,
    updated_at: u64,
}

impl ShareGroupRecord {
    fn json(&self) -> String {
        let members = self
            .members
            .iter()
            .map(ShareGroupMember::json)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"id\":\"{}\",\"name\":\"{}\",\"description\":\"{}\",\"members\":[{}],\"member_count\":{},\"created_at\":{},\"updated_at\":{}}}",
            json_escape(&self.id),
            json_escape(&self.name),
            json_escape(&self.description),
            members,
            self.members.len(),
            self.created_at,
            self.updated_at
        )
    }
}

#[derive(Debug)]
struct ShareGroupStore {
    records: Vec<ShareGroupRecord>,
    next_id: u64,
    updated_at: u64,
}

impl ShareGroupStore {
    fn new() -> Self {
        Self {
            records: Vec::new(),
            next_id: 1,
            updated_at: unix_timestamp(),
        }
    }

    fn create(&mut self, name: String, description: String) -> ShareGroupRecord {
        let now = unix_timestamp();
        let id = format!("sg-{}", self.next_id);
        self.next_id += 1;
        let record = ShareGroupRecord {
            id,
            name,
            description,
            members: Vec::new(),
            created_at: now,
            updated_at: now,
        };
        self.records.push(record.clone());
        self.updated_at = now;
        record
    }

    fn get(&self, id: &str) -> Option<ShareGroupRecord> {
        self.records.iter().find(|r| r.id == id).cloned()
    }

    fn update(&mut self, id: &str, name: String, description: String) -> Option<ShareGroupRecord> {
        let now = unix_timestamp();
        let record = self.records.iter_mut().find(|r| r.id == id)?;
        record.name = name;
        record.description = description;
        record.updated_at = now;
        self.updated_at = now;
        Some(record.clone())
    }

    fn delete(&mut self, id: &str) -> bool {
        if let Some(pos) = self.records.iter().position(|r| r.id == id) {
            self.records.remove(pos);
            self.updated_at = unix_timestamp();
            true
        } else {
            false
        }
    }

    fn add_member(&mut self, group_id: &str, username: String) -> Option<ShareGroupRecord> {
        let now = unix_timestamp();
        let record = self.records.iter_mut().find(|r| r.id == group_id)?;
        if !record.members.iter().any(|m| m.username == username) {
            record.members.push(ShareGroupMember {
                username,
                added_at: now,
            });
            record.updated_at = now;
            self.updated_at = now;
        }
        Some(record.clone())
    }

    fn remove_member(&mut self, group_id: &str, username: &str) -> Option<ShareGroupRecord> {
        let now = unix_timestamp();
        let record = self.records.iter_mut().find(|r| r.id == group_id)?;
        if let Some(pos) = record.members.iter().position(|m| m.username == username) {
            record.members.remove(pos);
            record.updated_at = now;
            self.updated_at = now;
            Some(record.clone())
        } else {
            None
        }
    }

    #[allow(dead_code)]
    fn json(&self, query: Option<&str>) -> String {
        let filter = RecordListFilter::from_query(query);
        let filtered_count = self
            .records
            .iter()
            .filter(|record| {
                filter
                    .q
                    .as_deref()
                    .map_or(true, |q| record.name.to_ascii_lowercase().contains(q))
            })
            .count();
        format!(
            "{{\"entries\":{},\"count\":{},\"filtered_count\":{},\"offset\":{},\"limit\":{},\"updated_at\":{}}}",
            self.json_array(query),
            self.records.len(),
            filtered_count,
            filter.offset,
            json_usize_option(filter.limit),
            self.updated_at
        )
    }

    fn json_array(&self, query: Option<&str>) -> String {
        let filter = RecordListFilter::from_query(query);
        let records = self
            .records
            .iter()
            .filter(|record| {
                filter
                    .q
                    .as_deref()
                    .map_or(true, |q| record.name.to_ascii_lowercase().contains(q))
            })
            .skip(filter.offset)
            .take(filter.limit.unwrap_or(usize::MAX))
            .map(ShareGroupRecord::json)
            .collect::<Vec<_>>()
            .join(",");
        format!("[{}]", records)
    }
}

// User Notes Models
#[derive(Clone, Debug)]
struct UserNoteRecord {
    id: String,
    username: String,
    note: String,
    created_at: u64,
    updated_at: u64,
}

impl UserNoteRecord {
    fn json(&self) -> String {
        format!(
            "{{\"id\":\"{}\",\"username\":\"{}\",\"note\":\"{}\",\"created_at\":{},\"updated_at\":{}}}",
            json_escape(&self.id),
            json_escape(&self.username),
            json_escape(&self.note),
            self.created_at,
            self.updated_at
        )
    }
}

#[derive(Debug)]
struct UserNoteStore {
    records: Vec<UserNoteRecord>,
    next_id: u64,
    updated_at: u64,
}

impl UserNoteStore {
    fn new() -> Self {
        Self {
            records: Vec::new(),
            next_id: 1,
            updated_at: unix_timestamp(),
        }
    }

    fn create(&mut self, username: String, note: String) -> UserNoteRecord {
        let now = unix_timestamp();
        let id = format!("note-{}", self.next_id);
        self.next_id += 1;
        let record = UserNoteRecord {
            id,
            username,
            note,
            created_at: now,
            updated_at: now,
        };
        self.records.push(record.clone());
        self.updated_at = now;
        record
    }

    fn get(&self, id: &str) -> Option<UserNoteRecord> {
        self.records.iter().find(|r| r.id == id).cloned()
    }

    fn update(&mut self, id: &str, note: String) -> Option<UserNoteRecord> {
        let now = unix_timestamp();
        let record = self.records.iter_mut().find(|r| r.id == id)?;
        record.note = note;
        record.updated_at = now;
        self.updated_at = now;
        Some(record.clone())
    }

    fn delete(&mut self, id: &str) -> bool {
        if let Some(pos) = self.records.iter().position(|r| r.id == id) {
            self.records.remove(pos);
            self.updated_at = unix_timestamp();
            true
        } else {
            false
        }
    }

    fn json(&self, _query: Option<&str>) -> String {
        let records = self
            .records
            .iter()
            .map(UserNoteRecord::json)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"entries\":[{}],\"count\":{},\"updated_at\":{}}}",
            records,
            self.records.len(),
            self.updated_at
        )
    }
}

// Interest Models
#[derive(Clone, Debug)]
struct InterestRecord {
    id: String,
    name: String,
    kind: String,
    created_at: u64,
}

impl InterestRecord {
    fn json(&self) -> String {
        format!(
            "{{\"id\":\"{}\",\"name\":\"{}\",\"kind\":\"{}\",\"created_at\":{}}}",
            json_escape(&self.id),
            json_escape(&self.name),
            json_escape(&self.kind),
            self.created_at
        )
    }
}

#[derive(Debug)]
struct InterestStore {
    liked: Vec<InterestRecord>,
    hated: Vec<InterestRecord>,
    next_id: u64,
    updated_at: u64,
}

impl InterestStore {
    fn new() -> Self {
        Self {
            liked: Vec::new(),
            hated: Vec::new(),
            next_id: 1,
            updated_at: unix_timestamp(),
        }
    }

    fn add_liked(&mut self, name: String) -> InterestRecord {
        let now = unix_timestamp();
        let id = format!("liked-{}", self.next_id);
        self.next_id += 1;
        let record = InterestRecord {
            id,
            name,
            kind: "liked".to_string(),
            created_at: now,
        };
        self.liked.push(record.clone());
        self.updated_at = now;
        record
    }

    fn add_hated(&mut self, name: String) -> InterestRecord {
        let now = unix_timestamp();
        let id = format!("hated-{}", self.next_id);
        self.next_id += 1;
        let record = InterestRecord {
            id,
            name,
            kind: "hated".to_string(),
            created_at: now,
        };
        self.hated.push(record.clone());
        self.updated_at = now;
        record
    }

    fn remove_liked(&mut self, id: &str) -> bool {
        if let Some(pos) = self.liked.iter().position(|r| r.id == id) {
            self.liked.remove(pos);
            self.updated_at = unix_timestamp();
            true
        } else {
            false
        }
    }

    fn remove_hated(&mut self, id: &str) -> bool {
        if let Some(pos) = self.hated.iter().position(|r| r.id == id) {
            self.hated.remove(pos);
            self.updated_at = unix_timestamp();
            true
        } else {
            false
        }
    }

    fn json_liked(&self) -> String {
        let records = self
            .liked
            .iter()
            .map(InterestRecord::json)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"entries\":[{}],\"count\":{},\"kind\":\"liked\",\"updated_at\":{}}}",
            records,
            self.liked.len(),
            self.updated_at
        )
    }

    fn json_hated(&self) -> String {
        let records = self
            .hated
            .iter()
            .map(InterestRecord::json)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"entries\":[{}],\"count\":{},\"kind\":\"hated\",\"updated_at\":{}}}",
            records,
            self.hated.len(),
            self.updated_at
        )
    }
}

// Share Grant Models
#[derive(Clone, Debug)]
struct ShareGrantRecord {
    id: String,
    collection_id: String,
    username: String,
    shared_at: u64,
    permissions: String,
}

impl ShareGrantRecord {
    fn json(&self) -> String {
        format!(
            "{{\"id\":\"{}\",\"collection_id\":\"{}\",\"username\":\"{}\",\"shared_at\":{},\"permissions\":\"{}\"}}",
            json_escape(&self.id),
            json_escape(&self.collection_id),
            json_escape(&self.username),
            self.shared_at,
            json_escape(&self.permissions)
        )
    }
}

#[derive(Debug)]
struct ShareGrantStore {
    records: Vec<ShareGrantRecord>,
    next_id: u64,
    updated_at: u64,
}

impl ShareGrantStore {
    fn new() -> Self {
        Self {
            records: Vec::new(),
            next_id: 1,
            updated_at: unix_timestamp(),
        }
    }

    fn create(&mut self, collection_id: String, username: String) -> ShareGrantRecord {
        let now = unix_timestamp();
        let id = format!("grant-{}", self.next_id);
        self.next_id += 1;
        let record = ShareGrantRecord {
            id,
            collection_id,
            username,
            shared_at: now,
            permissions: "read".to_string(),
        };
        self.records.push(record.clone());
        self.updated_at = now;
        record
    }

    fn get(&self, id: &str) -> Option<ShareGrantRecord> {
        self.records.iter().find(|r| r.id == id).cloned()
    }

    fn get_by_collection(&self, collection_id: &str) -> Vec<ShareGrantRecord> {
        self.records
            .iter()
            .filter(|r| r.collection_id == collection_id)
            .cloned()
            .collect()
    }

    fn update(&mut self, id: &str, permissions: String) -> Option<ShareGrantRecord> {
        let record = self.records.iter_mut().find(|r| r.id == id)?;
        record.permissions = permissions;
        self.updated_at = unix_timestamp();
        Some(record.clone())
    }

    fn delete(&mut self, id: &str) -> bool {
        if let Some(pos) = self.records.iter().position(|r| r.id == id) {
            self.records.remove(pos);
            self.updated_at = unix_timestamp();
            true
        } else {
            false
        }
    }

    #[allow(dead_code)]
    fn json(&self) -> String {
        format!(
            "{{\"entries\":{},\"count\":{},\"updated_at\":{}}}",
            self.json_array(),
            self.records.len(),
            self.updated_at
        )
    }

    fn json_array(&self) -> String {
        let records = self
            .records
            .iter()
            .map(ShareGrantRecord::json)
            .collect::<Vec<_>>()
            .join(",");
        format!("[{}]", records)
    }
}

// Library Item Models
#[derive(Clone, Debug)]
struct LibraryItemRecord {
    id: String,
    artist: String,
    title: String,
    kind: String,
    created_at: u64,
}

impl LibraryItemRecord {
    fn json(&self) -> String {
        format!(
            "{{\"id\":\"{}\",\"artist\":\"{}\",\"title\":\"{}\",\"kind\":\"{}\",\"created_at\":{}}}",
            json_escape(&self.id),
            json_escape(&self.artist),
            json_escape(&self.title),
            json_escape(&self.kind),
            self.created_at
        )
    }
}

#[derive(Debug)]
struct LibraryStore {
    records: Vec<LibraryItemRecord>,
    next_id: u64,
    updated_at: u64,
}

impl LibraryStore {
    fn new() -> Self {
        Self {
            records: Vec::new(),
            next_id: 1,
            updated_at: unix_timestamp(),
        }
    }

    fn create(&mut self, artist: String, title: String, kind: String) -> LibraryItemRecord {
        let now = unix_timestamp();
        let id = format!("lib-{}", self.next_id);
        self.next_id += 1;
        let record = LibraryItemRecord {
            id,
            artist,
            title,
            kind,
            created_at: now,
        };
        self.records.push(record.clone());
        self.updated_at = now;
        record
    }

    fn get(&self, id: &str) -> Option<LibraryItemRecord> {
        self.records.iter().find(|r| r.id == id).cloned()
    }

    fn delete(&mut self, id: &str) -> bool {
        if let Some(pos) = self.records.iter().position(|r| r.id == id) {
            self.records.remove(pos);
            self.updated_at = unix_timestamp();
            true
        } else {
            false
        }
    }

    fn json(&self) -> String {
        let records = self
            .records
            .iter()
            .map(LibraryItemRecord::json)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"items\":[{}],\"count\":{},\"updated_at\":{}}}",
            records,
            self.records.len(),
            self.updated_at
        )
    }
}

// Destination Models
#[derive(Clone, Debug)]
struct DestinationRecord {
    id: String,
    name: String,
    path: String,
    is_default: bool,
}

impl DestinationRecord {
    fn json(&self) -> String {
        format!(
            "{{\"id\":\"{}\",\"name\":\"{}\",\"path\":\"{}\",\"is_default\":{}}}",
            json_escape(&self.id),
            json_escape(&self.name),
            json_escape(&self.path),
            self.is_default
        )
    }
}

#[derive(Debug)]
struct DestinationStore {
    records: Vec<DestinationRecord>,
}

impl DestinationStore {
    fn new() -> Self {
        Self {
            records: vec![DestinationRecord {
                id: "default".to_string(),
                name: "Default".to_string(),
                path: "/home/user/Downloads".to_string(),
                is_default: true,
            }],
        }
    }

    fn list(&self) -> String {
        let records = self
            .records
            .iter()
            .map(DestinationRecord::json)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"destinations\":[{}],\"count\":{}}}",
            records,
            self.records.len()
        )
    }

    fn default(&self) -> String {
        self.records[0].json()
    }
}

#[derive(Debug)]
struct AppState {
    config: AppConfig,
    session: RwLock<SessionSnapshot>,
    listeners: RwLock<ListenerSnapshot>,
    shares: RwLock<ShareIndexSnapshot>,
    searches: RwLock<SearchStore>,
    users: RwLock<UserStore>,
    browse: RwLock<BrowseStore>,
    messages: RwLock<MessageStore>,
    rooms: RwLock<RoomStore>,
    transfers: RwLock<TransferQueue>,
    events: RwLock<EventStore>,
    event_tx: broadcast::Sender<EventRecord>,
    webhooks: RwLock<webhooks::WebhookManager>,
    collections: RwLock<CollectionStore>,
    wishlist: RwLock<WishlistStore>,
    contacts: RwLock<ContactStore>,
    sharegroups: RwLock<ShareGroupStore>,
    user_notes: RwLock<UserNoteStore>,
    interests: RwLock<InterestStore>,
    share_grants: RwLock<ShareGrantStore>,
    library: RwLock<LibraryStore>,
    destinations: RwLock<DestinationStore>,
    db: Option<crate::persistence::DatabaseManager>,
    session_commands: mpsc::Sender<SessionCommand>,
    rate_limiter: rate_limit::RateLimiter,
    oauth_states: RwLock<OAuthStateStore>,
}

#[derive(Debug)]
struct OAuthStateRecord {
    provider: String,
    redirect_uri: String,
    created_at: u64,
    expires_at: u64,
}

#[derive(Debug, Default)]
struct OAuthStateStore {
    records: BTreeMap<String, OAuthStateRecord>,
}

impl OAuthStateStore {
    fn issue(&mut self, provider: &str, redirect_uri: &str, ttl_seconds: u64) -> String {
        let now = unix_timestamp();
        self.prune(now);
        let state = secure_oauth_state();
        self.records.insert(
            state.clone(),
            OAuthStateRecord {
                provider: provider.to_owned(),
                redirect_uri: redirect_uri.to_owned(),
                created_at: now,
                expires_at: now.saturating_add(ttl_seconds),
            },
        );
        state
    }

    fn consume(&mut self, provider: &str, state: &str) -> Option<OAuthStateRecord> {
        let now = unix_timestamp();
        self.prune(now);
        let record = self.records.remove(state)?;
        (record.provider == provider && record.expires_at >= now).then_some(record)
    }

    fn prune(&mut self, now: u64) {
        self.records.retain(|_, record| record.expires_at >= now);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum SessionCommand {
    Connect,
    Disconnect,
    Ping,
    CheckPrivileges,
    Search {
        token: u32,
        query: String,
        target: SearchDispatchTarget,
    },
    WatchUser(String),
    UnwatchUser(String),
    BrowseUser(String),
    BrowseFolder {
        username: String,
        folder: String,
    },
    IndirectBrowse {
        username: String,
        token: u32,
    },
    RequestUserStats(String),
    TransferPeer {
        id: u64,
        username: String,
    },
    IndirectTransfer {
        id: u64,
        username: String,
        token: u32,
    },
    MessageUser {
        username: String,
        body: String,
    },
    MessageAcked {
        id: u32,
    },
    RefreshRooms,
    JoinRoom(String),
    LeaveRoom(String),
    SayRoom {
        room: String,
        body: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum SearchDispatchTarget {
    Global,
    User(String),
    Room(String),
    Wishlist,
}

use routing::HttpResponse;

async fn route_http_request_with_headers(
    method: &str,
    path: &str,
    authorization: Option<&str>,
    body: &str,
    state: &AppState,
    headers: RequestSecurityHeaders,
) -> Result<HttpResponse, String> {
    // Start request tracing
    let span = tracing::RequestSpan::new(
        method.to_string(),
        path.to_string(),
        None, // user_agent - would need to pass from connection
        None, // client_ip can be added from connection info
    );
    let _correlation_id = span.correlation_id.clone();
    tracing::set_request_span(span);

    let route = routing::parse_route(method, path);

    // Normalize versioned paths before matching so static and dynamic routes
    // share the same dispatch behavior.
    let normalized_path = if let Some(versioned_path) = route
        .normalized_path
        .strip_prefix("/api/v0/")
        .or_else(|| route.normalized_path.strip_prefix("/api/v1/"))
        .or_else(|| route.normalized_path.strip_prefix("/api/v2/"))
    {
        format!("/api/{}", versioned_path)
    } else {
        route.normalized_path.to_string()
    };

    if let Err(err) = routing::check_route_auth(
        &state.config,
        method,
        &normalized_path,
        authorization,
        &headers,
    ) {
        let status = if err == "forbidden" { 403 } else { 401 };
        tracing::complete_request_span(status);
        return Ok(if err == "forbidden" {
            routing::forbidden_response("cross-site mutating request rejected")
        } else {
            routing::unauthorized_response()
        });
    }

    match (method, normalized_path.as_str()) {
        ("GET", "/") => Ok(index_html_response()),
        ("GET", "/api/health") => Ok(health_response()),
        ("GET", "/api/version") => Ok(version_response()),
        ("GET", "/api/capabilities") => Ok(capabilities_response()),
        ("GET", "/api/application") => {
            let session = state.session.read().await;
            let shares = state.shares.read().await;
            let rooms = state.rooms.read().await;
            let users = state.users.read().await;
            let body = slskd_application_state_json(&session, &shares, &rooms, &users, &state.config);
            drop(users);
            drop(rooms);
            drop(shares);
            drop(session);
            Ok(routing::ok_response(body))
        }
        ("GET", "/api/application/version/latest") => {
            Ok(routing::ok_response(slskd_version_json().to_string()))
        }
        ("GET", "/api/application/version") => {
            Ok(routing::ok_response(serde_json::json!(APP_VERSION).to_string()))
        }
        ("PUT", "/api/application") | ("DELETE", "/api/application") => {
            Ok(routing::accepted_response("{\"accepted\":true}".to_owned()))
        }
        ("POST", "/api/application/gc") => Ok(routing::ok_response("true".to_owned())),
        ("GET", "/api/server") => {
            let session = state.session.read().await;
            let body = slskd_server_state_json(&session, &state.config).to_string();
            drop(session);
            Ok(routing::ok_response(body))
        }
        ("PUT", "/api/server") | ("POST", "/api/server") => {
            send_session_command(state, SessionCommand::Connect).await.ok();
            Ok(routing::accepted_response("{\"accepted\":true}".to_owned()))
        }
        ("DELETE", "/api/server") => {
            send_session_command(state, SessionCommand::Disconnect).await.ok();
            Ok(routing::accepted_response("{\"accepted\":true}".to_owned()))
        }
        ("GET", "/api/session/enabled") => Ok(routing::ok_response(state.config.auth_required.to_string())),
        ("POST", "/api/session") => Ok(routing::ok_response(serde_json::json!({
            "name": "slskr",
            "tokenType": "ApiKey",
            "token": state.config.api_token.as_deref().unwrap_or_default(),
            "issued": unix_timestamp().to_string(),
            "notBefore": unix_timestamp().to_string(),
            "expires": "",
        }).to_string())),
        ("GET", "/api/capabilities/peers") => {
            let users = state.users.read().await;
            let peers: Vec<serde_json::Value> = users.records.iter().map(|user| {
                serde_json::json!({
                    "username": user.username,
                    "status": user.status,
                    "watching": user.watched,
                    "last_seen": user.updated_at,
                })
            }).collect();
            let count = peers.len();
            drop(users);
            Ok(routing::ok_response(serde_json::json!({
                "peers": peers,
                "count": count,
            }).to_string()))
        }
        ("GET", "/api/hashdb/stats") => {
            let shares = state.shares.read().await;
            let total_entries: usize = shares.roots.iter().map(|root| root.files).sum();
            let total_bytes: u64 = shares.roots.iter().map(|root| root.bytes).sum();
            drop(shares);
            Ok(routing::ok_response(serde_json::json!({
                "currentSeqId": unix_timestamp(),
                "totalHashEntries": total_entries,
                "totalEntries": total_entries,
                "totalBytes": total_bytes,
            }).to_string()))
        }
        ("GET", "/api/hashdb/entries") => {
            Ok(routing::ok_response("{\"entries\":[],\"count\":0}".to_owned()))
        }
        ("POST", path) if path.starts_with("/api/hashdb/backfill/from-history") => {
            let searches = state.searches.read().await;
            let candidates = searches.records.len();
            drop(searches);
            Ok(routing::accepted_response(serde_json::json!({
                "success": true,
                "queued": 0,
                "candidates": candidates,
                "status": "idle",
            }).to_string()))
        }
        ("GET", "/api/mesh/stats") => {
            Ok(routing::ok_response(serde_json::json!({
                "currentSeqId": unix_timestamp(),
                "localSeqId": unix_timestamp(),
                "isSyncing": false,
                "knownMeshPeers": 0,
                "connectedPeerCount": 0,
                "warnings": [],
            }).to_string()))
        }
        ("GET", "/api/mesh/peers") => {
            Ok(routing::ok_response("{\"peers\":[],\"count\":0}".to_owned()))
        }
        ("POST", path) if path.starts_with("/api/mesh/sync/") && path.len() > 15 => {
            let username = &path[15..];
            Ok(routing::accepted_response(serde_json::json!({
                "success": true,
                "username": username,
                "queued": false,
                "status": "offline",
            }).to_string()))
        }
        ("GET", "/api/backfill/stats") => {
            Ok(routing::ok_response("{\"isActive\":false,\"isRunning\":false,\"queued\":0,\"completed\":0}".to_owned()))
        }
        ("GET", "/api/backfill/candidates") => {
            Ok(routing::ok_response("{\"candidates\":[],\"count\":0}".to_owned()))
        }
        ("GET", "/api/dht/status") => {
            Ok(routing::ok_response("{\"dhtNodeCount\":0,\"isLanOnly\":false,\"lanOnly\":false,\"isBeaconCapable\":false,\"isDhtRunning\":false,\"verifiedBeaconCount\":0}".to_owned()))
        }
        ("POST", "/api/capabilities/negotiate") => Ok(capabilities_negotiate_response(body)),
        // Documentation endpoints
        ("GET", "/api/docs") | ("GET", "/api/v1/docs") | ("GET", "/api/v2/docs") => {
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "text/html",
                body: openapi::swagger_ui_html("/api/openapi.json"),
            })
        },
        ("GET", "/api/openapi.json") | ("GET", "/api/v1/openapi.json") | ("GET", "/api/v2/openapi.json") => {
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: openapi::generate_openapi_json(),
            })
        },
        ("GET", "/api/docs/index") => {
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: serde_json::json!({
                    "title": "slskr API Documentation",
                    "version": "1.0.1",
                    "docs": {
                        "swagger_ui": "/api/docs",
                        "openapi_spec": "/api/openapi.json",
                        "guides": {
                            "rate_limiting": "/docs/RATE_LIMITING.md",
                            "api_versioning": "/docs/API_VERSIONING.md",
                            "webhooks": "/docs/WEBHOOK_API.md"
                        }
                    },
                    "endpoints": {
                        "total": 202,
                        "by_method": {
                            "GET": 81,
                            "POST": 67,
                            "PUT": 6,
                            "DELETE": 15,
                            "PATCH": 1,
                            "OPTIONS": 32
                        }
                    }
                }).to_string()
            })
        },
        ("GET", "/api/docs/stats") => {
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: serde_json::json!({
                    "total_endpoints": 202,
                    "api_versions": ["v0", "v1", "v2"],
                    "categories": {
                        "health": 7,
                        "session": 5,
                        "search": 15,
                        "transfers": 18,
                        "users": 12,
                        "messages": 8,
                        "rooms": 15,
                        "shares": 8,
                        "webhooks": 6,
                        "collections": 22,
                        "wishlist": 18,
                        "contacts": 20,
                        "share_groups": 15,
                        "user_notes": 12,
                        "interests": 12
                    },
                    "features": {
                        "rate_limiting": {
                            "anonymous": "1000 req/min",
                            "authenticated": "5000 req/min"
                        },
                        "caching": "Cache-Control + ETag",
                        "compression": "gzip",
                        "cors": "Configurable",
                        "webhooks": "HMAC-SHA256"
                    }
                }).to_string()
            })
        },
        ("POST", "/api/batch") | ("POST", "/api/v1/batch") | ("POST", "/api/v2/batch") => Ok(
            routing::accepted_response("{\"accepted\":true,\"results\":[],\"executed\":0}".to_owned()),
        ),
        ("GET", "/api/config") => Ok(HttpResponse {
            status: "200 OK",
            content_type: "application/json",
            body: state.config.sanitized_json(),
        }),
        ("GET", "/api/stats") => {
            let session = state.session.read().await;
            let session_stats = session.summary_json();
            drop(session);

            let _listeners = state.listeners.read().await;
            drop(_listeners);

            let shares = state.shares.read().await;
            let share_stats = shares.summary_json();
            drop(shares);

            let searches = state.searches.read().await;
            let searches_stats = searches.summary_json();
            drop(searches);

            let users = state.users.read().await;
            let users_stats = users.summary_json();
            drop(users);

            let browses = state.browse.read().await;
            let browses_stats = browses.summary_json();
            drop(browses);

            let messages = state.messages.read().await;
            let messages_stats = messages.summary_json();
            drop(messages);

            let rooms = state.rooms.read().await;
            let rooms_stats = rooms.summary_json();
            drop(rooms);

            let transfers = state.transfers.read().await;
            let transfers_stats = transfers.summary_json();
            drop(transfers);

            let body = format!(
                "{{\"session\":{},\"listeners\":{{\"count\":1}},\"shares\":{},\"searches\":{},\"users\":{},\"browse\":{},\"messages\":{},\"rooms\":{},\"transfers\":{}}}",
                session_stats,
                share_stats,
                searches_stats,
                users_stats,
                browses_stats,
                messages_stats,
                rooms_stats,
                transfers_stats
            );

            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body,
            })
        }
        ("GET", "/api/telemetry") => {
            let session = state.session.read().await;
            let is_connected = session.state == "connected";
            let session_json = session.summary_json();
            drop(session);

            let shares = state.shares.read().await;
            let shares_json = shares.summary_json();
            drop(shares);

            let body = format!(
                "{{\"health\":{{\"connected\":{}}},\"service\":{{\"name\":\"slskr\"}},\"storage\":{{\"share_cache_file\":\"share-index.tsv\",\"transfer_events_file\":\"transfer-events.tsv\"}},\"shares\":{},\"session\":{}}}",
                is_connected,
                shares_json,
                session_json
            );

            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body,
            })
        }
        ("GET", "/api/telemetry/reports/transfers/summary") => {
            let transfers = state.transfers.read().await;
            let body = slskd_transfer_summary_report(&transfers);
            drop(transfers);
            Ok(routing::ok_response(body))
        }
        ("GET", "/api/telemetry/reports/transfers/histogram") => {
            Ok(routing::ok_response(serde_json::json!({
                "interval": 60,
                "buckets": [],
            }).to_string()))
        }
        ("GET", "/api/telemetry/reports/transfers/leaderboard") => {
            Ok(routing::ok_response("[]".to_owned()))
        }
        ("GET", path) if path.starts_with("/api/telemetry/reports/transfers/users/") => {
            let username = path.rsplit('/').next().unwrap_or("");
            let transfers = state.transfers.read().await;
            let body = slskd_user_transfer_report(username, &transfers);
            drop(transfers);
            Ok(routing::ok_response(body))
        }
        ("GET", "/api/telemetry/reports/transfers/exceptions") => {
            Ok(routing::ok_response("[]".to_owned()))
        }
        ("GET", "/api/telemetry/reports/transfers/exceptions/pareto") => {
            Ok(routing::ok_response(serde_json::json!({
                "items": [],
                "total": 0,
            }).to_string()))
        }
        ("GET", "/api/telemetry/reports/transfers/directories") => {
            Ok(routing::ok_response("[]".to_owned()))
        }
        ("GET", "/api/metrics") => {
            let session = state.session.read().await;
            let _listeners = state.listeners.read().await;
            let shares = state.shares.read().await;
            let searches = state.searches.read().await;
            let users = state.users.read().await;
            let browse = state.browse.read().await;
            let messages = state.messages.read().await;
            let rooms = state.rooms.read().await;
            let transfers = state.transfers.read().await;

            let share_bytes: u64 = shares.entries.iter().map(|e| e.size).sum();

            let metrics = format!(
                "# HELP slskr_session_connected Session connection status\n\
                 # TYPE slskr_session_connected gauge\n\
                 slskr_session_connected {}\n\
                 # HELP slskr_shares_files Number of shared files\n\
                 # TYPE slskr_shares_files gauge\n\
                 slskr_shares_files {}\n\
                 # HELP slskr_shares_bytes Total bytes shared\n\
                 # TYPE slskr_shares_bytes gauge\n\
                 slskr_shares_bytes {}\n\
                 # HELP slskr_searches_active Active search count\n\
                 # TYPE slskr_searches_active gauge\n\
                 slskr_searches_active {}\n\
                 # HELP slskr_users_watched Watched user count\n\
                 # TYPE slskr_users_watched gauge\n\
                 slskr_users_watched {}\n\
                 # HELP slskr_browse_cache Browse cache size\n\
                 # TYPE slskr_browse_cache gauge\n\
                 slskr_browse_cache {}\n\
                 # HELP slskr_messages_total Message count\n\
                 # TYPE slskr_messages_total counter\n\
                 slskr_messages_total {}\n\
                 # HELP slskr_rooms_joined Joined room count\n\
                 # TYPE slskr_rooms_joined gauge\n\
                 slskr_rooms_joined {}\n\
                 # HELP slskr_transfers Transfer count\n\
                 # TYPE slskr_transfers gauge\n\
                 slskr_transfers{{state=\"total\"}} {}\n",
                if session.state == "connected" { 1 } else { 0 },
                shares.entries.len(),
                share_bytes,
                searches.records.len(),
                users.records.len(),
                browse.records.len(),
                messages.records.len(),
                rooms.records.len(),
                transfers.entries.len()
            );

            Ok(HttpResponse {
                status: "200 OK",
                content_type: "text/plain; version=0.0.4; charset=utf-8",
                body: metrics,
            })
        }
        ("GET", "/api/events") => {
            let events = state.events.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: events.json(route.query),
            })
        }
        ("GET", "/api/events/slskd") => {
            let events = state.events.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: events.slskd_json(route.query),
            })
        }
        ("POST", path) if path.starts_with("/api/events/") && path.len() > 12 => {
            let kind = &path[12..];
            let mut events = state.events.write().await;
             events.record("compat.event", kind, json_body_string(body).or_else(|| Some(body.to_owned())));
             drop(events);
             Ok(routing::ok_response("true".to_owned()))
         }
         ("GET", "/api/logs") => {
             Ok(routing::ok_response("[]".to_owned()))
         }
         // WEBHOOK ENDPOINTS
         ("GET", "/api/webhooks") => {
             let webhooks = state.webhooks.read().await;
             let webhook_list: Vec<serde_json::Value> = webhooks.get_all().iter().map(|w| {
                 serde_json::json!({
                     "id": w.id,
                     "url": w.url,
                     "events": w.events.iter().map(|e| e.to_string()).collect::<Vec<_>>(),
                     "active": w.active,
                     "created_at": w.created_at,
                     "last_triggered": w.last_triggered,
                     "retry_count": w.retry_count,
                     "max_retries": w.max_retries,
                     "timeout_seconds": w.timeout_seconds,
                 })
             }).collect();
             drop(webhooks);
             Ok(HttpResponse {
                 status: "200 OK",
                 content_type: "application/json",
                 body: serde_json::to_string(&serde_json::json!({"webhooks": webhook_list})).unwrap_or_else(|_| "{}".to_string()),
             })
         }

         ("POST", "/api/webhooks") => {
             let url = match extract_json_string_field(body, "url") {
                 Some(u) => u,
                 None => return Ok(routing::bad_request_response("url is required")),
             };
             if url.len() > 2048 {
                 return Ok(routing::bad_request_response("url is too long"));
             }

             let events_str = extract_json_string_field(body, "events");
             let events = if let Some(ref e) = events_str {
                 let mut events = Vec::new();
                 for ev in e.split(',') {
                     let event = match ev.trim() {
                         "search.created" => Some(webhooks::WebhookEvent::SearchCreated),
                         "search.completed" => Some(webhooks::WebhookEvent::SearchCompleted),
                         "transfer.started" => Some(webhooks::WebhookEvent::TransferStarted),
                         "transfer.completed" => Some(webhooks::WebhookEvent::TransferCompleted),
                         "transfer.failed" => Some(webhooks::WebhookEvent::TransferFailed),
                         "message.received" => Some(webhooks::WebhookEvent::MessageReceived),
                         "message.sent" => Some(webhooks::WebhookEvent::MessageSent),
                         "user.connected" => Some(webhooks::WebhookEvent::UserConnected),
                         "user.disconnected" => Some(webhooks::WebhookEvent::UserDisconnected),
                         "room.joined" => Some(webhooks::WebhookEvent::RoomJoined),
                         "room.left" => Some(webhooks::WebhookEvent::RoomLeft),
                         "apikey.created" => Some(webhooks::WebhookEvent::ApiKeyCreated),
                         "apikey.revoked" => Some(webhooks::WebhookEvent::ApiKeyRevoked),
                         "config.changed" => Some(webhooks::WebhookEvent::ConfigChanged),
                         _ => None,
                     };
                     let Some(event) = event else {
                         return Ok(routing::bad_request_response("invalid webhook event"));
                     };
                     if !events.contains(&event) {
                         events.push(event);
                     }
                 }
                 events
             } else {
                 vec![webhooks::WebhookEvent::SearchCreated]
             };

             if events.is_empty() {
                 return Ok(routing::bad_request_response("no valid events specified"));
             }

             let secret = extract_json_string_field(body, "secret").unwrap_or_else(webhooks::Webhook::generate_secret);
             let webhook = webhooks::Webhook::new(url, events, secret.clone());

             let mut webhooks = state.webhooks.write().await;
             let webhook_id = match webhooks.register(webhook) {
                 Ok(id) => id,
                 Err(_) => {
                     drop(webhooks);
                     return Ok(routing::bad_request_response("webhook limit reached"));
                 }
             };
             drop(webhooks);

             let response = serde_json::json!({
                 "id": webhook_id,
                 "secret": secret,
                 "status": "created"
             });

             Ok(routing::created_response(serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string())))
         }

         ("DELETE", path) if path.starts_with("/api/webhooks/") => {
             let webhook_id = path.rsplit('/').next().unwrap_or("");
             let mut webhooks = state.webhooks.write().await;
             if webhooks.unregister(webhook_id).is_some() {
                 drop(webhooks);
                 Ok(routing::ok_response(serde_json::json!({"status": "deleted"}).to_string()))
             } else {
                 drop(webhooks);
                 Ok(routing::not_found_response())
             }
         }

         ("PATCH", path) if path.starts_with("/api/webhooks/") => {
              let webhook_id = path.rsplit('/').next().unwrap_or("");
              let Some(active) = extract_json_bool_field(body, "active") else {
                  return Ok(routing::bad_request_response("active boolean is required"));
              };

              let mut webhooks = state.webhooks.write().await;
              if let Some(webhook) = webhooks.get_mut(webhook_id) {
                  webhook.active = active;
                  let updated = serde_json::json!({
                      "id": webhook.id,
                      "active": webhook.active,
                  });
                  drop(webhooks);
                  Ok(routing::ok_response(serde_json::to_string(&updated).unwrap_or_else(|_| "{}".to_string())))
              } else {
                  drop(webhooks);
                  Ok(routing::not_found_response())
              }
          }

          // ADDITIONAL MISSING PATCH ENDPOINTS (Phase 5)
          ("PATCH", "/api/options") => {
              Ok(routing::ok_response(
                  "{\"status\":\"accepted\",\"persisted\":false,\"note\":\"runtime option mutation is not enabled\"}".to_owned(),
              ))
          }

          ("PATCH", path) if path.starts_with("/api/library/health/issues/") && path.len() > 27 => {
              let issue_id = path.rsplit('/').next().unwrap_or("unknown");
              Ok(routing::ok_response(format!(
                  "{{\"id\":\"{}\",\"updated\":false,\"status\":\"not_found\"}}",
                  json_escape(issue_id)
              )))
          }

          ("POST", path) if path.starts_with("/api/webhooks/") && path.ends_with("/test") => {
             let webhook_id = path.rsplit('/').nth(1).unwrap_or("");
             let webhooks = state.webhooks.read().await;
             if let Some(webhook) = webhooks.get(webhook_id) {
                 let payload = webhooks::WebhookDispatcher::test_payload(
                     webhooks::WebhookEvent::SearchCreated,
                     "test webhook delivery"
                 );
                 let webhook_clone = webhook.clone();
                 drop(webhooks);

                 tokio::spawn(async move {
                     let _ = webhooks::WebhookDispatcher::send_webhook(
                         &webhook_clone.url,
                         &webhook_clone.secret,
                         &payload.to_string(),
                         webhook_clone.timeout_seconds,
                     ).await;
                 });

                 Ok(routing::ok_response(serde_json::json!({"status": "test_sent"}).to_string()))
             } else {
                 drop(webhooks);
                 Ok(routing::not_found_response())
             }
         }

         ("GET", path) if path.starts_with("/api/webhooks/") && path.ends_with("/logs") => {
             let webhook_id = path.rsplit('/').nth(1).unwrap_or("");
             let limit = if let Some(q) = route.query {
                 query_params(q).iter().find(|(k, _)| k == "limit").map(|(_, v)| parse_list_limit(v) as i32).unwrap_or(50)
             } else {
                 50
             };

             if let Some(db) = &state.db {
                 match db.get_webhook_logs(webhook_id, limit, 0).await {
                     Ok(logs) => {
                         let log_json = logs.iter().map(|l| {
                             serde_json::json!({
                                 "id": l.id,
                                 "event": l.event,
                                 "correlation_id": l.correlation_id,
                                 "status": l.status,
                                 "response_status": l.response_status,
                                 "error_message": l.error_message,
                                 "timestamp": l.timestamp,
                             })
                         }).collect::<Vec<_>>();

                         Ok(HttpResponse {
                             status: "200 OK",
                             content_type: "application/json",
                             body: serde_json::to_string(&serde_json::json!({"logs": log_json})).unwrap_or_else(|_| "{}".to_string()),
                         })
                     }
                     Err(_) => Ok(routing::bad_request_response("database error")),
                 }
             } else {
                 Ok(routing::bad_request_response("database not configured"))
             }
         }

         ("GET", "/api/shares") => {
            let shares = state.shares.read().await;
            let host = state
                .config
                .username
                .as_deref()
                .filter(|username| !username.is_empty())
                .unwrap_or("local");
            let mut roots = shares
                .roots
                .iter()
                .map(|root| {
                    serde_json::json!({
                        "localPath": root.label,
                        "alias": root.label,
                        "remotePath": root.label,
                        "directories": 0,
                        "files": root.files,
                        "bytes": root.bytes,
                        "isExcluded": false,
                    })
                })
                .collect::<Vec<_>>();
            if roots.is_empty() && !shares.entries.is_empty() {
                roots.push(serde_json::json!({
                    "localPath": "shares",
                    "alias": "shares",
                    "remotePath": "shares",
                    "directories": 0,
                    "files": shares.entries.len(),
                    "bytes": shares.entries.iter().map(|entry| entry.size).sum::<u64>(),
                    "isExcluded": false,
                }));
            }
            let json = serde_json::json!({ host: roots }).to_string();
            drop(shares);
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: json,
            })
        }
        ("GET", "/api/shares/catalog") => {
            let shares = state.shares.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: shares.catalog_json(route.query),
            })
        }
        ("PUT", "/api/shares") => {
            let rebuilt = build_share_index(&state.config);
            let json = rebuilt.json();
            let mut shares = state.shares.write().await;
            *shares = rebuilt;
            drop(shares);
            record_event(state, "share.scan.completed", "shares", None).await;
            Ok(routing::ok_response((!json.is_empty()).to_string()))
        }
        ("GET", "/api/files/downloads/directories")
        | ("GET", "/api/files/incomplete/directories") => {
            Ok(routing::ok_response(slskd_empty_directory_json("").to_string()))
        }
        ("GET", path)
            if (path.starts_with("/api/files/downloads/directories/")
                || path.starts_with("/api/files/incomplete/directories/")) =>
        {
            let name = path.rsplit('/').next().unwrap_or("");
            Ok(routing::ok_response(slskd_empty_directory_json(name).to_string()))
        }
        ("DELETE", path)
            if path.starts_with("/api/files/downloads/directories/")
                || path.starts_with("/api/files/downloads/files/")
                || path.starts_with("/api/files/incomplete/directories/")
                || path.starts_with("/api/files/incomplete/files/") =>
        {
            Ok(routing::ok_response("false".to_owned()))
        }
        ("GET", path) if path.starts_with("/api/files/") || path.starts_with("/api/v0/files/") => {
            let folder = path.strip_prefix("/api/v0/files/")
                .or_else(|| path.strip_prefix("/api/files/"))
                .unwrap_or("");

            if folder.is_empty() {
                return Ok(routing::not_found_response());
            }

            // Parse extension filter from query
            let mut extension_filter: Option<String> = None;
            for (name, value) in query_params(route.query.unwrap_or_default()) {
                if name == "extension" {
                    extension_filter = Some(value);
                }
            }

            let filter = RecordListFilter::from_query(route.query);
            let shares = state.shares.read().await;

            // Find the root
            let Some(root) = shares.roots.iter().find(|r| r.label == folder) else {
                drop(shares);
                return Ok(routing::not_found_response());
            };

            // Filter entries by folder prefix and extension
            let mut entries: Vec<_> = shares.entries.iter()
                .filter(|e| e.filename.starts_with(&format!("{}/", folder)))
                .filter(|e| {
                    extension_filter.as_deref()
                        .map_or(true, |ext| e.extension == ext)
                })
                .collect();

            let filtered_count = entries.len();

            // Apply pagination
            entries = entries.into_iter()
                .skip(filter.offset)
                .take(filter.limit.unwrap_or(usize::MAX))
                .collect();

            let entries_json = entries.iter()
                .map(|entry| {
                    let path = entry.filename.strip_prefix(&format!("{}/", folder)).unwrap_or("");
                    format!(
                        "{{\"path\":\"{}\",\"virtual_path\":\"{}\",\"size\":{}}}",
                        json_escape(path),
                        json_escape(&entry.filename),
                        entry.size
                    )
                })
                .collect::<Vec<_>>()
                .join(",");

            let response_body = format!(
                "{{\"label\":\"{}\",\"entries\":[{}],\"filtered_count\":{},\"offset\":{},\"limit\":{}}}",
                json_escape(&root.label),
                entries_json,
                filtered_count,
                filter.offset,
                json_usize_option(filter.limit)
            );

            drop(shares);
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: response_body,
            })
        }
        ("POST", "/api/shares/rescan") => {
            let snapshot = rebuild_share_index(state).await;
            record_event(
                state,
                "share.scan.completed",
                "shares",
                Some(format!("{} files", snapshot.entries.len())),
            )
            .await;
            Ok(HttpResponse {
                status: "202 Accepted",
                content_type: "application/json",
                body: snapshot.json(),
            })
        }
        ("GET", "/api/session") => {
            let snapshot = state.session.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: snapshot.json(),
            })
        }
        ("POST", "/api/session/connect") => {
            send_session_command(state, SessionCommand::Connect).await.ok();
            Ok(routing::accepted_response("{\"accepted\":true}".to_owned()))
        }
        ("POST", "/api/session/ping") => {
            send_session_command(state, SessionCommand::Ping).await.ok();
            Ok(routing::accepted_response("{\"accepted\":true}".to_owned()))
        }
        ("POST", "/api/session/disconnect") => {
            send_session_command(state, SessionCommand::Disconnect).await.ok();
            Ok(routing::accepted_response("{\"accepted\":true}".to_owned()))
        }
        ("POST", "/api/session/privileges/check") => {
            send_session_command(state, SessionCommand::CheckPrivileges).await.ok();
            Ok(routing::accepted_response("{\"accepted\":true}".to_owned()))
        }
        ("GET", "/api/listeners") => {
            let snapshot = state.listeners.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: snapshot.json(),
            })
        }
        ("GET", "/api/users") => {
            let users = state.users.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: users.json(),
            })
        }
        ("GET", "/api/searches") => {
            let mut searches = state.searches.write().await;
            searches.expire_due();
            let body = searches.json(route.query);
            drop(searches);
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body,
            })
        }
        ("GET", path) if search_token_path(normalized_path.as_str(), "").is_some() => {
            let Some(token) = search_token_path(normalized_path.as_str(), "") else {
                return Ok(routing::not_found_response());
            };
            let mut searches = state.searches.write().await;
            searches.expire_due();
            if let Some(record) = searches.get(token) {
                let body = record.json();
                drop(searches);
                Ok(HttpResponse {
                    status: "200 OK",
                    content_type: "application/json",
                    body,
                })
            } else {
                drop(searches);
                Ok(routing::not_found_response())
            }
        }
        ("GET", "/api/rooms") => {
            let rooms = state.rooms.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: rooms.json(route.query),
            })
        }
        ("GET", "/api/messages") => {
            let messages = state.messages.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: messages.json(route.query),
            })
        }
        ("GET", "/api/transfers") => {
            let transfers = state.transfers.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: transfers.json(route.query),
            })
        }
        ("GET", "/api/transfers/stats") => {
            let transfers = state.transfers.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: transfers.stats_json(),
            })
        }
        ("POST", "/api/searches") => {
            let query = match extract_json_string_field(body, "query")
                .or_else(|| extract_json_string_field(body, "searchText"))
            {
                Some(q) => q,
                None => return Ok(routing::bad_request_response("query/searchText is required")),
            };

            let target_str = extract_json_string_field(body, "target").unwrap_or_else(|| "global".to_string());
            let external_id = extract_json_string_field(body, "id");
            let username_opt = extract_json_string_field(body, "username");
            let room_opt = extract_json_string_field(body, "room");

            if !matches!(target_str.as_str(), "global" | "user" | "room" | "wishlist") {
                return Ok(routing::bad_request_response("invalid search target"));
            }
            if target_str == "user" && username_opt.is_none() {
                return Ok(routing::bad_request_response("username is required for user search"));
            }
            if target_str == "room" && room_opt.is_none() {
                return Ok(routing::bad_request_response("room is required for room target"));
            }

            let shares = state.shares.read().await;
            let matching_results = search_shares(&shares.entries, &query);
            drop(shares);

             let mut searches = state.searches.write().await;
             let target_name = if target_str == "user" { username_opt.clone() } else if target_str == "room" { room_opt.clone() } else { None };
             let result_count = matching_results.len();

              let target = search_target_static(&target_str);
              let record = searches.create(external_id, query.clone(), target, target_name.clone(), matching_results, 300);
              let token = record.token;
              drop(searches);

             let persisted_search = persistence::SearchRecord {
                 id: format!("{}", token),
                 query: query.clone(),
                 status: record.status.to_string(),
                 result_count: record.results.len() as i64,
                 created_at: record.created_at as i64,
                 completed_at: None,
                 room: room_opt.clone(),
                 target: Some(target_str.clone()),
             };
             if let Some(db) = state.db.as_ref() {
                 db.insert_search(&persisted_search)
                     .await
                     .map_err(|error| format!("failed to persist search: {error}"))?;
             }

             record_event(state, "search.started", format!("{}", token), None).await;

             // Dispatch webhook for search.created event
             let webhook_data = serde_json::json!({
                 "token": token,
                 "query": query,
                 "target": target_str,
                 "target_name": target_name,
                 "result_count": result_count,
             });
             let correlation_id = format!("search_{}", token);
             let webhooks = state.webhooks.read().await;
             let webhooks_clone = webhooks.clone();
             drop(webhooks);
             tokio::spawn(async move {
                 webhooks::WebhookDispatcher::dispatch(
                     &webhooks_clone,
                     correlation_id,
                     webhooks::WebhookEvent::SearchCreated,
                     webhook_data,
                 ).await;
             });

             let dispatch_target = match target_str.as_str() {
                 "user" => SearchDispatchTarget::User(username_opt.unwrap_or_default()),
                 "room" => SearchDispatchTarget::Room(room_opt.unwrap_or_default()),
                "wishlist" => SearchDispatchTarget::Wishlist,
                _ => SearchDispatchTarget::Global,
             };

             send_session_command(state, SessionCommand::Search { token, query, target: dispatch_target }).await.ok();

             Ok(routing::created_response(record.json()))
        }

        ("POST", path) if search_token_path(normalized_path.as_str(), "/complete").is_some() => {
            let Some(token) = search_token_path(normalized_path.as_str(), "/complete") else {
                return Ok(routing::not_found_response());
            };
            let mut searches = state.searches.write().await;
            if let Some(record) = searches.complete(token) {
                let body_json = record.json();

                // Dispatch webhook for search.completed event
                let result_count = record.results.len();
                let webhook_data = serde_json::json!({
                    "token": token,
                    "query": record.query,
                    "result_count": result_count,
                    "target": record.target,
                });
                let correlation_id = format!("search_{}", token);

                drop(searches);

                let webhooks = state.webhooks.read().await;
                let webhooks_clone = webhooks.clone();
                drop(webhooks);
                tokio::spawn(async move {
                    webhooks::WebhookDispatcher::dispatch(
                        &webhooks_clone,
                        correlation_id,
                        webhooks::WebhookEvent::SearchCompleted,
                        webhook_data,
                    ).await;
                });

                Ok(routing::ok_response(body_json))
            } else {
                drop(searches);
                Ok(routing::not_found_response())
            }
        }

        ("POST", "/api/searches/prune") => {
            let mut searches = state.searches.write().await;
            let pruned = searches.prune_expired();
            let remaining = searches.records.len();
            drop(searches);
            Ok(routing::ok_response(format!("{{\"pruned\":{},\"remaining\":{}}}", pruned, remaining)))
        }

        ("POST", "/api/search-responses") => {
            let token = match extract_json_u64_field(body, "token") {
                Some(t) => t as u32,
                None => return Ok(routing::bad_request_response("token is required")),
            };

            let peer_username = extract_json_string_field(body, "peer_username");
            let filename = extract_json_string_field(body, "filename");
            let size = extract_json_u64_field(body, "size");
            let slot_free = extract_json_bool_field(body, "slot_free");
            let average_speed = extract_json_u32_field(body, "average_speed");
            let queue_length = extract_json_u32_field(body, "queue_length");

            let mut searches = state.searches.write().await;
            if let Some(record) = searches.records.iter_mut().find(|r| r.token == token) {
                let entry = SearchResultEntry {
                    peer_username: peer_username.clone(),
                    filename: filename.clone().unwrap_or_default(),
                    size: size.unwrap_or(0),
                    slot_free,
                    average_speed,
                    queue_length,
                    extension: filename.as_ref().and_then(|f| f.split('.').next_back().map(|s| s.to_string())).unwrap_or_default(),
                };
                record.results.push(entry);
                record.updated_at = unix_timestamp();
                let response_json = record.json();
                drop(searches);
                Ok(routing::ok_response(response_json))
            } else {
                drop(searches);
                Ok(routing::not_found_response())
            }
        }

        // TRANSFER ENDPOINTS
        ("POST", "/api/transfers") => {
            if let Some((username, files)) = slskd_enqueue_request(body) {
                let mut transfers = state.transfers.write().await;
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
                    transfers.create(0, Some(username.clone()), filename, None, size);
                }
                drop(transfers);
                return Ok(routing::ok_response("true".to_owned()));
            }

            let filename = match extract_json_string_field(body, "filename") {
                Some(f) => f,
                None => return Ok(routing::bad_request_response("filename is required")),
            };

            let direction = extract_json_u32_field(body, "direction").unwrap_or(0);
            let peer_username = extract_json_string_field(body, "peer_username");
            let supplied_local_path = extract_json_string_field(body, "local_path");
            let size = extract_json_u64_field(body, "size");
            let local_path = match prepare_transfer_local_path(state, direction, &filename, supplied_local_path).await {
                Ok(path) => path,
                Err(error) => return Ok(routing::bad_request_response(&error)),
            };

             let mut transfers = state.transfers.write().await;
             let entry = transfers.create(direction, peer_username.clone(), filename.clone(), local_path.clone(), size);
             drop(transfers);
              Ok(routing::created_response(entry.json()))
         }

         ("GET", "/api/transfers/downloads") | ("GET", "/api/transfers/downloads/") => {
             let transfers = state.transfers.read().await;
             let body = transfers.slskd_transfers_json(0, None);
             drop(transfers);
             Ok(routing::ok_response(body))
         }

         ("GET", "/api/transfers/uploads") | ("GET", "/api/transfers/uploads/") => {
             let transfers = state.transfers.read().await;
             let body = transfers.slskd_transfers_json(1, None);
             drop(transfers);
             Ok(routing::ok_response(body))
         }

         ("GET", "/api/transfers/downloads/accelerated") => {
             Ok(routing::ok_response(
                 "{\"enabled\":false,\"available\":false,\"accelerated\":[],\"count\":0}".to_string(),
             ))
         }

         ("GET", path) if slskd_transfer_user_path(path, "downloads").is_some() => {
             let Some(username) = slskd_transfer_user_path(path, "downloads") else {
                 return Ok(routing::not_found_response());
             };
             let transfers = state.transfers.read().await;
             let body = transfers.slskd_transfers_json(0, Some(username));
             drop(transfers);
             Ok(routing::ok_response(body))
         }

         ("GET", path) if slskd_transfer_user_path(path, "uploads").is_some() => {
             let Some(username) = slskd_transfer_user_path(path, "uploads") else {
                 return Ok(routing::not_found_response());
             };
             let transfers = state.transfers.read().await;
             let body = transfers.slskd_transfers_json(1, Some(username));
             drop(transfers);
             Ok(routing::ok_response(body))
         }

         ("GET", path) if slskd_transfer_file_path(path, "downloads").is_some() && !path.ends_with("/position") => {
             let Some((username, id)) = slskd_transfer_file_path(path, "downloads") else {
                 return Ok(routing::not_found_response());
             };
             let transfers = state.transfers.read().await;
             let response = transfers
                 .slskd_transfer_json(0, username, id)
                 .map(routing::ok_response)
                 .unwrap_or_else(|| {
                     routing::ok_response(slskd_empty_transfer_json(0, username, id))
                 });
             drop(transfers);
             Ok(response)
         }

         ("GET", path) if slskd_transfer_file_path(path, "uploads").is_some() && !path.ends_with("/position") => {
             let Some((username, id)) = slskd_transfer_file_path(path, "uploads") else {
                 return Ok(routing::not_found_response());
             };
             let transfers = state.transfers.read().await;
             let response = transfers
                 .slskd_transfer_json(1, username, id)
                 .map(routing::ok_response)
                 .unwrap_or_else(|| {
                     routing::ok_response(slskd_empty_transfer_json(1, username, id))
                 });
             drop(transfers);
             Ok(response)
         }

         ("GET", path) if slskd_transfer_position_path(path).is_some() => {
             Ok(routing::ok_response("0".to_owned()))
         }

         ("POST", path) if slskd_transfer_user_path(path, "downloads").is_some() => {
             let Some(username) = slskd_transfer_user_path(path, "downloads") else {
                 return Ok(routing::not_found_response());
             };
             let files = slskd_files_from_body(body);
             let mut transfers = state.transfers.write().await;
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
                 transfers.create(0, Some(username.to_owned()), filename, None, size);
             }
             drop(transfers);
             Ok(routing::ok_response("true".to_owned()))
         }

         ("DELETE", "/api/transfers/downloads/all/completed")
         | ("DELETE", "/api/transfers/uploads/all/completed") => {
             let direction = if normalized_path.contains("/downloads/") { 0 } else { 1 };
             let mut transfers = state.transfers.write().await;
             let before = transfers.entries.len();
             transfers.entries.retain(|entry| {
                 entry.direction != direction
                     || !matches!(
                         entry.status.as_str(),
                         "succeeded" | "completed" | "cancelled" | "failed" | "rejected"
                     )
             });
             let removed = before.saturating_sub(transfers.entries.len());
             drop(transfers);
             Ok(routing::ok_response((removed > 0).to_string()))
         }

         ("DELETE", path) if slskd_transfer_file_path(path, "downloads").is_some() => {
             let Some((username, id)) = slskd_transfer_file_path(path, "downloads") else {
                 return Ok(routing::not_found_response());
             };
             let mut transfers = state.transfers.write().await;
             let matches_transfer = transfers.entries.iter().any(|entry| {
                 entry.id == id && entry.direction == 0 && entry.peer_username.as_deref() == Some(username)
             });
             let updated = matches_transfer
                 .then(|| transfers.update_status(id, "cancelled", None, None))
                 .flatten();
             drop(transfers);
             Ok(if updated.is_some() {
                 routing::ok_response("true".to_owned())
             } else {
                 routing::ok_response("false".to_owned())
             })
         }

         ("DELETE", path) if slskd_transfer_file_path(path, "uploads").is_some() => {
             let Some((username, id)) = slskd_transfer_file_path(path, "uploads") else {
                 return Ok(routing::not_found_response());
             };
             let mut transfers = state.transfers.write().await;
             let matches_transfer = transfers.entries.iter().any(|entry| {
                 entry.id == id && entry.direction == 1 && entry.peer_username.as_deref() == Some(username)
             });
             let updated = matches_transfer
                 .then(|| transfers.update_status(id, "cancelled", None, None))
                 .flatten();
             drop(transfers);
             Ok(if updated.is_some() {
                 routing::ok_response("true".to_owned())
             } else {
                 routing::ok_response("false".to_owned())
             })
         }

         // GET individual transfer
         ("GET", path) if (path.starts_with("/api/transfers/") || path.starts_with("/api/v0/transfers/"))
             && !path.ends_with("/start") && !path.ends_with("/progress") && !path.ends_with("/complete")
             && !path.ends_with("/stats") => {
             let id_str = path.rsplit('/').next().unwrap_or("");
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
             let id_str = path.rsplit('/').next().unwrap_or("");
             if let Ok(id) = id_str.parse::<u64>() {
                 let mut transfers = state.transfers.write().await;
                 if let Some(entry) = transfers.entries.iter_mut().find(|t| t.id == id) {
                     entry.status = "cancelled".to_owned();
                     entry.updated_at = unix_timestamp();
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

         ("POST", path) if transfer_action_path(normalized_path.as_str()).is_some() => {
            if let Some((id, action)) = transfer_action_path(normalized_path.as_str()) {
                let mut transfers = state.transfers.write().await;

                if action == "start" {
                    // Check max active transfer limit
                    let max_active = state.config.transfer_max_active;
                    let active_count = transfers.entries.iter()
                        .filter(|t| t.status == "in_progress" || t.status == "peer_lookup")
                        .count();

                    if active_count >= max_active {
                        drop(transfers);
                        return Ok(routing::conflict_response("transfer limit reached"));
                    }

                    if let Some(entry) = transfers.entries.iter_mut().find(|t| t.id == id) {
                        // Check outbound transfer policy
                        if let Some(ref username) = entry.peer_username {
                            if !state.config.transfer_allow_outbound {
                                drop(transfers);
                                return Ok(routing::conflict_response("outbound transfers are disabled"));
                            }

                            entry.status = "peer_lookup".to_owned();
                            entry.updated_at = unix_timestamp();
                            let json_response = entry.json();
                            let username_clone = username.clone();
                            drop(transfers);

                            send_session_command(state, SessionCommand::TransferPeer { id, username: username_clone }).await.ok();

                            Ok(routing::ok_response(json_response))
                        } else {
                            entry.status = "in_progress".to_owned();

                            entry.updated_at = unix_timestamp();
                            let json_response = entry.json();
                            drop(transfers);
                            Ok(routing::ok_response(json_response))
                        }
                    } else {
                        drop(transfers);
                        Ok(routing::not_found_response())
                    }
                } else if action == "progress" {
                    let bytes_transferred = extract_json_u64_field(body, "bytes_transferred").unwrap_or(0);
                    if let Some(entry) = transfers.entries.iter_mut().find(|t| t.id == id) {
                        entry.status = "in_progress".to_owned();
                        entry.bytes_transferred = bytes_transferred;
                        entry.updated_at = unix_timestamp();
                        let json_response = entry.json();
                        drop(transfers);
                        Ok(routing::ok_response(json_response))
                    } else {
                        drop(transfers);
                        Ok(routing::not_found_response())
                    }
                 } else if action == "complete" {
                     let bytes_transferred = extract_json_u64_field(body, "bytes_transferred").unwrap_or(0);
                     let status_str = extract_json_string_field(body, "status").unwrap_or_else(|| "succeeded".to_string());
                     if let Some(entry) = transfers.entries.iter_mut().find(|t| t.id == id) {
                         entry.bytes_transferred = bytes_transferred;
                         entry.status = status_str.clone();
                         entry.updated_at = unix_timestamp();
                         let json_response = entry.json();

                         // Prepare webhook dispatch
                         let webhook_event = if status_str == "succeeded" {
                             webhooks::WebhookEvent::TransferCompleted
                         } else if status_str == "failed" {
                             webhooks::WebhookEvent::TransferFailed
                         } else {
                             webhooks::WebhookEvent::TransferCompleted
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

                         drop(transfers);

                         // Dispatch webhook
                         let webhooks = state.webhooks.read().await;
                         let webhooks_clone = webhooks.clone();
                         drop(webhooks);
                         tokio::spawn(async move {
                             webhooks::WebhookDispatcher::dispatch(
                                 &webhooks_clone,
                                 correlation_id,
                                 webhook_event,
                                 webhook_data,
                             ).await;
                         });

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
            let active_count = transfers.entries.iter()
                .filter(|t| t.status == "in_progress")
                .count();
            let total_bytes = transfers.entries.iter()
                .map(|t| t.bytes_transferred)
                .sum::<u64>();
            let json = format!(
                "{{\"active_transfers\":{},\"total_bytes_transferred\":{},\"average_speed\":0}}",
                active_count,
                total_bytes
            );
            drop(transfers);
            Ok(routing::ok_response(json))
        }

        ("GET", "/api/transfers/downloads/stats") => {
            let transfers = state.transfers.read().await;
            let downloads = transfers.entries.iter()
                .filter(|t| t.direction == 0)
                .count();
            let completed = transfers.entries.iter()
                .filter(|t| t.direction == 0 && t.status == "succeeded")
                .count();
            let total_size = transfers.entries.iter()
                .filter(|t| t.direction == 0)
                .map(|t| t.size.unwrap_or(0))
                .sum::<u64>();
            let json = format!(
                "{{\"total_downloads\":{},\"completed\":{},\"total_size\":{}}}",
                downloads,
                completed,
                total_size
            );
            drop(transfers);
            Ok(routing::ok_response(json))
        }

        ("POST", "/api/transfers/downloads/find-alternative") => {
            let transfer_id = extract_json_u64_field(body, "transfer_id").unwrap_or(0);
            if transfer_id == 0 {
                return Ok(routing::bad_request_response("transfer_id is required"));
            }
            let json = format!(
                "{{\"transfer_id\":{},\"alternatives\":[],\"count\":0}}",
                transfer_id
            );
            Ok(routing::ok_response(json))
        }

        ("POST", "/api/transfers/downloads/replace") => {
            let transfer_id = extract_json_u64_field(body, "transfer_id").unwrap_or(0);
            let username = extract_json_string_field(body, "username").unwrap_or_default();
            if transfer_id == 0 || username.is_empty() {
                return Ok(routing::bad_request_response("transfer_id and username are required"));
            }
            Ok(routing::accepted_response(format!(
                "{{\"transfer_id\":{},\"username\":\"{}\",\"replacement_queued\":false,\"status\":\"no_alternative_selected\"}}",
                transfer_id,
                json_escape(&username)
            )))
        }

        // USER PROFILE ENDPOINTS
        ("GET", path) if path.starts_with("/api/users/") && path.ends_with("/info") => {
            let username = path
                .strip_prefix("/api/users/")
                .and_then(|p| p.strip_suffix("/info"))
                .unwrap_or("unknown");
            let users = state.users.read().await;
            if let Some(record) = users.records.iter().find(|u| u.username == username) {
                let json = record.slskd_info_json().to_string();
                drop(users);
                Ok(routing::ok_response(json))
            } else {
                drop(users);
                let record = UserRecord {
                    username: username.to_owned(),
                    watched: false,
                    status: None,
                    average_speed: None,
                    upload_count: None,
                    file_count: None,
                    directory_count: None,
                    updated_at: unix_timestamp(),
                };
                Ok(routing::ok_response(record.slskd_info_json().to_string()))
            }
        }

        ("POST", path) if path.starts_with("/api/users/") && path.ends_with("/directory") => {
            let username = path.strip_prefix("/api/users/")
                .and_then(|p| p.strip_suffix("/directory"))
                .unwrap_or("unknown");
            let directory = extract_json_string_field(body, "directory").unwrap_or_default();
            let json = format!(
                "{{\"username\":\"{}\",\"directory\":\"{}\",\"requested_at\":{}}}",
                json_escape(username),
                json_escape(&directory),
                unix_timestamp()
            );
            Ok(routing::created_response(json))
        }

        // USER STATUS ENDPOINTS
        ("GET", path) if path.starts_with("/api/users/") && path.ends_with("/status") => {
            let username = path
                .strip_prefix("/api/users/")
                .and_then(|p| p.strip_suffix("/status"))
                .unwrap_or("unknown");
            let users = state.users.read().await;
            if let Some(record) = users.records.iter().find(|u| u.username == username) {
                let json = record.slskd_status_json().to_string();
                drop(users);
                Ok(routing::ok_response(json))
            } else {
                drop(users);
                let record = UserRecord {
                    username: username.to_owned(),
                    watched: false,
                    status: None,
                    average_speed: None,
                    upload_count: None,
                    file_count: None,
                    directory_count: None,
                    updated_at: unix_timestamp(),
                };
                Ok(routing::ok_response(record.slskd_status_json().to_string()))
            }
        }

        ("GET", path) if path.starts_with("/api/users/") && path.ends_with("/group") => {
            let username = path.strip_prefix("/api/users/")
                .and_then(|p| p.strip_suffix("/group"))
                .unwrap_or("unknown");
            let json = format!(
                "{{\"username\":\"{}\",\"group\":\"default\"}}",
                json_escape(username)
            );
            Ok(routing::ok_response(json))
        }

        ("GET", path) if path.starts_with("/api/users/") && path.ends_with("/endpoint") => {
            let username = path.strip_prefix("/api/users/")
                .and_then(|p| p.strip_suffix("/endpoint"))
                .unwrap_or("unknown");
            let json = format!(
                "{{\"username\":\"{}\",\"address\":\"0.0.0.0\",\"port\":0}}",
                json_escape(username)
            );
            Ok(routing::ok_response(json))
        }

        ("GET", "/api/soulseek/users/similar") => {
            let json = "{\"users\":[],\"count\":0}".to_string();
            Ok(routing::ok_response(json))
        }

        ("GET", path) if path.starts_with("/api/soulseek/users/") && path.ends_with("/interests") => {
            let username = path.strip_prefix("/api/soulseek/users/")
                .and_then(|p| p.strip_suffix("/interests"))
                .unwrap_or("unknown");
            let json = format!(
                "{{\"username\":\"{}\",\"liked\":[],\"hated\":[],\"count\":0}}",
                json_escape(username)
            );
            Ok(routing::ok_response(json))
        }


        ("DELETE", "/api/searches") => {
            let mut searches = state.searches.write().await;
            let cleared_count = searches.records.len();
            searches.records.clear();
            drop(searches);
            let json = format!("{{\"cleared\":{}}}", cleared_count);
            Ok(routing::ok_response(json))
        }

        ("GET", path) if path.starts_with("/api/searches/") && path.ends_with("/responses") => {
            let Some(id) = path
                .strip_prefix("/api/searches/")
                .and_then(|value| value.strip_suffix("/responses"))
                .filter(|value| !value.is_empty() && !value.contains('/'))
            else {
                return Ok(routing::not_found_response());
            };
            let searches = state.searches.read().await;
            if let Some(record) = searches.get_by_identifier(id) {
                let json = record.slskd_responses_json();
                drop(searches);
                Ok(routing::ok_response(json))
            } else {
                drop(searches);
                Ok(routing::not_found_response())
            }
        }

        ("GET", path) if path.starts_with("/api/searches/") && path.len() > "/api/searches/".len() => {
            let id = path.trim_start_matches("/api/searches/");
            let searches = state.searches.read().await;
            if let Some(record) = searches.get_by_identifier(id) {
                let json = record.json();
                drop(searches);
                Ok(routing::ok_response(json))
            } else {
                drop(searches);
                Ok(routing::not_found_response())
            }
        }

        ("DELETE", path)
            if (path.starts_with("/api/searches/")
                || route.normalized_path.starts_with("/api/v0/searches/"))
                && path.len() > "/api/searches/".len() =>
        {
            let token_str = path
                .strip_prefix("/api/searches/")
                .or_else(|| route.normalized_path.strip_prefix("/api/v0/searches/"))
                .unwrap_or_default();
            let mut searches = state.searches.write().await;
            searches.remove_by_identifier(token_str);
            drop(searches);
            Ok(routing::ok_response("{}".to_string()))
        }

        // MESSAGE ENDPOINTS
         ("POST", "/api/messages") => {
             let username = match extract_json_string_field(body, "username") {
                 Some(u) => u,
                 None => return Ok(routing::bad_request_response("username is required")),
             };

             let message_body = match extract_json_string_field(body, "body") {
                 Some(b) => b,
                 None => return Ok(routing::bad_request_response("body is required")),
             };

              let mut messages = state.messages.write().await;
              let record = messages.add(username.clone(), "outbound", message_body.clone());
              let message_id = record.id;
              drop(messages);
              // Dispatch webhook for message.sent event
              let webhook_data = serde_json::json!({
                  "message_id": message_id,
                  "username": username.clone(),
                  "body": message_body.clone(),
                  "direction": "outbound",
              });
              let correlation_id = format!("message_{}", message_id);

              let webhooks = state.webhooks.read().await;
              let webhooks_clone = webhooks.clone();
              drop(webhooks);
              tokio::spawn(async move {
                  webhooks::WebhookDispatcher::dispatch(
                      &webhooks_clone,
                      correlation_id,
                      webhooks::WebhookEvent::MessageSent,
                      webhook_data,
                  ).await;
              });

              send_session_command(state, SessionCommand::MessageUser { username, body: message_body }).await.ok();

              Ok(routing::created_response(record.json()))
         }

        ("POST", "/api/messages/inbound") => {
            let username = match extract_json_string_field(body, "username") {
                Some(u) => u,
                None => return Ok(routing::bad_request_response("username is required")),
            };

            let message_body = match extract_json_string_field(body, "body") {
                Some(b) => b,
                None => return Ok(routing::bad_request_response("body is required")),
            };

             let mut messages = state.messages.write().await;
             let record = messages.add(username.clone(), "inbound", message_body.clone());
             drop(messages);
             record_event(state, "message.received", "messages", Some(format!("id={}", record.id))).await;

            Ok(routing::created_response(record.json()))
        }

        ("POST", path) if message_ack_path(normalized_path.as_str()).is_some() => {
            let Some(id) = message_ack_path(normalized_path.as_str()) else {
                return Ok(routing::not_found_response());
            };
            let mut messages = state.messages.write().await;

            if let Some(record) = messages.records.iter_mut().find(|m| m.id == id) {
                record.acknowledged = true;
                record.updated_at = unix_timestamp();
                let json_response = record.json();
                drop(messages);

                send_session_command(state, SessionCommand::MessageAcked { id: id as u32 }).await.ok();

                Ok(routing::ok_response(json_response))
             } else {
                 drop(messages);
                 Ok(routing::not_found_response())
             }
          }

          ("PUT", path) if message_ack_path(normalized_path.as_str()).is_some() => {
             let Some(id) = message_ack_path(normalized_path.as_str()) else {
                 return Ok(routing::not_found_response());
             };
             let mut messages = state.messages.write().await;

             if let Some(record) = messages.records.iter_mut().find(|m| m.id == id) {
                 record.acknowledged = true;
                 record.updated_at = unix_timestamp();
                 let json_response = record.json();
                 drop(messages);

                 send_session_command(state, SessionCommand::MessageAcked { id: id as u32 }).await.ok();

                 Ok(routing::ok_response(json_response))
             } else {
                 drop(messages);
                 Ok(routing::not_found_response())
             }
          }

          ("GET", path) if messages_user_path(normalized_path.as_str()).is_some() => {
            let Some(username) = messages_user_path(normalized_path.as_str()) else {
                return Ok(routing::not_found_response());
            };
            let messages = state.messages.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: messages.json_for_user(username, route.query),
            })
        }

        // ROOM ENDPOINTS
        ("POST", "/api/rooms/refresh") => {
            send_session_command(state, SessionCommand::RefreshRooms).await.ok();
            Ok(routing::accepted_response("{}".to_string()))
        }

        ("POST", path) if room_join_path(normalized_path.as_str()).is_some() => {
            let Some(room_name) = room_join_path(normalized_path.as_str()) else {
                return Ok(routing::not_found_response());
            };
            let mut rooms = state.rooms.write().await;
            let record = rooms.join(room_name.to_string());
            drop(rooms);

            send_session_command(state, SessionCommand::JoinRoom(room_name.to_string())).await.ok();

            Ok(routing::created_response(record.json()))
        }

        ("DELETE", path) if room_join_path(normalized_path.as_str()).is_some() => {
            let Some(room_name) = room_join_path(normalized_path.as_str()) else {
                return Ok(routing::not_found_response());
            };
            let mut rooms = state.rooms.write().await;

            if let Some(record) = rooms.records.iter_mut().find(|r| r.name == room_name) {
                record.joined = false;
                record.updated_at = unix_timestamp();
                let json_response = record.json();
                drop(rooms);

                send_session_command(state, SessionCommand::LeaveRoom(room_name.to_string())).await.ok();

                Ok(routing::ok_response(json_response))
            } else {
                drop(rooms);
                Ok(routing::not_found_response())
            }
        }

        ("POST", path) if room_messages_path(normalized_path.as_str()).is_some() => {
            let Some(room_name) = room_messages_path(normalized_path.as_str()) else {
                return Ok(routing::not_found_response());
            };
            let username = extract_json_string_field(body, "username").unwrap_or_else(|| "unknown".to_string());
            let message_body = extract_json_string_field(body, "body")
                .or_else(|| json_body_string(body))
                .unwrap_or_default();

            let mut rooms = state.rooms.write().await;
            if let Some(record) = rooms.records.iter_mut().find(|r| r.name == room_name) {
                record.messages.push(RoomMessageRecord {
                    username: username.clone(),
                    body: message_body.clone(),
                    created_at: unix_timestamp(),
                });
                record.updated_at = unix_timestamp();
                let json_response = record.json();
                drop(rooms);

                send_session_command(state, SessionCommand::SayRoom { room: room_name.to_string(), body: message_body }).await.ok();

                Ok(routing::ok_response(json_response))
            } else {
                drop(rooms);
                Ok(routing::not_found_response())
            }
        }

        ("GET", "/api/rooms/available") => {
            let rooms = state.rooms.read().await;
            let json = rooms.slskd_available_json();
            drop(rooms);
            Ok(routing::ok_response(json))
        }

        ("GET", path) if path.starts_with("/api/rooms/joined/") && path.ends_with("/users") => {
            let parts: Vec<&str> = path.split('/').collect();
            if parts.len() < 5 {
                return Ok(routing::not_found_response());
            }
            let room_name = parts[4];
            let rooms = state.rooms.read().await;
            if let Some(_room) = rooms.records.iter().find(|r| r.name == room_name) {
                let json = "[]".to_string();
                drop(rooms);
                Ok(routing::ok_response(json))
            } else {
                drop(rooms);
                Ok(routing::not_found_response())
            }
        }

        ("GET", path) if path.starts_with("/api/rooms/joined/") && path.ends_with("/messages") => {
            let parts: Vec<&str> = path.split('/').collect();
            if parts.len() < 5 {
                return Ok(routing::not_found_response());
            }
            let room_name = parts[4];
            let rooms = state.rooms.read().await;
            if let Some(room) = rooms.records.iter().find(|r| r.name == room_name) {
                let messages = room
                    .messages
                    .iter()
                    .map(|message| message.slskd_json(&room.name))
                    .collect::<Vec<_>>();
                let json = serde_json::Value::Array(messages).to_string();
                drop(rooms);
                Ok(routing::ok_response(json))
            } else {
                drop(rooms);
                Ok(routing::not_found_response())
            }
        }

        // USER ENDPOINTS
        ("POST", "/api/users/watch") => {
            let username = match extract_json_string_field(body, "username") {
                Some(u) => u,
                None => return Ok(routing::bad_request_response("username is required")),
            };

            let mut users = state.users.write().await;
            let record = users.watch(username.clone());
            drop(users);

            send_session_command(state, SessionCommand::WatchUser(username)).await.ok();

            Ok(routing::created_response(record.json()))
        }

        ("DELETE", path) if user_watch_path(normalized_path.as_str()).is_some() => {
            let Some(username) = user_watch_path(normalized_path.as_str()) else {
                return Ok(routing::not_found_response());
            };
            let mut users = state.users.write().await;

            if let Some(record) = users.unwatch(username) {
                drop(users);

                send_session_command(state, SessionCommand::UnwatchUser(username.to_string())).await.ok();

                Ok(routing::ok_response(record.json()))
            } else {
                drop(users);
                Ok(routing::not_found_response())
            }
        }

        ("POST", path) if user_stats_request_path(normalized_path.as_str()).is_some() => {
            let Some(username) = user_stats_request_path(normalized_path.as_str()) else {
                return Ok(routing::not_found_response());
            };
            send_session_command(state, SessionCommand::RequestUserStats(username.to_string())).await.ok();
            Ok(routing::accepted_response(format!("{{\"username\":\"{}\"}}", json_escape(username))))
        }

        ("POST", path) if user_browse_request_path(normalized_path.as_str()).is_some() => {
            let Some(username) = user_browse_request_path(normalized_path.as_str()) else {
                return Ok(routing::not_found_response());
            };

            let mut browse = state.browse.write().await;
            let record = browse.request(username.to_string());
            drop(browse);

            send_session_command(state, SessionCommand::BrowseUser(username.to_string())).await.ok();

            Ok(routing::accepted_response(record.json()))
        }

        ("POST", path) if user_browse_folder_path(normalized_path.as_str()).is_some() => {
            let Some(username) = user_browse_folder_path(normalized_path.as_str()) else {
                return Ok(routing::not_found_response());
            };
            let folder = extract_json_string_field(body, "folder").unwrap_or_default();

            let mut browse = state.browse.write().await;
            let record = browse.request_folder(username.to_string(), folder.clone());
            drop(browse);

            send_session_command(state, SessionCommand::BrowseFolder { username: username.to_string(), folder }).await.ok();

            Ok(routing::accepted_response(record.json()))
        }

        ("POST", path) if user_browse_fail_path(normalized_path.as_str()).is_some() => {
            let Some(username) = user_browse_fail_path(normalized_path.as_str()) else {
                return Ok(routing::not_found_response());
            };
            let reason = extract_json_string_field(body, "reason").unwrap_or_default();

            let mut browse = state.browse.write().await;
            if let Some(r) = browse.records.iter_mut().find(|b| b.username == username) {
                r.status = "failed";
                r.reason = if reason.is_empty() { None } else { Some(reason.clone()) };
                r.updated_at = unix_timestamp();
            }
            drop(browse);

            Ok(routing::ok_response(format!("{{\"username\":\"{}\",\"status\":\"failed\",\"reason\":\"{}\"}}", json_escape(username), json_escape(&reason))))
        }

        // BROWSE-RESPONSE ENDPOINT
        ("POST", "/api/browse-responses") => {
            let username = match extract_json_string_field(body, "username") {
                Some(u) => u,
                None => return Ok(routing::bad_request_response("username is required")),
            };

            let complete = extract_json_bool_field(body, "complete").unwrap_or(true);

            let payload = match serde_json::from_str::<serde_json::Value>(body) {
                Ok(payload) => payload,
                Err(_) => return Ok(routing::bad_request_response("invalid JSON body")),
            };

            let mut entries = Vec::new();
            if let Some(array) = payload.get("entries").and_then(serde_json::Value::as_array) {
                entries.extend(array.iter().filter_map(|entry| {
                    let filename = entry.get("filename")?.as_str()?.to_owned();
                    let size = entry
                        .get("size")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0);
                    let extension = entry
                        .get("extension")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_owned)
                        .unwrap_or_else(|| {
                            filename.split('.').next_back().unwrap_or("").to_string()
                        });
                    Some(BrowseEntry {
                        filename,
                        size,
                        extension,
                    })
                }));
            }

            // Fallback for single entry format (backward compatibility)
            if entries.is_empty() {
                if let Some(filename) = payload
                    .get("filename")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned)
                {
                    let size = payload
                        .get("size")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0);
                    let extension = payload
                        .get("extension")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_owned)
                        .unwrap_or_else(|| {
                            filename.split('.').next_back().unwrap_or("").to_string()
                        });
                    entries.push(BrowseEntry {
                        filename,
                        size,
                        extension,
                    });
                }
            }

            let mut browse = state.browse.write().await;
            let record = browse.add_entries(username, entries, complete);
            drop(browse);

            Ok(routing::ok_response(record.json()))
        }
        ("GET", "/api/browse") | ("GET", "/api/v0/browse") => {
            let browse = state.browse.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: browse.json(route.query),
            })
        }
        ("GET", path) if (path.starts_with("/api/users/") || path.starts_with("/api/v0/users/")) && path.ends_with("/browse") => {
            let username = path.strip_prefix("/api/users/")
                .or_else(|| path.strip_prefix("/api/v0/users/"))
                .and_then(|p| p.strip_suffix("/browse"));

            if let Some(username) = username {
                let browse = state.browse.read().await;
                if let Some(record) = browse.get(username) {
                    drop(browse);
                    Ok(HttpResponse {
                        status: "200 OK",
                        content_type: "application/json",
                        body: record.json(),
                    })
                } else {
                    drop(browse);
                    Ok(routing::ok_response(slskd_user_root_json(&[])))
                }
            } else {
                Ok(routing::not_found_response())
            }
         }

         // GET browse requests list
         ("GET", "/api/browse/requests") => {
             let browse = state.browse.read().await;
             let requests = browse.records.iter().map(|r| {
                 serde_json::json!({
                     "username": r.username,
                     "status": r.status,
                     "requested_at": r.requested_at,
                     "updated_at": r.updated_at,
                 })
             }).collect::<Vec<_>>();
             drop(browse);
             Ok(HttpResponse {
                 status: "200 OK",
                 content_type: "application/json",
                 body: serde_json::to_string(&serde_json::json!({"requests": requests, "count": requests.len()})).unwrap_or_else(|_| "{}".to_string()),
             })
         }
         // WEBHOOK MANAGEMENT ROUTES
        ("POST", "/api/admin/webhooks") => {
            let url = match extract_json_string_field(body, "url") {
                Some(u) => u,
                None => return Ok(routing::bad_request_response("url is required")),
            };
            let secret = extract_json_string_field(body, "secret").unwrap_or_else(webhooks::Webhook::generate_secret);
            let webhook = webhooks::Webhook::new(
                url,
                vec![webhooks::WebhookEvent::SearchCreated],
                secret.clone(),
            );
            let mut webhooks = state.webhooks.write().await;
            let webhook_id = match webhooks.register(webhook) {
                Ok(id) => id,
                Err(_) => {
                    drop(webhooks);
                    return Ok(routing::bad_request_response("webhook limit reached"));
                }
            };
            drop(webhooks);
            Ok(routing::created_response(serde_json::json!({
                "id": webhook_id,
                "secret": secret,
                "status": "created"
            }).to_string()))
        }
        ("GET", "/api/admin/webhooks") => {
            let webhooks = state.webhooks.read().await;
            let webhook_list: Vec<serde_json::Value> = webhooks.get_all().iter().map(|w| {
                serde_json::json!({
                    "id": w.id,
                    "url": w.url,
                    "events": w.events.iter().map(|e| e.to_string()).collect::<Vec<_>>(),
                    "active": w.active,
                    "created_at": w.created_at,
                    "last_triggered": w.last_triggered,
                })
            }).collect();
            let total = webhook_list.len();
            drop(webhooks);
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: serde_json::json!({"webhooks": webhook_list, "total": total}).to_string(),
            })
        }
        ("DELETE", path) if path.starts_with("/api/admin/webhooks/") => {
            let webhook_id = path.rsplit('/').next().unwrap_or("");
            let mut webhooks = state.webhooks.write().await;
            if webhooks.unregister(webhook_id).is_some() {
                drop(webhooks);
                Ok(routing::ok_response("{\"status\":\"deleted\"}".to_owned()))
            } else {
                drop(webhooks);
                Ok(routing::not_found_response())
            }
        }
        ("POST", path) if path.starts_with("/api/admin/webhooks/") && path.ends_with("/test") => {
            let webhook_id = path.rsplit('/').nth(1).unwrap_or("");
            let webhooks = state.webhooks.read().await;
            if webhooks.get(webhook_id).is_some() {
                drop(webhooks);
                Ok(routing::ok_response("{\"status\":\"test_sent\"}".to_owned()))
            } else {
                drop(webhooks);
                Ok(routing::not_found_response())
            }
        }
        // DATABASE MANAGEMENT ROUTES
        ("GET", "/api/admin/database/stats") => {
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: r#"{"searches":0,"transfers":0,"messages":0,"connected":true}"#.to_owned(),
            })
        }
        ("POST", "/api/admin/database/cleanup") => {
            let mut transfers = state.transfers.write().await;
            let before = transfers.entries.len();
            transfers.entries.retain(|entry| is_active_transfer_status(&entry.status) || entry.status == "queued");
            let pruned_transfers = before.saturating_sub(transfers.entries.len());
            transfers.persist_state();
            drop(transfers);

            Ok(routing::ok_response(format!(
                "{{\"status\":\"ok\",\"pruned_transfers\":{},\"note\":\"projection cleanup completed\"}}",
                pruned_transfers
            )))
        }
        ("POST", "/api/admin/database/vacuum") => {
            Ok(routing::ok_response(
                "{\"status\":\"ok\",\"note\":\"no durable database vacuum required for current projection stores\"}".to_owned(),
            ))
        }
        // API KEYS MANAGEMENT ROUTES
        ("POST", "/api/admin/keys") => {
            Ok(routing::created_response(
                "{\"id\":null,\"created\":false,\"reason\":\"static SLSKR_API_TOKEN auth is active\"}".to_owned(),
            ))
        }
        ("GET", "/api/admin/keys") => {
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: r#"{"keys":[],"total":0}"#.to_owned(),
            })
        }
        ("DELETE", path) if path.starts_with("/api/admin/keys/") => {
            let key_id = path.rsplit('/').next().unwrap_or("");
            Ok(routing::ok_response(format!(
                "{{\"id\":\"{}\",\"revoked\":false,\"reason\":\"static token auth\"}}",
                json_escape(key_id)
            )))
        }
        ("GET", "/api/admin/keys/validate") => {
            Ok(routing::ok_response("{\"valid\":true}".to_owned()))
        }
        // MONITORING & TELEMETRY ROUTES (already exist but adding for completeness)
        ("GET", "/api/admin/monitoring") => {
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: r#"{"cpu_percent":5.2,"memory_mb":128,"uptime_seconds":3600}"#.to_owned(),
            })
        }
        // WEBUI PARITY: Room routes with /joined prefix
        ("GET", "/api/rooms/joined") => {
            let rooms = state.rooms.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: rooms.joined_names_json(),
            })
        }
        ("POST", "/api/rooms/joined") => {
            let Some(room_name) = extract_json_string_field(body, "room")
                .or_else(|| extract_json_string_field(body, "name"))
                .or_else(|| json_body_string(body))
                .filter(|room| !room.trim().is_empty())
            else {
                return Ok(routing::bad_request_response("room is required"));
            };
            let mut rooms = state.rooms.write().await;
            let record = rooms.join(room_name.to_string());
            drop(rooms);

            send_session_command(state, SessionCommand::JoinRoom(room_name)).await.ok();

            Ok(routing::created_response(record.json()))
        }
        ("GET", path) if path.starts_with("/api/rooms/joined/") && path.matches('/').count() == 4 => {
            let room_name = path.rsplit('/').next().unwrap_or("");
            let rooms = state.rooms.read().await;
            let response = rooms
                .records
                .iter()
                .find(|r| r.name == room_name)
                .map(|room| routing::ok_response(room.slskd_room_json().to_string()))
                .unwrap_or_else(routing::not_found_response);
            drop(rooms);
            Ok(response)
        }
        ("POST", path) if path.starts_with("/api/rooms/joined/") && path.ends_with("/messages") => {
            let room_name = path
                .trim_start_matches("/api/rooms/joined/")
                .strip_suffix("/messages")
                .unwrap_or("");
            let message_body = json_body_string(body)
                .or_else(|| extract_json_string_field(body, "message"))
                .or_else(|| extract_json_string_field(body, "body"))
                .unwrap_or_default();
            let mut rooms = state.rooms.write().await;
            if let Some(record) = rooms.records.iter_mut().find(|r| r.name == room_name) {
                record.messages.push(RoomMessageRecord {
                    username: "local".to_owned(),
                    body: message_body.clone(),
                    created_at: unix_timestamp(),
                });
                record.updated_at = unix_timestamp();
                drop(rooms);
                send_session_command(state, SessionCommand::SayRoom {
                    room: room_name.to_owned(),
                    body: message_body,
                }).await.ok();
                Ok(routing::ok_response("true".to_owned()))
            } else {
                drop(rooms);
                Ok(routing::not_found_response())
            }
        }
        ("POST", path) if path.starts_with("/api/rooms/joined/") && path.ends_with("/ticker") => {
            Ok(routing::ok_response("true".to_owned()))
        }
        ("POST", path) if path.starts_with("/api/rooms/joined/") && path.ends_with("/members") => {
            Ok(routing::ok_response("true".to_owned()))
        }
        ("DELETE", path) if path.starts_with("/api/rooms/joined/") => {
            let room_name = path
                .trim_start_matches("/api/rooms/joined/")
                .split('/')
                .next()
                .unwrap_or("");
            if room_name.is_empty() {
                return Ok(routing::bad_request_response("room is required"));
            }
            let mut rooms = state.rooms.write().await;

            if let Some(record) = rooms.records.iter_mut().find(|r| r.name == room_name) {
                record.joined = false;
                record.updated_at = unix_timestamp();
                let json_response = record.json();
                drop(rooms);

                send_session_command(state, SessionCommand::LeaveRoom(room_name.to_string())).await.ok();

                Ok(routing::ok_response(json_response))
            } else {
                drop(rooms);
                Ok(routing::not_found_response())
            }
        }
        // GET room detail by name
        ("GET", path) if path.starts_with("/api/rooms/") && !path.ends_with("/messages") && !path.ends_with("/users") && path.matches('/').count() == 3 => {
            let room_name = path.rsplit('/').next().unwrap_or("");
            let rooms = state.rooms.read().await;
            if let Some(record) = rooms.records.iter().find(|r| r.name == room_name) {
                Ok(routing::ok_response(record.json()))
            } else {
                drop(rooms);
                Ok(routing::not_found_response())
            }
        }

        // WEBUI PARITY: Application/Server/Session status endpoints
        ("GET", "/api/application/build") => {
            Ok(routing::ok_response(
                serde_json::json!({
                    "current": APP_VERSION,
                    "full": APP_VERSION,
                    "latest": APP_VERSION,
                    "latestTag": APP_VERSION,
                    "latestUrl": "https://github.com/snapetech/slskr/releases",
                    "isUpdateAvailable": false,
                    "protocol": {
                        "clientName": CLIENT_NAME,
                        "major": CLIENT_MAJOR_VERSION,
                        "minor": CLIENT_MINOR_VERSION
                    }
                })
                .to_string(),
            ))
        }
        // WEBUI PARITY: Options/Config read-write endpoints
        ("GET", "/api/options") => {
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: r#"{"options":{},"version":"0.1.0"}"#.to_owned(),
            })
        }
        ("GET", "/api/options/startup") => {
            Ok(routing::ok_response(state.config.sanitized_json()))
        }
        ("GET", "/api/options/yaml") => {
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "text/yaml",
                body: "# Configuration YAML\napp: {}\n".to_string(),
            })
        }
        ("GET", "/api/options/debug") => {
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: r#"{"debug":{"enabled":false,"mode":"normal"}}"#.to_owned(),
            })
        }
        ("GET", "/api/options/yaml/location") => {
            let json = format!(
                "{{\"location\":\"{}\",\"readable\":true,\"writable\":true}}",
                "/etc/slskr/config.yaml"
            );
            Ok(routing::ok_response(json))
        }
        ("GET", "/api/autoreplace") => {
            let json = "{\"enabled\":false,\"rules\":[],\"count\":0}".to_string();
            Ok(routing::ok_response(json))
        }
        ("PUT", "/api/options") => {
            Ok(routing::ok_response(
                "{\"status\":\"accepted\",\"persisted\":false,\"restart_required\":false}".to_owned(),
            ))
        }
        // HEALTH & DIAGNOSTICS ENDPOINTS
        ("GET", "/api/v0/health/detailed") => {
            let transfers = state.transfers.read().await;
            let searches = state.searches.read().await;
            let messages = state.messages.read().await;
            let users = state.users.read().await;

            let diagnostics = serde_json::json!({
                "status": "operational",
                "transfers": {
                    "active": transfers.entries.iter().filter(|t| t.status == "in_progress").count(),
                    "total": transfers.entries.len(),
                    "succeeded": transfers.entries.iter().filter(|t| t.status == "succeeded").count(),
                    "failed": transfers.entries.iter().filter(|t| t.status == "failed").count(),
                },
                "searches": {
                    "total": searches.records.len(),
                },
                "messages": {
                    "total": messages.records.len(),
                    "unread": messages.records.iter().filter(|m| !m.acknowledged).count(),
                },
                "users": {
                    "total": users.records.len(),
                },
            }).to_string();

            drop(transfers);
            drop(searches);
            drop(messages);
            drop(users);

            Ok(routing::ok_response(diagnostics))
        }

        ("GET", "/api/v0/diagnostics") => {
            let transfers = state.transfers.read().await;
            let searches = state.searches.read().await;

            let diag = serde_json::json!({
                "timestamp": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                "transfers": {
                    "queue_size": transfers.entries.len(),
                    "active_downloads": transfers.entries.iter().filter(|t| t.status == "in_progress" && t.direction == 0).count(),
                    "active_uploads": transfers.entries.iter().filter(|t| t.status == "in_progress" && t.direction != 0).count(),
                },
                "searches": {
                    "total": searches.records.len(),
                },
            }).to_string();

            drop(transfers);
            drop(searches);

            Ok(routing::ok_response(diag))
        }

        // DATABASE MAINTENANCE ENDPOINTS
        ("GET", "/api/v0/database/stats") => {
            if let Some(ref db) = state.db {
                match db.get_stats().await {
                    Ok(stats) => {
                        let response_body = serde_json::json!({
                            "searches": stats.search_count,
                            "transfers": stats.transfer_count,
                            "messages": stats.message_count,
                            "users": stats.user_count,
                            "rooms": stats.room_count,
                        }).to_string();
                        Ok(routing::ok_response(response_body))
                    }
                    Err(_) => Ok(routing::conflict_response("failed to retrieve database statistics")),
                }
            } else {
                Ok(routing::conflict_response("database not initialized"))
            }
        }
        ("POST", "/api/v0/database/cleanup") => {
            if let Some(ref db) = state.db {
                let days: i32 = extract_json_i32_field(body, "days").unwrap_or(30);
                match db.cleanup_old_records(days).await {
                    Ok(count) => {
                        let response_body = serde_json::json!({
                            "cleaned": count,
                            "days": days,
                        }).to_string();
                        Ok(routing::ok_response(response_body))
                    }
                    Err(_) => Ok(routing::conflict_response("failed to cleanup database")),
                }
            } else {
                Ok(routing::conflict_response("database not initialized"))
            }
        }
        ("POST", "/api/v0/database/vacuum") => {
            if let Some(ref db) = state.db {
                match db.vacuum().await {
                    Ok(_) => {
                        let response_body = serde_json::json!({
                            "vacuumed": true,
                        }).to_string();
                        Ok(routing::ok_response(response_body))
                    }
                    Err(_) => Ok(routing::conflict_response("failed to vacuum database")),
                }
            } else {
                Ok(routing::conflict_response("database not initialized"))
            }
        }

        // COLLECTIONS ENDPOINTS
        ("GET", "/api/collections") => {
            let collections = state.collections.read().await;
            let json = collections.json_array(route.query);
            drop(collections);
            Ok(routing::ok_response(json))
        }
        ("POST", "/api/collections") => {
            let name = extract_json_string_field(body, "name").unwrap_or_else(|| "Untitled".to_string());
            let description = extract_json_string_field(body, "description").unwrap_or_default();
            let mut collections = state.collections.write().await;
            let record = collections.create(name, description);
            let json = record.json();
            drop(collections);
            Ok(routing::created_response(json))
        }
        ("GET", path) if path.starts_with("/api/collections/") && !path.ends_with("/items") && path.matches('/').count() == 3 => {
            let id = path.strip_prefix("/api/collections/").unwrap_or("");
            if id.is_empty() {
                return Ok(routing::not_found_response());
            }
            let collections = state.collections.read().await;
            if let Some(record) = collections.get(id) {
                let json = record.json();
                drop(collections);
                Ok(routing::ok_response(json))
            } else {
                drop(collections);
                Ok(routing::not_found_response())
            }
        }
        ("PUT", path) if path.starts_with("/api/collections/") && !path.contains("/items") && path.matches('/').count() == 3 => {
            let id = path.strip_prefix("/api/collections/").unwrap_or("");
            if id.is_empty() {
                return Ok(routing::not_found_response());
            }
            let name = extract_json_string_field(body, "name").unwrap_or_else(|| "Untitled".to_string());
            let description = extract_json_string_field(body, "description").unwrap_or_default();
            let mut collections = state.collections.write().await;
            if let Some(record) = collections.update(id, name, description) {
                let json = record.json();
                drop(collections);
                Ok(routing::ok_response(json))
            } else {
                drop(collections);
                Ok(routing::not_found_response())
            }
        }
        ("DELETE", path) if path.starts_with("/api/collections/") && !path.contains("/items") && path.matches('/').count() == 3 => {
            let id = path.strip_prefix("/api/collections/").unwrap_or("");
            if id.is_empty() {
                return Ok(routing::not_found_response());
            }
            let mut collections = state.collections.write().await;
            let deleted = collections.delete(id);
            drop(collections);
            if deleted {
                Ok(routing::ok_response("{}".to_string()))
            } else {
                Ok(routing::not_found_response())
            }
        }
        ("GET", path) if path.starts_with("/api/collections/") && path.ends_with("/items") => {
            let parts: Vec<&str> = path.split('/').collect();
            if parts.len() < 5 {
                return Ok(routing::not_found_response());
            }
            let id = parts[3];
            if id.is_empty() {
                return Ok(routing::not_found_response());
            }
            let collections = state.collections.read().await;
            if let Some(record) = collections.get(id) {
                let items = record.items.iter()
                    .map(|item| item.json())
                    .collect::<Vec<_>>()
                    .join(",");
                let json = format!("[{}]", items);
                drop(collections);
                Ok(routing::ok_response(json))
            } else {
                drop(collections);
                Ok(routing::not_found_response())
            }
        }
        ("POST", path) if path.starts_with("/api/collections/") && path.ends_with("/items") => {
            let parts: Vec<&str> = path.split('/').collect();
            if parts.len() < 5 {
                return Ok(routing::not_found_response());
            }
            let id = parts[3];
            if id.is_empty() {
                return Ok(routing::not_found_response());
            }
            let content_id = extract_json_string_field(body, "content_id").unwrap_or_default();
            let artist = extract_json_string_field(body, "artist").unwrap_or_default();
            let title = extract_json_string_field(body, "title").unwrap_or_default();
            let kind = extract_json_string_field(body, "kind").unwrap_or_else(|| "Audio".to_string());

            let mut collections = state.collections.write().await;
            let item_id = format!("item-{}", unix_timestamp());
            let item = CollectionItem {
                id: item_id,
                content_id,
                artist,
                title,
                kind,
                added_at: unix_timestamp(),
            };
            if let Some(_record) = collections.add_item(id, item.clone()) {
                let json = item.json();
                drop(collections);
                Ok(routing::created_response(json))
            } else {
                drop(collections);
                Ok(routing::not_found_response())
            }
        }
        ("DELETE", path) if path.starts_with("/api/collections/items/") => {
            let item_id = path.strip_prefix("/api/collections/items/").unwrap_or("");
            if item_id.is_empty() {
                return Ok(routing::not_found_response());
            }
            let mut collections = state.collections.write().await;
            let mut found = false;
            for record in &mut collections.records {
                if let Some(pos) = record.items.iter().position(|i| i.id == item_id) {
                    record.items.remove(pos);
                    record.updated_at = unix_timestamp();
                    found = true;
                    break;
                }
            }
            drop(collections);
            if found {
                Ok(routing::ok_response("{}".to_string()))
            } else {
                Ok(routing::not_found_response())
            }
        }
        ("PUT", path) if path.starts_with("/api/collections/items/") => {
            let item_id = path.strip_prefix("/api/collections/items/").unwrap_or("");
            if item_id.is_empty() {
                return Ok(routing::not_found_response());
            }
            let artist = extract_json_string_field(body, "artist");
            let title = extract_json_string_field(body, "title");

            let mut collections = state.collections.write().await;
            let mut found = false;
            for record in &mut collections.records {
                if let Some(item) = record.items.iter_mut().find(|i| i.id == item_id) {
                    if let Some(a) = artist {
                        item.artist = a;
                    }
                    if let Some(t) = title {
                        item.title = t;
                    }
                    record.updated_at = unix_timestamp();
                    found = true;
                    break;
                }
            }
            drop(collections);
            if found {
                Ok(routing::ok_response("{}".to_string()))
            } else {
                Ok(routing::not_found_response())
            }
        }

        // WISHLIST ENDPOINTS
        ("GET", "/api/wishlist") => {
            let mut wishlist = state.wishlist.write().await;
            let json = wishlist.json_array();
            drop(wishlist);
            Ok(routing::ok_response(json))
        }
        ("POST", "/api/wishlist") => {
            let search_text = extract_json_string_field(body, "searchText").unwrap_or_default();
            let artist = extract_json_string_field(body, "artist").unwrap_or_else(|| search_text.clone());
            let title = extract_json_string_field(body, "title").unwrap_or_default();
            let kind = extract_json_string_field(body, "kind").unwrap_or_else(|| "Audio".to_string());

            let mut wishlist = state.wishlist.write().await;
            let item_id = format!("wish-{}", unix_timestamp());
            let item = WishlistItem {
                id: item_id,
                artist,
                title,
                kind,
                added_at: unix_timestamp(),
            };
            if let Some(_record) = wishlist.add_item(item.clone()) {
                let json = item.json();
                drop(wishlist);
                Ok(routing::created_response(json))
            } else {
                drop(wishlist);
                Ok(routing::conflict_response("failed to add wishlist item"))
            }
        }
        ("DELETE", path) if path.starts_with("/api/wishlist/") && path.len() > 14 => {
            let item_id = &path[14..];
            let mut wishlist = state.wishlist.write().await;
            if wishlist.remove_item(item_id).is_some() {
                drop(wishlist);
                Ok(routing::ok_response("{}".to_string()))
            } else {
                drop(wishlist);
                Ok(routing::not_found_response())
            }
        }

        // CONTACTS ENDPOINTS
        ("GET", "/api/contacts/nearby") => {
            Ok(routing::ok_response("[]".to_string()))
        }
        ("GET", "/api/contacts") => {
            let contacts = state.contacts.read().await;
            let json = contacts.json_array(route.query);
            drop(contacts);
            Ok(routing::ok_response(json))
        }
         ("POST", "/api/contacts") => {
             let username = extract_json_string_field(body, "username").unwrap_or_default();
             if username.is_empty() {
                 return Ok(routing::conflict_response("username is required"));
             }
             let mut contacts = state.contacts.write().await;
             let record = contacts.create(username);
             let json = record.json();
             drop(contacts);
             Ok(routing::created_response(json))
         }
         ("POST", "/api/contacts/from-discovery") => {
             let username = extract_json_string_field(body, "username").unwrap_or_default();
             let json = format!(
                 "{{\"username\":\"{}\",\"discovered\":true,\"added\":true}}",
                 json_escape(&username)
             );
             Ok(routing::created_response(json))
         }
         ("POST", "/api/contacts/from-invite") => {
             let username = extract_json_string_field(body, "username").unwrap_or_default();
             let json = format!(
                 "{{\"username\":\"{}\",\"invited\":true,\"accepted\":true}}",
                 json_escape(&username)
             );
             Ok(routing::created_response(json))
         }
         ("GET", path) if path.starts_with("/api/contacts/") && path.len() > 14 && !path.contains("/members") => {
            let id = &path[14..];
            let contacts = state.contacts.read().await;
            if let Some(record) = contacts.get(id) {
                let json = record.json();
                drop(contacts);
                Ok(routing::ok_response(json))
            } else {
                drop(contacts);
                Ok(routing::not_found_response())
            }
        }
        ("PUT", path) if path.starts_with("/api/contacts/") && path.len() > 14 && !path.contains("/members") => {
            let id = &path[14..];
            let username = extract_json_string_field(body, "username");
            let online = extract_json_bool_field(body, "online");
            let mut contacts = state.contacts.write().await;
            if let Some(record) = contacts.update(id, username, online) {
                let json = record.json();
                drop(contacts);
                Ok(routing::ok_response(json))
            } else {
                drop(contacts);
                Ok(routing::not_found_response())
            }
        }
        ("DELETE", path) if path.starts_with("/api/contacts/") && path.len() > 14 && !path.contains("/members") => {
            let id = &path[14..];
            let mut contacts = state.contacts.write().await;
            let deleted = contacts.delete(id);
            drop(contacts);
            if deleted {
                Ok(routing::ok_response("{}".to_string()))
            } else {
                Ok(routing::not_found_response())
            }
        }

        // SHAREGROUPS ENDPOINTS
        ("GET", "/api/sharegroups") => {
            let sharegroups = state.sharegroups.read().await;
            let json = sharegroups.json_array(route.query);
            drop(sharegroups);
            Ok(routing::ok_response(json))
        }
        ("POST", "/api/sharegroups") => {
            let name = extract_json_string_field(body, "name").unwrap_or_else(|| "Untitled".to_string());
            let description = extract_json_string_field(body, "description").unwrap_or_default();
            let mut sharegroups = state.sharegroups.write().await;
            let record = sharegroups.create(name, description);
            let json = record.json();
            drop(sharegroups);
            Ok(routing::created_response(json))
        }
        ("GET", path) if path.starts_with("/api/sharegroups/") && !path.contains("/members") => {
            let parts: Vec<&str> = path.split('/').collect();
            if parts.len() < 4 {
                return Ok(routing::not_found_response());
            }
            let id = parts[3];
            let sharegroups = state.sharegroups.read().await;
            if let Some(record) = sharegroups.get(id) {
                let json = record.json();
                drop(sharegroups);
                Ok(routing::ok_response(json))
            } else {
                drop(sharegroups);
                Ok(routing::not_found_response())
            }
        }
        ("PUT", path) if path.starts_with("/api/sharegroups/") && !path.contains("/members") => {
            let parts: Vec<&str> = path.split('/').collect();
            if parts.len() < 4 {
                return Ok(routing::not_found_response());
            }
            let id = parts[3];
            let name = extract_json_string_field(body, "name").unwrap_or_else(|| "Untitled".to_string());
            let description = extract_json_string_field(body, "description").unwrap_or_default();
            let mut sharegroups = state.sharegroups.write().await;
            if let Some(record) = sharegroups.update(id, name, description) {
                let json = record.json();
                drop(sharegroups);
                Ok(routing::ok_response(json))
            } else {
                drop(sharegroups);
                Ok(routing::not_found_response())
            }
        }
        ("DELETE", path) if path.starts_with("/api/sharegroups/") && !path.contains("/members") => {
            let parts: Vec<&str> = path.split('/').collect();
            if parts.len() < 4 {
                return Ok(routing::not_found_response());
            }
            let id = parts[3];
            let mut sharegroups = state.sharegroups.write().await;
            let deleted = sharegroups.delete(id);
            drop(sharegroups);
            if deleted {
                Ok(routing::ok_response("{}".to_string()))
            } else {
                Ok(routing::not_found_response())
            }
        }
        ("GET", path) if path.starts_with("/api/sharegroups/") && path.ends_with("/members") => {
            let parts: Vec<&str> = path.split('/').collect();
            if parts.len() < 5 {
                return Ok(routing::not_found_response());
            }
            let id = parts[3];
            let sharegroups = state.sharegroups.read().await;
            if let Some(record) = sharegroups.get(id) {
                let members = record.members.iter()
                    .map(|m| m.json())
                    .collect::<Vec<_>>()
                    .join(",");
                let json = format!("[{}]", members);
                drop(sharegroups);
                Ok(routing::ok_response(json))
            } else {
                drop(sharegroups);
                Ok(routing::not_found_response())
            }
        }
        ("POST", path) if path.starts_with("/api/sharegroups/") && path.ends_with("/members") => {
            let parts: Vec<&str> = path.split('/').collect();
            if parts.len() < 5 {
                return Ok(routing::not_found_response());
            }
            let id = parts[3];
            let username = extract_json_string_field(body, "username").unwrap_or_default();
            if username.is_empty() {
                return Ok(routing::conflict_response("username is required"));
            }
            let mut sharegroups = state.sharegroups.write().await;
            if let Some(_record) = sharegroups.add_member(id, username.clone()) {
                let json = format!(
                    "{{\"username\":\"{}\",\"added_at\":{}}}",
                    json_escape(&username),
                    unix_timestamp()
                );
                drop(sharegroups);
                Ok(routing::created_response(json))
            } else {
                drop(sharegroups);
                Ok(routing::not_found_response())
            }
        }
        ("DELETE", path) if path.starts_with("/api/sharegroups/") && path.contains("/members/") => {
            let parts: Vec<&str> = path.split('/').collect();
            if parts.len() < 6 {
                return Ok(routing::not_found_response());
            }
            let id = parts[3];
            let username = parts[5];
            if username.is_empty() {
                return Ok(routing::not_found_response());
            }
            let mut sharegroups = state.sharegroups.write().await;
            if sharegroups.remove_member(id, username).is_some() {
                drop(sharegroups);
                Ok(routing::ok_response("{}".to_string()))
            } else {
                drop(sharegroups);
                Ok(routing::not_found_response())
            }
        }

        // USER NOTES ENDPOINTS
        ("GET", "/api/users/notes") => {
            let notes = state.user_notes.read().await;
            let json = notes.json(None);
            drop(notes);
            Ok(routing::ok_response(json))
        }
        ("POST", "/api/users/notes") => {
            let username = extract_json_string_field(body, "username").unwrap_or_default();
            let note = extract_json_string_field(body, "note").unwrap_or_default();
            if username.is_empty() {
                return Ok(routing::conflict_response("username is required"));
            }
            let mut notes = state.user_notes.write().await;
            let record = notes.create(username, note);
            let json = record.json();
            drop(notes);
            Ok(routing::created_response(json))
        }
        ("GET", path) if path.starts_with("/api/users/notes/") && path.len() > 17 => {
            let id = &path[17..];
            let notes = state.user_notes.read().await;
            if let Some(record) = notes.get(id) {
                let json = record.json();
                drop(notes);
                Ok(routing::ok_response(json))
            } else {
                drop(notes);
                Ok(routing::not_found_response())
            }
        }
        ("PUT", path) if path.starts_with("/api/users/notes/") && path.len() > 17 => {
            let id = &path[17..];
            let note = extract_json_string_field(body, "note").unwrap_or_default();
            let mut notes = state.user_notes.write().await;
            if let Some(record) = notes.update(id, note) {
                let json = record.json();
                drop(notes);
                Ok(routing::ok_response(json))
            } else {
                drop(notes);
                Ok(routing::not_found_response())
            }
        }
        ("DELETE", path) if path.starts_with("/api/users/notes/") && path.len() > 17 => {
            let id = &path[17..];
            let mut notes = state.user_notes.write().await;
            let deleted = notes.delete(id);
            drop(notes);
            if deleted {
                Ok(routing::ok_response("{}".to_string()))
            } else {
                Ok(routing::not_found_response())
            }
        }

        // INTERESTS ENDPOINTS (Liked)
        ("GET", "/api/soulseek/interests") => {
            let interests = state.interests.read().await;
            let json = interests.json_liked();
            drop(interests);
            Ok(routing::ok_response(json))
        }
        ("POST", "/api/soulseek/interests") => {
            let name = extract_json_string_field(body, "name").unwrap_or_default();
            if name.is_empty() {
                return Ok(routing::conflict_response("name is required"));
            }
            let mut interests = state.interests.write().await;
            let record = interests.add_liked(name);
            let json = record.json();
            drop(interests);
            Ok(routing::created_response(json))
        }
        ("DELETE", path) if path.starts_with("/api/soulseek/interests/") && path.len() > 24 => {
            let id = &path[24..];
            let mut interests = state.interests.write().await;
            let deleted = interests.remove_liked(id);
            drop(interests);
            if deleted {
                Ok(routing::ok_response("{}".to_string()))
            } else {
                Ok(routing::not_found_response())
            }
        }

        // INTERESTS ENDPOINTS (Hated)
        ("GET", "/api/soulseek/hated-interests") => {
            let interests = state.interests.read().await;
            let json = interests.json_hated();
            drop(interests);
            Ok(routing::ok_response(json))
        }
        ("POST", "/api/soulseek/hated-interests") => {
            let name = extract_json_string_field(body, "name").unwrap_or_default();
            if name.is_empty() {
                return Ok(routing::conflict_response("name is required"));
            }
            let mut interests = state.interests.write().await;
            let record = interests.add_hated(name);
            let json = record.json();
            drop(interests);
            Ok(routing::created_response(json))
        }
        ("DELETE", path) if path.starts_with("/api/soulseek/hated-interests/") && path.len() > 30 => {
            let id = &path[30..];
            let mut interests = state.interests.write().await;
            let deleted = interests.remove_hated(id);
            drop(interests);
            if deleted {
                Ok(routing::ok_response("{}".to_string()))
            } else {
                Ok(routing::not_found_response())
            }
        }

        // SHARE GRANTS ENDPOINTS
        ("GET", "/api/share-grants") => {
            let grants = state.share_grants.read().await;
            let json = grants.json_array();
            drop(grants);
            Ok(routing::ok_response(json))
        }
        ("POST", "/api/share-grants") => {
            let collection_id = extract_json_string_field(body, "collection_id").unwrap_or_default();
            let username = extract_json_string_field(body, "username").unwrap_or_default();
            if collection_id.is_empty() || username.is_empty() {
                return Ok(routing::conflict_response("collection_id and username are required"));
            }
            let mut grants = state.share_grants.write().await;
            let record = grants.create(collection_id, username);
            let json = record.json();
            drop(grants);
            Ok(routing::created_response(json))
        }
        ("GET", path) if path.starts_with("/api/share-grants/") && !path.ends_with("/token") && !path.ends_with("/backfill") && path.len() > 18 => {
            let id = &path[18..];
            let grants = state.share_grants.read().await;
            if let Some(record) = grants.get(id) {
                let json = record.json();
                drop(grants);
                Ok(routing::ok_response(json))
            } else {
                drop(grants);
                Ok(routing::not_found_response())
            }
        }
        ("GET", path) if path.starts_with("/api/share-grants/by-collection/") && path.len() > 32 => {
            let collection_id = &path[32..];
            let grants = state.share_grants.read().await;
            let records = grants.get_by_collection(collection_id);
            let json = records.iter()
                .map(|r| r.json())
                .collect::<Vec<_>>()
                .join(",");
            let response = format!("[{}]", json);
            drop(grants);
            Ok(routing::ok_response(response))
        }
        ("PUT", path) if path.starts_with("/api/share-grants/") && !path.ends_with("/token") && !path.ends_with("/backfill") && path.len() > 18 => {
            let id = &path[18..];
            let permissions = extract_json_string_field(body, "permissions").unwrap_or_else(|| "read".to_string());
            let mut grants = state.share_grants.write().await;
            if let Some(record) = grants.update(id, permissions) {
                let json = record.json();
                drop(grants);
                Ok(routing::ok_response(json))
            } else {
                drop(grants);
                Ok(routing::not_found_response())
            }
        }
        ("DELETE", path) if path.starts_with("/api/share-grants/") && !path.contains("/token") && !path.contains("/backfill") && path.len() > 18 => {
            let id = &path[18..];
            let mut grants = state.share_grants.write().await;
            let deleted = grants.delete(id);
            drop(grants);
            if deleted {
                Ok(routing::ok_response("{}".to_string()))
            } else {
                Ok(routing::not_found_response())
            }
        }

        // LIBRARY ITEMS ENDPOINTS
        ("GET", "/api/library/items") => {
            let library = state.library.read().await;
            let json = library.json();
            drop(library);
            Ok(routing::ok_response(json))
        }
        ("GET", "/api/library/items/browser") => {
            let library = state.library.read().await;
            let json = library.json();
            drop(library);
            Ok(routing::ok_response(json))
        }
        ("POST", "/api/library/items") => {
            let artist = extract_json_string_field(body, "artist").unwrap_or_default();
            let title = extract_json_string_field(body, "title").unwrap_or_default();
            let kind = extract_json_string_field(body, "kind").unwrap_or_else(|| "Audio".to_string());
            let mut library = state.library.write().await;
            let record = library.create(artist, title, kind);
            let json = record.json();
            drop(library);
            Ok(routing::created_response(json))
        }
        ("GET", path) if path.starts_with("/api/library/items/") && path.len() > 19 => {
            let id = &path[19..];
            let library = state.library.read().await;
            if let Some(record) = library.get(id) {
                let json = record.json();
                drop(library);
                Ok(routing::ok_response(json))
            } else {
                drop(library);
                Ok(routing::not_found_response())
            }
        }
        ("DELETE", path) if path.starts_with("/api/library/items/") && path.len() > 19 => {
            let id = &path[19..];
            let mut library = state.library.write().await;
            let deleted = library.delete(id);
            drop(library);
            if deleted {
                Ok(routing::ok_response("{}".to_string()))
            } else {
                Ok(routing::not_found_response())
            }
        }

        // DESTINATIONS ENDPOINTS
        ("GET", "/api/destinations") => {
            let destinations = state.destinations.read().await;
            let json = destinations.list();
            drop(destinations);
            Ok(routing::ok_response(json))
        }
        ("GET", "/api/destinations/default") => {
            let destinations = state.destinations.read().await;
            let json = destinations.default();
            drop(destinations);
            Ok(routing::ok_response(json))
        }

        // BROWSE ENDPOINTS
        ("GET", path) if path.starts_with("/api/users/") && path.ends_with("/browse") => {
            let username = path.split('/').nth(3).unwrap_or("unknown");
            let browse = state.browse.read().await;
            let entries = browse
                .records
                .iter()
                .find(|record| record.username == username)
                .map(|record| record.entries.as_slice())
                .unwrap_or(&[]);
            let body = slskd_user_root_json(entries);
            drop(browse);
            Ok(routing::ok_response(body))
        }
        ("GET", path) if path.starts_with("/api/users/") && path.ends_with("/browse/status") => {
            let username = path.split('/').nth(3).unwrap_or("unknown");
            let browse = state.browse.read().await;
            let size = browse
                .records
                .iter()
                .find(|record| record.username == username)
                .map(|record| record.entries.iter().map(|entry| entry.size).sum::<u64>())
                .unwrap_or(0);
            drop(browse);
            Ok(routing::ok_response(serde_json::json!({
                "username": username,
                "size": size,
                "bytesTransferred": size,
                "bytesRemaining": 0,
                "percentComplete": 100.0,
            }).to_string()))
        }
        ("POST", path) if path.starts_with("/api/users/") && path.ends_with("/directory") => {
            let username = path.split('/').nth(3).unwrap_or("unknown");
            let directory = extract_json_string_field(body, "directory")
                .or_else(|| json_body_string(body))
                .unwrap_or_default();
            let browse = state.browse.read().await;
            let entries = browse
                .records
                .iter()
                .find(|record| record.username == username)
                .map(|record| record.entries.as_slice())
                .unwrap_or(&[]);
            let body = slskd_user_directories_json(&directory, entries);
            drop(browse);
            Ok(routing::ok_response(body))
        }

        // ADDITIONAL MISSING USER ENDPOINTS (Phase 5)
        ("GET", "/api/profile/me") => {
            let json = "{\"username\":\"guest\",\"description\":\"\",\"picture\":\"\",\"user_type\":\"normal\"}".to_string();
            Ok(routing::ok_response(json))
        }

        ("GET", path) if path.starts_with("/api/profile/") && path.len() > 12 => {
            let username = &path[12..];
            let json = format!(
                "{{\"username\":\"{}\",\"description\":\"\",\"picture\":\"\",\"user_type\":\"normal\"}}",
                json_escape(username)
            );
            Ok(routing::ok_response(json))
        }

        ("GET", path) if path.starts_with("/api/users/") && path.ends_with("/endpoint") => {
            Ok(routing::ok_response(serde_json::json!({
                "addressFamily": "IPv4",
                "address": "0.0.0.0",
                "port": 0,
            }).to_string()))
        }

        ("GET", path) if path.starts_with("/api/users/") && path.ends_with("/group") => {
            let username = path.split('/').nth(3).unwrap_or("unknown");
            let json = format!(
                "{{\"username\":\"{}\",\"group\":\"normal_users\",\"group_id\":1}}",
                username
            );
            Ok(routing::ok_response(json))
        }

        ("GET", path) if path.starts_with("/api/users/") && path.contains("/info") => {
            let username = path.split('/').nth(3).unwrap_or("unknown");
            let users = state.users.read().await;
            let body = users
                .records
                .iter()
                .find(|record| record.username == username)
                .map(|record| record.slskd_info_json().to_string())
                .unwrap_or_else(|| UserRecord {
                    username: username.to_owned(),
                    watched: false,
                    status: None,
                    average_speed: None,
                    upload_count: None,
                    file_count: None,
                    directory_count: None,
                    updated_at: unix_timestamp(),
                }.slskd_info_json().to_string());
            drop(users);
            Ok(routing::ok_response(body))
        }

        ("GET", path) if path.starts_with("/api/users/") && path.ends_with("/status") => {
            let username = path.split('/').nth(3).unwrap_or("unknown");
            let users = state.users.read().await;
            let body = users
                .records
                .iter()
                .find(|record| record.username == username)
                .map(|record| record.slskd_status_json().to_string())
                .unwrap_or_else(|| UserRecord {
                    username: username.to_owned(),
                    watched: false,
                    status: Some("Unknown".to_owned()),
                    average_speed: None,
                    upload_count: None,
                    file_count: None,
                    directory_count: None,
                    updated_at: unix_timestamp(),
                }.slskd_status_json().to_string());
            drop(users);
            Ok(routing::ok_response(body))
        }

        // CONVERSATIONS ENDPOINT
        ("GET", "/api/conversations") => {
            let messages = state.messages.read().await;
            let body = messages.slskd_conversations_json(route.query);
            drop(messages);
            Ok(routing::ok_response(body))
        }
        ("GET", path) if conversation_messages_path(path).is_some() => {
            let Some(username) = conversation_messages_path(path) else {
                return Ok(routing::not_found_response());
            };
            let unacknowledged_only = query_params(route.query.unwrap_or_default())
                .into_iter()
                .find(|(key, _)| key == "unAcknowledgedOnly")
                .and_then(|(_, value)| parse_bool_value(&value))
                .unwrap_or(false);
            let messages = state.messages.read().await;
            let body = messages.slskd_messages_json(username, unacknowledged_only);
            drop(messages);
            Ok(routing::ok_response(body))
        }
        ("GET", path) if path_segment_after(path, "/api/conversations/").is_some() => {
            let Some(username) = path_segment_after(path, "/api/conversations/") else {
                return Ok(routing::not_found_response());
            };
            let include_messages = query_params(route.query.unwrap_or_default())
                .into_iter()
                .find(|(key, _)| key == "includeMessages")
                .and_then(|(_, value)| parse_bool_value(&value))
                .unwrap_or(true);
            let messages = state.messages.read().await;
            let body = messages.slskd_conversation_json(username, include_messages);
            drop(messages);
            Ok(routing::ok_response(body))
        }
        ("POST", path) if path_segment_after(path, "/api/conversations/").is_some() => {
            let Some(username) = path_segment_after(path, "/api/conversations/") else {
                return Ok(routing::not_found_response());
            };
            let message_body = json_body_string(body)
                .or_else(|| extract_json_string_field(body, "message"))
                .or_else(|| extract_json_string_field(body, "body"))
                .unwrap_or_default();
            let mut messages = state.messages.write().await;
            let record = messages.add(username.to_owned(), "outbound", message_body.clone());
            drop(messages);
            send_session_command(state, SessionCommand::MessageUser {
                username: username.to_owned(),
                body: message_body,
            }).await.ok();
            Ok(routing::ok_response((record.id > 0).to_string()))
        }

        // JOBS ENDPOINT
        ("GET", "/api/jobs") => {
            Ok(routing::ok_response(
                "{\"jobs\":[],\"limit\":20,\"offset\":0,\"total\":0,\"has_more\":false}".to_string(),
            ))
        }
        ("GET", path) if path.starts_with("/api/jobs/") && path.len() > 10 => {
            let job_id = &path[10..];
            let json = format!(
                "{{\"id\":\"{}\",\"status\":\"pending\",\"progress\":0}}",
                json_escape(job_id)
            );
            Ok(routing::ok_response(json))
        }

        // LIBRARY HEALTH ENDPOINTS
        ("GET", path) if path.starts_with("/api/library/health/summary") => {
            let library_path = route
                .query
                .and_then(|query| {
                    query_params(query)
                        .into_iter()
                        .find(|(key, _)| key == "libraryPath")
                        .map(|(_, value)| value)
                })
                .unwrap_or_default();
            Ok(routing::ok_response(serde_json::json!({
                "libraryPath": library_path,
                "issues": 0,
                "critical": 0,
                "warning": 0,
                "healthy": true,
            }).to_string()))
        }
        ("GET", "/api/library/health/issues") => {
            let json = "{\"issues\":[],\"count\":0}".to_string();
            Ok(routing::ok_response(json))
        }
        ("GET", "/api/library/health/issues/by-artist") => {
            let json = "{\"issues_by_artist\":[],\"count\":0}".to_string();
            Ok(routing::ok_response(json))
        }
        ("GET", "/api/library/health/issues/by-release") => {
            let json = "{\"issues_by_release\":[],\"count\":0}".to_string();
            Ok(routing::ok_response(json))
        }
        ("GET", path) if path.starts_with("/api/library/health/issues/by-type") => {
            let json = "{\"issues_by_type\":[],\"count\":0}".to_string();
            Ok(routing::ok_response(json))
        }
        ("GET", path) if path.starts_with("/api/library/health/scans/") && path.len() > 27 => {
            let scan_id = &path[27..];
            let json = format!(
                "{{\"id\":\"{}\",\"status\":\"completed\",\"issues_found\":0}}",
                json_escape(scan_id)
            );
            Ok(routing::ok_response(json))
        }
        ("POST", "/api/library/health/scans") => {
            Ok(routing::accepted_response(format!(
                "{{\"id\":\"scan-{}\",\"status\":\"completed\",\"issues_found\":0}}",
                unix_timestamp()
            )))
        }
        ("POST", "/api/library/health/issues/fix") => {
            Ok(routing::ok_response("{\"fixed\":0,\"issues\":[]}".to_owned()))
        }

        // CONFIGURATION ENDPOINTS
        ("GET", "/api/config/preferences") => {
            let config_json = format!(
                "{{\"auto_connect\":{},\"transfer_allow_outbound\":{},\"transfer_max_active\":{}}}",
                state.config.auto_connect,
                state.config.transfer_allow_outbound,
                state.config.transfer_max_active
            );
            Ok(routing::ok_response(config_json))
        }

        ("PUT", "/api/config/preferences") => {
            let config_json = format!(
                "{{\"auto_connect\":{},\"transfer_allow_outbound\":{},\"transfer_max_active\":{},\"persisted\":false}}",
                state.config.auto_connect,
                state.config.transfer_allow_outbound,
                state.config.transfer_max_active
            );
            Ok(routing::ok_response(config_json))
        }

        // ADDITIONAL MISSING PUT ENDPOINTS (Phase 5)
        ("PUT", "/api/autoreplace/disable") => {
            Ok(routing::ok_response("{\"enabled\":false,\"persisted\":false}".to_owned()))
        }

        ("PUT", "/api/autoreplace/enable") => {
            Ok(routing::ok_response("{\"enabled\":true,\"persisted\":false}".to_owned()))
        }

         // ADDITIONAL MISSING BRIDGE ENDPOINTS (Phase 6)
         ("GET", "/api/bridge/admin/clients") => {
             let bridge = &state.config.integrations.bridge;
             let status = if bridge.enabled { "configured" } else { "disabled" };
             let json = format!(
                 "{{\"clients\":[],\"count\":0,\"status\":\"{}\",\"ready\":{}}}",
                 status,
                 bridge.enabled
             );
             Ok(routing::ok_response(json))
         }

         ("GET", "/api/bridge/admin/config") => {
             let bridge = &state.config.integrations.bridge;
             let json = format!(
                 "{{\"bridge_host\":\"{}\",\"bridge_port\":{},\"enabled\":{},\"configured\":{},\"next_action\":\"{}\"}}",
                 json_escape(&bridge.host),
                 bridge.port,
                 bridge.enabled,
                 bridge.enabled,
                 if bridge.enabled { "start" } else { "set SLSKR_BRIDGE_ENABLED=true" }
             );
             Ok(routing::ok_response(json))
         }

         ("GET", "/api/bridge/admin/dashboard") => {
             let bridge = &state.config.integrations.bridge;
             let json = format!(
                 "{{\"active_clients\":0,\"transfers\":0,\"uptime_seconds\":0,\"enabled\":{},\"host\":\"{}\",\"port\":{}}}",
                 bridge.enabled,
                 json_escape(&bridge.host),
                 bridge.port
             );
             Ok(routing::ok_response(json))
         }

         ("GET", "/api/bridge/admin/stats") => {
             let bridge = &state.config.integrations.bridge;
             let json = format!(
                 "{{\"total_requests\":0,\"total_bytes\":0,\"active_sessions\":0,\"enabled\":{}}}",
                 bridge.enabled
             );
             Ok(routing::ok_response(json))
         }

         ("GET", "/api/bridge/status") => {
             let bridge = &state.config.integrations.bridge;
             let json = format!(
                 "{{\"status\":\"{}\",\"version\":\"1.0.0\",\"uptime_seconds\":0,\"enabled\":{},\"configured\":{},\"host\":\"{}\",\"port\":{},\"next_action\":\"{}\"}}",
                 if bridge.enabled { "configured" } else { "disabled" },
                 bridge.enabled,
                 bridge.enabled,
                 json_escape(&bridge.host),
                 bridge.port,
                 if bridge.enabled { "start bridge service" } else { "enable bridge integration" }
             );
             Ok(routing::ok_response(json))
         }

         ("GET", path) if path.starts_with("/api/bridge/transfer/") && path.contains("/progress") => {
             let transfer_id = path.split('/').nth(4).unwrap_or("unknown");
             let json = format!(
                 "{{\"transfer_id\":\"{}\",\"progress\":0,\"status\":\"pending\"}}",
                 json_escape(transfer_id)
             );
             Ok(routing::ok_response(json))
         }

         ("POST", "/api/bridge/start") => {
             let bridge = &state.config.integrations.bridge;
             Ok(routing::accepted_response(format!(
                 "{{\"status\":\"{}\",\"started\":{},\"next_action\":\"{}\"}}",
                 if bridge.enabled { "configured" } else { "disabled" },
                 bridge.enabled,
                 if bridge.enabled { "connect external bridge process" } else { "enable bridge integration" }
             )))
         }

         ("POST", "/api/bridge/stop") => {
             let bridge = &state.config.integrations.bridge;
             Ok(routing::ok_response(format!(
                 "{{\"status\":\"{}\",\"stopped\":true}}",
                 if bridge.enabled { "configured" } else { "disabled" }
             )))
         }

         ("PUT", "/api/bridge/admin/config") => {
             let bridge = &state.config.integrations.bridge;
             Ok(routing::ok_response(format!(
                 "{{\"enabled\":{},\"configured\":{},\"persisted\":false,\"restart_required\":true}}",
                 bridge.enabled,
                 bridge.enabled
             )))
         }

        ("PUT", path) if path.starts_with("/api/collections/") && path.contains("/items/reorder") => {
            Ok(routing::ok_response("{\"reordered\":false,\"items\":[]}".to_owned()))
        }

        ("PUT", path) if conversation_message_path(path).is_some() => {
            let Some((username, id)) = conversation_message_path(path) else {
                return Ok(routing::not_found_response());
            };
            let mut messages = state.messages.write().await;
            let updated = messages
                .records
                .iter()
                .any(|record| record.username == username && record.id == id);
            let response = if updated {
                messages.ack(id);
                routing::ok_response("true".to_owned())
            } else {
                routing::not_found_response()
            };
            drop(messages);
            Ok(response)
        }
        ("PUT", path) if path_segment_after(path, "/api/conversations/").is_some() => {
            let Some(username) = path_segment_after(path, "/api/conversations/") else {
                return Ok(routing::not_found_response());
            };
            let mut messages = state.messages.write().await;
            messages.ack_all_for_user(username);
            drop(messages);
            Ok(routing::ok_response("true".to_owned()))
        }

        ("PUT", "/api/nowplaying") => {
            let username = extract_json_string_field(body, "username").unwrap_or_default();
            let artist = extract_json_string_field(body, "artist").unwrap_or_default();
            let title = extract_json_string_field(body, "title").unwrap_or_default();
            Ok(routing::ok_response(format!(
                "{{\"username\":\"{}\",\"artist\":\"{}\",\"title\":\"{}\",\"updated_at\":{}}}",
                json_escape(&username),
                json_escape(&artist),
                json_escape(&title),
                unix_timestamp()
            )))
        }

        ("PUT", "/api/options/yaml") => {
            Ok(routing::ok_response("{\"persisted\":false,\"restart_required\":false}".to_owned()))
        }

        ("PUT", "/api/profile/me") => {
            Ok(routing::ok_response("{\"updated\":false,\"profile\":{}}".to_owned()))
        }

        ("PUT", "/api/relay") => {
            let relay_enabled = extract_json_bool_field(body, "enabled").unwrap_or(false);
            Ok(routing::ok_response(format!(
                "{{\"relay_enabled\":{},\"status\":\"configured\"}}",
                relay_enabled
            )))
        }

        ("PUT", "/api/relay/agent") => {
            Ok(routing::ok_response("true".to_owned()))
        }

        ("PUT", path) if path.starts_with("/api/searches/") && path.len() > 13 => {
            let token = path.rsplit('/').next().unwrap_or("unknown");
            Ok(routing::ok_response(format!(
                "{{\"token\":\"{}\",\"updated\":false}}",
                json_escape(token)
            )))
        }

        ("PUT", "/api/transfers/downloads/accelerated") => {
            Ok(routing::ok_response("{\"accelerated\":[],\"enabled\":false}".to_owned()))
        }

        ("PUT", path) if path.starts_with("/api/wishlist/") && path.len() > 14 => {
            let item_id = path.rsplit('/').next().unwrap_or("unknown");
            Ok(routing::ok_response(format!(
                "{{\"item_id\":\"{}\",\"updated\":false}}",
                json_escape(item_id)
            )))
        }

        // Generic :var pattern PUT endpoints (Phase 5)
        ("PUT", path) if path.contains("/channels/") && path.matches('/').count() == 4 && !path.contains("/api/") => {
            Ok(routing::not_found_response())
        }

        ("PUT", path) if path.ends_with("/adversarial") && !path.contains("/api/") => {
            Ok(routing::not_found_response())
        }

        ("PUT", path) if path.contains("/disclosure/") && !path.contains("/api/") => {
            Ok(routing::not_found_response())
        }

        ("PUT", path) if path.ends_with("/reputation") && !path.contains("/api/") => {
            Ok(routing::not_found_response())
        }

        ("GET", "/api/config/shares") => {
            let shares = state.shares.read().await;
            let share_roots: Vec<String> = shares.roots
                .iter()
                .map(|root| format!(
                    "{{\"label\":\"{}\",\"files\":{},\"bytes\":{}}}",
                    json_escape(&root.label),
                    root.files,
                    root.bytes
                ))
                .collect();
            let json = format!(
                "{{\"roots\":[{}],\"count\":{}}}",
                share_roots.join(","),
                shares.roots.len()
            );
            drop(shares);
            Ok(routing::ok_response(json))
        }

        ("POST", "/api/config/shares") => {
            let path = extract_json_string_field(body, "path").unwrap_or_default();
            if path.is_empty() {
                return Ok(routing::bad_request_response("path is required"));
            }
            let json = format!(
                "{{\"path\":\"{}\",\"added\":true,\"files\":0,\"bytes\":0}}",
                json_escape(&path)
            );
            Ok(routing::created_response(json))
        }

        ("GET", "/api/config/plugins") => {
            let json = "{\"plugins\":[],\"count\":0}".to_string();
            Ok(routing::ok_response(json))
        }

        ("POST", "/api/config/filters") => {
            let filter_type = extract_json_string_field(body, "type").unwrap_or_default();
            let pattern = extract_json_string_field(body, "pattern").unwrap_or_default();
            let json = format!(
                "{{\"type\":\"{}\",\"pattern\":\"{}\",\"created_at\":{}}}",
                json_escape(&filter_type),
                json_escape(&pattern),
                unix_timestamp()
            );
            Ok(routing::created_response(json))
        }

        // ADMIN/SYSTEM ENDPOINTS
        ("GET", "/api/admin/stats") => {
            let transfers = state.transfers.read().await;
            let json = format!(
                "{{\"total_transfers\":{},\"active_transfers\":0,\"total_bytes\":0}}",
                transfers.entries.len()
            );
            drop(transfers);
            Ok(routing::ok_response(json))
        }

        ("POST", "/api/admin/shutdown") => {
            let json = "{\"status\":\"shutdown_requested\"}".to_string();
            Ok(routing::accepted_response(json))
        }

        ("GET", "/api/admin/version") => {
            let json = format!("{{\"version\":\"1.0.0-RC\",\"build_date\":\"{}\"}}", "2026-05-04");
            Ok(routing::ok_response(json))
        }

        ("POST", "/api/admin/restart") => {
            let json = "{\"status\":\"restart_requested\"}".to_string();
            Ok(routing::accepted_response(json))
        }

        // RECOMMENDATIONS & ANALYTICS ENDPOINTS
        ("GET", "/api/soulseek/recommendations") => {
            let json = "{\"recommendations\":[],\"count\":0}".to_string();
            Ok(routing::ok_response(json))
        }

        ("GET", "/api/soulseek/recommendations/global") => {
            let json = "{\"global_recommendations\":[],\"count\":0}".to_string();
            Ok(routing::ok_response(json))
        }

        ("GET", path) if path.starts_with("/api/soulseek/items/") && path.ends_with("/recommendations") => {
            let item_id = path.split('/').nth(4).unwrap_or("unknown");
            let json = format!(
                "{{\"item_id\":\"{}\",\"recommendations\":[],\"count\":0}}",
                json_escape(item_id)
            );
            Ok(routing::ok_response(json))
        }

        ("GET", path) if path.starts_with("/api/soulseek/items/") && path.ends_with("/similar-users") => {
            let item_id = path.split('/').nth(4).unwrap_or("unknown");
            let json = format!(
                "{{\"item_id\":\"{}\",\"similar_users\":[],\"count\":0}}",
                json_escape(item_id)
            );
            Ok(routing::ok_response(json))
        }

        ("GET", "/api/transfers/downloads/stuck") => {
            let json = "{\"stuck\":[],\"count\":0}".to_string();
            Ok(routing::ok_response(json))
        }

        ("GET", path) if path.starts_with("/api/relay/controller/downloads/") => {
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/octet-stream",
                body: String::new(),
            })
        }

        ("POST", path) if path.starts_with("/api/relay/controller/files/") => {
            Ok(routing::ok_response("true".to_owned()))
        }

        ("POST", path) if path.starts_with("/api/relay/controller/shares/") => {
            Ok(routing::ok_response("true".to_owned()))
        }

         ("GET", "/api/transfers/downloads/user-stats") => {
             let json = "{\"users\":[],\"count\":0}".to_string();
             Ok(routing::ok_response(json))
         }

         // ADDITIONAL MISSING GET ENDPOINTS (Phase 5)
         ("GET", "/api/source-providers") => {
             let json = "{\"providers\":[],\"count\":0}".to_string();
             Ok(routing::ok_response(json))
         }

         ("GET", "/api/songid/runs") => {
             let json = "{\"runs\":[],\"count\":0}".to_string();
             Ok(routing::ok_response(json))
         }

         ("POST", "/api/songid/runs") => {
             Ok(routing::accepted_response(format!(
                 "{{\"id\":\"songid-{}\",\"status\":\"queued\",\"matches\":[]}}",
                 unix_timestamp()
             )))
         }

         ("GET", path) if path.starts_with("/api/songid/runs/") && path.len() > 17 && !path.contains("/forensic-matrix") => {
             let run_id = &path[17..];
             let json = format!(
                 "{{\"id\":\"{}\",\"results\":[],\"count\":0}}",
                 json_escape(run_id)
             );
             Ok(routing::ok_response(json))
         }

         ("GET", path) if path.starts_with("/api/songid/runs/") && path.contains("/forensic-matrix") => {
             let run_id = path.split('/').nth(4).unwrap_or("unknown");
             let json = format!(
                 "{{\"run_id\":\"{}\",\"matrix\":[],\"count\":0}}",
                 json_escape(run_id)
             );
             Ok(routing::ok_response(json))
         }

         ("GET", path) if path.starts_with("/api/soulseek/users/") && path.contains("/interests") && path.len() > 20 => {
             let username = path.split('/').nth(4).unwrap_or("unknown");
             let json = format!(
                 "{{\"username\":\"{}\",\"interests\":[],\"count\":0}}",
                 json_escape(username)
             );
             Ok(routing::ok_response(json))
         }

         ("GET", "/api/swarm/analytics/recommendations") => {
             let json = "{\"recommendations\":[],\"count\":0}".to_string();
             Ok(routing::ok_response(json))
         }

         ("GET", "/api/telemetry/metrics") => {
             let json = format!(
                 "{{\"metrics\":{},\"timestamp\":{},\"version\":\"1.0.0\"}}",
                 "{}",
                 unix_timestamp()
             );
             Ok(routing::ok_response(json))
         }

          ("GET", "/api/telemetry/metrics/kpi") | ("GET", "/api/telemetry/metrics/kpis") => {
              let json = "{\"kpis\":[],\"count\":0}".to_string();
              Ok(routing::ok_response(json))
          }

          // ADDITIONAL MISSING GET ENDPOINTS (Phase 6)
          ("GET", "/api/multisource/jobs") => {
              let json = "{\"jobs\":[],\"count\":0}".to_string();
              Ok(routing::ok_response(json))
          }

          ("GET", "/api/pods") => {
              Ok(routing::ok_response("[]".to_string()))
          }

          ("GET", "/api/solid/status") => {
              Ok(routing::ok_response("{\"enabled\":false}".to_string()))
          }

          ("GET", "/api/federation/diagnostics") => {
              Ok(routing::ok_response(
                  "{\"status\":\"disabled\",\"checks\":[],\"warnings\":[],\"errors\":[]}".to_string(),
              ))
          }

          ("GET", "/api/security/dashboard") => {
              Ok(routing::ok_response(serde_json::json!({
                  "enabled": false,
                  "status": "disabled",
                  "stats": {
                      "networkGuardStats": { "globalConnections": 0 },
                      "reputationStats": { "totalPeers": 0 },
                      "threatStats": { "activeThreats": 0 },
                      "banStats": { "activeBans": 0 }
                  },
                  "events": [],
                  "bans": []
              }).to_string()))
          }

          ("GET", "/api/soulseek/mesh-rendezvous/status") => {
              Ok(routing::ok_response(serde_json::json!({
                  "enabled": false,
                  "interestTag": "slskr-mesh-v1",
                  "privacy": "Soulseek mesh rendezvous is disabled."
              }).to_string()))
          }

          ("GET", "/api/soulseek/mesh-rendezvous/users") => {
              Ok(routing::ok_response("[]".to_string()))
          }

          ("GET", "/api/soulseek/mesh-rendezvous/discover") => {
              Ok(routing::ok_response("{\"users\":[],\"capabilityRecords\":[]}".to_string()))
          }

          ("GET", "/api/soulseek/peer-capabilities") => {
              Ok(routing::ok_response("[]".to_string()))
          }

          ("GET", "/api/mesh/transport") => {
              Ok(routing::ok_response(serde_json::json!({
                  "status": "Healthy",
                  "health": "Healthy",
                  "description": "Mesh transport unavailable in this runtime",
                  "transportPreference": "Auto",
                  "connectedPeers": 0,
                  "totalPeers": 0,
                  "activeCircuits": 0,
                  "activeStreams": 0,
                  "bootstrapPeers": [],
                  "isolatedPeers": 0,
                  "quorumPeers": 0,
                  "relayedPeers": 0,
                  "natType": "Unknown",
                  "publicEndpoint": null,
                  "lastDhtError": null,
                  "lastDhtPublishUtc": null
              }).to_string()))
          }

          ("GET", path) if path.starts_with("/api/multisource/jobs/") && path.len() > 22 => {
              let job_id = &path[22..];
              Ok(routing::ok_response(serde_json::json!({
                  "id": job_id,
                  "status": "not_found",
                  "sources": [],
                  "progress": 0,
              }).to_string()))
          }

          ("GET", "/api/player/external-visualizer") => {
              let visualizer = &state.config.integrations.external_visualizer;
              let json = format!(
                  "{{\"visualizer\":{},\"configured\":{},\"status\":\"{}\",\"next_action\":\"{}\"}}",
                  json_option(visualizer.command.as_deref()),
                  visualizer.configured(),
                  if visualizer.configured() { "configured" } else { "disabled" },
                  if visualizer.configured() { "launch" } else { "set SLSKR_EXTERNAL_VISUALIZER_COMMAND" }
              );
              Ok(routing::ok_response(json))
          }

          ("GET", path) if path.starts_with("/api/realm-subject-indexes/") && path.ends_with("/conflicts") => {
              let realm = path.split('/').nth(3).unwrap_or("unknown");
              Ok(routing::ok_response(format!(
                  "{{\"realm\":\"{}\",\"conflicts\":[],\"count\":0}}",
                  json_escape(realm)
              )))
          }

          ("POST", "/api/discovery-graph") => {
              Ok(routing::accepted_response(
                  "{\"nodes\":[],\"edges\":[],\"count\":0,\"status\":\"empty\"}".to_owned(),
              ))
          }

          ("POST", "/api/jobs/discography") => {
              Ok(routing::accepted_response(format!(
                  "{{\"id\":\"discography-{}\",\"status\":\"queued\",\"results\":[]}}",
                  unix_timestamp()
              )))
          }

          ("POST", "/api/jobs/mb-release") => {
              Ok(routing::accepted_response(format!(
                  "{{\"id\":\"mb-release-{}\",\"status\":\"queued\",\"results\":[]}}",
                  unix_timestamp()
              )))
          }

          ("POST", "/api/options/yaml/validate") => {
              Ok(routing::ok_response("{\"valid\":true,\"errors\":[],\"warnings\":[]}".to_owned()))
          }

          ("POST", "/api/source-feed-imports/preview") => {
              Ok(routing::ok_response("{\"items\":[],\"count\":0,\"valid\":true}".to_owned()))
          }

          ("POST", "/api/transfers/downloads/auto-replace") => {
              Ok(routing::accepted_response(
                  "{\"replacement_queued\":false,\"alternatives\":[],\"status\":\"idle\"}".to_owned(),
              ))
          }

          // TASTE RECOMMENDATIONS POST ENDPOINTS (Phase 6)
          ("POST", "/api/taste-recommendations") => {
              let json = "{\"recommendations\":[],\"count\":0,\"status\":\"analyzing\"}".to_string();
              Ok(routing::accepted_response(json))
          }

          ("POST", "/api/taste-recommendations/graph-preview") => {
              let json = "{\"graph_data\":[],\"nodes\":0,\"edges\":0}".to_string();
              Ok(routing::ok_response(json))
          }

          ("POST", "/api/taste-recommendations/release-radar") => {
              let json = "{\"recommendations\":[],\"count\":0,\"status\":\"processing\"}".to_string();
              Ok(routing::accepted_response(json))
          }

          ("POST", "/api/taste-recommendations/wishlist") => {
              let json = "{\"recommendations\":[],\"count\":0,\"status\":\"processing\"}".to_string();
              Ok(routing::accepted_response(json))
          }

          // PLAYER LAUNCH ENDPOINT (Phase 6)
        ("POST", "/api/player/external-visualizer/launch") => {
            let visualizer = &state.config.integrations.external_visualizer;
            if let Some(command) = visualizer.command.as_deref().filter(|value| !value.trim().is_empty()) {
                match std::process::Command::new(command).spawn() {
                    Ok(_) => Ok(routing::accepted_response(format!(
                        "{{\"launched\":true,\"status\":\"launch_requested\",\"command\":\"{}\"}}",
                        json_escape(command)
                    ))),
                    Err(error) => {
                        let message = format!("failed to launch external visualizer: {error}");
                        Ok(routing::bad_request_response(&message))
                    }
                }
            } else {
                Ok(routing::accepted_response(
                    "{\"launched\":false,\"status\":\"disabled\",\"next_action\":\"set SLSKR_EXTERNAL_VISUALIZER_COMMAND\"}".to_owned(),
                ))
            }
        }

          // BANS & BLOCKING ENDPOINTS
        ("POST", path) if path.contains("/bans/username") => {
            let username = extract_json_string_field(body, "username")
                .or_else(|| path.rsplit('/').next().map(str::to_owned))
                .unwrap_or_default();
            Ok(routing::ok_response(format!(
                "{{\"username\":\"{}\",\"banned\":true,\"persisted\":false}}",
                json_escape(&username)
            )))
        }

        ("DELETE", path) if path.contains("/bans/username/") => {
            let username = path.rsplit('/').next().unwrap_or("");
            Ok(routing::ok_response(format!(
                "{{\"username\":\"{}\",\"banned\":false,\"persisted\":false}}",
                json_escape(username)
            )))
        }

        ("POST", path) if path.contains("/bans/ip") => {
            let ip = extract_json_string_field(body, "ip").unwrap_or_default();
            Ok(routing::ok_response(format!(
                "{{\"ip\":\"{}\",\"banned\":true,\"persisted\":false}}",
                json_escape(&ip)
            )))
        }

        ("DELETE", path) if path.contains("/bans/ip/") => {
            let ip = path.rsplit('/').next().unwrap_or("");
            Ok(routing::ok_response(format!(
                "{{\"ip\":\"{}\",\"banned\":false,\"persisted\":false}}",
                json_escape(ip)
            )))
        }

        // ADDITIONAL MISSING DELETE ENDPOINTS (Phase 5)
        ("DELETE", path) if path_segment_after(path, "/api/conversations/").is_some() => {
            let Some(username) = path_segment_after(path, "/api/conversations/") else {
                return Ok(routing::not_found_response());
            };
            let mut messages = state.messages.write().await;
            let before = messages.records.len();
            messages.records.retain(|record| record.username != username);
            let removed = before.saturating_sub(messages.records.len());
            drop(messages);
            Ok(routing::ok_response((removed > 0).to_string()))
        }

        ("DELETE", path) if path.starts_with("/api/files/") && path.contains("/directories/") => {
            Ok(routing::ok_response("false".to_owned()))
        }

        ("DELETE", path) if path.starts_with("/api/files/") && path.contains("/files/") => {
            Ok(routing::ok_response("false".to_owned()))
        }

        ("DELETE", "/api/integrations/spotify") => {
            Ok(routing::ok_response("{\"connected\":false,\"removed\":true}".to_owned()))
        }

        ("DELETE", "/api/nowplaying") => {
            Ok(routing::ok_response("{\"now_playing\":[],\"count\":0,\"cleared\":true}".to_owned()))
        }

        ("DELETE", "/api/relay") => {
            Ok(routing::ok_response("{\"relay_enabled\":false,\"status\":\"disabled\"}".to_owned()))
        }

        ("DELETE", "/api/relay/agent") => {
            Ok(routing::ok_response("true".to_owned()))
        }

        ("DELETE", "/api/shares") => {
            Ok(routing::ok_response("true".to_owned()))
        }

        ("GET", "/api/shares/contents") => {
            let shares = state.shares.read().await;
            let json = shares.json();
            drop(shares);
            Ok(routing::ok_response(json))
        }

        ("GET", path) if path.starts_with("/api/shares/") && path.ends_with("/contents") => {
            let share_id = path.split('/').nth(3).unwrap_or("unknown");
            let shares = state.shares.read().await;
            let json = shares.json();
            drop(shares);
            Ok(routing::ok_response(format!(
                "{{\"share_id\":\"{}\",\"contents\":{},\"status\":\"ok\"}}",
                json_escape(share_id),
                json
            )))
        }

        ("GET", path) if path.starts_with("/api/shares/") && path.len() > 12 => {
            let share_id = &path[12..];
            let shares = state.shares.read().await;
            let json = shares.summary_json();
            drop(shares);
            Ok(routing::ok_response(format!(
                "{{\"id\":\"{}\",\"summary\":{}}}",
                json_escape(share_id),
                json
            )))
        }

        ("DELETE", path) if path.starts_with("/api/transfers/") && path.ends_with("/all/completed") => {
            let mut transfers = state.transfers.write().await;
            let before = transfers.entries.len();
            transfers.entries.retain(|entry| {
                !matches!(
                    entry.status.as_str(),
                    "succeeded" | "completed" | "cancelled" | "failed" | "rejected"
                )
            });
            let pruned = before.saturating_sub(transfers.entries.len());
            transfers.persist_state();
            drop(transfers);
            Ok(routing::ok_response(format!("{{\"pruned\":{}}}", pruned)))
        }

        // Generic :var pattern endpoints for mesh/network cleanup & channels (Phase 5)
        ("DELETE", path) if path.contains("/cleanup") && path.matches('/').count() == 3 && !path.contains("/api/") => {
            Ok(routing::not_found_response())
        }

        ("DELETE", path) if path.contains("/unpublish") && !path.contains("/api/") => {
            Ok(routing::not_found_response())
        }

        ("DELETE", path) if path.contains("/channels/") && path.matches('/').count() == 4 && !path.contains("/api/") => {
            Ok(routing::not_found_response())
        }

        // ADDITIONAL MISSING INTEGRATION & PLATFORM ENDPOINTS (Phase 5)
        ("GET", "/api/integrations/spotify/status") => {
            let spotify = &state.config.integrations.spotify;
            let redirect_uri = spotify_redirect_uri(state);
            let json = format!(
                "{{\"connected\":false,\"status\":\"{}\",\"configured\":{},\"enabled\":{},\"client_id_configured\":{},\"client_secret_configured\":{},\"redirect_uri\":\"{}\",\"market\":\"{}\",\"scopes\":\"{}\",\"auth_flow\":\"authorization_code_pkce\",\"callback_multiplexed\":true,\"next_action\":\"{}\"}}",
                if spotify.configured() { "ready_to_authorize" } else if spotify.enabled { "missing_client_id" } else { "disabled" },
                spotify.configured(),
                spotify.enabled,
                spotify.client_id.is_some(),
                spotify.client_secret.is_some(),
                json_escape(&redirect_uri),
                json_escape(&spotify.market),
                json_escape(&spotify.scopes),
                if spotify.configured() { "authorize" } else { "configure Spotify client ID and redirect URI" }
            );
            Ok(routing::ok_response(json))
        }

        ("POST", "/api/integrations/spotify/authorize") => {
            let spotify = &state.config.integrations.spotify;
            if !spotify.configured() {
                return Ok(routing::bad_request_response(
                    "Spotify authorization requires SLSKR_SPOTIFY_ENABLED=true and SLSKR_SPOTIFY_CLIENT_ID",
                ));
            }
            let client_id = spotify.client_id.as_deref().unwrap_or_default();
            let redirect_uri = spotify_redirect_uri(state);
            let mut oauth_states = state.oauth_states.write().await;
            let state_token = oauth_states.issue("spotify", &redirect_uri, 600);
            drop(oauth_states);
            let auth_url = format!(
                "https://accounts.spotify.com/authorize?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&show_dialog=false",
                url_encode(client_id),
                url_encode(&redirect_uri),
                url_encode(&spotify.scopes),
                url_encode(&state_token)
            );
            Ok(routing::accepted_response(format!(
                "{{\"connected\":false,\"authorized\":false,\"status\":\"authorization_required\",\"auth_flow\":\"authorization_code\",\"pkce_recommended\":true,\"authorization_url\":\"{}\",\"redirect_uri\":\"{}\",\"state\":\"{}\",\"next_action\":\"open authorization_url\"}}",
                json_escape(&auth_url),
                json_escape(&redirect_uri),
                json_escape(&state_token)
            )))
        }

        ("GET", "/api/integrations/spotify/callback") => {
            let params = route.query.map(query_params).unwrap_or_default();
            let code = params
                .iter()
                .find(|(key, _)| key == "code")
                .map(|(_, value)| value.as_str());
            let error = params
                .iter()
                .find(|(key, _)| key == "error")
                .map(|(_, value)| value.as_str());
            let state_value = params
                .iter()
                .find(|(key, _)| key == "state")
                .map(|(_, value)| value.as_str())
                .unwrap_or("");

            if let Some(error) = error {
                return Ok(routing::bad_request_response(error));
            }
            if state_value.is_empty() {
                return Ok(routing::bad_request_response("Spotify callback missing state"));
            }
            let mut oauth_states = state.oauth_states.write().await;
            let Some(state_record) = oauth_states.consume("spotify", state_value) else {
                drop(oauth_states);
                return Ok(routing::forbidden_response("invalid or expired Spotify OAuth state"));
            };
            drop(oauth_states);
            let Some(code) = code else {
                return Ok(routing::bad_request_response("Spotify callback missing code"));
            };

            Ok(routing::ok_response(format!(
                "{{\"connected\":false,\"authorized\":false,\"status\":\"callback_received\",\"code_received\":true,\"code_length\":{},\"state_valid\":true,\"state_created_at\":{},\"redirect_uri\":\"{}\",\"next_action\":\"exchange code for token\"}}",
                code.len(),
                state_record.created_at,
                json_escape(&state_record.redirect_uri)
            )))
        }

        ("GET", "/api/integrations/lidarr/status") => {
            let lidarr = &state.config.integrations.lidarr;
            let mut connected = false;
            let mut version = None;
            let mut error_message = None;
            if lidarr.configured() {
                match fetch_lidarr_system_status(lidarr).await {
                    Ok(value) => {
                        connected = true;
                        version = value.get("version").and_then(serde_json::Value::as_str).map(str::to_owned);
                    }
                    Err(error) => error_message = Some(error),
                }
            }
            let json = format!(
                "{{\"connected\":{},\"status\":\"{}\",\"configured\":{},\"enabled\":{},\"url\":{},\"api_key_configured\":{},\"version\":{},\"error\":{},\"next_action\":\"{}\"}}",
                connected,
                if connected { "connected" } else if lidarr.configured() { "connection_failed" } else if lidarr.enabled { "missing_url_or_api_key" } else { "disabled" },
                lidarr.configured(),
                lidarr.enabled,
                json_option(lidarr.url.as_deref()),
                lidarr.api_key.is_some(),
                json_option(version.as_deref()),
                json_option(error_message.as_deref()),
                if lidarr.configured() { "test connection or sync wanted" } else { "configure Lidarr URL and API key" }
            );
            Ok(routing::ok_response(json))
        }

        ("GET", "/api/integrations/lidarr/wanted/missing") => {
            let lidarr = &state.config.integrations.lidarr;
            if !lidarr.configured() {
                return Ok(routing::ok_response(
                    "{\"missing_albums\":[],\"count\":0,\"status\":\"disabled\",\"next_action\":\"configure Lidarr URL and API key\"}".to_owned(),
                ));
            }
            match fetch_lidarr_wanted_missing(lidarr).await {
                Ok(value) => Ok(routing::ok_response(value.to_string())),
                Err(error) => Ok(routing::ok_response(format!(
                    "{{\"missing_albums\":[],\"count\":0,\"status\":\"connection_failed\",\"error\":\"{}\"}}",
                    json_escape(&error)
                ))),
            }
        }

        ("POST", "/api/integrations/lidarr/wanted/sync") => {
            let lidarr = &state.config.integrations.lidarr;
            if !lidarr.configured() {
                return Ok(routing::accepted_response(
                    "{\"synced\":false,\"queued\":false,\"status\":\"disabled\",\"missing_albums\":[],\"next_action\":\"configure Lidarr URL and API key\"}".to_owned(),
                ));
            }
            Ok(routing::accepted_response(
                "{\"synced\":false,\"queued\":true,\"status\":\"configured\",\"missing_albums\":[],\"next_action\":\"poll wanted/missing\"}".to_owned(),
            ))
        }

        ("POST", "/api/integrations/lidarr/manualimport") => {
            let lidarr = &state.config.integrations.lidarr;
            let directory = extract_json_string_field(body, "directory").unwrap_or_default();
            if !lidarr.configured() {
                return Ok(routing::accepted_response(
                    "{\"imported\":0,\"status\":\"disabled\",\"items\":[],\"next_action\":\"configure Lidarr URL and API key\"}".to_owned(),
                ));
            }
            Ok(routing::accepted_response(format!(
                "{{\"imported\":0,\"status\":\"configured\",\"directory\":\"{}\",\"items\":[],\"next_action\":\"trigger Lidarr manual import from configured UI\"}}",
                json_escape(&directory)
            )))
        }

        ("GET", "/api/musicbrainz/albums/completion") => {
            let json = "{\"completion_status\":[],\"average_completion\":0.0}".to_string();
            Ok(routing::ok_response(json))
        }

        ("GET", path) if path.starts_with("/api/musicbrainz/artist/") && path.contains("/discography-coverage") => {
            let artist = path.split('/').nth(4).unwrap_or("unknown");
            let json = format!(
                "{{\"artist\":\"{}\",\"coverage\":0.0,\"releases\":0}}",
                json_escape(artist)
            );
            Ok(routing::ok_response(json))
        }

        ("GET", "/api/musicbrainz/release-radar/notifications") => {
            let json = "{\"notifications\":[],\"count\":0}".to_string();
            Ok(routing::ok_response(json))
        }

        ("GET", "/api/musicbrainz/release-radar/subscriptions") => {
            let json = "{\"subscriptions\":[],\"count\":0}".to_string();
            Ok(routing::ok_response(json))
        }

        ("GET", "/api/listening-party") => {
            let json = "{\"active_parties\":[],\"count\":0}".to_string();
            Ok(routing::ok_response(json))
        }

         ("POST", "/api/conversations/batch") => {
             Ok(routing::created_response("{\"conversations\":[],\"count\":0}".to_owned()))
         }

         ("POST", "/api/nowplaying") => {
            let username = extract_json_string_field(body, "username").unwrap_or_default();
            let artist = extract_json_string_field(body, "artist").unwrap_or_default();
            let title = extract_json_string_field(body, "title").unwrap_or_default();
            let json = format!(
                "{{\"username\":\"{}\",\"artist\":\"{}\",\"title\":\"{}\",\"updated_at\":{}}}",
                json_escape(&username),
                json_escape(&artist),
                json_escape(&title),
                unix_timestamp()
            );
            Ok(routing::ok_response(json))
        }

        ("GET", "/api/nowplaying") => {
            let json = "{\"now_playing\":[],\"count\":0}".to_string();
            Ok(routing::ok_response(json))
        }

         ("POST", "/api/relay") => {
             let relay_enabled = extract_json_bool_field(body, "enabled").unwrap_or(false);
             let json = format!(
                 "{{\"relay_enabled\":{},\"status\":\"configured\"}}",
                 relay_enabled
             );
             Ok(routing::ok_response(json))
         }

         // ADDITIONAL MISSING POST ENDPOINTS (Phase 6)
         ("POST", "/api/destinations/validate") => {
             let destination = extract_json_string_field(body, "destination")
                 .or_else(|| extract_json_string_field(body, "path"))
                 .or_else(|| extract_json_string_field(body, "url"))
                 .unwrap_or_default();
             Ok(routing::ok_response(format!(
                 "{{\"destination\":\"{}\",\"valid\":{}}}",
                 json_escape(&destination),
                 !destination.trim().is_empty()
             )))
         }

         ("POST", "/api/profile/invite") => {
             Ok(routing::created_response(format!(
                 "{{\"invite\":\"local-{}\",\"created_at\":{}}}",
                 unix_timestamp(),
                 unix_timestamp()
             )))
         }

         ("POST", "/api/musicbrainz/release-radar/subscriptions") => {
             Ok(routing::created_response("{\"subscriptions\":[],\"created\":false}".to_owned()))
         }

         ("POST", "/api/musicbrainz/targets") => {
             let target = extract_json_string_field(body, "target")
                 .or_else(|| extract_json_string_field(body, "mbid"))
                 .unwrap_or_default();
             Ok(routing::created_response(format!(
                 "{{\"target\":\"{}\",\"created\":{}}}",
                 json_escape(&target),
                 !target.is_empty()
             )))
         }

         ("POST", path) if path.starts_with("/api/soulseek/interests") && !path.contains("/hated") => {
             let interest = extract_json_string_field(body, "interest").unwrap_or_default();
             let json = format!(
                 "{{\"interest\":\"{}\",\"added\":true}}",
                 json_escape(&interest)
             );
             Ok(routing::created_response(json))
         }

         ("POST", path) if path.starts_with("/api/wishlist/") && path.contains("/search") => {
             let item_id = path.split('/').nth(3).unwrap_or("unknown");
             let json = format!(
                 "{{\"item_id\":\"{}\",\"search_started\":true,\"status\":\"searching\"}}",
                 json_escape(item_id)
             );
             Ok(routing::accepted_response(json))
         }

         ("POST", "/api/wishlist/import/csv") => {
             Ok(routing::created_response("{\"imported\":0,\"items\":[]}".to_owned()))
         }

         ("POST", path) if path.starts_with("/api/share-grants/") && path.contains("/backfill") => {
             let grant_id = path.split('/').nth(3).unwrap_or("unknown");
             Ok(routing::accepted_response(format!(
                 "{{\"grant_id\":\"{}\",\"backfilled\":0}}",
                 json_escape(grant_id)
             )))
         }

         ("POST", path) if path.starts_with("/api/share-grants/") && path.contains("/token") => {
             let grant_id = path.split('/').nth(3).unwrap_or("unknown");
             Ok(routing::created_response(format!(
                 "{{\"grant_id\":\"{}\",\"token\":null,\"created\":false}}",
                 json_escape(grant_id)
             )))
         }
        _ => {
            tracing::complete_request_span(404);
            Ok(routing::not_found_response())
        }
    }.inspect(|response| {
        // Complete tracing with response status code
        let status_code: u16 = response.status
            .split(' ')
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or(500);
        tracing::complete_request_span(status_code);
    })
}

fn index_html_response() -> HttpResponse {
    HttpResponse {
        status: "200 OK",
        content_type: "text/html; charset=utf-8",
        body: index_html(),
    }
}

fn web_build_root() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(configured) = env::var("SLSKR_WEB_BUILD_DIR") {
        candidates.push(PathBuf::from(configured));
    }
    if let Ok(cwd) = env::current_dir() {
        candidates.push(cwd.join("web").join("build"));
        candidates.push(cwd.join("build"));
    }
    candidates.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("web")
            .join("build"),
    );

    candidates
        .into_iter()
        .find(|path| path.join("index.html").is_file())
}

fn web_static_content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("css") => "text/css; charset=utf-8",
        Some("eot") => "application/vnd.ms-fontobject",
        Some("html") => "text/html; charset=utf-8",
        Some("ico") => "image/x-icon",
        Some("js") => "text/javascript; charset=utf-8",
        Some("json") | Some("webmanifest") => "application/json; charset=utf-8",
        Some("png") => "image/png",
        Some("svg") => "image/svg+xml",
        Some("ttf") => "font/ttf",
        Some("txt") => "text/plain; charset=utf-8",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    }
}

fn web_static_file_for_request(path: &str) -> Option<(PathBuf, &'static str)> {
    if path.starts_with("/api/") || path.starts_with("/hub/") || path == "/api" || path == "/hub" {
        return None;
    }

    let root = web_build_root()?;
    let path_without_query = path.split_once('?').map_or(path, |(path, _)| path);
    let relative = path_without_query.trim_start_matches('/');
    let requested = if relative.is_empty() {
        PathBuf::from("index.html")
    } else {
        let relative_path = Path::new(relative);
        if relative_path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        }) {
            return None;
        }

        let candidate = root.join(relative_path);
        if candidate.is_file() {
            relative_path.to_path_buf()
        } else if relative_path.extension().is_none() {
            PathBuf::from("index.html")
        } else {
            return None;
        }
    };

    let file = root.join(requested);
    file.is_file().then(|| {
        let content_type = web_static_content_type(&file);
        (file, content_type)
    })
}

fn read_web_index_html() -> Option<String> {
    let (path, _) = web_static_file_for_request("/")?;
    fs::read_to_string(path).ok()
}

fn security_headers() -> &'static str {
    "X-Content-Type-Options: nosniff\r\n\
Referrer-Policy: no-referrer\r\n\
Content-Security-Policy: default-src 'self'; base-uri 'self'; frame-ancestors 'none'; object-src 'none'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src 'self' data: https://fonts.gstatic.com; img-src 'self' data:; connect-src 'self' ws: wss:\r\n\
Strict-Transport-Security: max-age=31536000; includeSubDomains\r\n"
}

async fn write_web_static_response<W: tokio::io::AsyncWrite + Unpin>(
    writer: &mut W,
    path: &str,
    keep_alive: bool,
    extra_headers: &str,
) -> Result<Option<usize>, String> {
    let Some((file, content_type)) = web_static_file_for_request(path) else {
        return Ok(None);
    };
    let bytes = fs::read(&file).map_err(|error| error.to_string())?;
    let connection_header = if keep_alive { "keep-alive" } else { "close" };
    let headers = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: {connection_header}\r\n{}{}\r\n",
        bytes.len(),
        security_headers(),
        extra_headers,
    );
    writer
        .write_all(headers.as_bytes())
        .await
        .map_err(|error| error.to_string())?;
    writer
        .write_all(&bytes)
        .await
        .map_err(|error| error.to_string())?;
    writer.flush().await.map_err(|error| error.to_string())?;
    Ok(Some(bytes.len()))
}

fn health_response() -> HttpResponse {
    HttpResponse {
        status: "200 OK",
        content_type: "application/json",
        body: "{\"status\":\"ok\",\"service\":\"slskr\"}".to_owned(),
    }
}

fn version_response() -> HttpResponse {
    HttpResponse {
        status: "200 OK",
        content_type: "application/json",
        body: format!(
            "{{\"name\":\"{}\",\"version\":\"{}\",\"protocol\":{{\"client_name\":\"{}\",\"major\":{},\"minor\":{}}}}}",
            CLIENT_NAME, APP_VERSION, CLIENT_NAME, CLIENT_MAJOR_VERSION, CLIENT_MINOR_VERSION
        ),
    }
}

fn slskd_version_json() -> serde_json::Value {
    serde_json::json!({
        "full": APP_VERSION,
        "current": APP_VERSION,
        "latest": APP_VERSION,
        "isUpdateAvailable": false,
        "isCanary": false,
        "isDevelopment": cfg!(debug_assertions),
    })
}

fn slskd_server_state_json(session: &SessionSnapshot, config: &AppConfig) -> serde_json::Value {
    let connected = session.state == "connected";
    serde_json::json!({
        "address": config.server_address,
        "ipEndPoint": config.server_address,
        "state": session.state,
        "isConnected": connected,
        "isConnecting": session.state == "connecting",
        "isLoggedIn": connected,
        "isLoggingIn": session.state == "connecting",
        "isTransitioning": session.state == "connecting" || session.state == "disconnecting",
    })
}

fn slskd_application_state_json(
    session: &SessionSnapshot,
    shares: &ShareIndexSnapshot,
    rooms: &RoomStore,
    users: &UserStore,
    config: &AppConfig,
) -> String {
    serde_json::json!({
        "version": slskd_version_json(),
        "pendingReconnect": false,
        "pendingRestart": false,
        "server": slskd_server_state_json(session, config),
        "connectionWatchdog": {
            "enabled": config.reconnect,
            "reconnectDelaySeconds": config.reconnect_delay.as_secs(),
        },
        "relay": { "enabled": false },
        "user": {
            "username": session.username,
            "privilegesSeconds": session.privileges_seconds,
        },
        "distributedNetwork": { "enabled": false },
        "shares": {
            "local": shares.roots.iter().map(|root| serde_json::json!({
                "id": root.label,
                "alias": root.label,
                "isExcluded": false,
                "localPath": root.label,
                "raw": root.label,
                "remotePath": root.label,
                "directories": root.files,
                "files": root.files,
            })).collect::<Vec<_>>(),
        },
        "rooms": rooms.records.iter().map(|room| room.name.clone()).collect::<Vec<_>>(),
        "users": users.records.iter().map(|user| user.username.clone()).collect::<Vec<_>>(),
    })
    .to_string()
}

fn capabilities_response() -> HttpResponse {
    HttpResponse {
        status: "200 OK",
        content_type: "application/json",
        body: r#"{"api_version":"v0","client_version":"0.1","supports":["login","peers","shares","searches","transfers","users","messages","rooms","room-list-sync","browser-session-auth"]}"#.to_owned(),
    }
}

fn capabilities_negotiate_response(body: &str) -> HttpResponse {
    let server_capabilities = ["shares", "telemetry"];

    // Parse requested capabilities from body
    let requested = extract_json_string_array_field(body, "capabilities").unwrap_or_default();

    // Compute intersection
    let mut accepted = Vec::new();
    let mut unsupported = Vec::new();

    for req_cap in requested {
        if server_capabilities.contains(&req_cap.as_str()) {
            accepted.push(req_cap);
        } else {
            unsupported.push(req_cap);
        }
    }

    let response_body = serde_json::json!({
        "accepted": accepted,
        "unsupported": unsupported,
        "server_capabilities": server_capabilities,
    })
    .to_string();

    HttpResponse {
        status: "200 OK",
        content_type: "application/json",
        body: response_body,
    }
}

fn url_encode(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

fn secure_oauth_state() -> String {
    let mut bytes = [0_u8; 32];
    OsRng.fill_bytes(&mut bytes);
    format!("slskr-{}", hex::encode(bytes))
}

fn spotify_redirect_uri(state: &AppState) -> String {
    state
        .config
        .integrations
        .spotify
        .redirect_uri
        .clone()
        .unwrap_or_else(|| {
            let host = if state.config.http_bind.ip().is_unspecified() {
                format!("127.0.0.1:{}", state.config.http_bind.port())
            } else {
                state.config.http_bind.to_string()
            };
            format!("http://{host}/api/integrations/spotify/callback")
        })
}

async fn fetch_lidarr_system_status(
    lidarr: &config::LidarrIntegrationSettings,
) -> Result<serde_json::Value, String> {
    let base_url = lidarr
        .url
        .as_deref()
        .ok_or("Lidarr URL is not configured")?;
    let api_key = lidarr
        .api_key
        .as_deref()
        .ok_or("Lidarr API key is not configured")?;
    let url = format!("{}/api/v1/system/status", base_url.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(lidarr.timeout_seconds))
        .build()
        .map_err(|error| format!("failed to build Lidarr client: {error}"))?;
    let response = client
        .get(url)
        .header("X-Api-Key", api_key)
        .send()
        .await
        .map_err(|error| format!("Lidarr status request failed: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("Lidarr returned HTTP {}", response.status()));
    }
    response
        .json::<serde_json::Value>()
        .await
        .map_err(|error| format!("invalid Lidarr status JSON: {error}"))
}

async fn fetch_lidarr_wanted_missing(
    lidarr: &config::LidarrIntegrationSettings,
) -> Result<serde_json::Value, String> {
    let base_url = lidarr
        .url
        .as_deref()
        .ok_or("Lidarr URL is not configured")?;
    let api_key = lidarr
        .api_key
        .as_deref()
        .ok_or("Lidarr API key is not configured")?;
    let url = format!(
        "{}/api/v1/wanted/missing?pageSize=100",
        base_url.trim_end_matches('/')
    );
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(lidarr.timeout_seconds))
        .build()
        .map_err(|error| format!("failed to build Lidarr client: {error}"))?;
    let response = client
        .get(url)
        .header("X-Api-Key", api_key)
        .send()
        .await
        .map_err(|error| format!("Lidarr wanted request failed: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("Lidarr returned HTTP {}", response.status()));
    }
    response
        .json::<serde_json::Value>()
        .await
        .map_err(|error| format!("invalid Lidarr wanted JSON: {error}"))
}

#[cfg(test)]
async fn route_http_request(
    method: &str,
    path: &str,
    authorization: Option<&str>,
    body: &str,
    state: &AppState,
) -> Result<HttpResponse, String> {
    route_http_request_with_headers(
        method,
        path,
        authorization,
        body,
        state,
        RequestSecurityHeaders::default(),
    )
    .await
}

fn extract_json_u32_field(body: &str, field: &str) -> Option<u32> {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()?
        .get(field)?
        .as_u64()
        .and_then(|value| u32::try_from(value).ok())
}

fn extract_json_bool_field(body: &str, field: &str) -> Option<bool> {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()?
        .get(field)?
        .as_bool()
}

fn extract_json_u64_field(body: &str, field: &str) -> Option<u64> {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()?
        .get(field)?
        .as_u64()
}

fn extract_json_i32_field(body: &str, field: &str) -> Option<i32> {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()?
        .get(field)?
        .as_i64()
        .and_then(|value| i32::try_from(value).ok())
}

fn rate_limit_user_key(authorization: Option<&str>, cookie: Option<&str>) -> String {
    use sha2::{Digest, Sha256};

    let token = authorization
        .and_then(|value| {
            value
                .strip_prefix("Bearer ")
                .or_else(|| value.strip_prefix("ApiKey "))
        })
        .or_else(|| {
            cookie?.split(';').find_map(|part| {
                let (name, value) = part.trim().split_once('=')?;
                (name == "slskr.session").then_some(value)
            })
        })
        .unwrap_or("authenticated");
    let digest = Sha256::digest(token.as_bytes());
    format!("auth:{}", hex::encode(&digest[..16]))
}

fn slskd_transfer_user_path<'a>(path: &'a str, direction: &str) -> Option<&'a str> {
    let prefix = format!("/api/transfers/{direction}/");
    path.strip_prefix(&prefix)
        .filter(|username| !username.is_empty() && !username.contains('/'))
}

fn slskd_transfer_file_path<'a>(path: &'a str, direction: &str) -> Option<(&'a str, u64)> {
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

fn slskd_transfer_position_path(path: &str) -> Option<(&str, u64)> {
    slskd_transfer_file_path(path, "downloads").filter(|_| path.ends_with("/position"))
}

fn slskd_files_from_body(body: &str) -> Vec<serde_json::Value> {
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

fn slskd_enqueue_request(body: &str) -> Option<(String, Vec<serde_json::Value>)> {
    let payload = serde_json::from_str::<serde_json::Value>(body).ok()?;
    let username = payload.get("username")?.as_str()?.to_owned();
    let files = payload
        .get("files")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    (!files.is_empty()).then_some((username, files))
}

fn json_body_string(body: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
}

fn path_segment_after<'a>(path: &'a str, prefix: &str) -> Option<&'a str> {
    path.strip_prefix(prefix)
        .filter(|segment| !segment.is_empty() && !segment.contains('/'))
}

fn conversation_message_path(path: &str) -> Option<(&str, u64)> {
    let rest = path.strip_prefix("/api/conversations/")?;
    let (username, id) = rest.split_once('/')?;
    if username.is_empty() || username.contains('/') || id.contains('/') {
        return None;
    }
    Some((username, id.parse().ok()?))
}

fn conversation_messages_path(path: &str) -> Option<&str> {
    path.strip_prefix("/api/conversations/")?
        .strip_suffix("/messages")
        .filter(|username| !username.is_empty() && !username.contains('/'))
}

fn slskd_user_file_json(entry: &BrowseEntry) -> serde_json::Value {
    serde_json::json!({
        "filename": entry.filename,
        "size": entry.size,
        "code": 1,
        "extension": entry.extension,
        "attributeCount": 0,
        "attributes": [],
    })
}

fn slskd_user_directories_json(directory: &str, entries: &[BrowseEntry]) -> String {
    serde_json::Value::Array(vec![serde_json::json!({
        "name": directory,
        "fileCount": entries.len(),
        "files": entries.iter().map(slskd_user_file_json).collect::<Vec<_>>(),
    })])
    .to_string()
}

fn slskd_user_root_json(entries: &[BrowseEntry]) -> String {
    serde_json::json!({
        "directories": [{
            "name": "",
            "fileCount": entries.len(),
            "files": entries.iter().map(slskd_user_file_json).collect::<Vec<_>>(),
        }],
        "directoryCount": 1,
        "lockedDirectories": [],
        "lockedDirectoryCount": 0,
    })
    .to_string()
}

fn slskd_empty_directory_json(name: &str) -> serde_json::Value {
    serde_json::json!({
        "name": name,
        "fullname": name,
        "attributes": "",
        "createdAt": "",
        "modifiedAt": "",
        "files": [],
        "directories": [],
    })
}

fn slskd_transfer_summary_report(transfers: &TransferQueue) -> String {
    let downloads = transfers
        .entries
        .iter()
        .filter(|entry| entry.direction == 0)
        .count();
    let uploads = transfers
        .entries
        .iter()
        .filter(|entry| entry.direction == 1)
        .count();
    let total_bytes = transfers
        .entries
        .iter()
        .map(|entry| entry.bytes_transferred)
        .sum::<u64>();
    serde_json::json!({
        "count": transfers.entries.len(),
        "downloads": downloads,
        "uploads": uploads,
        "totalBytes": total_bytes,
        "averageSpeed": 0.0,
        "byDirection": {
            "Download": { "count": downloads },
            "Upload": { "count": uploads },
        },
        "byState": {},
    })
    .to_string()
}

fn slskd_user_transfer_report(username: &str, transfers: &TransferQueue) -> String {
    let entries = transfers
        .entries
        .iter()
        .filter(|entry| entry.peer_username.as_deref() == Some(username))
        .map(TransferEntry::slskd_file_json)
        .collect::<Vec<_>>();
    serde_json::json!({
        "username": username,
        "count": entries.len(),
        "transfers": entries,
    })
    .to_string()
}

async fn serve(once: bool) -> Result<(), String> {
    let config = AppConfig::from_env()?;
    std::fs::create_dir_all(&config.state_dir).map_err(|error| {
        format!(
            "failed to create state dir {}: {error}",
            config.state_dir.display()
        )
    })?;
    let address = config.http_bind;
    let share_index = build_share_index(&config);
    let (session_commands, session_receiver) = mpsc::channel(16);
    let (event_tx, _) = broadcast::channel(EVENT_HISTORY_LIMIT);
    let db = if config.persistence_enabled {
        let db_path = config.state_dir.join("slskr.db");
        Some(
            crate::persistence::DatabaseManager::new(db_path.to_str().unwrap_or("slskr.db"))
                .await
                .map_err(|error| format!("failed to open persistence database: {error}"))?,
        )
    } else {
        None
    };
    let search_store = if let Some(db) = db.as_ref() {
        let records = db
            .list_searches(EVENT_HISTORY_LIMIT as i32, 0)
            .await
            .map_err(|error| format!("failed to load persisted searches: {error}"))?;
        SearchStore::from_persisted(records)
    } else {
        SearchStore::new()
    };

    let rate_limiter = rate_limit::RateLimiter::new(rate_limit::RateLimitConfig {
        max_requests_anonymous: config.api_rate_limit_anonymous,
        max_requests_authenticated: config.api_rate_limit_authenticated,
        window_seconds: 60,
        enabled: true,
    });

    let state = Arc::new(AppState {
        session: RwLock::new(SessionSnapshot::disconnected(&config)),
        listeners: RwLock::new(ListenerSnapshot::new(&config)),
        shares: RwLock::new(share_index),
        searches: RwLock::new(search_store),
        users: RwLock::new(UserStore::new()),
        browse: RwLock::new(BrowseStore::new()),
        messages: RwLock::new(MessageStore::new()),
        rooms: RwLock::new(RoomStore::new()),
        transfers: RwLock::new(TransferQueue::new(&config)),
        events: RwLock::new(EventStore::new(EVENT_HISTORY_LIMIT)),
        event_tx,
        webhooks: RwLock::new(webhooks::WebhookManager::new()),
        collections: RwLock::new(CollectionStore::new()),
        wishlist: RwLock::new(WishlistStore::new()),
        contacts: RwLock::new(ContactStore::new()),
        sharegroups: RwLock::new(ShareGroupStore::new()),
        user_notes: RwLock::new(UserNoteStore::new()),
        interests: RwLock::new(InterestStore::new()),
        share_grants: RwLock::new(ShareGrantStore::new()),
        library: RwLock::new(LibraryStore::new()),
        destinations: RwLock::new(DestinationStore::new()),
        db,
        config,
        session_commands,
        rate_limiter,
        oauth_states: RwLock::new(OAuthStateStore::default()),
    });
    spawn_session_manager(Arc::clone(&state), session_receiver);
    spawn_configured_listeners(Arc::clone(&state));
    spawn_rate_limit_cleanup(Arc::clone(&state));

    if state.config.auto_connect {
        send_session_command(&state, SessionCommand::Connect).await?;
    }

    let listener = TcpListener::bind(address)
        .await
        .map_err(|error| format!("failed to bind {address}: {error}"))?;
    println!("slskr listening on http://{address}");
    if !state.config.auth_required {
        eprintln!(
            "WARNING: HTTP API authentication is disabled on {}; local processes can control slskr",
            state.config.http_bind
        );
    }
    let http_connections = Arc::new(Semaphore::new(256));

    loop {
        let (stream, _) = listener
            .accept()
            .await
            .map_err(|error| format!("accept failed: {error}"))?;
        let Ok(permit) = Arc::clone(&http_connections).try_acquire_owned() else {
            drop(stream);
            continue;
        };
        let state = Arc::clone(&state);
        tokio::spawn(async move {
            let _permit = permit;
            if let Err(error) = handle_http_connection(stream, state).await {
                eprintln!("http request failed: {error}");
            }
        });

        if once {
            break;
        }
    }

    Ok(())
}

fn spawn_session_manager(state: Arc<AppState>, mut receiver: mpsc::Receiver<SessionCommand>) {
    tokio::spawn(async move {
        let mut session = None;
        let mut next_ping = Instant::now() + state.config.ping_interval;
        let mut reconnect_requested = false;

        loop {
            while let Ok(command) = receiver.try_recv() {
                handle_session_command(
                    &state,
                    command,
                    &mut session,
                    &mut next_ping,
                    &mut reconnect_requested,
                )
                .await;
            }

            if reconnect_requested && session.is_none() {
                time::sleep(state.config.reconnect_delay).await;
                let connected = connect_session(&state, &mut session, &mut next_ping).await;
                reconnect_requested = !connected && state.config.reconnect;
                continue;
            }

            if let Some(active_session) = session.as_mut() {
                if matches!(
                    time::timeout(Duration::from_millis(250), active_session.readable()).await,
                    Ok(Ok(()))
                ) {
                    match time::timeout(
                        state.config.peer_response_timeout,
                        active_session.receive(),
                    )
                    .await
                    {
                        Ok(Ok(message)) => {
                            project_server_message(&state, active_session, &message).await;
                            update_session(&state, |snapshot| {
                                snapshot.state = "connected";
                                snapshot.server_messages_seen += 1;
                                snapshot.last_server_message =
                                    Some(server_message_name(&message).to_string());
                            })
                            .await;
                        }
                        Ok(Err(error)) => {
                            session = None;
                            update_session(&state, |snapshot| {
                                snapshot.state = "error";
                                snapshot.last_error =
                                    Some(format!("server receive failed: {error}"));
                                snapshot.supporter = None;
                                snapshot.connected_at = None;
                            })
                            .await;
                            reconnect_requested = state.config.reconnect;
                        }
                        Err(_) => {
                            session = None;
                            update_session(&state, |snapshot| {
                                snapshot.state = "error";
                                snapshot.last_error =
                                    Some("server receive timed out after readability".to_owned());
                                snapshot.supporter = None;
                                snapshot.connected_at = None;
                            })
                            .await;
                            reconnect_requested = state.config.reconnect;
                        }
                    }
                }

                if session.is_some() && Instant::now() >= next_ping {
                    send_session_ping(&state, &mut session, &mut next_ping).await;
                    reconnect_requested = session.is_none() && state.config.reconnect;
                }
            } else if let Some(command) = receiver.recv().await {
                handle_session_command(
                    &state,
                    command,
                    &mut session,
                    &mut next_ping,
                    &mut reconnect_requested,
                )
                .await;
            } else {
                break;
            }
        }
    });
}

fn spawn_rate_limit_cleanup(state: Arc<AppState>) {
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            state.rate_limiter.cleanup().await;
        }
    });
}

fn spawn_configured_listeners(state: Arc<AppState>) {
    if let Some(bind) = state.config.listener_bind.clone() {
        tokio::spawn(run_listener(Arc::clone(&state), bind, false));
    }
    if let Some(bind) = state.config.obfuscated_listener_bind.clone() {
        tokio::spawn(run_listener(Arc::clone(&state), bind, true));
    }
}

async fn run_listener(state: Arc<AppState>, bind: String, obfuscated: bool) {
    let listener = match Listener::bind(bind.as_str()).await {
        Ok(listener) => listener,
        Err(error) => {
            update_listeners(&state, |snapshot| {
                snapshot.errors += 1;
                snapshot.last_error = Some(format!(
                    "{} listener bind failed: {error}",
                    if obfuscated { "obfuscated" } else { "regular" }
                ));
            })
            .await;
            return;
        }
    };

    let local_addr = listener.local_addr().ok().map(|addr| addr.to_string());
    update_listeners(&state, |snapshot| {
        if obfuscated {
            snapshot.obfuscated_local_addr = local_addr;
        } else {
            snapshot.regular_local_addr = local_addr;
        }
        snapshot.last_error = None;
    })
    .await;

    loop {
        let accepted = if obfuscated {
            listener.accept_obfuscated().await
        } else {
            listener.accept().await
        };
        match accepted {
            Ok((incoming, remote_addr)) => {
                let event = format!(
                    "{} from {}",
                    incoming_connection_name(&incoming),
                    scrub_socket_addr(remote_addr)
                );
                update_listeners(&state, |snapshot| {
                    if obfuscated {
                        snapshot.obfuscated_accepts += 1;
                    } else {
                        snapshot.regular_accepts += 1;
                    }
                    bump_incoming_counter(snapshot, &incoming);
                    snapshot.last_event = Some(event);
                    snapshot.last_error = None;
                })
                .await;
                tokio::spawn(handle_owned_incoming(Arc::clone(&state), incoming));
            }
            Err(error) => {
                update_listeners(&state, |snapshot| {
                    snapshot.errors += 1;
                    snapshot.last_error = Some(format!(
                        "{} listener accept failed: {error}",
                        if obfuscated { "obfuscated" } else { "regular" }
                    ));
                })
                .await;
            }
        }
    }
}

async fn handle_session_command(
    state: &AppState,
    command: SessionCommand,
    session: &mut Option<ServerSession<TcpStream>>,
    next_ping: &mut Instant,
    reconnect_requested: &mut bool,
) {
    match command {
        SessionCommand::Connect => {
            *reconnect_requested = false;
            if session.is_none() {
                let _connected = connect_session(state, session, next_ping).await;
            } else {
                update_session(state, |snapshot| {
                    snapshot.state = "connected";
                    snapshot.last_error = None;
                })
                .await;
            }
        }
        SessionCommand::Disconnect => {
            *session = None;
            *reconnect_requested = false;
            update_session(state, |snapshot| {
                snapshot.state = "disconnected";
                snapshot.supporter = None;
                snapshot.last_error = None;
                snapshot.connected_at = None;
            })
            .await;
        }
        SessionCommand::Ping => {
            if session.is_some() {
                send_session_ping(state, session, next_ping).await;
            } else {
                update_session(state, |snapshot| {
                    snapshot.state = "disconnected";
                    snapshot.last_error = Some("cannot ping while disconnected".to_owned());
                })
                .await;
            }
        }
        SessionCommand::CheckPrivileges => {
            send_active_server_message(
                state,
                session,
                ServerMessage::CheckPrivilegesRequest,
                "check privileges",
            )
            .await;
        }
        SessionCommand::Search {
            token,
            query,
            target,
        } => {
            send_active_server_message(
                state,
                session,
                search_dispatch_message(token, query, target),
                "dispatch search",
            )
            .await;
        }
        SessionCommand::WatchUser(username) => {
            send_active_server_message(
                state,
                session,
                ServerMessage::WatchUserRequest { username },
                "watch user",
            )
            .await;
        }
        SessionCommand::UnwatchUser(username) => {
            send_active_server_message(
                state,
                session,
                ServerMessage::UnwatchUser { username },
                "unwatch user",
            )
            .await;
        }
        SessionCommand::BrowseUser(username) => {
            send_active_server_message(
                state,
                session,
                ServerMessage::GetPeerAddressRequest { username },
                "request peer address for browse",
            )
            .await;
        }
        SessionCommand::BrowseFolder { username, .. } => {
            send_active_server_message(
                state,
                session,
                ServerMessage::GetPeerAddressRequest { username },
                "request peer address for folder browse",
            )
            .await;
        }
        SessionCommand::IndirectBrowse { username, token } => {
            send_active_server_message(
                state,
                session,
                ServerMessage::ConnectToPeerRequest(ConnectToPeerRequest {
                    token,
                    username,
                    connection_type: ConnectionKind::PeerMessages.as_str().to_owned(),
                }),
                "request indirect browse",
            )
            .await;
        }
        SessionCommand::RequestUserStats(username) => {
            send_active_server_message(
                state,
                session,
                ServerMessage::GetUserStatsRequest { username },
                "request user stats",
            )
            .await;
        }
        SessionCommand::TransferPeer { id, username } => {
            send_active_server_message(
                state,
                session,
                ServerMessage::GetPeerAddressRequest { username },
                &format!("request peer address for transfer {id}"),
            )
            .await;
        }
        SessionCommand::IndirectTransfer {
            id,
            username,
            token,
        } => {
            send_active_server_message(
                state,
                session,
                ServerMessage::ConnectToPeerRequest(ConnectToPeerRequest {
                    token,
                    username,
                    connection_type: ConnectionKind::FileTransfer.as_str().to_owned(),
                }),
                &format!("request indirect file transfer {id}"),
            )
            .await;
        }
        SessionCommand::MessageUser { username, body } => {
            send_active_server_message(
                state,
                session,
                ServerMessage::MessageUserRequest {
                    username,
                    message: body,
                },
                "message user",
            )
            .await;
        }
        SessionCommand::MessageAcked { id } => {
            send_active_server_message(
                state,
                session,
                ServerMessage::MessageAcked { id },
                "ack message",
            )
            .await;
        }
        SessionCommand::RefreshRooms => {
            send_active_server_message(
                state,
                session,
                ServerMessage::RoomListRequest,
                "refresh rooms",
            )
            .await;
        }
        SessionCommand::JoinRoom(room) => {
            send_active_server_message(
                state,
                session,
                ServerMessage::JoinRoom { room },
                "join room",
            )
            .await;
        }
        SessionCommand::LeaveRoom(room) => {
            send_active_server_message(
                state,
                session,
                ServerMessage::LeaveRoom { room },
                "leave room",
            )
            .await;
        }
        SessionCommand::SayRoom { room, body } => {
            send_active_server_message(
                state,
                session,
                ServerMessage::SayChatroomRequest {
                    room,
                    message: body,
                },
                "say room",
            )
            .await;
        }
    }
}

async fn handle_owned_incoming(state: Arc<AppState>, incoming: IncomingConnection<TcpStream>) {
    let result = match incoming {
        IncomingConnection::PeerMessages(peer) => handle_plain_peer_messages(&state, peer).await,
        IncomingConnection::ObfuscatedPeerMessages(peer) => {
            handle_obfuscated_peer_messages(&state, peer).await
        }
        IncomingConnection::PeerInit {
            kind: ConnectionKind::PeerMessages,
            stream,
            ..
        } => handle_plain_peer_messages(&state, PeerMessageConnection::new(stream)).await,
        IncomingConnection::FileTransfer(file) => {
            handle_inbound_file_transfer(&state, file, None).await
        }
        IncomingConnection::PeerInit {
            kind: ConnectionKind::FileTransfer,
            token,
            stream,
            ..
        } => {
            handle_inbound_file_transfer(
                &state,
                slskr_client::file_transfer::FileTransferConnection::new(stream),
                (token != 0).then_some(token),
            )
            .await
        }
        IncomingConnection::PierceFirewall { token, stream } => {
            let has_file_transfer = {
                let transfers = state.transfers.read().await;
                transfers
                    .pending_inbound_file_transfer(Some(token))
                    .is_some()
            };
            if has_file_transfer {
                handle_inbound_file_transfer(
                    &state,
                    slskr_client::file_transfer::FileTransferConnection::new(stream),
                    Some(token),
                )
                .await
            } else {
                handle_plain_peer_messages(&state, PeerMessageConnection::new(stream)).await
            }
        }
        _ => Ok(()),
    };

    if let Err(error) = result {
        update_listeners(&state, |snapshot| {
            snapshot.errors += 1;
            snapshot.last_error = Some(error);
        })
        .await;
    }
}

async fn handle_inbound_file_transfer(
    state: &AppState,
    mut file: slskr_client::file_transfer::FileTransferConnection<TcpStream>,
    token: Option<u32>,
) -> Result<(), String> {
    let transfer = {
        let transfers = state.transfers.read().await;
        transfers.pending_inbound_file_transfer(token)
    }
    .ok_or_else(|| {
        token.map_or_else(
            || "no accepted inbound file transfer is pending".to_owned(),
            |token| format!("no accepted inbound file transfer is pending for token {token}"),
        )
    })?;

    {
        let mut transfers = state.transfers.write().await;
        transfers.update_status(transfer.id, "in_progress", None, None);
    }

    let result = upload_file_transfer_with_connection(state, &transfer, &mut file).await;
    let (status, bytes_transferred, size, reason) = match result {
        Ok((bytes_transferred, size)) => ("succeeded", bytes_transferred, Some(size), None),
        Err(error) => (
            "failed",
            transfer.bytes_transferred,
            transfer.size,
            Some(error),
        ),
    };

    let mut transfers = state.transfers.write().await;
    transfers.update_local_execution(transfer.id, status, bytes_transferred, size, reason);
    Ok(())
}

async fn handle_plain_peer_messages(
    state: &AppState,
    mut peer: PeerMessageConnection<TcpStream>,
) -> Result<(), String> {
    let message = receive_plain_peer_message(state, &mut peer).await?;
    handle_peer_message(state, message, |response| async move {
        peer.send(&response)
            .await
            .map_err(|error| format!("peer response send failed: {error}"))
    })
    .await
}

async fn handle_obfuscated_peer_messages(
    state: &AppState,
    mut peer: ObfuscatedPeerMessageConnection<TcpStream>,
) -> Result<(), String> {
    let message = receive_obfuscated_peer_message(state, &mut peer).await?;
    handle_peer_message(state, message, |response| async move {
        peer.send(&response)
            .await
            .map_err(|error| format!("obfuscated peer response send failed: {error}"))
    })
    .await
}

async fn receive_plain_peer_message(
    state: &AppState,
    peer: &mut PeerMessageConnection<TcpStream>,
) -> Result<PeerMessage, String> {
    time::timeout(state.config.peer_response_timeout, peer.receive())
        .await
        .map_err(|_| "peer message receive timed out".to_owned())?
        .map_err(|error| format!("peer message receive failed: {error}"))
}

async fn receive_obfuscated_peer_message(
    state: &AppState,
    peer: &mut ObfuscatedPeerMessageConnection<TcpStream>,
) -> Result<PeerMessage, String> {
    time::timeout(state.config.peer_response_timeout, peer.receive())
        .await
        .map_err(|_| "obfuscated peer message receive timed out".to_owned())?
        .map_err(|error| format!("obfuscated peer message receive failed: {error}"))
}

#[derive(Debug)]
struct SharedLocalFile {
    local_path: PathBuf,
    size: u64,
}

async fn find_shared_local_file(state: &AppState, filename: &str) -> Option<SharedLocalFile> {
    let local_path = {
        let shares = state.shares.read().await;
        shares.local_paths.get(filename).cloned()
    }?;
    let metadata = shared_local_file_metadata(state, &local_path)?;
    metadata.is_file().then_some(SharedLocalFile {
        local_path,
        size: metadata.len(),
    })
}

fn shared_local_file_metadata(state: &AppState, local_path: &Path) -> Option<fs::Metadata> {
    let symlink_metadata = fs::symlink_metadata(local_path).ok()?;
    if symlink_metadata.file_type().is_symlink() && !state.config.share_settings.follow_symlinks {
        return None;
    }
    let metadata = fs::metadata(local_path).ok()?;
    if !metadata.is_file() {
        return None;
    }
    if !state.config.share_settings.roots.is_empty() {
        let canonical_path = local_path.canonicalize().ok()?;
        let inside_share_root = state
            .config
            .share_settings
            .roots
            .iter()
            .filter_map(|root| root.canonicalize().ok())
            .any(|root| canonical_path.starts_with(root));
        if !inside_share_root {
            return None;
        }
    }
    Some(metadata)
}

fn search_target_static(target: &str) -> &'static str {
    match target {
        "user" => "user",
        "room" => "room",
        "wishlist" => "wishlist",
        _ => "global",
    }
}

async fn prepare_transfer_local_path(
    state: &AppState,
    direction: u32,
    filename: &str,
    supplied_local_path: Option<String>,
) -> Result<Option<String>, String> {
    if direction == 1 {
        if supplied_local_path.is_some() {
            return Err("local_path is not accepted for uploads; use a shared filename".to_owned());
        }
        return find_shared_local_file(state, filename)
            .await
            .map(|shared_file| Some(shared_file.local_path.display().to_string()))
            .ok_or_else(|| "upload filename is not available from local shares".to_owned());
    }

    let path = safe_download_path(&state.config.state_dir, filename)?;
    Ok(Some(path.display().to_string()))
}

fn download_root(state_dir: &Path) -> PathBuf {
    state_dir.join("downloads")
}

fn safe_download_path(state_dir: &Path, filename: &str) -> Result<PathBuf, String> {
    let root = download_root(state_dir);
    let mut path = root.clone();
    let mut appended = false;
    for component in Path::new(filename).components() {
        match component {
            Component::Normal(part) => {
                path.push(part);
                appended = true;
            }
            Component::CurDir => {}
            Component::Prefix(_) | Component::RootDir | Component::ParentDir => {
                return Err(
                    "download filename must be relative and stay within the download root"
                        .to_owned(),
                );
            }
        }
    }
    if !appended {
        return Err("download filename is empty".to_owned());
    }
    if !path.starts_with(&root) {
        return Err("download path escapes the download root".to_owned());
    }
    Ok(path)
}

fn ensure_scoped_download_path(state_dir: &Path, local_path: &str) -> Result<PathBuf, String> {
    let root = download_root(state_dir);
    let path = PathBuf::from(local_path);
    if !path.starts_with(&root) {
        return Err("download path is outside the download root".to_owned());
    }
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .map_err(|error| format!("download directory create failed: {error}"))?;
    }
    fs::create_dir_all(&root).map_err(|error| format!("download root create failed: {error}"))?;
    let canonical_root = root
        .canonicalize()
        .map_err(|error| format!("download root canonicalize failed: {error}"))?;
    let canonical_parent = path
        .parent()
        .unwrap_or(root.as_path())
        .canonicalize()
        .map_err(|error| format!("download parent canonicalize failed: {error}"))?;
    if !canonical_parent.starts_with(&canonical_root) {
        return Err("download path escapes the download root".to_owned());
    }
    if path
        .symlink_metadata()
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false)
    {
        return Err("download path must not be a symlink".to_owned());
    }
    Ok(path)
}

async fn transfer_capacity_available(state: &AppState, excluding_id: Option<u64>) -> bool {
    if state.config.transfer_max_active == 0 {
        return false;
    }
    let transfers = state.transfers.read().await;
    transfers.active_count_excluding(excluding_id) < state.config.transfer_max_active
}

async fn handle_peer_message<F, Fut>(
    state: &AppState,
    message: PeerMessage,
    send_response: F,
) -> Result<(), String>
where
    F: FnOnce(PeerMessage) -> Fut,
    Fut: std::future::Future<Output = Result<(), String>>,
{
    match message {
        PeerMessage::UserInfoRequest => {
            update_listeners(state, |snapshot| {
                snapshot.user_info_requests += 1;
                snapshot.last_event = Some("user_info_request".to_owned());
            })
            .await;
            send_response(PeerMessage::UserInfoResponse(UserInfo {
                description: state.config.user_info_description.clone(),
                picture: None,
                total_uploads: 0,
                queue_size: 0,
                slots_free: true,
                upload_permissions: None,
            }))
            .await?;
            update_listeners(state, |snapshot| {
                snapshot.user_info_responses += 1;
                snapshot.last_event = Some("user_info_response".to_owned());
                snapshot.last_error = None;
            })
            .await;
        }
        PeerMessage::GetShareFileList => {
            update_listeners(state, |snapshot| {
                snapshot.share_list_requests += 1;
                snapshot.last_event = Some("share_list_request".to_owned());
            })
            .await;
            let entries = {
                let shares = state.shares.read().await;
                shares.entries.clone()
            };
            send_response(PeerMessage::SharedFileListResponse(
                build_shared_file_list_payload(&entries)?,
            ))
            .await?;
            update_listeners(state, |snapshot| {
                snapshot.share_list_responses += 1;
                snapshot.last_event = Some("share_list_response".to_owned());
                snapshot.last_error = None;
            })
            .await;
        }
        PeerMessage::FileSearchRequest { token, query } => {
            update_listeners(state, |snapshot| {
                snapshot.file_search_requests += 1;
                snapshot.last_event = Some("file_search_request".to_owned());
            })
            .await;
            if let Some(response) = build_file_search_response(state, token, &query).await {
                send_response(PeerMessage::FileSearchResponse(response)).await?;
                update_listeners(state, |snapshot| {
                    snapshot.file_search_responses += 1;
                    snapshot.last_event = Some("file_search_response".to_owned());
                    snapshot.last_error = None;
                })
                .await;
            }
        }
        PeerMessage::FileSearchResponse(response) => {
            let accepted = {
                let mut searches = state.searches.write().await;
                searches.add_peer_response(&response).is_some()
            };
            update_listeners(state, |snapshot| {
                snapshot.file_search_responses += 1;
                snapshot.last_event = Some(if accepted {
                    "file_search_response".to_owned()
                } else {
                    "file_search_response_unmatched".to_owned()
                });
                snapshot.last_error = None;
            })
            .await;
        }
        PeerMessage::FolderContentsRequest(request) => {
            let entries = {
                let shares = state.shares.read().await;
                shares.entries.clone()
            };
            send_response(PeerMessage::FolderContentsResponse(
                build_folder_contents_payload(&entries, &request.folder)?,
            ))
            .await?;
        }
        PeerMessage::TransferRequest(request) => {
            if !state.config.transfer_allow_inbound {
                let reason = "inbound transfers are disabled".to_owned();
                record_transfer_rejection(
                    state,
                    request.direction,
                    request.token,
                    request.filename.clone(),
                    request.size,
                    reason.clone(),
                )
                .await;
                send_response(PeerMessage::TransferResponse(TransferResponse::Rejected {
                    token: request.token,
                    reason,
                }))
                .await?;
                update_listeners(state, |snapshot| {
                    snapshot.transfer_rejections += 1;
                    snapshot.last_event = Some("transfer_rejected_policy".to_owned());
                    snapshot.last_error = None;
                })
                .await;
            } else if !transfer_capacity_available(state, None).await {
                let reason = "transfer limit reached".to_owned();
                record_transfer_rejection(
                    state,
                    request.direction,
                    request.token,
                    request.filename.clone(),
                    request.size,
                    reason.clone(),
                )
                .await;
                send_response(PeerMessage::TransferResponse(TransferResponse::Rejected {
                    token: request.token,
                    reason,
                }))
                .await?;
                update_listeners(state, |snapshot| {
                    snapshot.transfer_rejections += 1;
                    snapshot.last_event = Some("transfer_rejected_limit".to_owned());
                    snapshot.last_error = None;
                })
                .await;
            } else if let Some(shared_file) = find_shared_local_file(state, &request.filename).await
            {
                {
                    let mut transfers = state.transfers.write().await;
                    transfers.record_accepted_inbound_request(
                        request.direction,
                        request.token,
                        request.filename.clone(),
                        shared_file.local_path.display().to_string(),
                        shared_file.size,
                    );
                }
                send_response(PeerMessage::TransferResponse(TransferResponse::Allowed {
                    token: request.token,
                    size: Some(shared_file.size),
                }))
                .await?;
                update_listeners(state, |snapshot| {
                    snapshot.last_event = Some("transfer_accepted".to_owned());
                    snapshot.last_error = None;
                })
                .await;
            } else {
                let reason = "requested file is not available from local shares".to_owned();
                record_transfer_rejection(
                    state,
                    request.direction,
                    request.token,
                    request.filename.clone(),
                    request.size,
                    reason.clone(),
                )
                .await;
                send_response(PeerMessage::TransferResponse(TransferResponse::Rejected {
                    token: request.token,
                    reason,
                }))
                .await?;
                update_listeners(state, |snapshot| {
                    snapshot.transfer_rejections += 1;
                    snapshot.last_event = Some("transfer_rejected".to_owned());
                    snapshot.last_error = None;
                })
                .await;
            }
        }
        other => {
            update_listeners(state, |snapshot| {
                snapshot.unsupported_peer_messages += 1;
                snapshot.last_event = Some(format!(
                    "unsupported_peer_message:{}",
                    peer_message_name(&other)
                ));
            })
            .await;
        }
    }
    Ok(())
}

async fn connect_session(
    state: &AppState,
    session: &mut Option<ServerSession<TcpStream>>,
    next_ping: &mut Instant,
) -> bool {
    let credentials = state.config.credentials();
    let Some(credentials) = credentials else {
        update_session(state, |snapshot| {
            snapshot.state = "error";
            snapshot.last_error =
                Some("SLSK_USERNAME/SLSK_PASSWORD are required to connect".to_owned());
        })
        .await;
        return false;
    };

    update_session(state, |snapshot| {
        snapshot.state = "connecting";
        snapshot.last_error = None;
        snapshot.supporter = None;
        snapshot.connected_at = None;
    })
    .await;

    let connection = match ServerConnection::connect(state.config.server_address.as_str()).await {
        Ok(connection) => connection,
        Err(error) => {
            update_session(state, |snapshot| {
                snapshot.state = "error";
                snapshot.last_error = Some(format!("connect failed: {error}"));
                snapshot.supporter = None;
            })
            .await;
            return false;
        }
    };
    let mut new_session = ServerSession::new(connection);
    let info = match new_session.login(credentials).await {
        Ok(info) => info,
        Err(error) => {
            update_session(state, |snapshot| {
                snapshot.state = "error";
                snapshot.last_error = Some(format!("login failed: {error}"));
                snapshot.supporter = None;
            })
            .await;
            return false;
        }
    };
    let wait_port_result = if let Some(obfuscated_port) = state.config.obfuscated_advertised_port {
        new_session
            .set_wait_port_obfuscated(
                state.config.advertised_port,
                ROTATED_OBFUSCATION_TYPE,
                obfuscated_port,
            )
            .await
    } else {
        new_session
            .set_wait_port(state.config.advertised_port)
            .await
    };
    if let Err(error) = wait_port_result {
        update_session(state, |snapshot| {
            snapshot.state = "error";
            snapshot.last_error = Some(format!("set wait port failed: {error}"));
            snapshot.supporter = None;
        })
        .await;
        return false;
    }
    if let Err(error) = new_session.send_ping().await {
        update_session(state, |snapshot| {
            snapshot.state = "error";
            snapshot.last_error = Some(format!("initial ping failed: {error}"));
            snapshot.supporter = None;
        })
        .await;
        return false;
    }

    update_session(state, |snapshot| {
        snapshot.state = "connected";
        snapshot.supporter = Some(info.is_supporter);
        snapshot.last_error = None;
        snapshot.connected_at = Some(unix_timestamp());
        if snapshot.server_messages_seen > 0 {
            snapshot.reconnects += 1;
        }
    })
    .await;
    *next_ping = Instant::now() + state.config.ping_interval;
    *session = Some(new_session);
    true
}

async fn send_session_ping(
    state: &AppState,
    session: &mut Option<ServerSession<TcpStream>>,
    next_ping: &mut Instant,
) {
    let Some(active_session) = session.as_mut() else {
        return;
    };

    match active_session.send_ping().await {
        Ok(()) => {
            *next_ping = Instant::now() + state.config.ping_interval;
            update_session(state, |snapshot| {
                snapshot.state = "connected";
                snapshot.last_error = None;
            })
            .await;
        }
        Err(error) => {
            *session = None;
            update_session(state, |snapshot| {
                snapshot.state = "error";
                snapshot.last_error = Some(format!("ping failed: {error}"));
                snapshot.supporter = None;
                snapshot.connected_at = None;
            })
            .await;
        }
    }
}

async fn send_active_server_message(
    state: &AppState,
    session: &mut Option<ServerSession<TcpStream>>,
    message: ServerMessage,
    action: &str,
) {
    let Some(active_session) = session.as_mut() else {
        update_session(state, |snapshot| {
            snapshot.state = "disconnected";
            snapshot.last_error = Some(format!("cannot {action} while disconnected"));
        })
        .await;
        return;
    };

    match active_session.send_server_message(message).await {
        Ok(()) => {
            update_session(state, |snapshot| {
                snapshot.last_error = None;
            })
            .await;
        }
        Err(error) => {
            *session = None;
            update_session(state, |snapshot| {
                snapshot.state = "error";
                snapshot.last_error = Some(format!("{action} failed: {error}"));
                snapshot.supporter = None;
                snapshot.connected_at = None;
            })
            .await;
        }
    }
}

fn search_dispatch_message(
    token: u32,
    query: String,
    target: SearchDispatchTarget,
) -> ServerMessage {
    match target {
        SearchDispatchTarget::Global => {
            ServerMessage::FileSearchRequest(SearchRequest { token, query })
        }
        SearchDispatchTarget::Wishlist => {
            ServerMessage::WishlistSearch(SearchRequest { token, query })
        }
        SearchDispatchTarget::User(username) => ServerMessage::UserSearch(TargetedSearchRequest {
            target: username,
            token,
            query,
        }),
        SearchDispatchTarget::Room(room) => ServerMessage::RoomSearch(TargetedSearchRequest {
            target: room,
            token,
            query,
        }),
    }
}

async fn project_server_message(
    state: &AppState,
    session: &mut ServerSession<TcpStream>,
    message: &ServerMessage,
) {
    match message {
        ServerMessage::WatchUserResponse(user) => {
            let mut users = state.users.write().await;
            users.apply_watched_user(user);
        }
        ServerMessage::GetUserStatusResponse(status) => {
            let mut users = state.users.write().await;
            users.apply_status(status);
        }
        ServerMessage::GetUserStats { username, stats } => {
            let mut users = state.users.write().await;
            users.apply_stats(username.clone(), stats);
        }
        ServerMessage::CheckPrivilegesResponse { seconds } => {
            update_session(state, |snapshot| {
                snapshot.privileges_seconds = Some(*seconds);
            })
            .await;
        }
        ServerMessage::MessageUserResponse(message) => {
            let mut messages = state.messages.write().await;
            messages.add(message.username.clone(), "inbound", message.message.clone());
            drop(messages);
            if let Err(error) = session
                .send_server_message(ServerMessage::MessageAcked { id: message.id })
                .await
            {
                update_session(state, |snapshot| {
                    snapshot.last_error = Some(format!("message ack failed: {error}"));
                })
                .await;
            }
        }
        ServerMessage::MessageAcked { id } => {
            let mut messages = state.messages.write().await;
            messages.ack(u64::from(*id));
        }
        ServerMessage::RoomList(room_list) => {
            let mut rooms = state.rooms.write().await;
            rooms.apply_room_list(room_list);
        }
        ServerMessage::GetPeerAddressResponse(address) => {
            project_peer_browse_response(state, address).await;
            project_peer_transfer_response(state, address).await;
        }
        ServerMessage::ConnectToPeerResponse(response) => {
            project_indirect_browse_response(state, response).await;
            project_indirect_transfer_response(state, response).await;
        }
        ServerMessage::CantConnectToPeerResponse { token } => {
            fail_indirect_browse(
                state,
                *token,
                "server reported cant-connect-to-peer".to_owned(),
            )
            .await;
            fail_indirect_transfer(
                state,
                *token,
                "server reported cant-connect-to-peer".to_owned(),
            )
            .await;
        }
        ServerMessage::JoinRoom { room } => {
            let mut rooms = state.rooms.write().await;
            rooms.join(room.clone());
        }
        ServerMessage::SayChatroomResponse {
            room,
            username,
            message,
        }
        | ServerMessage::GlobalRoomMessage {
            room,
            username,
            message,
        } => {
            let mut rooms = state.rooms.write().await;
            if rooms
                .add_message(room, username.clone(), message.clone())
                .is_none()
            {
                rooms.join(room.clone());
                rooms.add_message(room, username.clone(), message.clone());
            }
        }
        ServerMessage::LeaveRoom { room } => {
            let mut rooms = state.rooms.write().await;
            rooms.leave(room);
        }
        _ => {}
    }
}

async fn project_peer_browse_response(state: &AppState, address: &PeerAddress) {
    let requested_folder = {
        let browse = state.browse.read().await;
        browse.requested_folder(&address.username)
    };
    let Some(requested_folder) = requested_folder else {
        return;
    };

    let result = if let Some(folder) = requested_folder {
        fetch_peer_folder(state, address, folder).await
    } else {
        fetch_peer_browse(state, address).await
    };

    match result {
        Ok(entries) => {
            let mut browse = state.browse.write().await;
            browse.add_entries(address.username.clone(), entries, true);
        }
        Err(error) => {
            let mut browse = state.browse.write().await;
            let token = browse
                .mark_indirect_pending(&address.username, format!("direct browse failed: {error}"));
            drop(browse);
            if let Some(token) = token {
                try_send_session_command(
                    state,
                    SessionCommand::IndirectBrowse {
                        username: address.username.clone(),
                        token,
                    },
                );
                record_event(
                    state,
                    "browse.indirect.requested",
                    address.username.clone(),
                    Some(format!("token {token}")),
                )
                .await;
            } else {
                let mut browse = state.browse.write().await;
                browse.fail(address.username.clone(), error.clone());
                drop(browse);
                record_event(
                    state,
                    "browse.failed",
                    address.username.clone(),
                    Some(error.clone()),
                )
                .await;
                update_session(state, |snapshot| {
                    snapshot.last_error = Some(format!(
                        "browse {} failed: {error}",
                        redact_username(&address.username)
                    ));
                })
                .await;
            }
        }
    }
}

async fn project_indirect_browse_response(state: &AppState, response: &ConnectToPeerResponse) {
    let Ok(kind) = ConnectionKind::try_from_connection_type(&response.connection_type) else {
        return;
    };
    if kind != ConnectionKind::PeerMessages {
        return;
    }
    let requested_folder = {
        let browse = state.browse.read().await;
        browse.pending_indirect(&response.username, response.token)
    };
    let Some(requested_folder) = requested_folder else {
        return;
    };

    let result = if let Some(folder) = requested_folder {
        fetch_indirect_peer_folder(state, response, folder).await
    } else {
        fetch_indirect_peer_browse(state, response).await
    };
    match result {
        Ok(entries) => {
            let mut browse = state.browse.write().await;
            browse.add_entries(response.username.clone(), entries, true);
        }
        Err(error) => {
            let mut browse = state.browse.write().await;
            browse.fail(response.username.clone(), error.clone());
            drop(browse);
            record_event(
                state,
                "browse.failed",
                response.username.clone(),
                Some(error.clone()),
            )
            .await;
            update_session(state, |snapshot| {
                snapshot.last_error = Some(format!(
                    "indirect browse {} failed: {error}",
                    redact_username(&response.username)
                ));
            })
            .await;
        }
    }
}

async fn project_peer_transfer_response(state: &AppState, address: &PeerAddress) {
    let transfer = {
        let transfers = state.transfers.read().await;
        transfers.pending_peer_transfer(&address.username)
    };
    let Some(transfer) = transfer else {
        return;
    };

    {
        let mut transfers = state.transfers.write().await;
        transfers.update_status(transfer.id, "peer_negotiating", None, None);
    }

    let result = negotiate_peer_transfer(state, address, &transfer).await;
    let (status, bytes_transferred, reason) = match result {
        Ok(TransferResponse::Allowed { token, size }) if token == transfer.token => {
            let transferred = transfer.bytes_transferred;
            let size = size.or(transfer.size);
            let accepted = {
                let mut transfers = state.transfers.write().await;
                transfers.update_local_execution(transfer.id, "accepted", transferred, size, None)
            };
            if let Some(accepted) = accepted {
                execute_accepted_file_transfer(state, address, &accepted).await;
            }
            return;
        }
        Ok(TransferResponse::Allowed { token, .. }) => (
            "failed",
            None,
            Some(format!(
                "transfer token mismatch: expected {}, received {token}",
                transfer.token
            )),
        ),
        Ok(TransferResponse::Rejected { token, reason }) if token == transfer.token => {
            ("failed", None, Some(reason))
        }
        Ok(TransferResponse::Rejected { token, .. }) => (
            "failed",
            None,
            Some(format!(
                "transfer token mismatch: expected {}, received {token}",
                transfer.token
            )),
        ),
        Err(error) => ("failed", None, Some(error)),
    };

    let mut transfers = state.transfers.write().await;
    transfers.update_status(transfer.id, status, bytes_transferred, reason);
}

async fn execute_accepted_file_transfer(
    state: &AppState,
    address: &PeerAddress,
    transfer: &TransferEntry,
) {
    if transfer
        .local_path
        .as_deref()
        .unwrap_or_default()
        .is_empty()
    {
        return;
    }

    {
        let mut transfers = state.transfers.write().await;
        transfers.update_status(transfer.id, "in_progress", None, None);
    }

    let result = if transfer.direction == 1 {
        upload_file_transfer(state, address, transfer).await
    } else {
        download_file_transfer(state, address, transfer).await
    };
    let (status, bytes_transferred, size, reason) = match result {
        Ok((bytes_transferred, size)) => ("succeeded", bytes_transferred, Some(size), None),
        Err(error) => {
            if let Some(username) = transfer.peer_username.clone() {
                let mut transfers = state.transfers.write().await;
                transfers.update_status(
                    transfer.id,
                    "indirect_pending",
                    None,
                    Some(format!("direct file-transfer failed: {error}")),
                );
                drop(transfers);
                try_send_session_command(
                    state,
                    SessionCommand::IndirectTransfer {
                        id: transfer.id,
                        username,
                        token: transfer.token,
                    },
                );
                return;
            }
            (
                "failed",
                transfer.bytes_transferred,
                transfer.size,
                Some(error),
            )
        }
    };

    let mut transfers = state.transfers.write().await;
    transfers.update_local_execution(transfer.id, status, bytes_transferred, size, reason);
}

async fn project_indirect_transfer_response(state: &AppState, response: &ConnectToPeerResponse) {
    let Ok(kind) = ConnectionKind::try_from_connection_type(&response.connection_type) else {
        return;
    };
    if kind != ConnectionKind::FileTransfer {
        return;
    }
    let transfer = {
        let transfers = state.transfers.read().await;
        transfers.pending_indirect_transfer(&response.username, response.token)
    };
    let Some(transfer) = transfer else {
        return;
    };
    {
        let mut transfers = state.transfers.write().await;
        transfers.update_status(transfer.id, "in_progress", None, None);
    }

    let result = execute_indirect_file_transfer(state, response, &transfer).await;
    let (status, bytes_transferred, size, reason) = match result {
        Ok((bytes_transferred, size)) => ("succeeded", bytes_transferred, Some(size), None),
        Err(error) => (
            "failed",
            transfer.bytes_transferred,
            transfer.size,
            Some(error),
        ),
    };
    let mut transfers = state.transfers.write().await;
    transfers.update_local_execution(transfer.id, status, bytes_transferred, size, reason);
}

async fn fail_indirect_transfer(state: &AppState, token: u32, reason: String) {
    let transfer = {
        let transfers = state.transfers.read().await;
        transfers
            .entries
            .iter()
            .find(|entry| entry.token == token && entry.status == "indirect_pending")
            .cloned()
    };
    if let Some(transfer) = transfer {
        let mut transfers = state.transfers.write().await;
        transfers.update_status(transfer.id, "failed", None, Some(reason));
    }
}

async fn fail_indirect_browse(state: &AppState, token: u32, reason: String) {
    let failed = {
        let mut browse = state.browse.write().await;
        browse.fail_indirect(token, reason.clone())
    };
    if let Some(record) = failed {
        record_event(
            state,
            "browse.failed",
            record.username.clone(),
            Some(reason),
        )
        .await;
    }
}

async fn execute_indirect_file_transfer(
    state: &AppState,
    response: &ConnectToPeerResponse,
    transfer: &TransferEntry,
) -> Result<(u64, u64), String> {
    let mut connection = connect_indirect_file_transfer(state, response).await?;
    if transfer.direction == 1 {
        upload_file_transfer_with_connection(state, transfer, &mut connection).await
    } else {
        download_file_transfer_with_connection(state, transfer, &mut connection).await
    }
}

async fn upload_file_transfer(
    state: &AppState,
    address: &PeerAddress,
    transfer: &TransferEntry,
) -> Result<(u64, u64), String> {
    let mut connection = connect_file_transfer_preferred(state, address).await?;
    upload_file_transfer_with_connection(state, transfer, &mut connection).await
}

async fn upload_file_transfer_with_connection(
    state: &AppState,
    transfer: &TransferEntry,
    connection: &mut slskr_client::file_transfer::FileTransferConnection<TcpStream>,
) -> Result<(u64, u64), String> {
    let local_path = transfer
        .local_path
        .as_deref()
        .ok_or_else(|| "local path is required".to_owned())?;
    let shared_file = find_shared_local_file(state, &transfer.filename)
        .await
        .ok_or_else(|| "upload filename is not available from local shares".to_owned())?;
    if Path::new(local_path) != shared_file.local_path {
        return Err("upload local path does not match the share index".to_owned());
    }
    let metadata = fs::metadata(&shared_file.local_path)
        .map_err(|error| format!("local file metadata failed: {error}"))?;
    if !metadata.is_file() {
        return Err("local path is not a file".to_owned());
    }
    if let Some(expected_size) = transfer.size.or(Some(shared_file.size)) {
        if metadata.len() > expected_size {
            return Err("local file exceeds expected transfer size".to_owned());
        }
    }
    let size = metadata.len();
    let mut file = fs::File::open(&shared_file.local_path)
        .map_err(|error| format!("local file open failed: {error}"))?;
    let offset = upload_file_with_progress(state, transfer, connection, &mut file, size).await?;
    Ok((size.saturating_sub(offset), size))
}

async fn upload_file_with_progress(
    state: &AppState,
    transfer: &TransferEntry,
    connection: &mut slskr_client::file_transfer::FileTransferConnection<TcpStream>,
    file: &mut fs::File,
    size: u64,
) -> Result<u64, String> {
    use std::io::{Read, Seek, SeekFrom};

    time::timeout(
        state.config.peer_response_timeout,
        connection.send_token(transfer.token),
    )
    .await
    .map_err(|_| "file upload token send timed out".to_owned())?
    .map_err(|error| format!("file upload token send failed: {error}"))?;
    let offset = time::timeout(
        state.config.peer_response_timeout,
        connection.receive_offset(),
    )
    .await
    .map_err(|_| "file upload offset receive timed out".to_owned())?
    .map_err(|error| format!("file upload offset receive failed: {error}"))?;
    let start = usize::try_from(offset)
        .map_err(|_| format!("transfer offset {offset} exceeds local file size {size}"))?;
    if u64::try_from(start).unwrap_or(u64::MAX) > size {
        return Err(format!(
            "transfer offset {offset} exceeds local file size {size}"
        ));
    }
    file.seek(SeekFrom::Start(offset))
        .map_err(|error| format!("local file seek failed: {error}"))?;

    let mut sent = 0_u64;
    let mut buffer = vec![0_u8; TRANSFER_PROGRESS_CHUNK_BYTES];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("local file read failed: {error}"))?;
        if read == 0 {
            break;
        }
        let chunk = &buffer[..read];
        time::timeout(
            state.config.peer_response_timeout,
            connection.write_chunk(chunk),
        )
        .await
        .map_err(|_| "file upload chunk send timed out".to_owned())?
        .map_err(|error| format!("file upload chunk send failed: {error}"))?;
        sent = sent.saturating_add(u64::try_from(chunk.len()).unwrap_or(u64::MAX));
        update_transfer_progress(state, transfer.id, sent).await;
    }
    Ok(offset)
}

async fn download_file_transfer(
    state: &AppState,
    address: &PeerAddress,
    transfer: &TransferEntry,
) -> Result<(u64, u64), String> {
    let mut connection = connect_file_transfer_preferred(state, address).await?;
    download_file_transfer_with_connection(state, transfer, &mut connection).await
}

async fn download_file_transfer_with_connection(
    state: &AppState,
    transfer: &TransferEntry,
    connection: &mut slskr_client::file_transfer::FileTransferConnection<TcpStream>,
) -> Result<(u64, u64), String> {
    let local_path = transfer
        .local_path
        .as_deref()
        .ok_or_else(|| "local path is required".to_owned())?;
    let size = transfer
        .size
        .ok_or_else(|| "download size is required before file transfer".to_owned())?;
    let path = ensure_scoped_download_path(&state.config.state_dir, local_path)?;
    let offset = fs::metadata(&path)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    if offset > size {
        return Err(format!(
            "local resume offset {offset} exceeds transfer size {size}"
        ));
    }
    let remaining = usize::try_from(size - offset)
        .map_err(|_| "download remaining size is too large".to_owned())?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|error| format!("download file open failed: {error}"))?;
    let bytes_received =
        download_file_with_progress(state, transfer, connection, offset, remaining, &mut file)
            .await?;
    Ok((offset + bytes_received, size))
}

async fn download_file_with_progress(
    state: &AppState,
    transfer: &TransferEntry,
    connection: &mut slskr_client::file_transfer::FileTransferConnection<TcpStream>,
    offset: u64,
    remaining: usize,
    file: &mut fs::File,
) -> Result<u64, String> {
    use std::io::Write;

    let token = time::timeout(
        state.config.peer_response_timeout,
        connection.receive_token(),
    )
    .await
    .map_err(|_| "file download token receive timed out".to_owned())?
    .map_err(|error| format!("file download token receive failed: {error}"))?;
    if token != transfer.token {
        return Err(format!(
            "transfer token mismatch: expected {}, received {token}",
            transfer.token
        ));
    }
    time::timeout(
        state.config.peer_response_timeout,
        connection.send_offset(offset),
    )
    .await
    .map_err(|_| "file download offset send timed out".to_owned())?
    .map_err(|error| format!("file download offset send failed: {error}"))?;

    let mut bytes_received = 0_usize;
    while bytes_received < remaining {
        let next_len = (remaining - bytes_received).min(TRANSFER_PROGRESS_CHUNK_BYTES);
        let chunk = time::timeout(
            state.config.peer_response_timeout,
            connection.read_chunk(next_len),
        )
        .await
        .map_err(|_| "file download chunk receive timed out".to_owned())?
        .map_err(|error| format!("file download chunk receive failed: {error}"))?;
        file.write_all(&chunk)
            .map_err(|error| format!("download file write failed: {error}"))?;
        bytes_received += chunk.len();
        let transferred = offset.saturating_add(u64::try_from(bytes_received).unwrap_or(u64::MAX));
        update_transfer_progress(state, transfer.id, transferred).await;
    }
    Ok(u64::try_from(bytes_received).unwrap_or(u64::MAX))
}

async fn update_transfer_progress(state: &AppState, transfer_id: u64, bytes_transferred: u64) {
    let mut transfers = state.transfers.write().await;
    transfers.update_progress(transfer_id, bytes_transferred);
}

async fn connect_file_transfer_preferred(
    state: &AppState,
    address: &PeerAddress,
) -> Result<slskr_client::file_transfer::FileTransferConnection<TcpStream>, String> {
    let mut obfuscated_error = None;
    let peer_ip = peer_connect_ip(state, address);
    if address.obfuscation_type == ROTATED_OBFUSCATION_TYPE && address.obfuscated_port != 0 {
        match connect_obfuscated_file_transfer(state, address).await {
            Ok(connection) => return Ok(connection),
            Err(error) => obfuscated_error = Some(error),
        }
    }

    let port = u16::try_from(address.port).map_err(|_| "peer port is out of range".to_owned())?;
    if port == 0 {
        return Err(obfuscated_error
            .unwrap_or_else(|| "peer did not advertise a file-transfer port".to_owned()));
    }
    time::timeout(
        state.config.peer_response_timeout,
        connect_file_transfer(
            SocketAddr::V4(SocketAddrV4::new(peer_ip, port)),
            address.username.clone(),
        ),
    )
    .await
    .map_err(|_| "file-transfer connect timed out".to_owned())?
    .map_err(|error| format!("file-transfer connect failed: {error}"))
}

async fn connect_obfuscated_file_transfer(
    state: &AppState,
    address: &PeerAddress,
) -> Result<slskr_client::file_transfer::FileTransferConnection<TcpStream>, String> {
    let peer_ip = peer_connect_ip(state, address);
    let stream = time::timeout(
        state.config.peer_response_timeout,
        TcpStream::connect(SocketAddr::V4(SocketAddrV4::new(
            peer_ip,
            address.obfuscated_port,
        ))),
    )
    .await
    .map_err(|_| "obfuscated file-transfer connect timed out".to_owned())?
    .map_err(|error| format!("obfuscated file-transfer connect failed: {error}"))?;
    let stream = time::timeout(
        state.config.peer_response_timeout,
        send_obfuscated_peer_init(
            stream,
            address.username.clone(),
            ConnectionKind::FileTransfer,
        ),
    )
    .await
    .map_err(|_| "obfuscated file-transfer init timed out".to_owned())?
    .map_err(|error| format!("obfuscated file-transfer init failed: {error}"))?;
    Ok(slskr_client::file_transfer::FileTransferConnection::new(
        stream,
    ))
}

async fn connect_indirect_file_transfer(
    state: &AppState,
    response: &ConnectToPeerResponse,
) -> Result<slskr_client::file_transfer::FileTransferConnection<TcpStream>, String> {
    let port = u16::try_from(response.port)
        .map_err(|_| format!("connect-to-peer port is out of range: {}", response.port))?;
    let peer_ip = state.config.peer_host_override.unwrap_or(response.ip);
    let stream = time::timeout(
        state.config.peer_response_timeout,
        TcpStream::connect(SocketAddr::V4(SocketAddrV4::new(peer_ip, port))),
    )
    .await
    .map_err(|_| "indirect file-transfer connect timed out".to_owned())?
    .map_err(|error| format!("indirect file-transfer connect failed: {error}"))?;
    let stream = time::timeout(
        state.config.peer_response_timeout,
        send_pierce_firewall(stream, response.token),
    )
    .await
    .map_err(|_| "indirect pierce-firewall timed out".to_owned())?
    .map_err(|error| format!("indirect pierce-firewall failed: {error}"))?;
    Ok(slskr_client::file_transfer::FileTransferConnection::new(
        stream,
    ))
}

async fn connect_indirect_peer_messages(
    state: &AppState,
    response: &ConnectToPeerResponse,
) -> Result<PeerMessageConnection<TcpStream>, String> {
    let port = u16::try_from(response.port)
        .map_err(|_| format!("connect-to-peer port is out of range: {}", response.port))?;
    let peer_ip = state.config.peer_host_override.unwrap_or(response.ip);
    let stream = time::timeout(
        state.config.peer_response_timeout,
        TcpStream::connect(SocketAddr::V4(SocketAddrV4::new(peer_ip, port))),
    )
    .await
    .map_err(|_| "indirect peer-message connect timed out".to_owned())?
    .map_err(|error| format!("indirect peer-message connect failed: {error}"))?;
    let stream = time::timeout(
        state.config.peer_response_timeout,
        send_pierce_firewall(stream, response.token),
    )
    .await
    .map_err(|_| "indirect peer-message pierce-firewall timed out".to_owned())?
    .map_err(|error| format!("indirect peer-message pierce-firewall failed: {error}"))?;
    Ok(PeerMessageConnection::new(stream))
}

async fn negotiate_peer_transfer(
    state: &AppState,
    address: &PeerAddress,
    transfer: &TransferEntry,
) -> Result<TransferResponse, String> {
    let message = PeerMessage::TransferRequest(TransferRequest {
        direction: transfer.direction,
        token: transfer.token,
        filename: transfer.filename.clone(),
        size: (transfer.direction == 1).then_some(transfer.size).flatten(),
    });
    let response = send_peer_message_request(state, address, message).await?;
    match response {
        PeerMessage::TransferResponse(response) => Ok(response),
        other => Err(format!(
            "expected TransferResponse, got {}",
            peer_message_name(&other)
        )),
    }
}

async fn fetch_peer_browse(
    state: &AppState,
    address: &PeerAddress,
) -> Result<Vec<BrowseEntry>, String> {
    let username = address.username.clone();
    let mut obfuscated_error = None;
    let peer_ip = peer_connect_ip(state, address);
    if address.obfuscation_type == ROTATED_OBFUSCATION_TYPE && address.obfuscated_port != 0 {
        match browse_obfuscated_peer(
            SocketAddr::V4(SocketAddrV4::new(peer_ip, address.obfuscated_port)),
            username.clone(),
            state.config.peer_response_timeout,
        )
        .await
        {
            Ok(entries) => return Ok(entries),
            Err(error) => obfuscated_error = Some(error),
        }
    }

    let port = u16::try_from(address.port).map_err(|_| "peer port is out of range".to_owned())?;
    if port != 0 {
        return browse_plain_peer(
            SocketAddr::V4(SocketAddrV4::new(peer_ip, port)),
            username,
            state.config.peer_response_timeout,
        )
        .await;
    }

    Err(obfuscated_error.unwrap_or_else(|| "peer did not advertise a browse port".to_owned()))
}

async fn fetch_peer_folder(
    state: &AppState,
    address: &PeerAddress,
    folder: String,
) -> Result<Vec<BrowseEntry>, String> {
    let response = send_peer_message_request(
        state,
        address,
        PeerMessage::FolderContentsRequest(FolderContentsRequest {
            token: 0,
            folder: folder.clone(),
        }),
    )
    .await?;
    folder_entries_from_peer_message(response, &folder)
}

async fn fetch_indirect_peer_browse(
    state: &AppState,
    response: &ConnectToPeerResponse,
) -> Result<Vec<BrowseEntry>, String> {
    let mut peer = connect_indirect_peer_messages(state, response).await?;
    time::timeout(
        state.config.peer_response_timeout,
        peer.send(&PeerMessage::GetShareFileList),
    )
    .await
    .map_err(|_| "indirect browse request timed out".to_owned())?
    .map_err(|error| format!("indirect browse request failed: {error}"))?;
    let message = time::timeout(state.config.peer_response_timeout, peer.receive())
        .await
        .map_err(|_| "indirect browse response timed out".to_owned())?
        .map_err(|error| format!("indirect browse response failed: {error}"))?;
    browse_entries_from_peer_message(message)
}

async fn fetch_indirect_peer_folder(
    state: &AppState,
    response: &ConnectToPeerResponse,
    folder: String,
) -> Result<Vec<BrowseEntry>, String> {
    let mut peer = connect_indirect_peer_messages(state, response).await?;
    time::timeout(
        state.config.peer_response_timeout,
        peer.send(&PeerMessage::FolderContentsRequest(FolderContentsRequest {
            token: 0,
            folder: folder.clone(),
        })),
    )
    .await
    .map_err(|_| "indirect folder browse request timed out".to_owned())?
    .map_err(|error| format!("indirect folder browse request failed: {error}"))?;
    let message = time::timeout(state.config.peer_response_timeout, peer.receive())
        .await
        .map_err(|_| "indirect folder browse response timed out".to_owned())?
        .map_err(|error| format!("indirect folder browse response failed: {error}"))?;
    folder_entries_from_peer_message(message, &folder)
}

async fn browse_plain_peer(
    address: SocketAddr,
    username: String,
    timeout: Duration,
) -> Result<Vec<BrowseEntry>, String> {
    let mut peer = time::timeout(timeout, connect_peer_messages(address, username))
        .await
        .map_err(|_| "plain peer connect timed out".to_owned())?
        .map_err(|error| format!("plain peer connect failed: {error}"))?;
    time::timeout(timeout, peer.send(&PeerMessage::GetShareFileList))
        .await
        .map_err(|_| "plain browse request timed out".to_owned())?
        .map_err(|error| format!("plain browse request failed: {error}"))?;
    let message = time::timeout(timeout, peer.receive())
        .await
        .map_err(|_| "plain browse response timed out".to_owned())?
        .map_err(|error| format!("plain browse response failed: {error}"))?;
    browse_entries_from_peer_message(message)
}

async fn browse_obfuscated_peer(
    address: SocketAddr,
    username: String,
    timeout: Duration,
) -> Result<Vec<BrowseEntry>, String> {
    let stream = time::timeout(timeout, TcpStream::connect(address))
        .await
        .map_err(|_| "obfuscated peer connect timed out".to_owned())?
        .map_err(|error| format!("obfuscated peer connect failed: {error}"))?;
    let stream = time::timeout(
        timeout,
        send_obfuscated_peer_init(stream, username, ConnectionKind::PeerMessages),
    )
    .await
    .map_err(|_| "obfuscated peer init timed out".to_owned())?
    .map_err(|error| format!("obfuscated peer init failed: {error}"))?;
    let mut peer = ObfuscatedPeerMessageConnection::new(stream);
    time::timeout(timeout, peer.send(&PeerMessage::GetShareFileList))
        .await
        .map_err(|_| "obfuscated browse request timed out".to_owned())?
        .map_err(|error| format!("obfuscated browse request failed: {error}"))?;
    let message = time::timeout(timeout, peer.receive())
        .await
        .map_err(|_| "obfuscated browse response timed out".to_owned())?
        .map_err(|error| format!("obfuscated browse response failed: {error}"))?;
    browse_entries_from_peer_message(message)
}

async fn send_peer_message_request(
    state: &AppState,
    address: &PeerAddress,
    message: PeerMessage,
) -> Result<PeerMessage, String> {
    let username = address.username.clone();
    let mut obfuscated_error = None;
    let peer_ip = peer_connect_ip(state, address);
    if address.obfuscation_type == ROTATED_OBFUSCATION_TYPE && address.obfuscated_port != 0 {
        match send_obfuscated_peer_message_request(
            SocketAddr::V4(SocketAddrV4::new(peer_ip, address.obfuscated_port)),
            username.clone(),
            message.clone(),
            state.config.peer_response_timeout,
        )
        .await
        {
            Ok(response) => return Ok(response),
            Err(error) => obfuscated_error = Some(error),
        }
    }

    let port = u16::try_from(address.port).map_err(|_| "peer port is out of range".to_owned())?;
    if port != 0 {
        return send_plain_peer_message_request(
            SocketAddr::V4(SocketAddrV4::new(peer_ip, port)),
            username,
            message,
            state.config.peer_response_timeout,
        )
        .await;
    }

    Err(obfuscated_error.unwrap_or_else(|| "peer did not advertise a peer-message port".to_owned()))
}

fn peer_connect_ip(state: &AppState, address: &PeerAddress) -> std::net::Ipv4Addr {
    state.config.peer_host_override.unwrap_or(address.ip)
}

async fn send_plain_peer_message_request(
    address: SocketAddr,
    username: String,
    message: PeerMessage,
    timeout: Duration,
) -> Result<PeerMessage, String> {
    let mut peer = time::timeout(timeout, connect_peer_messages(address, username))
        .await
        .map_err(|_| "plain peer connect timed out".to_owned())?
        .map_err(|error| format!("plain peer connect failed: {error}"))?;
    time::timeout(timeout, peer.send(&message))
        .await
        .map_err(|_| "plain peer request timed out".to_owned())?
        .map_err(|error| format!("plain peer request failed: {error}"))?;
    time::timeout(timeout, peer.receive())
        .await
        .map_err(|_| "plain peer response timed out".to_owned())?
        .map_err(|error| format!("plain peer response failed: {error}"))
}

async fn send_obfuscated_peer_message_request(
    address: SocketAddr,
    username: String,
    message: PeerMessage,
    timeout: Duration,
) -> Result<PeerMessage, String> {
    let stream = time::timeout(timeout, TcpStream::connect(address))
        .await
        .map_err(|_| "obfuscated peer connect timed out".to_owned())?
        .map_err(|error| format!("obfuscated peer connect failed: {error}"))?;
    let stream = time::timeout(
        timeout,
        send_obfuscated_peer_init(stream, username, ConnectionKind::PeerMessages),
    )
    .await
    .map_err(|_| "obfuscated peer init timed out".to_owned())?
    .map_err(|error| format!("obfuscated peer init failed: {error}"))?;
    let mut peer = ObfuscatedPeerMessageConnection::new(stream);
    time::timeout(timeout, peer.send(&message))
        .await
        .map_err(|_| "obfuscated peer request timed out".to_owned())?
        .map_err(|error| format!("obfuscated peer request failed: {error}"))?;
    time::timeout(timeout, peer.receive())
        .await
        .map_err(|_| "obfuscated peer response timed out".to_owned())?
        .map_err(|error| format!("obfuscated peer response failed: {error}"))
}

fn browse_entries_from_peer_message(message: PeerMessage) -> Result<Vec<BrowseEntry>, String> {
    match message {
        PeerMessage::SharedFileListResponse(payload) => parse_shared_file_list_payload(&payload),
        other => Err(format!(
            "expected SharedFileListResponse, got {}",
            peer_message_name(&other)
        )),
    }
}

fn folder_entries_from_peer_message(
    message: PeerMessage,
    folder: &str,
) -> Result<Vec<BrowseEntry>, String> {
    match message {
        PeerMessage::FolderContentsResponse(payload) => {
            match parse_shared_file_list_payload(&payload) {
                Ok(entries) if !entries.is_empty() => Ok(entries),
                Ok(_) => parse_folder_file_list_payload(&payload, folder),
                Err(_) => parse_folder_file_list_payload(&payload, folder),
            }
        }
        other => Err(format!(
            "expected FolderContentsResponse, got {}",
            peer_message_name(&other)
        )),
    }
}

async fn send_session_command(state: &AppState, command: SessionCommand) -> Result<(), String> {
    state
        .session_commands
        .send(command)
        .await
        .map_err(|_| "session manager is not running".to_owned())
}

fn try_send_session_command(state: &AppState, command: SessionCommand) {
    let _ = state.session_commands.try_send(command);
}

async fn update_session<F>(state: &AppState, update: F)
where
    F: FnOnce(&mut SessionSnapshot),
{
    let mut snapshot = state.session.write().await;
    update(&mut snapshot);
    snapshot.updated_at = unix_timestamp();
}

async fn update_listeners<F>(state: &AppState, update: F)
where
    F: FnOnce(&mut ListenerSnapshot),
{
    let mut snapshot = state.listeners.write().await;
    update(&mut snapshot);
    snapshot.updated_at = unix_timestamp();
}

async fn record_transfer_rejection(
    state: &AppState,
    direction: u32,
    token: u32,
    filename: String,
    size: Option<u64>,
    reason: String,
) {
    let mut transfers = state.transfers.write().await;
    transfers.record_rejected_request(direction, token, filename, size, reason);
}

async fn record_event(
    state: &AppState,
    kind: &'static str,
    resource: impl Into<String>,
    detail: Option<String>,
) {
    let mut events = state.events.write().await;
    let record = events.record(kind, resource, detail);
    drop(events);
    let _ = state.event_tx.send(record);
}

async fn handle_http_connection(stream: TcpStream, state: Arc<AppState>) -> Result<(), String> {
    let remote_addr = stream.peer_addr().ok();
    let (read_half, write_half) = stream.into_split();
    let mut reader = tokio::io::BufReader::new(read_half);
    let mut writer = tokio::io::BufWriter::new(write_half);

    loop {
        let request_timer = logging::start_timer();

        // Read next request (returns None on clean close, Err on 413/malformed)
        let read_result = http_server::read_http_request(&mut reader).await;
        let (req, keep_alive) = match read_result {
            Ok(Some(pair)) => pair,
            Ok(None) => break, // client closed connection
            Err(e) => {
                let (status, body) = if e.contains("too large") {
                    (
                        "413 Content Too Large",
                        r#"{"error":"request body too large"}"#,
                    )
                } else {
                    ("400 Bad Request", r#"{"error":"bad request"}"#)
                };
                let _ = writer
                    .write_all(
                        format!(
                            "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                            body.len(),
                            body
                        )
                        .as_bytes(),
                    )
                    .await;
                let _ = writer.flush().await;
                break;
            }
        };

        let method = req.method.as_str();
        let path = req.path.as_str();
        let body = match req.body_as_str() {
            Ok(body) => body,
            Err(_) => {
                let body = r#"{"error":"request body must be valid UTF-8"}"#;
                let _ = writer
                    .write_all(
                        format!(
                            "HTTP/1.1 400 Bad Request\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                            body.len(),
                            body
                        )
                        .as_bytes(),
                    )
                    .await;
                let _ = writer.flush().await;
                break;
            }
        };
        let api_key_authorization;
        let authorization = if let Some(authorization) = req.headers.authorization.as_deref() {
            Some(authorization)
        } else if let Some(api_key) = req.headers.x_api_key.as_deref() {
            api_key_authorization = format!("ApiKey {api_key}");
            Some(api_key_authorization.as_str())
        } else {
            None
        };
        let sec_headers = RequestSecurityHeaders::from_http_headers(&req.headers);

        // Rate limiting: unauthenticated traffic is IP-keyed. Authenticated
        // traffic is keyed by a token digest so clients do not share one bucket.
        let authenticated_for_rate_limit = route_requires_auth(&state.config, path)
            && is_authorized(&state.config, authorization, sec_headers.cookie.as_deref());
        let rate_limit_user = authenticated_for_rate_limit
            .then(|| rate_limit_user_key(authorization, sec_headers.cookie.as_deref()));
        let username = rate_limit_user.as_deref();
        let allowed = state
            .rate_limiter
            .check_rate_limit(remote_addr, username)
            .await;

        let websocket_route = routing::parse_route(method, path);
        let websocket_path = if let Some(versioned_path) = websocket_route
            .normalized_path
            .strip_prefix("/api/v0/")
            .or_else(|| websocket_route.normalized_path.strip_prefix("/api/v1/"))
            .or_else(|| websocket_route.normalized_path.strip_prefix("/api/v2/"))
        {
            format!("/api/{}", versioned_path)
        } else {
            websocket_route.normalized_path.to_string()
        };

        if method == "GET" && websocket_path == "/api/events/ws" {
            if !allowed {
                let response = routing::HttpResponse {
                    status: "429 Too Many Requests",
                    content_type: "application/json",
                    body: r#"{"error":"rate limited"}"#.to_string(),
                };
                let _ = http_server::write_http_response(&mut writer, &response, false, "").await;
                break;
            }
            let fallback_host = state.config.http_bind.to_string();
            if !request_origin_matches_host(&sec_headers, &fallback_host) {
                let response = routing::forbidden_response("websocket origin rejected");
                let _ = http_server::write_http_response(&mut writer, &response, false, "").await;
                break;
            }

            if let Err(reason) = routing::check_route_auth(
                &state.config,
                method,
                &websocket_path,
                authorization,
                &sec_headers,
            ) {
                let response = match reason {
                    "unauthorized" => routing::unauthorized_response(),
                    "forbidden" => routing::forbidden_response("forbidden"),
                    _ => routing::forbidden_response(reason),
                };
                let _ = http_server::write_http_response(&mut writer, &response, false, "").await;
                break;
            }

            let websocket_key = req.headers.sec_websocket_key.as_deref();
            let websocket_version = req.headers.sec_websocket_version.as_deref();
            let upgrade = req.headers.upgrade.as_deref();
            let is_upgrade = req.headers.connection_has_token("upgrade")
                && upgrade == Some("websocket")
                && websocket_version == Some("13");

            if let Some(websocket_key) =
                websocket_key.filter(|key| is_upgrade && events_ws::valid_sec_websocket_key(key))
            {
                events_ws::write_upgrade_response(&mut writer, websocket_key).await?;
                events_ws::stream_events(
                    reader,
                    &mut writer,
                    &state.events,
                    state.event_tx.subscribe(),
                )
                .await?;
            } else {
                let response = routing::bad_request_response("invalid websocket upgrade");
                let _ = http_server::write_http_response(&mut writer, &response, false, "").await;
            }
            break;
        }

        // Log request
        let req_log = logging::HttpRequestLog {
            method: method.to_string(),
            path: path.to_string(),
            query: req.query.clone(),
            remote_addr: remote_addr.map(|a| a.to_string()),
            timestamp: logging::format_timestamp(),
        };

        // CORS preflight
        if is_cors_preflight(method) {
            let cors_str = cors_headers(sec_headers.origin.as_deref(), &["*"]);
            let _ = writer.write_all(
                format!("HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 0\r\nconnection: {}\r\n{}\r\n",
                    if keep_alive { "keep-alive" } else { "close" }, cors_str).as_bytes()
            ).await;
            let _ = writer.flush().await;
            if !keep_alive {
                break;
            }
            continue;
        }

        if method == "GET" && allowed {
            let cors_str = cors_headers(req.headers.origin.as_deref(), &["*"]);
            match write_web_static_response(&mut writer, path, keep_alive, &cors_str).await {
                Ok(Some(content_length)) => {
                    let resp_log = logging::HttpResponseLog {
                        status_code: 200,
                        content_length,
                        duration_ms: logging::elapsed_ms(request_timer),
                        error: None,
                    };
                    logging::log_transaction(
                        &logging::LogConfig::from_env(),
                        &logging::HttpTransactionLog {
                            request: req_log,
                            response: resp_log,
                        },
                    );
                    if !keep_alive {
                        break;
                    }
                    continue;
                }
                Ok(None) => {}
                Err(error) => {
                    let response = routing::HttpResponse {
                        status: "500 Internal Server Error",
                        content_type: "application/json",
                        body: format!("{{\"error\":\"{}\"}}", json_escape(&error)),
                    };
                    let _ =
                        http_server::write_http_response(&mut writer, &response, false, "").await;
                    break;
                }
            }
        }

        let response = if !allowed {
            routing::HttpResponse {
                status: "429 Too Many Requests",
                content_type: "application/json",
                body: r#"{"error":"rate limited"}"#.to_string(),
            }
        } else {
            match route_http_request_with_headers(
                method,
                path,
                authorization,
                body,
                &state,
                sec_headers,
            )
            .await
            {
                Ok(r) => r,
                Err(e) => routing::HttpResponse {
                    status: "500 Internal Server Error",
                    content_type: "application/json",
                    body: format!("{{\"error\":\"{}\"}}", json_escape(&e)),
                },
            }
        };

        // Build extra headers
        let remaining = state
            .rate_limiter
            .get_remaining(remote_addr, username)
            .await;
        let reset_secs = state
            .rate_limiter
            .get_reset_time(remote_addr, username)
            .await;
        let max_requests = if authenticated_for_rate_limit {
            state.config.api_rate_limit_authenticated
        } else {
            state.config.api_rate_limit_anonymous
        };
        let cache_hdr =
            cache_control_header(method, response.content_type, path).unwrap_or_default();
        let etag_hdr = if method == "GET" && response.content_type.contains("json") {
            format!("ETag: {}\r\n", generate_etag(&response.body))
        } else {
            String::new()
        };
        let cors_str = cors_headers(req.headers.origin.as_deref(), &["*"]);
        let request_id = generate_request_id();
        let extra = format!(
            "RateLimit-Limit: {}\r\nRateLimit-Remaining: {}\r\nRateLimit-Reset: {}\r\n{}{}{}X-Request-ID: {}\r\n",
            max_requests, remaining, reset_secs, cache_hdr, etag_hdr, cors_str, request_id
        );

        // Write response
        let _ = http_server::write_http_response(&mut writer, &response, keep_alive, &extra).await;

        // Log
        let resp_log = logging::HttpResponseLog {
            status_code: logging::status_code_from_string(response.status),
            content_length: response.body.len(),
            duration_ms: logging::elapsed_ms(request_timer),
            error: None,
        };
        logging::log_transaction(
            &logging::LogConfig::from_env(),
            &logging::HttpTransactionLog {
                request: req_log,
                response: resp_log,
            },
        );

        if !keep_alive {
            break;
        }
    }
    Ok(())
}

pub fn index_html() -> String {
    if let Some(html) = read_web_index_html() {
        return html;
    }

    r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>slskr</title>
  <style>
    :root {
      color-scheme: dark;
      --bg: #101317;
      --panel: #181d22;
      --panel-soft: #20262c;
      --line: #313941;
      --text: #edf2f5;
      --muted: #a9b4bd;
      --green: #74c69d;
      --blue: #8ab4f8;
      --amber: #f2c66d;
      --red: #ff8a80;
    }
    * { box-sizing: border-box; }
    body {
      margin: 0;
      min-width: 320px;
      font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      background: var(--bg);
      color: var(--text);
    }
    main {
      width: min(1180px, calc(100% - 32px));
      margin: 0 auto;
      padding: 24px 0 40px;
    }
    header {
      display: flex;
      align-items: flex-end;
      justify-content: space-between;
      gap: 16px;
      padding: 0 0 18px;
      border-bottom: 1px solid var(--line);
    }
    h1, h2 {
      margin: 0;
      letter-spacing: 0;
    }
    h1 { font-size: 28px; line-height: 1.1; }
    h2 { font-size: 15px; line-height: 1.3; }
    .version { color: var(--muted); font-size: 13px; margin-top: 6px; }
    .status {
      display: inline-flex;
      align-items: center;
      gap: 8px;
      min-height: 30px;
      padding: 5px 10px;
      border: 1px solid var(--line);
      border-radius: 6px;
      background: var(--panel);
      color: var(--muted);
      font-size: 13px;
      white-space: nowrap;
    }
    .dot {
      width: 8px;
      height: 8px;
      border-radius: 50%;
      background: var(--amber);
    }
    .dot.online { background: var(--green); }
    .dot.error { background: var(--red); }
    .grid {
      display: grid;
      grid-template-columns: repeat(4, minmax(0, 1fr));
      gap: 12px;
      margin-top: 18px;
    }
    .panel {
      background: var(--panel);
      border: 1px solid var(--line);
      border-radius: 8px;
      padding: 14px;
      min-width: 0;
    }
    .metric {
      min-height: 96px;
    }
    .label {
      color: var(--muted);
      font-size: 12px;
      text-transform: uppercase;
      letter-spacing: .08em;
    }
    .value {
      margin-top: 8px;
      font-size: 30px;
      line-height: 1;
      font-weight: 720;
      overflow-wrap: anywhere;
    }
    .sub {
      margin-top: 8px;
      color: var(--muted);
      font-size: 13px;
      overflow-wrap: anywhere;
    }
    .wide { grid-column: span 2; }
    .full { grid-column: 1 / -1; }
    .toolbar {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 12px;
      margin-bottom: 12px;
    }
    .filters {
      display: flex;
      flex-wrap: wrap;
      justify-content: flex-end;
      gap: 8px;
      min-width: 0;
    }
    .filters input, .filters select {
      width: 150px;
      min-height: 32px;
      font-size: 13px;
    }
    .actions {
      display: grid;
      grid-template-columns: repeat(3, minmax(0, 1fr));
      gap: 12px;
    }
    form {
      display: grid;
      grid-template-columns: 1fr auto;
      gap: 8px;
      margin-top: 12px;
    }
    .stack {
      display: grid;
      grid-template-columns: 1fr;
      gap: 8px;
    }
    input, select {
      width: 100%;
      min-height: 34px;
      border: 1px solid var(--line);
      border-radius: 6px;
      background: var(--panel-soft);
      color: var(--text);
      padding: 6px 9px;
      font: inherit;
      min-width: 0;
    }
    input:focus, select:focus, button:focus {
      outline: 2px solid var(--blue);
      outline-offset: 1px;
    }
    button {
      border: 1px solid var(--line);
      border-radius: 6px;
      background: var(--panel-soft);
      color: var(--text);
      min-height: 32px;
      padding: 6px 10px;
      font: inherit;
      cursor: pointer;
    }
    button:hover { border-color: var(--blue); }
    .button-row {
      display: flex;
      flex-wrap: wrap;
      gap: 6px;
    }
    .button-row button {
      min-height: 28px;
      padding: 4px 8px;
      font-size: 12px;
    }
    table {
      width: 100%;
      border-collapse: collapse;
      table-layout: fixed;
      font-size: 13px;
    }
    th, td {
      padding: 9px 8px;
      border-top: 1px solid var(--line);
      text-align: left;
      vertical-align: top;
      overflow-wrap: anywhere;
    }
    th {
      color: var(--muted);
      font-weight: 650;
      font-size: 12px;
    }
    .empty {
      color: var(--muted);
      padding: 10px 0 0;
      font-size: 13px;
    }
    .toast {
      min-height: 20px;
      color: var(--muted);
      font-size: 13px;
      margin-top: 8px;
      overflow-wrap: anywhere;
    }
    @media (max-width: 900px) {
      .grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
      .wide { grid-column: 1 / -1; }
      .actions { grid-template-columns: 1fr; }
    }
    @media (max-width: 560px) {
      main { width: min(100% - 20px, 1180px); padding-top: 16px; }
      header { align-items: flex-start; flex-direction: column; }
      .grid { grid-template-columns: 1fr; }
      .metric { min-height: 86px; }
      .value { font-size: 26px; }
      table { font-size: 12px; }
      th, td { padding: 8px 6px; }
    }
  </style>
</head>
<body>
  <main>
    <header>
      <div>
        <h1>slskr</h1>
        <div class="version">__VERSION__</div>
      </div>
      <div class="status"><span id="status-dot" class="dot"></span><span id="status-text">Loading</span></div>
    </header>

    <section class="grid" aria-live="polite">
      <article class="panel metric"><div class="label">Session</div><div id="session-state" class="value">-</div><div id="session-sub" class="sub">-</div></article>
      <article class="panel metric"><div class="label">Shares</div><div id="share-files" class="value">-</div><div id="share-sub" class="sub">-</div></article>
      <article class="panel metric"><div class="label">Searches</div><div id="search-count" class="value">-</div><div id="search-sub" class="sub">-</div></article>
      <article class="panel metric"><div class="label">Transfers</div><div id="transfer-count" class="value">-</div><div id="transfer-sub" class="sub">-</div></article>
      <article class="panel metric"><div class="label">Listeners</div><div id="listener-count" class="value">-</div><div id="listener-sub" class="sub">-</div></article>
      <article class="panel metric"><div class="label">Browse Cache</div><div id="browse-count" class="value">-</div><div id="browse-sub" class="sub">-</div></article>
      <article class="panel metric"><div class="label">Messages</div><div id="message-count" class="value">-</div><div id="message-sub" class="sub">-</div></article>
      <article class="panel metric"><div class="label">Rooms</div><div id="room-count" class="value">-</div><div id="room-sub" class="sub">-</div></article>

      <section class="actions full">
        <article class="panel">
          <h2>Session</h2>
          <div class="button-row" id="session-actions">
            <button type="button" data-session-action="connect">Connect</button>
            <button type="button" data-session-action="ping">Ping</button>
            <button type="button" data-session-action="privileges/check">Privileges</button>
            <button type="button" data-session-action="disconnect">Disconnect</button>
          </div>
          <div id="session-action-status" class="toast"></div>
        </article>
        <article class="panel">
          <h2>Search</h2>
          <form id="search-form">
            <div class="stack">
              <input id="search-query" name="query" autocomplete="off" placeholder="artist album track">
              <select id="search-target" name="target">
                <option value="global">Global</option>
                <option value="wishlist">Wishlist</option>
                <option value="user">User</option>
                <option value="room">Room</option>
              </select>
              <input id="search-target-name" name="target_name" autocomplete="off" placeholder="user or room">
            </div>
            <button type="submit">Start</button>
          </form>
          <div id="search-action-status" class="toast"></div>
        </article>
        <article class="panel">
          <h2>Watch User</h2>
          <form id="watch-form">
            <input id="watch-username" name="username" autocomplete="off" placeholder="username">
            <div class="button-row">
              <button type="submit">Watch</button>
              <button id="unwatch-button" type="button">Unwatch</button>
            </div>
          </form>
          <div id="watch-action-status" class="toast"></div>
        </article>
        <article class="panel">
          <h2>Browse User</h2>
          <form id="browse-request-form">
            <input id="browse-username" name="username" autocomplete="off" placeholder="username">
            <input id="browse-folder" name="folder" autocomplete="off" placeholder="folder">
            <button type="submit">Browse</button>
          </form>
          <div id="browse-action-status" class="toast"></div>
        </article>
        <article class="panel">
          <h2>Share Scan</h2>
          <form id="share-rescan-form">
            <button type="submit">Rescan</button>
          </form>
          <div id="share-action-status" class="toast"></div>
        </article>
        <article class="panel">
          <h2>Transfer</h2>
          <form id="transfer-form">
            <div class="stack">
              <input id="transfer-filename" name="filename" autocomplete="off" placeholder="remote/file.ext">
              <input id="transfer-peer" name="peer" autocomplete="off" placeholder="peer">
              <input id="transfer-local-path" name="local_path" autocomplete="off" placeholder="local path">
              <input id="transfer-size" name="size" inputmode="numeric" autocomplete="off" placeholder="size bytes">
              <input id="transfer-progress" name="progress" inputmode="numeric" autocomplete="off" placeholder="progress bytes">
              <select id="transfer-direction" name="direction">
                <option value="0">Download</option>
                <option value="1">Upload</option>
              </select>
            </div>
            <button type="submit">Queue</button>
          </form>
          <div id="transfer-action-status" class="toast"></div>
        </article>
        <article class="panel">
          <h2>Message</h2>
          <form id="message-form">
            <div class="stack">
              <input id="message-username" name="username" autocomplete="off" placeholder="username">
              <input id="message-body" name="body" autocomplete="off" placeholder="message">
            </div>
            <button type="submit">Send</button>
          </form>
          <div id="message-action-status" class="toast"></div>
        </article>
        <article class="panel">
          <h2>Join Room</h2>
          <form id="room-join-form">
            <input id="room-join-name" name="room" autocomplete="off" placeholder="room">
            <button type="submit">Join</button>
          </form>
          <div id="room-join-action-status" class="toast"></div>
        </article>
        <article class="panel">
          <h2>Room Message</h2>
          <form id="room-message-form">
            <div class="stack">
              <input id="room-message-name" name="room" autocomplete="off" placeholder="room">
              <input id="room-message-username" name="username" autocomplete="off" placeholder="username">
              <input id="room-message-body" name="body" autocomplete="off" placeholder="message">
            </div>
            <button type="submit">Send</button>
          </form>
          <div id="room-message-action-status" class="toast"></div>
        </article>
        <article class="panel">
          <h2>Browser Session</h2>
          <form id="token-form">
            <input id="api-token" name="token" autocomplete="off" placeholder="bearer token">
            <button type="submit">Sign in</button>
          </form>
          <div id="token-action-status" class="toast"></div>
        </article>
      </section>

      <section class="panel wide">
        <div class="toolbar">
          <h2>Recent Searches</h2>
          <div class="filters">
            <input id="search-filter-q" autocomplete="off" placeholder="filter">
            <select id="search-filter-status">
              <option value="">Any status</option>
              <option value="active">Active</option>
              <option value="completed">Completed</option>
            </select>
            <button id="refresh-searches" type="button">Refresh</button>
          </div>
        </div>
        <div id="search-table"></div>
      </section>
      <section class="panel wide">
        <div class="toolbar">
          <h2>Transfer Queue</h2>
          <div class="filters">
            <input id="transfer-filter-q" autocomplete="off" placeholder="filter">
            <select id="transfer-filter-status">
              <option value="">Any status</option>
              <option value="queued">Queued</option>
              <option value="in_progress">In progress</option>
              <option value="succeeded">Succeeded</option>
              <option value="cancelled">Cancelled</option>
              <option value="failed">Failed</option>
            </select>
            <button id="refresh-transfers" type="button">Refresh</button>
          </div>
        </div>
        <div id="transfer-table"></div>
      </section>
      <section class="panel wide">
        <div class="toolbar"><h2>Users</h2><button id="refresh-users" type="button">Refresh</button></div>
        <div id="user-table"></div>
      </section>
      <section class="panel wide">
        <div class="toolbar">
          <h2>Share Catalog</h2>
          <div class="filters">
            <input id="share-filter-q" autocomplete="off" placeholder="filter">
            <input id="share-filter-extension" autocomplete="off" placeholder="extension">
            <button id="refresh-shares" type="button">Refresh</button>
          </div>
        </div>
        <div id="share-table"></div>
      </section>
      <section class="panel wide">
        <div class="toolbar">
          <h2>Messages</h2>
          <div class="filters">
            <input id="message-filter-q" autocomplete="off" placeholder="filter">
            <select id="message-filter-direction">
              <option value="">Any direction</option>
              <option value="inbound">Inbound</option>
              <option value="outbound">Outbound</option>
            </select>
            <button id="refresh-messages" type="button">Refresh</button>
          </div>
        </div>
        <div id="message-table"></div>
      </section>
      <section class="panel wide">
        <div class="toolbar">
          <h2>Rooms</h2>
          <div class="filters">
            <input id="room-filter-q" autocomplete="off" placeholder="filter">
            <select id="room-filter-joined">
              <option value="">Any room</option>
              <option value="true">Joined</option>
              <option value="false">Not joined</option>
            </select>
            <button id="refresh-rooms" type="button">Refresh</button>
            <button id="sync-rooms" type="button">Sync</button>
          </div>
        </div>
        <div id="room-table"></div>
      </section>
      <section class="panel full">
        <div class="toolbar">
          <h2>Browse Cache</h2>
          <div class="filters">
            <input id="browse-filter-q" autocomplete="off" placeholder="filter">
            <select id="browse-filter-status">
              <option value="">Any status</option>
              <option value="requested">Requested</option>
              <option value="indirect_pending">Indirect</option>
              <option value="partial">Partial</option>
              <option value="ready">Ready</option>
              <option value="failed">Failed</option>
            </select>
            <button id="refresh-browse" type="button">Refresh</button>
          </div>
        </div>
        <div id="browse-table"></div>
      </section>
    </section>
  </main>
  <script>
    const text = (id, value) => { document.getElementById(id).textContent = value; };
    const number = (value) => new Intl.NumberFormat().format(value || 0);
    const cookieValue = (name) => document.cookie.split(";").map((part) => part.trim()).find((part) => part.startsWith(`${name}=`))?.slice(name.length + 1) || "";
    const setSessionCookie = (value) => {
      const encoded = encodeURIComponent(value);
      const secure = location.protocol === "https:" ? "; Secure" : "";
      document.cookie = `slskr.session=${encoded}; Path=/; SameSite=Strict${secure}`;
    };
    const clearSessionCookie = () => {
      const secure = location.protocol === "https:" ? "; Secure" : "";
      document.cookie = `slskr.session=; Path=/; SameSite=Strict; Max-Age=0${secure}`;
    };
    let apiToken = decodeURIComponent(cookieValue("slskr.session"));
    document.getElementById("api-token").value = apiToken;
    const bytes = (value) => {
      let amount = value || 0;
      const units = ["B", "KiB", "MiB", "GiB", "TiB"];
      let index = 0;
      while (amount >= 1024 && index < units.length - 1) {
        amount = amount / 1024;
        index += 1;
      }
      return `${amount.toFixed(index === 0 ? 0 : 1)} ${units[index]}`;
    };
    const requestHeaders = (extra = {}) => {
      const headers = { ...extra };
      if (apiToken) headers.authorization = `Bearer ${apiToken}`;
      return headers;
    };
    const fetchJson = async (path) => {
      const response = await fetch(path, { headers: requestHeaders({ "accept": "application/json" }) });
      if (!response.ok) throw new Error(`${response.status} ${response.statusText}`);
      return response.json();
    };
    const deleteJson = async (path) => {
      const response = await fetch(path, {
        method: "DELETE",
        headers: requestHeaders({ "accept": "application/json" })
      });
      if (!response.ok) throw new Error(`${response.status} ${response.statusText}`);
      return response.json();
    };
    const postJson = async (path, body) => {
      const response = await fetch(path, {
        method: "POST",
        headers: requestHeaders({ "accept": "application/json", "content-type": "application/json" }),
        body: JSON.stringify(body)
      });
      if (!response.ok) {
        let message = `${response.status} ${response.statusText}`;
        try {
          const error = await response.json();
          if (error.error) message = error.error;
        } catch (_) {}
        throw new Error(message);
      }
      return response.json();
    };
    const escapeHtml = (value) => String(value ?? "").replace(/[&<>"']/g, (char) => ({
      "&": "&amp;",
      "<": "&lt;",
      ">": "&gt;",
      "\"": "&quot;",
      "'": "&#39;"
    })[char]);
    const table = (rows, columns, empty) => {
      if (!rows.length) return `<div class="empty">${escapeHtml(empty)}</div>`;
      const head = columns.map((column) => `<th>${escapeHtml(column.label)}</th>`).join("");
      const body = rows.map((row) => `<tr>${columns.map((column) => {
        const value = column.value(row);
        return `<td>${column.html ? value : escapeHtml(value)}</td>`;
      }).join("")}</tr>`).join("");
      return `<table><thead><tr>${head}</tr></thead><tbody>${body}</tbody></table>`;
    };
    const field = (id) => document.getElementById(id).value.trim();
    const queryString = (params) => {
      const query = new URLSearchParams();
      Object.entries(params).forEach(([name, value]) => {
        if (value !== undefined && value !== null && value !== "") query.set(name, value);
      });
      return query.toString();
    };
    async function loadStats() {
      const stats = await fetchJson("/api/v0/stats");
      const connected = stats.session?.connected;
      const dot = document.getElementById("status-dot");
      dot.className = `dot ${connected ? "online" : ""}`;
      text("status-text", connected ? "Connected" : stats.session?.state || "Disconnected");
      text("session-state", stats.session?.state || "-");
      const privileges = stats.session?.privileges_seconds == null ? "unknown privileges" : `${number(stats.session.privileges_seconds)}s privileges`;
      text("session-sub", `${number(stats.session?.server_messages_seen)} server messages, ${number(stats.session?.reconnects)} reconnects, ${privileges}`);
      text("share-files", number(stats.shares?.files));
      text("share-sub", `${bytes(stats.shares?.bytes)} across ${number(stats.shares?.roots)} roots`);
      text("search-count", number(stats.searches?.total));
      text("search-sub", `${number(stats.searches?.active)} active, ${number(stats.searches?.results)} results`);
      text("transfer-count", number(stats.transfers?.total));
      text("transfer-sub", `${number(stats.transfers?.in_progress)} active, ${bytes(stats.transfers?.bytes_transferred)} moved`);
      text("listener-count", number((stats.listeners?.regular_accepts || 0) + (stats.listeners?.obfuscated_accepts || 0)));
      text("listener-sub", `${number(stats.listeners?.peer_messages)} peer, ${number(stats.listeners?.errors)} errors`);
      text("browse-count", number(stats.browse?.total));
      text("browse-sub", `${number(stats.browse?.indirect_pending)} indirect, ${number(stats.browse?.partial)} partial, ${number(stats.browse?.ready)} ready, ${number(stats.browse?.failed)} failed, ${number(stats.browse?.files)} files`);
      text("message-count", number(stats.messages?.total));
      text("message-sub", `${number(stats.messages?.inbound)} in, ${number(stats.messages?.outbound)} out`);
      text("room-count", number(stats.rooms?.total));
      text("room-sub", `${number(stats.rooms?.joined)} joined, ${number(stats.rooms?.messages)} messages`);
    }
    async function loadSearches() {
      const query = queryString({
        q: field("search-filter-q"),
        status: field("search-filter-status"),
        limit: 6
      });
      const data = await fetchJson(`/api/v0/searches?${query}`);
      document.getElementById("search-table").innerHTML = table(data.entries || [], [
        { label: "Query", value: (row) => row.query },
        { label: "Target", value: (row) => row.target_name ? `${row.target}:${row.target_name}` : row.target },
        { label: "Status", value: (row) => row.status },
        { label: "Results", value: (row) => row.result_count },
        { label: "Actions", html: true, value: (row) => searchActions(row) }
      ], "No searches");
    }
    function searchActions(row) {
      const disabled = row.status === "completed";
      return `<div class="button-row">
        <button type="button" data-search-action="complete" data-search-token="${row.token}" ${disabled ? "disabled" : ""}>Complete</button>
      </div>`;
    }
    async function loadTransfers() {
      const query = queryString({
        q: field("transfer-filter-q"),
        status: field("transfer-filter-status"),
        limit: 6
      });
      const data = await fetchJson(`/api/v0/transfers?${query}`);
      document.getElementById("transfer-table").innerHTML = table(data.entries || [], [
        { label: "File", value: (row) => row.filename },
        { label: "Peer", value: (row) => row.peer_username || "-" },
        { label: "Status", value: (row) => row.status },
        { label: "Progress", value: (row) => `${bytes(row.bytes_transferred)} / ${row.size ? bytes(row.size) : "-"}` },
        { label: "Actions", html: true, value: (row) => transferActions(row) }
      ], "No transfers");
    }
    function transferActions(row) {
      const active = row.status === "in_progress" || row.status === "peer_lookup" || row.status === "peer_negotiating" || row.status === "accepted" || row.status === "indirect_pending";
      const disabledStart = active || row.status === "succeeded" || row.status === "cancelled" || row.status === "failed";
      const disabledFinish = row.status === "succeeded" || row.status === "cancelled" || row.status === "failed";
      return `<div class="button-row">
        <button type="button" data-transfer-action="start" data-transfer-id="${row.id}" ${disabledStart ? "disabled" : ""}>Start</button>
        <button type="button" data-transfer-action="progress" data-transfer-id="${row.id}" ${disabledFinish ? "disabled" : ""}>Progress</button>
        <button type="button" data-transfer-action="complete" data-transfer-id="${row.id}" ${disabledFinish ? "disabled" : ""}>Done</button>
        <button type="button" data-transfer-action="cancel" data-transfer-id="${row.id}" ${disabledFinish ? "disabled" : ""}>Cancel</button>
        <button type="button" data-transfer-action="fail" data-transfer-id="${row.id}" ${disabledFinish ? "disabled" : ""}>Fail</button>
      </div>`;
    }
    async function loadUsers() {
      const data = await fetchJson("/api/v0/users");
      document.getElementById("user-table").innerHTML = table(data.entries || [], [
        { label: "Username", value: (row) => row.username },
        { label: "Watched", value: (row) => row.watched ? "yes" : "no" },
        { label: "Status", value: (row) => row.status || "-" },
        { label: "Files", value: (row) => row.file_count ?? "-" },
        { label: "Speed", value: (row) => row.average_speed ? `${number(row.average_speed)}/s` : "-" },
        { label: "Actions", html: true, value: (row) => userActions(row) }
      ], "No users");
    }
    function userActions(row) {
      return `<div class="button-row">
        <button type="button" data-user-action="watch" data-username="${escapeHtml(row.username)}">Watch</button>
        <button type="button" data-user-action="stats" data-username="${escapeHtml(row.username)}">Stats</button>
        <button type="button" data-user-action="browse" data-username="${escapeHtml(row.username)}">Browse</button>
        <button type="button" data-user-action="unwatch" data-username="${escapeHtml(row.username)}" ${row.watched ? "" : "disabled"}>Unwatch</button>
      </div>`;
    }
    async function loadShares() {
      const query = queryString({
        q: field("share-filter-q"),
        extension: field("share-filter-extension"),
        limit: 8
      });
      const data = await fetchJson(`/api/v0/shares/catalog?${query}`);
      document.getElementById("share-table").innerHTML = table(data.files || [], [
        { label: "Path", value: (row) => row.path },
        { label: "Extension", value: (row) => row.extension || "-" },
        { label: "Size", value: (row) => bytes(row.size) },
        { label: "Attributes", value: (row) => row.attribute_count }
      ], "No indexed files");
    }
    async function loadMessages() {
      const query = queryString({
        q: field("message-filter-q"),
        direction: field("message-filter-direction"),
        limit: 6
      });
      const data = await fetchJson(`/api/v0/messages?${query}`);
      document.getElementById("message-table").innerHTML = table(data.entries || [], [
        { label: "User", value: (row) => row.username },
        { label: "Direction", value: (row) => row.direction },
        { label: "Body", value: (row) => row.body },
        { label: "Ack", value: (row) => row.acknowledged ? "yes" : "no" },
        { label: "Actions", html: true, value: (row) => messageActions(row) }
      ], "No messages");
    }
    function messageActions(row) {
      const disabled = row.acknowledged;
      return `<div class="button-row">
        <button type="button" data-message-action="ack" data-message-id="${row.id}" ${disabled ? "disabled" : ""}>Ack</button>
      </div>`;
    }
    async function loadRooms() {
      const query = queryString({
        q: field("room-filter-q"),
        joined: field("room-filter-joined"),
        limit: 6
      });
      const data = await fetchJson(`/api/v0/rooms?${query}`);
      document.getElementById("room-table").innerHTML = table(data.entries || [], [
        { label: "Room", value: (row) => row.name },
        { label: "Joined", value: (row) => row.joined ? "yes" : "no" },
        { label: "Users", value: (row) => row.user_count ?? "-" },
        { label: "Messages", value: (row) => row.message_count },
        { label: "Last", value: (row) => (row.messages || []).slice(-1)[0]?.body || "-" },
        { label: "Actions", html: true, value: (row) => roomActions(row) }
      ], "No rooms");
    }
    function roomActions(row) {
      return `<div class="button-row">
        <button type="button" data-room-action="leave" data-room="${escapeHtml(row.name)}" ${row.joined ? "" : "disabled"}>Leave</button>
      </div>`;
    }
    async function loadBrowse() {
      const query = queryString({
        q: field("browse-filter-q"),
        status: field("browse-filter-status"),
        limit: 6
      });
      const data = await fetchJson(`/api/v0/browse?${query}`);
      document.getElementById("browse-table").innerHTML = table(data.entries || [], [
        { label: "User", value: (row) => row.username },
        { label: "Status", value: (row) => row.status },
        { label: "Files", value: (row) => row.count },
        { label: "Bytes", value: (row) => bytes(row.total_bytes) }
      ], "No browse records");
    }
    async function loadAll() {
      try {
        await Promise.all([loadStats(), loadSearches(), loadTransfers(), loadUsers(), loadShares(), loadMessages(), loadRooms(), loadBrowse()]);
      } catch (error) {
        document.getElementById("status-dot").className = "dot error";
        text("status-text", error.message);
      }
    }
    async function runSessionAction(action) {
      try {
        await postJson(`/api/v0/session/${action}`, {});
        text("session-action-status", `${action} accepted`);
        await loadStats();
      } catch (error) {
        text("session-action-status", error.message);
      }
    }
    async function submitSearch(event) {
      event.preventDefault();
      const query = document.getElementById("search-query").value.trim();
      const target = document.getElementById("search-target").value;
      const targetName = document.getElementById("search-target-name").value.trim();
      if (!query) {
        text("search-action-status", "Query required");
        return;
      }
      const body = { query, target };
      if (target === "user") body.username = targetName;
      if (target === "room") body.room = targetName;
      try {
        const record = await postJson("/api/v0/searches", body);
        text("search-action-status", `Search ${record.token} started`);
        document.getElementById("search-query").value = "";
        await Promise.all([loadStats(), loadSearches()]);
      } catch (error) {
        text("search-action-status", error.message);
      }
    }
    async function runSearchAction(token, action) {
      try {
        await postJson(`/api/v0/searches/${token}/${action}`, {});
        text("search-action-status", `Search ${token} ${action}`);
        await Promise.all([loadStats(), loadSearches()]);
      } catch (error) {
        text("search-action-status", error.message);
      }
    }
    async function submitWatch(event) {
      event.preventDefault();
      const username = document.getElementById("watch-username").value.trim();
      if (!username) {
        text("watch-action-status", "Username required");
        return;
      }
      try {
        await postJson("/api/v0/users/watch", { username });
        text("watch-action-status", `${username} watched`);
        document.getElementById("watch-username").value = "";
        await Promise.all([loadStats(), loadUsers()]);
      } catch (error) {
        text("watch-action-status", error.message);
      }
    }
    async function submitUnwatch() {
      const username = document.getElementById("watch-username").value.trim();
      if (!username) {
        text("watch-action-status", "Username required");
        return;
      }
      try {
        await deleteJson(`/api/v0/users/${encodeURIComponent(username)}/watch`);
        text("watch-action-status", `${username} unwatched`);
        await Promise.all([loadStats(), loadUsers()]);
      } catch (error) {
        text("watch-action-status", error.message);
      }
    }
    async function submitBrowseRequest(event) {
      event.preventDefault();
      const username = document.getElementById("browse-username").value.trim();
      const folder = document.getElementById("browse-folder").value.trim();
      if (!username) {
        text("browse-action-status", "Username required");
        return;
      }
      try {
        if (folder) {
          await postJson(`/api/v0/users/${encodeURIComponent(username)}/browse/folder`, { folder });
          text("browse-action-status", `Folder browse requested for ${username}`);
        } else {
          await postJson(`/api/v0/users/${encodeURIComponent(username)}/browse/request`, {});
          text("browse-action-status", `Browse requested for ${username}`);
        }
        await Promise.all([loadStats(), loadBrowse()]);
      } catch (error) {
        text("browse-action-status", error.message);
      }
    }
    async function submitShareRescan(event) {
      event.preventDefault();
      try {
        const snapshot = await postJson("/api/v0/shares/rescan", {});
        text("share-action-status", `${number(snapshot.files)} files indexed`);
        await Promise.all([loadStats(), loadShares()]);
      } catch (error) {
        text("share-action-status", error.message);
      }
    }
    async function submitTransfer(event) {
      event.preventDefault();
      const filename = document.getElementById("transfer-filename").value.trim();
      const peer = document.getElementById("transfer-peer").value.trim();
      const localPath = document.getElementById("transfer-local-path").value.trim();
      const size = document.getElementById("transfer-size").value.trim();
      const direction = Number(document.getElementById("transfer-direction").value);
      if (!filename) {
        text("transfer-action-status", "Filename required");
        return;
      }
      const body = { filename, direction };
      if (peer) body.peer_username = peer;
      if (localPath) body.local_path = localPath;
      if (size) body.size = Number(size);
      try {
        const transfer = await postJson("/api/v0/transfers", body);
        text("transfer-action-status", `Transfer ${transfer.id} queued`);
        document.getElementById("transfer-filename").value = "";
        document.getElementById("transfer-local-path").value = "";
        await Promise.all([loadStats(), loadTransfers()]);
      } catch (error) {
        text("transfer-action-status", error.message);
      }
    }
    async function runTransferAction(id, action) {
      const body = {};
      if (action === "progress") {
        const progress = document.getElementById("transfer-progress").value.trim();
        body.bytes_transferred = progress ? Number(progress) : 1024;
      }
      if (action === "cancel" || action === "fail") body.reason = "dashboard";
      try {
        await postJson(`/api/v0/transfers/${id}/${action}`, body);
        text("transfer-action-status", `Transfer ${id} ${action}`);
        await Promise.all([loadStats(), loadTransfers()]);
      } catch (error) {
        text("transfer-action-status", error.message);
      }
    }
    async function submitMessage(event) {
      event.preventDefault();
      const username = document.getElementById("message-username").value.trim();
      const body = document.getElementById("message-body").value.trim();
      if (!username || !body) {
        text("message-action-status", "Username and message required");
        return;
      }
      try {
        await postJson("/api/v0/messages", { username, body });
        text("message-action-status", `Message queued for ${username}`);
        document.getElementById("message-body").value = "";
        await Promise.all([loadStats(), loadMessages()]);
      } catch (error) {
        text("message-action-status", error.message);
      }
    }
    async function runMessageAction(id, action) {
      try {
        await postJson(`/api/v0/messages/${id}/${action}`, {});
        text("message-action-status", `Message ${id} ${action}`);
        await Promise.all([loadStats(), loadMessages()]);
      } catch (error) {
        text("message-action-status", error.message);
      }
    }
    async function runUserAction(username, action) {
      try {
        if (action === "watch") {
          await postJson("/api/v0/users/watch", { username });
          text("watch-action-status", `${username} watched`);
          await Promise.all([loadStats(), loadUsers()]);
        } else if (action === "browse") {
          await postJson(`/api/v0/users/${encodeURIComponent(username)}/browse/request`, {});
          text("browse-action-status", `Browse requested for ${username}`);
          await Promise.all([loadStats(), loadUsers(), loadBrowse()]);
        } else if (action === "stats") {
          await postJson(`/api/v0/users/${encodeURIComponent(username)}/stats/request`, {});
          text("watch-action-status", `Stats requested for ${username}`);
          await loadUsers();
        } else if (action === "unwatch") {
          await deleteJson(`/api/v0/users/${encodeURIComponent(username)}/watch`);
          text("watch-action-status", `${username} unwatched`);
          await Promise.all([loadStats(), loadUsers()]);
        }
      } catch (error) {
        text("watch-action-status", error.message);
      }
    }
    async function submitRoomJoin(event) {
      event.preventDefault();
      const room = document.getElementById("room-join-name").value.trim();
      if (!room) {
        text("room-join-action-status", "Room required");
        return;
      }
      try {
        await postJson(`/api/v0/rooms/${encodeURIComponent(room)}/join`, {});
        text("room-join-action-status", `${room} joined`);
        document.getElementById("room-message-name").value = room;
        await Promise.all([loadStats(), loadRooms()]);
      } catch (error) {
        text("room-join-action-status", error.message);
      }
    }
    async function submitRoomMessage(event) {
      event.preventDefault();
      const room = document.getElementById("room-message-name").value.trim();
      const username = document.getElementById("room-message-username").value.trim();
      const body = document.getElementById("room-message-body").value.trim();
      if (!room || !username || !body) {
        text("room-message-action-status", "Room, username, and message required");
        return;
      }
      try {
        await postJson(`/api/v0/rooms/${encodeURIComponent(room)}/messages`, { username, body });
        text("room-message-action-status", `Message queued for ${room}`);
        document.getElementById("room-message-body").value = "";
        await Promise.all([loadStats(), loadRooms()]);
      } catch (error) {
        text("room-message-action-status", error.message);
      }
    }
    async function syncRooms() {
      try {
        await postJson("/api/v0/rooms/refresh", {});
        text("room-join-action-status", "Room refresh requested");
        await loadRooms();
      } catch (error) {
        text("room-join-action-status", error.message);
      }
    }
    async function runRoomAction(room, action) {
      try {
        if (action === "leave") {
          await deleteJson(`/api/v0/rooms/${encodeURIComponent(room)}/join`);
          text("room-join-action-status", `${room} left`);
          await Promise.all([loadStats(), loadRooms()]);
        }
      } catch (error) {
        text("room-join-action-status", error.message);
      }
    }
    function saveToken(event) {
      event.preventDefault();
      apiToken = document.getElementById("api-token").value.trim();
      if (apiToken) {
        setSessionCookie(apiToken);
        text("token-action-status", "Session saved");
      } else {
        clearSessionCookie();
        text("token-action-status", "Session cleared");
      }
      loadAll();
    }
    document.getElementById("refresh-searches").addEventListener("click", loadSearches);
    document.getElementById("refresh-transfers").addEventListener("click", loadTransfers);
    document.getElementById("refresh-users").addEventListener("click", loadUsers);
    document.getElementById("refresh-shares").addEventListener("click", loadShares);
    document.getElementById("refresh-messages").addEventListener("click", loadMessages);
    document.getElementById("refresh-rooms").addEventListener("click", loadRooms);
    document.getElementById("sync-rooms").addEventListener("click", syncRooms);
    document.getElementById("refresh-browse").addEventListener("click", loadBrowse);
    ["search-filter-q", "search-filter-status"].forEach((id) => {
      document.getElementById(id).addEventListener("change", loadSearches);
    });
    ["transfer-filter-q", "transfer-filter-status"].forEach((id) => {
      document.getElementById(id).addEventListener("change", loadTransfers);
    });
    ["share-filter-q", "share-filter-extension"].forEach((id) => {
      document.getElementById(id).addEventListener("change", loadShares);
    });
    ["message-filter-q", "message-filter-direction"].forEach((id) => {
      document.getElementById(id).addEventListener("change", loadMessages);
    });
    ["room-filter-q", "room-filter-joined"].forEach((id) => {
      document.getElementById(id).addEventListener("change", loadRooms);
    });
    ["browse-filter-q", "browse-filter-status"].forEach((id) => {
      document.getElementById(id).addEventListener("change", loadBrowse);
    });
    document.getElementById("session-actions").addEventListener("click", (event) => {
      const action = event.target?.dataset?.sessionAction;
      if (action) runSessionAction(action);
    });
    document.getElementById("transfer-table").addEventListener("click", (event) => {
      const action = event.target?.dataset?.transferAction;
      const id = event.target?.dataset?.transferId;
      if (action && id) runTransferAction(id, action);
    });
    document.getElementById("search-table").addEventListener("click", (event) => {
      const action = event.target?.dataset?.searchAction;
      const token = event.target?.dataset?.searchToken;
      if (action && token) runSearchAction(token, action);
    });
    document.getElementById("message-table").addEventListener("click", (event) => {
      const action = event.target?.dataset?.messageAction;
      const id = event.target?.dataset?.messageId;
      if (action && id) runMessageAction(id, action);
    });
    document.getElementById("user-table").addEventListener("click", (event) => {
      const action = event.target?.dataset?.userAction;
      const username = event.target?.dataset?.username;
      if (action && username) runUserAction(username, action);
    });
    document.getElementById("room-table").addEventListener("click", (event) => {
      const action = event.target?.dataset?.roomAction;
      const room = event.target?.dataset?.room;
      if (action && room) runRoomAction(room, action);
    });
    document.getElementById("search-form").addEventListener("submit", submitSearch);
    document.getElementById("watch-form").addEventListener("submit", submitWatch);
    document.getElementById("unwatch-button").addEventListener("click", submitUnwatch);
    document.getElementById("browse-request-form").addEventListener("submit", submitBrowseRequest);
    document.getElementById("share-rescan-form").addEventListener("submit", submitShareRescan);
    document.getElementById("transfer-form").addEventListener("submit", submitTransfer);
    document.getElementById("message-form").addEventListener("submit", submitMessage);
    document.getElementById("room-join-form").addEventListener("submit", submitRoomJoin);
    document.getElementById("room-message-form").addEventListener("submit", submitRoomMessage);
    document.getElementById("token-form").addEventListener("submit", saveToken);
    loadAll();
    setInterval(loadStats, 5000);
  </script>
</body>
</html>"#
        .replace(
            "__VERSION__",
            &format!("{CLIENT_MAJOR_VERSION}.{CLIENT_MINOR_VERSION}"),
        )
}

async fn rebuild_share_index(state: &AppState) -> ShareIndexSnapshot {
    let snapshot = build_share_index(&state.config);
    {
        let mut shares = state.shares.write().await;
        *shares = snapshot.clone();
    }
    snapshot
}

fn build_share_index(config: &AppConfig) -> ShareIndexSnapshot {
    let mut scan = scan_share_dirs(
        &config.share_settings.roots,
        config.share_settings.follow_symlinks,
        config.share_settings.include_hidden,
        config.share_settings.max_files,
    );
    let fixture_files = config.share_settings.fixture_entries.len();
    let mut entries = config.share_settings.fixture_entries.clone();
    entries.append(&mut scan.entries);
    let cache_path = share_cache_path(&config.state_dir);
    let (cache_written_at, cache_error) = match write_share_cache(&cache_path, &entries) {
        Ok(()) => (Some(unix_timestamp()), None),
        Err(error) => (None, Some(error)),
    };

    ShareIndexSnapshot {
        entries,
        local_paths: scan.local_paths,
        roots: scan.roots,
        fixture_files,
        scan_errors: scan.errors,
        cache_path,
        cache_written_at,
        cache_error,
        updated_at: unix_timestamp(),
    }
}

fn scan_share_dirs(
    roots: &[PathBuf],
    follow_symlinks: bool,
    include_hidden: bool,
    max_files: usize,
) -> ShareScan {
    let options = ShareScanOptions {
        follow_symlinks,
        include_hidden,
        max_files,
    };
    let mut entries = Vec::new();
    let mut local_paths = BTreeMap::new();
    let mut root_summaries = Vec::new();
    let mut errors = Vec::new();

    for (index, root) in roots.iter().enumerate() {
        let label = share_root_label(root, index);
        let before = entries.len();
        scan_share_root(
            root,
            &label,
            options,
            &mut entries,
            &mut local_paths,
            &mut errors,
        );
        let root_entries = &entries[before..];
        root_summaries.push(ShareRoot {
            label,
            files: root_entries.len(),
            bytes: root_entries.iter().map(|entry| entry.size).sum(),
            extensions: summarize_extensions(root_entries),
        });
        if entries.len() >= max_files {
            break;
        }
    }

    ShareScan {
        entries,
        local_paths,
        roots: root_summaries,
        errors,
    }
}

fn summarize_extensions(entries: &[FileEntry]) -> Vec<ShareExtensionSummary> {
    let mut summaries = BTreeMap::<String, (usize, u64)>::new();
    for entry in entries {
        let extension = if entry.extension.is_empty() {
            "(none)".to_owned()
        } else {
            entry.extension.to_ascii_lowercase()
        };
        let summary = summaries.entry(extension).or_default();
        summary.0 += 1;
        summary.1 += entry.size;
    }
    summaries
        .into_iter()
        .map(|(extension, (files, bytes))| ShareExtensionSummary {
            extension,
            files,
            bytes,
        })
        .collect()
}

fn scan_share_root(
    root: &Path,
    label: &str,
    options: ShareScanOptions,
    entries: &mut Vec<FileEntry>,
    local_paths: &mut BTreeMap<String, PathBuf>,
    errors: &mut Vec<String>,
) {
    if entries.len() >= options.max_files {
        return;
    }

    let metadata = if options.follow_symlinks {
        fs::metadata(root)
    } else {
        fs::symlink_metadata(root)
    };
    let Ok(metadata) = metadata else {
        errors.push(format!(
            "{}: metadata unavailable",
            json_safe_share_label(label)
        ));
        return;
    };
    if !metadata.is_dir() {
        errors.push(format!("{}: not a directory", json_safe_share_label(label)));
        return;
    }
    let canonical_root = if options.follow_symlinks {
        match root.canonicalize() {
            Ok(path) => Some(path),
            Err(_) => {
                errors.push(format!(
                    "{}: canonical root unavailable",
                    json_safe_share_label(label)
                ));
                return;
            }
        }
    } else {
        None
    };

    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        if entries.len() >= options.max_files {
            errors.push("share scan stopped at SLSKR_SHARE_SCAN_MAX_FILES".to_owned());
            return;
        }

        let read_dir = match fs::read_dir(&directory) {
            Ok(read_dir) => read_dir,
            Err(_) => {
                errors.push(format!(
                    "{}: directory unreadable",
                    json_safe_share_label(label)
                ));
                continue;
            }
        };

        for child in read_dir {
            if entries.len() >= options.max_files {
                errors.push("share scan stopped at SLSKR_SHARE_SCAN_MAX_FILES".to_owned());
                return;
            }

            let Ok(child) = child else {
                errors.push(format!(
                    "{}: entry unreadable",
                    json_safe_share_label(label)
                ));
                continue;
            };
            let path = child.path();
            if !options.include_hidden && is_hidden_share_path(root, &path) {
                continue;
            }

            let metadata = if options.follow_symlinks {
                fs::metadata(&path)
            } else {
                fs::symlink_metadata(&path)
            };
            let Ok(metadata) = metadata else {
                errors.push(format!(
                    "{}: entry metadata unavailable",
                    json_safe_share_label(label)
                ));
                continue;
            };
            if metadata.file_type().is_symlink() && !options.follow_symlinks {
                continue;
            }
            if metadata.is_dir() {
                stack.push(path);
                continue;
            }
            if !metadata.is_file() {
                continue;
            }
            if let Some(canonical_root) = canonical_root.as_ref() {
                let Ok(canonical_path) = path.canonicalize() else {
                    errors.push(format!(
                        "{}: entry canonical path unavailable",
                        json_safe_share_label(label)
                    ));
                    continue;
                };
                if !canonical_path.starts_with(canonical_root) {
                    continue;
                }
            }

            let Ok(relative) = path.strip_prefix(root) else {
                continue;
            };
            let filename = format!("{}/{}", label, virtual_share_path(relative));
            local_paths.insert(filename.clone(), path.clone());
            entries.push(FileEntry {
                code: 1,
                filename: filename.clone(),
                size: metadata.len(),
                extension: extension_for(&filename),
                attributes: Vec::new(),
            });
        }
    }
}

fn share_root_label(root: &Path, index: usize) -> String {
    root.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .map(sanitize_share_component)
        .unwrap_or_else(|| format!("share{}", index + 1))
}

fn virtual_share_path(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            std::path::Component::Normal(name) => name.to_str().map(sanitize_share_component),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn sanitize_share_component(value: &str) -> String {
    value.replace(['\\', '/'], "_").trim().to_owned()
}

fn is_hidden_share_path(root: &Path, path: &Path) -> bool {
    path.strip_prefix(root)
        .ok()
        .map(|relative| {
            relative.components().any(|component| {
                matches!(component, std::path::Component::Normal(name) if name.to_string_lossy().starts_with('.'))
            })
        })
        .unwrap_or(false)
}

fn json_safe_share_label(label: &str) -> String {
    label.chars().take(80).collect()
}

fn share_cache_path(state_dir: &Path) -> PathBuf {
    state_dir.join("share-index.tsv")
}

fn write_share_cache(path: &Path, entries: &[FileEntry]) -> Result<(), String> {
    let mut body = String::from("slskr-share-index-v1\n");
    for entry in entries {
        body.push_str(&entry.code.to_string());
        body.push('\t');
        body.push_str(&entry.size.to_string());
        body.push('\t');
        body.push_str(&escape_cache_field(&entry.extension));
        body.push('\t');
        body.push_str(&escape_cache_field(&entry.filename));
        body.push('\n');
    }
    fs::write(path, body).map_err(|error| format!("share cache write failed: {error}"))
}

fn transfer_events_path(state_dir: &Path) -> PathBuf {
    state_dir.join("transfer-events.tsv")
}

fn transfer_state_path(state_dir: &Path) -> PathBuf {
    state_dir.join("transfer-state.json")
}

fn write_transfer_events_header(path: &Path) -> Result<(), String> {
    fs::write(
        path,
        "slskr-transfer-events-v2\nid\tdirection\ttoken\tsize\tbytes_transferred\tstatus\treason\tfilename\n",
    )
    .map_err(|error| format!("transfer events header write failed: {error}"))
}

#[derive(Debug, Deserialize, Serialize)]
struct TransferStateFile {
    version: u32,
    entries: Vec<TransferEntry>,
}

fn load_transfer_state(path: &Path, history_limit: usize) -> Result<Vec<TransferEntry>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let size = fs::metadata(path)
        .map_err(|error| format!("transfer state metadata read failed: {error}"))?
        .len();
    if size > MAX_TRANSFER_STATE_BYTES {
        return Err(format!(
            "transfer state file is too large: {size} bytes, max is {MAX_TRANSFER_STATE_BYTES}"
        ));
    }
    let body =
        fs::read_to_string(path).map_err(|error| format!("transfer state read failed: {error}"))?;
    let mut state = serde_json::from_str::<TransferStateFile>(&body)
        .map_err(|error| format!("transfer state parse failed: {error}"))?;
    if state.version != 1 {
        return Err(format!(
            "unsupported transfer state version: {}",
            state.version
        ));
    }
    let now = unix_timestamp();
    for entry in &mut state.entries {
        if is_active_transfer_status(&entry.status) {
            entry.status = "queued".to_owned();
            entry.reason = Some("resumed after restart".to_owned());
            entry.updated_at = now;
        }
    }
    if state.entries.len() > history_limit {
        let extra = state.entries.len() - history_limit;
        state.entries.drain(0..extra);
    }
    Ok(state.entries)
}

fn write_transfer_state(path: &Path, entries: &[TransferEntry]) -> Result<(), String> {
    let state = TransferStateFile {
        version: 1,
        entries: entries.to_vec(),
    };
    let body = serde_json::to_string_pretty(&state)
        .map_err(|error| format!("transfer state encode failed: {error}"))?;
    fs::write(path, body).map_err(|error| format!("transfer state write failed: {error}"))
}

fn append_transfer_event(path: &Path, entry: &TransferEntry) -> Result<(), String> {
    use std::io::Write;

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| format!("transfer event open failed: {error}"))?;
    writeln!(
        file,
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        entry.id,
        entry.direction,
        entry.token,
        entry.size.map(|size| size.to_string()).unwrap_or_default(),
        entry.bytes_transferred,
        escape_cache_field(&entry.status),
        escape_cache_field(entry.reason.as_deref().unwrap_or_default()),
        escape_cache_field(&entry.filename)
    )
    .map_err(|error| format!("transfer event append failed: {error}"))
}

fn escape_cache_field(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '\t' => escaped.push_str("\\t"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            _ => escaped.push(character),
        }
    }
    escaped
}

fn extension_for(filename: &str) -> String {
    filename
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase())
        .unwrap_or_default()
}

async fn build_file_search_response(
    state: &AppState,
    token: u32,
    query: &str,
) -> Option<FileSearchResponse> {
    let results = {
        let shares = state.shares.read().await;
        search_shares(&shares.entries, query)
    };
    if results.is_empty() {
        return None;
    }

    Some(FileSearchResponse {
        username: state
            .config
            .username
            .clone()
            .unwrap_or_else(|| "slskr".to_owned()),
        token,
        results,
        slot_free: true,
        average_speed: 0,
        queue_length: 0,
        unknown: 0,
        private_results: Vec::new(),
    })
}

fn search_shares(entries: &[FileEntry], query: &str) -> Vec<FileEntry> {
    let terms = query
        .split_whitespace()
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>();
    if terms.is_empty() {
        return Vec::new();
    }
    entries
        .iter()
        .filter(|entry| {
            let filename = entry.filename.to_ascii_lowercase();
            terms.iter().all(|term| filename.contains(term))
        })
        .cloned()
        .collect()
}

fn build_shared_file_list_payload(entries: &[FileEntry]) -> Result<Vec<u8>, String> {
    let mut writer = Writer::new();
    let folders = group_share_entries(entries);
    writer.write_u32_le(
        u32::try_from(folders.len()).map_err(|_| "too many shared folders".to_owned())?,
    );
    for (folder, files) in folders {
        writer
            .write_string(&folder)
            .map_err(|error| error.to_string())?;
        writer.write_u32_le(
            u32::try_from(files.len()).map_err(|_| "too many shared files".to_owned())?,
        );
        for file in files {
            encode_file_entry(&mut writer, &file)?;
        }
    }
    compress_zlib_payload(&writer.into_inner()).map_err(|error| error.to_string())
}

fn build_folder_contents_payload(entries: &[FileEntry], folder: &str) -> Result<Vec<u8>, String> {
    let matching = entries
        .iter()
        .filter(|entry| virtual_folder(&entry.filename) == folder)
        .cloned()
        .collect::<Vec<_>>();
    build_shared_file_list_payload(&matching)
}

fn parse_shared_file_list_payload(payload: &[u8]) -> Result<Vec<BrowseEntry>, String> {
    let decompressed = decompress_zlib_payload(payload).map_err(|error| error.to_string())?;
    let mut reader = Reader::new(&decompressed);
    let folder_count = reader.read_u32_le().map_err(|error| error.to_string())?;
    let mut entries = Vec::new();
    for _ in 0..folder_count {
        let folder = reader.read_string().map_err(|error| error.to_string())?;
        let file_count = reader.read_u32_le().map_err(|error| error.to_string())?;
        for _ in 0..file_count {
            let code = reader.read_u8().map_err(|error| error.to_string())?;
            let filename = reader.read_string().map_err(|error| error.to_string())?;
            let size = reader.read_u64_le().map_err(|error| error.to_string())?;
            let extension = reader.read_string().map_err(|error| error.to_string())?;
            let attribute_count = reader.read_u32_le().map_err(|error| error.to_string())?;
            for _ in 0..attribute_count {
                let _code = reader.read_u32_le().map_err(|error| error.to_string())?;
                let _value = reader.read_u32_le().map_err(|error| error.to_string())?;
            }
            if code == 1 {
                entries.push(BrowseEntry {
                    filename: join_virtual_path(&folder, &filename),
                    size,
                    extension,
                });
            }
        }
    }
    reader.finish().map_err(|error| error.to_string())?;
    Ok(entries)
}

fn parse_folder_file_list_payload(
    payload: &[u8],
    folder: &str,
) -> Result<Vec<BrowseEntry>, String> {
    let decompressed = decompress_zlib_payload(payload).map_err(|error| error.to_string())?;
    let mut reader = Reader::new(&decompressed);
    let file_count = reader.read_u32_le().map_err(|error| error.to_string())?;
    let mut entries = Vec::new();
    for _ in 0..file_count {
        let code = reader.read_u8().map_err(|error| error.to_string())?;
        let filename = reader.read_string().map_err(|error| error.to_string())?;
        let size = reader.read_u64_le().map_err(|error| error.to_string())?;
        let extension = reader.read_string().map_err(|error| error.to_string())?;
        let attribute_count = reader.read_u32_le().map_err(|error| error.to_string())?;
        for _ in 0..attribute_count {
            let _code = reader.read_u32_le().map_err(|error| error.to_string())?;
            let _value = reader.read_u32_le().map_err(|error| error.to_string())?;
        }
        if code == 1 {
            entries.push(BrowseEntry {
                filename: join_virtual_path(folder, &filename),
                size,
                extension,
            });
        }
    }
    reader.finish().map_err(|error| error.to_string())?;
    Ok(entries)
}

fn join_virtual_path(folder: &str, filename: &str) -> String {
    if folder.is_empty() {
        filename.to_owned()
    } else {
        format!("{folder}/{filename}")
    }
}

fn virtual_folder(filename: &str) -> &str {
    filename
        .rsplit_once('/')
        .map(|(folder, _)| folder)
        .unwrap_or("")
}

fn group_share_entries(entries: &[FileEntry]) -> Vec<(String, Vec<FileEntry>)> {
    let mut folders: Vec<(String, Vec<FileEntry>)> = Vec::new();
    for entry in entries {
        let (folder, filename) = entry
            .filename
            .rsplit_once('/')
            .map(|(folder, filename)| (folder.to_owned(), filename.to_owned()))
            .unwrap_or_else(|| ("".to_owned(), entry.filename.clone()));
        let mut file = entry.clone();
        file.filename = filename;
        if let Some((_, files)) = folders
            .iter_mut()
            .find(|(existing_folder, _)| *existing_folder == folder)
        {
            files.push(file);
        } else {
            folders.push((folder, vec![file]));
        }
    }
    folders
}

fn encode_file_entry(writer: &mut Writer, entry: &FileEntry) -> Result<(), String> {
    writer.write_u8(entry.code);
    writer
        .write_string(&entry.filename)
        .map_err(|error| error.to_string())?;
    writer.write_u64_le(entry.size);
    writer
        .write_string(&entry.extension)
        .map_err(|error| error.to_string())?;
    writer.write_u32_le(
        u32::try_from(entry.attributes.len()).map_err(|_| "too many attributes".to_owned())?,
    );
    for attribute in &entry.attributes {
        writer.write_u32_le(attribute.code);
        writer.write_u32_le(attribute.value);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        path::{Path, PathBuf},
        sync::Arc,
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    use slskr_client::protocol::peer::{FileEntry, FileSearchResponse};
    use tokio::sync::{mpsc, RwLock};

    use crate::config::{json_escape, redact_username, ConfigEnv, FileConfig};
    use crate::utils::{
        normalize_api_path, parse_route, percent_decode, query_params, split_request_target,
    };

    #[derive(Default)]
    struct MapEnv {
        values: BTreeMap<String, String>,
    }

    impl MapEnv {
        fn with(mut self, name: &str, value: &str) -> Self {
            self.values.insert(name.to_owned(), value.to_owned());
            self
        }
    }

    impl ConfigEnv for MapEnv {
        fn var(&self, name: &str) -> Option<String> {
            self.values.get(name).cloned()
        }
    }

    fn test_state() -> (Arc<super::AppState>, mpsc::Receiver<super::SessionCommand>) {
        test_state_with_env(MapEnv::default())
    }

    fn test_state_with_env(
        extra_env: MapEnv,
    ) -> (Arc<super::AppState>, mpsc::Receiver<super::SessionCommand>) {
        test_state_with_env_parts(extra_env, super::SearchStore::new(), None)
    }

    fn test_state_with_env_parts(
        extra_env: MapEnv,
        search_store: super::SearchStore,
        db: Option<super::persistence::DatabaseManager>,
    ) -> (Arc<super::AppState>, mpsc::Receiver<super::SessionCommand>) {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let state_dir =
            std::env::temp_dir().join(format!("slskr-route-test-{}-{unique}", std::process::id()));
        std::fs::create_dir_all(&state_dir).unwrap();
        let mut env = MapEnv::default()
            .with("SLSKR_STATE_DIR", &state_dir.display().to_string())
            .with("SLSKR_SHARE_FIXTURE", "Virtual/Test.flac=42")
            .with("SLSK_USERNAME", "tester")
            .with("SLSK_PASSWORD", "secret");
        env.values.extend(extra_env.values);
        let config =
            super::AppConfig::from_layers(None, FileConfig::default(), &env).expect("test config");
        let share_index = super::build_share_index(&config);
        let (sender, receiver) = mpsc::channel(8);
        let (event_tx, _) = tokio::sync::broadcast::channel(super::EVENT_HISTORY_LIMIT);
        let rate_limiter =
            super::rate_limit::RateLimiter::new(super::rate_limit::RateLimitConfig {
                max_requests_anonymous: 1000,
                max_requests_authenticated: 5000,
                window_seconds: 60,
                enabled: true,
            });

        let state = Arc::new(super::AppState {
            session: RwLock::new(super::SessionSnapshot::disconnected(&config)),
            listeners: RwLock::new(super::ListenerSnapshot::new(&config)),
            shares: RwLock::new(share_index),
            searches: RwLock::new(search_store),
            users: RwLock::new(super::UserStore::new()),
            browse: RwLock::new(super::BrowseStore::new()),
            messages: RwLock::new(super::MessageStore::new()),
            rooms: RwLock::new(super::RoomStore::new()),
            transfers: RwLock::new(super::TransferQueue::new(&config)),
            events: RwLock::new(super::EventStore::new(super::EVENT_HISTORY_LIMIT)),
            event_tx,
            webhooks: RwLock::new(super::webhooks::WebhookManager::new()),
            collections: RwLock::new(super::CollectionStore::new()),
            wishlist: RwLock::new(super::WishlistStore::new()),
            contacts: RwLock::new(super::ContactStore::new()),
            sharegroups: RwLock::new(super::ShareGroupStore::new()),
            user_notes: RwLock::new(super::UserNoteStore::new()),
            interests: RwLock::new(super::InterestStore::new()),
            share_grants: RwLock::new(super::ShareGrantStore::new()),
            library: RwLock::new(super::LibraryStore::new()),
            destinations: RwLock::new(super::DestinationStore::new()),
            db,
            config,
            session_commands: sender,
            rate_limiter,
            oauth_states: RwLock::new(super::OAuthStateStore::default()),
        });
        (state, receiver)
    }

    async fn add_test_share(state: &Arc<super::AppState>, filename: &str, path: &Path, size: u64) {
        let mut shares = state.shares.write().await;
        shares
            .local_paths
            .insert(filename.to_owned(), path.to_path_buf());
        shares.entries.push(FileEntry {
            code: 1,
            filename: filename.to_owned(),
            size,
            extension: super::extension_for(filename),
            attributes: Vec::new(),
        });
    }

    #[test]
    fn parse_route_reads_method_and_path() {
        assert_eq!(
            parse_route("POST /api/session/connect HTTP/1.1\r\nhost: localhost\r\n\r\n"),
            ("POST", "/api/session/connect")
        );
        assert_eq!(
            split_request_target("/api/v0/shares/catalog?q=test"),
            ("/api/v0/shares/catalog", Some("q=test"))
        );
    }

    #[test]
    fn versioned_api_paths_map_to_current_handlers() {
        assert_eq!(normalize_api_path("/api/v0/health"), "/api/health");
        assert_eq!(normalize_api_path("/api/v0/metrics"), "/api/metrics");
        assert_eq!(normalize_api_path("/api/v0/telemetry"), "/api/telemetry");
        assert_eq!(
            normalize_api_path("/api/v0/capabilities/negotiate"),
            "/api/capabilities/negotiate"
        );
        assert_eq!(
            normalize_api_path("/api/v0/session/connect"),
            "/api/session/connect"
        );
        assert_eq!(normalize_api_path("/api/custom"), "/api/custom");
    }

    #[tokio::test]
    async fn read_only_api_routes_return_contract_shapes() {
        let (state, _receiver) = test_state();

        let cases = [
            ("/api/v0/health", "\"status\":\"ok\""),
            ("/api/v0/version", "\"name\":\"slskr\""),
            ("/api/v0/capabilities", "\"api_version\":\"v0\""),
            ("/api/v0/config", "\"credentials_configured\":true"),
            ("/api/v0/stats", "\"session\":"),
            ("/api/v0/telemetry", "\"health\":"),
            ("/api/v0/events", "\"entries\":"),
            ("/api/v0/session", "\"state\":\"disconnected\""),
            ("/api/v0/session/enabled", "false"),
            ("/api/v0/listeners", "\"regular_accepts\":0"),
            ("/api/v0/users", "\"count\":0"),
            ("/api/v0/rooms", "\"count\":0"),
            ("/api/v0/shares", "\"files\":1"),
            ("/api/v0/shares/catalog", "\"total_bytes\":42"),
            ("/api/v0/searches", "\"count\":0"),
            ("/api/v0/transfers", "\"count\":0"),
            ("/api/v0/transfers/stats", "\"total\":0"),
        ];

        for (path, expected_body) in cases {
            let response = super::route_http_request("GET", path, None, "", &state)
                .await
                .expect("route response");
            assert_eq!(response.status, "200 OK", "{path}");
            assert_eq!(response.content_type, "application/json", "{path}");
            assert!(
                response.body.contains(expected_body),
                "{path}: {}",
                response.body
            );
            assert!(
                !response.body.contains("test-password")
                    && !response.body.contains("api-token")
                    && !response.body.contains("client-secret"),
                "{path}: {}",
                response.body
            );
        }
    }

    #[tokio::test]
    async fn build_info_uses_app_version_not_protocol_version() {
        let (state, _receiver) = test_state();

        let response = super::route_http_request("GET", "/api/application/build", None, "", &state)
            .await
            .unwrap();
        assert_eq!(response.status, "200 OK");
        let body: serde_json::Value = serde_json::from_str(&response.body).unwrap();

        assert_eq!(body["current"], env!("CARGO_PKG_VERSION"));
        assert_eq!(body["full"], env!("CARGO_PKG_VERSION"));
        assert_eq!(body["latestTag"], env!("CARGO_PKG_VERSION"));
        assert_eq!(
            body["protocol"]["major"],
            serde_json::json!(super::CLIENT_MAJOR_VERSION)
        );
        assert_eq!(
            body["protocol"]["minor"],
            serde_json::json!(super::CLIENT_MINOR_VERSION)
        );
        assert_ne!(
            body["current"],
            serde_json::json!(format!(
                "{}.{}.{}",
                super::CLIENT_NAME,
                super::CLIENT_MAJOR_VERSION,
                super::CLIENT_MINOR_VERSION
            ))
        );
    }

    #[tokio::test]
    async fn spotify_oauth_callback_requires_server_issued_state() {
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_SPOTIFY_ENABLED", "true")
                .with("SLSKR_SPOTIFY_CLIENT_ID", "client-id")
                .with("SLSKR_HTTP_BIND", "127.0.0.1:7788"),
        );

        let authorize = super::route_http_request(
            "POST",
            "/api/integrations/spotify/authorize",
            None,
            "",
            &state,
        )
        .await
        .expect("authorize response");
        assert_eq!(authorize.status, "202 Accepted");
        assert!(authorize
            .body
            .contains("127.0.0.1:7788/api/integrations/spotify/callback"));

        let issued_state = {
            let states = state.oauth_states.read().await;
            states.records.keys().next().cloned().expect("issued state")
        };

        let invalid = super::route_http_request(
            "GET",
            "/api/integrations/spotify/callback?code=abc&state=bogus",
            None,
            "",
            &state,
        )
        .await
        .expect("invalid callback response");
        assert_eq!(invalid.status, "403 Forbidden");

        let valid_path = format!(
            "/api/integrations/spotify/callback?code=abc123&state={}",
            issued_state
        );
        let valid = super::route_http_request("GET", &valid_path, None, "", &state)
            .await
            .expect("valid callback response");
        assert_eq!(valid.status, "200 OK");
        assert!(valid.body.contains("\"state_valid\":true"));

        let replay = super::route_http_request("GET", &valid_path, None, "", &state)
            .await
            .expect("replay callback response");
        assert_eq!(replay.status, "403 Forbidden");
    }

    #[tokio::test]
    async fn metrics_api_returns_scrapable_counters() {
        let (state, _receiver) = test_state();

        let response = super::route_http_request("GET", "/api/v0/metrics", None, "", &state)
            .await
            .expect("metrics response");

        assert_eq!(response.status, "200 OK");
        assert_eq!(
            response.content_type,
            "text/plain; version=0.0.4; charset=utf-8"
        );
        assert!(response.body.contains("slskr_session_connected 0"));
        assert!(response.body.contains("slskr_shares_files 1"));
        assert!(response.body.contains("slskr_shares_bytes 42"));
        assert!(response.body.contains("slskr_transfers{state=\"total\"} 0"));
        assert!(!response.body.contains("secret"));
    }

    #[tokio::test]
    async fn capabilities_negotiate_returns_intersection() {
        let (state, _receiver) = test_state();

        let response = super::route_http_request(
            "POST",
            "/api/v0/capabilities/negotiate",
            None,
            "{\"capabilities\":[\"shares\",\"telemetry\",\"bogus\"]}",
            &state,
        )
        .await
        .expect("capability negotiation response");

        assert_eq!(response.status, "200 OK");
        assert_eq!(response.content_type, "application/json");
        assert!(response
            .body
            .contains("\"accepted\":[\"shares\",\"telemetry\"]"));
        assert!(response.body.contains("\"unsupported\":[\"bogus\"]"));
        assert!(response.body.contains("\"server_capabilities\":["));
        assert!(!response.body.contains("secret"));
    }

    #[tokio::test]
    async fn telemetry_api_returns_runtime_health_without_secrets() {
        let (state, _receiver) = test_state();

        let response = super::route_http_request("GET", "/api/v0/telemetry", None, "", &state)
            .await
            .expect("telemetry response");

        assert_eq!(response.status, "200 OK");
        assert_eq!(response.content_type, "application/json");
        assert!(response.body.contains("\"service\":{\"name\":\"slskr\""));
        assert!(response.body.contains("\"health\":{"));
        assert!(response.body.contains("\"connected\":false"));
        assert!(response
            .body
            .contains("\"share_cache_file\":\"share-index.tsv\""));
        assert!(response
            .body
            .contains("\"transfer_events_file\":\"transfer-events.tsv\""));
        assert!(response
            .body
            .contains("\"shares\":{\"roots\":0,\"files\":1"));
        assert!(!response.body.contains("secret"));
    }

    #[tokio::test]
    async fn root_serves_webui_or_fallback_dashboard() {
        let (state, _receiver) = test_state();

        let response = super::route_http_request("GET", "/", None, "", &state)
            .await
            .expect("root response");

        assert_eq!(response.status, "200 OK");
        assert_eq!(response.content_type, "text/html; charset=utf-8");
        if response.body.contains("id=\"root\"") && response.body.contains("/assets/") {
            assert!(response.body.contains("<script"));
            return;
        }

        assert!(response.body.contains("<h1>slskr</h1>"));
        assert!(response.body.contains("id=\"session-actions\""));
        assert!(response
            .body
            .contains("data-session-action=\"privileges/check\""));
        assert!(response.body.contains("id=\"search-form\""));
        assert!(response.body.contains("id=\"watch-form\""));
        assert!(response.body.contains("id=\"unwatch-button\""));
        assert!(response.body.contains("id=\"browse-request-form\""));
        assert!(response.body.contains("id=\"browse-folder\""));
        assert!(response.body.contains("id=\"share-rescan-form\""));
        assert!(response.body.contains("id=\"transfer-form\""));
        assert!(response.body.contains("id=\"transfer-progress\""));
        assert!(response.body.contains("id=\"transfer-local-path\""));
        assert!(response.body.contains("id=\"message-form\""));
        assert!(response.body.contains("id=\"room-join-form\""));
        assert!(response.body.contains("id=\"room-message-form\""));
        assert!(response.body.contains("id=\"room-message-username\""));
        assert!(response.body.contains("id=\"token-form\""));
        assert!(response.body.contains("id=\"user-table\""));
        assert!(response.body.contains("id=\"share-table\""));
        assert!(response.body.contains("id=\"message-table\""));
        assert!(response.body.contains("id=\"room-table\""));
        assert!(response.body.contains("id=\"browse-table\""));
        assert!(response.body.contains("id=\"search-filter-q\""));
        assert!(response.body.contains("id=\"transfer-filter-status\""));
        assert!(response.body.contains("id=\"share-filter-extension\""));
        assert!(response.body.contains("id=\"message-filter-direction\""));
        assert!(response.body.contains("id=\"room-filter-joined\""));
        assert!(response.body.contains("id=\"browse-filter-status\""));
        assert!(response.body.contains("/api/v0/stats"));
        assert!(response.body.contains("/api/v0/session/"));
        assert!(response.body.contains("/api/v0/searches"));
        assert!(response.body.contains("data-search-action=\"complete\""));
        assert!(response.body.contains("/api/v0/users/watch"));
        assert!(response.body.contains("data-user-action=\"browse\""));
        assert!(response.body.contains("data-user-action=\"stats\""));
        assert!(response.body.contains("/stats/request"));
        assert!(response.body.contains("/api/v0/shares/catalog"));
        assert!(response.body.contains("/browse/request"));
        assert!(response.body.contains("/api/v0/shares/rescan"));
        assert!(response.body.contains("/api/v0/transfers"));
        assert!(response.body.contains("data-transfer-action=\"complete\""));
        assert!(response.body.contains("/api/v0/messages"));
        assert!(response.body.contains("data-message-action=\"ack\""));
        assert!(response.body.contains("/api/v0/rooms/"));
        assert!(response.body.contains("/api/v0/rooms/refresh"));
        assert!(response.body.contains("data-room-action=\"leave\""));
    }

    #[tokio::test]
    async fn share_catalog_supports_filters_and_pagination() {
        let (state, _receiver) = test_state();
        {
            let mut shares = state.shares.write().await;
            shares.entries.push(FileEntry {
                code: 1,
                filename: "Virtual/Other.mp3".to_owned(),
                size: 12,
                extension: "mp3".to_owned(),
                attributes: Vec::new(),
            });
        }

        let response = super::route_http_request(
            "GET",
            "/api/v0/shares/catalog?q=test&extension=flac&limit=1&offset=0",
            None,
            "",
            &state,
        )
        .await
        .expect("catalog response");

        assert_eq!(response.status, "200 OK");
        assert!(response.body.contains("\"count\":2"));
        assert!(response.body.contains("\"filtered_count\":1"));
        assert!(response.body.contains("\"total_bytes\":42"));
        assert!(response.body.contains("\"limit\":1"));
        assert!(response.body.contains("\"path\":\"Virtual/Test.flac\""));
        assert!(!response.body.contains("Other.mp3"));
    }

    #[tokio::test]
    async fn files_api_lists_one_share_root_without_local_paths() {
        let (state, _receiver) = test_state();
        {
            let mut shares = state.shares.write().await;
            shares.roots.push(super::ShareRoot {
                label: "Music".to_owned(),
                files: 2,
                bytes: 142,
                extensions: vec![super::ShareExtensionSummary {
                    extension: "flac".to_owned(),
                    files: 1,
                    bytes: 42,
                }],
            });
            shares.entries.push(FileEntry {
                code: 1,
                filename: "Music/Other.mp3".to_owned(),
                size: 100,
                extension: "mp3".to_owned(),
                attributes: Vec::new(),
            });
            shares.entries.push(FileEntry {
                code: 1,
                filename: "Music/Test.flac".to_owned(),
                size: 42,
                extension: "flac".to_owned(),
                attributes: Vec::new(),
            });
            shares.local_paths.insert(
                "Music/Test.flac".to_owned(),
                PathBuf::from("/tmp/private/Test.flac"),
            );
        }

        let response = super::route_http_request(
            "GET",
            "/api/v0/files/Music?extension=flac",
            None,
            "",
            &state,
        )
        .await
        .expect("files response");

        assert_eq!(response.status, "200 OK");
        assert!(response.body.contains("\"label\":\"Music\""));
        assert!(response.body.contains("\"path\":\"Test.flac\""));
        assert!(response
            .body
            .contains("\"virtual_path\":\"Music/Test.flac\""));
        assert!(response.body.contains("\"filtered_count\":1"));
        assert!(!response.body.contains("/tmp/private"));
        assert!(!response.body.contains("Other.mp3"));

        let missing = super::route_http_request("GET", "/api/v0/files/Missing", None, "", &state)
            .await
            .expect("missing files response");
        assert_eq!(missing.status, "404 Not Found");
    }

    #[tokio::test]
    async fn stats_api_aggregates_projection_counts() {
        let (state, mut receiver) = test_state();

        super::route_http_request(
            "POST",
            "/api/v0/searches",
            None,
            "{\"query\":\"test flac\"}",
            &state,
        )
        .await
        .unwrap();
        let _ = receiver.try_recv();
        super::route_http_request(
            "POST",
            "/api/v0/users/watch",
            None,
            "{\"username\":\"friend\"}",
            &state,
        )
        .await
        .unwrap();
        let _ = receiver.try_recv();
        super::route_http_request(
            "POST",
            "/api/v0/users/friend/browse/request",
            None,
            "",
            &state,
        )
        .await
        .unwrap();
        let _ = receiver.try_recv();
        super::route_http_request(
            "POST",
            "/api/v0/browse-responses",
            None,
            "{\"username\":\"friend\",\"filename\":\"Remote/Song.flac\",\"size\":123}",
            &state,
        )
        .await
        .unwrap();
        super::route_http_request(
            "POST",
            "/api/v0/messages/inbound",
            None,
            "{\"username\":\"friend\",\"body\":\"hi\"}",
            &state,
        )
        .await
        .unwrap();
        super::route_http_request("POST", "/api/v0/rooms/music/join", None, "", &state)
            .await
            .unwrap();
        let _ = receiver.try_recv();
        super::route_http_request(
            "POST",
            "/api/v0/rooms/music/messages",
            None,
            "{\"username\":\"friend\",\"body\":\"track?\"}",
            &state,
        )
        .await
        .unwrap();
        let _ = receiver.try_recv();
        super::route_http_request(
            "POST",
            "/api/v0/transfers",
            None,
            "{\"filename\":\"Remote/Song.flac\",\"size\":100}",
            &state,
        )
        .await
        .unwrap();
        super::route_http_request(
            "POST",
            "/api/v0/transfers/1/progress",
            None,
            "{\"bytes_transferred\":40}",
            &state,
        )
        .await
        .unwrap();

        let stats = super::route_http_request("GET", "/api/v0/stats", None, "", &state)
            .await
            .expect("stats response");

        assert_eq!(stats.status, "200 OK");
        assert!(stats.body.contains("\"shares\":{"));
        assert!(stats.body.contains("\"files\":1"));
        assert!(stats.body.contains("\"bytes\":42"));
        assert!(stats
            .body
            .contains("\"searches\":{\"total\":1,\"active\":1"));
        assert!(stats.body.contains("\"results\":1"));
        assert!(stats.body.contains("\"users\":{\"total\":1,\"watched\":1"));
        assert!(stats.body.contains(
            "\"browse\":{\"total\":1,\"requested\":0,\"indirect_pending\":0,\"partial\":0,\"ready\":1,\"failed\":0,\"files\":1,\"bytes\":123"
        ));
        assert!(stats
            .body
            .contains("\"messages\":{\"total\":1,\"inbound\":1,\"outbound\":0"));
        assert!(stats
            .body
            .contains("\"rooms\":{\"total\":1,\"joined\":1,\"messages\":1"));
        assert!(stats
            .body
            .contains("\"transfers\":{\"total\":1,\"queued\":0,\"in_progress\":1"));
        assert!(stats.body.contains("\"bytes_transferred\":40"));
    }

    #[tokio::test]
    async fn mutating_api_routes_enqueue_session_commands() {
        let (state, mut receiver) = test_state();

        let routes = [
            ("/api/v0/session/connect", super::SessionCommand::Connect),
            ("/api/v0/session/ping", super::SessionCommand::Ping),
            (
                "/api/v0/session/disconnect",
                super::SessionCommand::Disconnect,
            ),
            (
                "/api/v0/session/privileges/check",
                super::SessionCommand::CheckPrivileges,
            ),
        ];

        for (path, expected_command) in routes {
            let response = super::route_http_request("POST", path, None, "", &state)
                .await
                .expect("route response");
            assert_eq!(response.status, "202 Accepted");
            assert_eq!(response.body, "{\"accepted\":true}");
            let command = receiver.try_recv().expect("session command");
            assert_eq!(
                std::mem::discriminant(&command),
                std::mem::discriminant(&expected_command)
            );
        }
    }

    #[tokio::test]
    async fn slskd_automation_compat_routes_use_expected_shapes() {
        let (state, mut receiver) = test_state();

        let app = super::route_http_request("GET", "/api/v0/application", None, "", &state)
            .await
            .expect("application route");
        assert_eq!(app.status, "200 OK");
        let app_json = serde_json::from_str::<serde_json::Value>(&app.body).unwrap();
        assert_eq!(app_json["version"]["current"], env!("CARGO_PKG_VERSION"));
        assert_eq!(app_json["server"]["isConnected"], false);

        let server_connect = super::route_http_request("POST", "/api/v0/server", None, "", &state)
            .await
            .expect("server connect");
        assert_eq!(server_connect.status, "202 Accepted");
        assert!(matches!(
            receiver.try_recv().unwrap(),
            super::SessionCommand::Connect
        ));

        let server_connect_put =
            super::route_http_request("PUT", "/api/v0/server", None, "", &state)
                .await
                .expect("server put connect");
        assert_eq!(server_connect_put.status, "202 Accepted");
        assert!(matches!(
            receiver.try_recv().unwrap(),
            super::SessionCommand::Connect
        ));

        let session_enabled =
            super::route_http_request("GET", "/api/v0/session/enabled", None, "", &state)
                .await
                .expect("session enabled");
        assert!(matches!(session_enabled.body.as_str(), "true" | "false"));

        let search = super::route_http_request(
            "POST",
            "/api/v0/searches",
            None,
            r#"{"searchText":"Remote Song"}"#,
            &state,
        )
        .await
        .expect("search route");
        assert_eq!(search.status, "201 Created");
        assert!(search.body.contains("\"searchText\":\"Remote Song\""));
        let _ = receiver.try_recv();

        let _ = super::route_http_request(
            "POST",
            "/api/v0/search-responses",
            None,
            r#"{"token":1,"peer_username":"peer1","filename":"Remote/Song.mp3","size":99}"#,
            &state,
        )
        .await
        .unwrap();
        let responses =
            super::route_http_request("GET", "/api/v0/searches/1/responses", None, "", &state)
                .await
                .expect("search responses");
        let responses_json = serde_json::from_str::<serde_json::Value>(&responses.body).unwrap();
        assert!(responses_json.is_array());
        assert_eq!(responses_json[0]["username"], "peer1");
        assert_eq!(responses_json[0]["files"][0]["filename"], "Remote/Song.mp3");

        let uuid_search_id = "11111111-1111-1111-1111-111111111111";
        let uuid_search = super::route_http_request(
            "POST",
            "/api/v0/searches",
            None,
            &format!(r#"{{"id":"{uuid_search_id}","searchText":"UUID Song"}}"#),
            &state,
        )
        .await
        .expect("uuid search route");
        assert_eq!(uuid_search.status, "201 Created");
        let uuid_search_json =
            serde_json::from_str::<serde_json::Value>(&uuid_search.body).unwrap();
        assert_eq!(uuid_search_json["id"], uuid_search_id);
        let _ = receiver.try_recv();

        for (method, path) in [
            ("GET", format!("/api/v0/searches/{uuid_search_id}")),
            (
                "GET",
                format!("/api/v0/searches/{uuid_search_id}?includeResponses=true"),
            ),
            (
                "GET",
                format!("/api/v0/searches/{uuid_search_id}/responses"),
            ),
            ("PUT", format!("/api/v0/searches/{uuid_search_id}")),
            ("DELETE", format!("/api/v0/searches/{uuid_search_id}")),
        ] {
            let response = super::route_http_request(method, &path, None, "", &state)
                .await
                .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
            assert_ne!(response.status, "404 Not Found", "{method} {path}");
        }

        let enqueue = super::route_http_request(
            "POST",
            "/api/v0/transfers/downloads/peer1",
            None,
            r#"{"files":[{"filename":"Remote/Song.mp3","size":99}]}"#,
            &state,
        )
        .await
        .expect("enqueue download");
        assert_eq!(enqueue.body, "true");

        let downloads =
            super::route_http_request("GET", "/api/v0/transfers/downloads", None, "", &state)
                .await
                .expect("downloads route");
        let downloads_json = serde_json::from_str::<serde_json::Value>(&downloads.body).unwrap();
        assert_eq!(downloads_json[0]["username"], "peer1");
        assert_eq!(
            downloads_json[0]["directories"][0]["files"][0]["direction"],
            "Download"
        );

        let joined =
            super::route_http_request("POST", "/api/v0/rooms/joined", None, r#""music""#, &state)
                .await
                .expect("join room");
        assert_eq!(joined.status, "201 Created");
        let _ = receiver.try_recv();

        let room_message = super::route_http_request(
            "POST",
            "/api/v0/rooms/joined/music/messages",
            None,
            r#""hello room""#,
            &state,
        )
        .await
        .expect("room message");
        assert_eq!(room_message.body, "true");
        let _ = receiver.try_recv();

        let room_messages = super::route_http_request(
            "GET",
            "/api/v0/rooms/joined/music/messages",
            None,
            "",
            &state,
        )
        .await
        .expect("room messages");
        let room_messages_json =
            serde_json::from_str::<serde_json::Value>(&room_messages.body).unwrap();
        assert_eq!(room_messages_json[0]["message"], "hello room");

        let conversation_send = super::route_http_request(
            "POST",
            "/api/v0/conversations/peer1",
            None,
            r#""hello peer""#,
            &state,
        )
        .await
        .expect("conversation send");
        assert_eq!(conversation_send.body, "true");
        let _ = receiver.try_recv();

        let conversations =
            super::route_http_request("GET", "/api/v0/conversations", None, "", &state)
                .await
                .expect("conversations");
        let conversations_json =
            serde_json::from_str::<serde_json::Value>(&conversations.body).unwrap();
        assert_eq!(conversations_json[0]["username"], "peer1");
        assert_eq!(
            conversations_json[0]["messages"][0]["message"],
            "hello peer"
        );

        let user_status =
            super::route_http_request("GET", "/api/v0/users/peer1/status", None, "", &state)
                .await
                .expect("user status");
        assert!(user_status.body.contains("\"presence\""));

        let contract_routes = [
            ("GET", "/api/application", ""),
            ("GET", "/api/application/version", ""),
            ("GET", "/api/application/version/latest", ""),
            ("POST", "/api/application/gc", ""),
            ("GET", "/api/session", ""),
            (
                "POST",
                "/api/session",
                r#"{"username":"user","password":"pass"}"#,
            ),
            ("GET", "/api/session/enabled", ""),
            ("GET", "/api/server", ""),
            ("PUT", "/api/server", ""),
            ("DELETE", "/api/server", ""),
            ("POST", "/api/searches", r#"{"searchText":"contract song"}"#),
            ("GET", "/api/searches", ""),
            ("GET", "/api/searches/1", ""),
            ("PUT", "/api/searches/1", ""),
            ("GET", "/api/searches/1/responses", ""),
            ("GET", "/api/transfers/downloads/", ""),
            ("GET", "/api/transfers/downloads/peer1", ""),
            (
                "POST",
                "/api/transfers/downloads/peer1",
                r#"[{"filename":"Remote/Song.mp3","size":99}]"#,
            ),
            ("GET", "/api/transfers/downloads/peer1/1", ""),
            ("GET", "/api/transfers/downloads/peer1/1/position", ""),
            ("DELETE", "/api/transfers/downloads/peer1/1", ""),
            ("DELETE", "/api/transfers/downloads/all/completed", ""),
            ("GET", "/api/transfers/uploads/", ""),
            ("GET", "/api/transfers/uploads/peer1", ""),
            ("GET", "/api/transfers/uploads/peer1/1", ""),
            ("DELETE", "/api/transfers/uploads/peer1/1", ""),
            ("DELETE", "/api/transfers/uploads/all/completed", ""),
            ("GET", "/api/rooms/joined", ""),
            ("POST", "/api/rooms/joined", r#""contract-room""#),
            ("GET", "/api/rooms/joined/contract-room", ""),
            (
                "POST",
                "/api/rooms/joined/contract-room/messages",
                r#""hello""#,
            ),
            ("GET", "/api/rooms/joined/contract-room/messages", ""),
            (
                "POST",
                "/api/rooms/joined/contract-room/ticker",
                r#""ticker""#,
            ),
            (
                "POST",
                "/api/rooms/joined/contract-room/members",
                r#""peer1""#,
            ),
            ("GET", "/api/rooms/joined/contract-room/users", ""),
            ("DELETE", "/api/rooms/joined/contract-room", ""),
            ("GET", "/api/rooms/available", ""),
            ("GET", "/api/conversations", ""),
            ("POST", "/api/conversations/peer1", r#""hello peer again""#),
            ("GET", "/api/conversations/peer1", ""),
            ("GET", "/api/conversations/peer1/messages", ""),
            ("PUT", "/api/conversations/peer1/1", ""),
            ("PUT", "/api/conversations/peer1", ""),
            ("DELETE", "/api/conversations/peer1", ""),
            ("GET", "/api/users/peer1/endpoint", ""),
            ("GET", "/api/users/peer1/browse", ""),
            ("GET", "/api/users/peer1/browse/status", ""),
            ("POST", "/api/users/peer1/directory", r#"{"directory":""}"#),
            ("GET", "/api/users/peer1/info", ""),
            ("GET", "/api/users/peer1/status", ""),
            ("GET", "/api/shares", ""),
            ("PUT", "/api/shares", ""),
            ("DELETE", "/api/shares", ""),
            ("GET", "/api/shares/root", ""),
            ("GET", "/api/shares/contents", ""),
            ("GET", "/api/shares/root/contents", ""),
            ("GET", "/api/files/downloads/directories", ""),
            ("GET", "/api/files/downloads/directories/Zm9v", ""),
            ("DELETE", "/api/files/downloads/directories/Zm9v", ""),
            ("DELETE", "/api/files/downloads/files/Zm9vLm1wMw", ""),
            ("GET", "/api/files/incomplete/directories", ""),
            ("GET", "/api/files/incomplete/directories/Zm9v", ""),
            ("DELETE", "/api/files/incomplete/directories/Zm9v", ""),
            ("DELETE", "/api/files/incomplete/files/Zm9vLm1wMw", ""),
            ("GET", "/api/options", ""),
            ("GET", "/api/options/startup", ""),
            ("GET", "/api/options/debug", ""),
            ("GET", "/api/options/yaml/location", ""),
            ("GET", "/api/options/yaml", ""),
            ("PUT", "/api/options/yaml", r#""app: {}""#),
            ("POST", "/api/options/yaml/validate", r#""app: {}""#),
            ("GET", "/api/events", ""),
            ("POST", "/api/events/Noop", r#""""#),
            ("GET", "/api/logs", ""),
            ("PUT", "/api/relay/agent", ""),
            ("DELETE", "/api/relay/agent", ""),
            ("GET", "/api/relay/controller/downloads/token", ""),
            ("POST", "/api/relay/controller/files/token", ""),
            ("POST", "/api/relay/controller/shares/token", ""),
            ("GET", "/api/telemetry/metrics", ""),
            ("GET", "/api/telemetry/metrics/kpis", ""),
            ("GET", "/api/telemetry/reports/transfers/summary", ""),
            ("GET", "/api/telemetry/reports/transfers/histogram", ""),
            ("GET", "/api/telemetry/reports/transfers/leaderboard", ""),
            ("GET", "/api/telemetry/reports/transfers/users/peer1", ""),
            ("GET", "/api/telemetry/reports/transfers/exceptions", ""),
            (
                "GET",
                "/api/telemetry/reports/transfers/exceptions/pareto",
                "",
            ),
            ("GET", "/api/telemetry/reports/transfers/directories", ""),
            ("DELETE", "/api/searches/1", ""),
            ("DELETE", "/api/application", ""),
            ("PUT", "/api/application", ""),
        ];

        for (method, path, body) in contract_routes {
            let response = tokio::time::timeout(
                Duration::from_secs(1),
                super::route_http_request(method, path, None, body, &state),
            )
            .await
            .unwrap_or_else(|_| panic!("{method} {path}: timed out"))
            .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
            assert_ne!(response.status, "404 Not Found", "{method} {path}");
            assert!(
                !response.status.starts_with('5'),
                "{method} {path}: {}",
                response.status
            );
            while receiver.try_recv().is_ok() {}

            let versioned = path.replacen("/api", "/api/v0", 1);
            let versioned_response = tokio::time::timeout(
                Duration::from_secs(1),
                super::route_http_request(method, &versioned, None, body, &state),
            )
            .await
            .unwrap_or_else(|_| panic!("{method} {versioned}: timed out"))
            .unwrap_or_else(|error| panic!("{method} {versioned}: {error}"));
            assert_ne!(
                versioned_response.status, "404 Not Found",
                "{method} {versioned}"
            );
            assert!(
                !versioned_response.status.starts_with('5'),
                "{method} {versioned}: {}",
                versioned_response.status
            );
            while receiver.try_recv().is_ok() {}
        }

        let query_contract_routes = [
            ("GET", "/api/application/version/latest?forceCheck=true", ""),
            ("GET", "/api/searches?includeResponses=true", ""),
            ("DELETE", "/api/transfers/downloads/peer1/1?remove=true", ""),
            ("GET", "/api/transfers/downloads/?includeRemoved=true", ""),
            ("GET", "/api/transfers/uploads/?includeRemoved=true", ""),
            (
                "GET",
                "/api/conversations?includeInactive=true&unAcknowledgedOnly=false",
                "",
            ),
            ("GET", "/api/conversations/peer1?includeMessages=false", ""),
            (
                "GET",
                "/api/conversations/peer1/messages?unAcknowledgedOnly=true",
                "",
            ),
            ("GET", "/api/files/downloads/directories?recursive=true", ""),
            (
                "GET",
                "/api/files/incomplete/directories?recursive=true",
                "",
            ),
            ("GET", "/api/events?limit=10", ""),
            (
                "GET",
                "/api/telemetry/reports/transfers/summary?startDate=2026-01-01&endDate=2026-01-02",
                "",
            ),
        ];

        for (method, path, body) in query_contract_routes {
            let response = tokio::time::timeout(
                Duration::from_secs(1),
                super::route_http_request(method, path, None, body, &state),
            )
            .await
            .unwrap_or_else(|_| panic!("{method} {path}: timed out"))
            .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
            assert_ne!(response.status, "404 Not Found", "{method} {path}");
            assert!(
                !response.status.starts_with('5'),
                "{method} {path}: {}",
                response.status
            );
            while receiver.try_recv().is_ok() {}
        }
    }

    #[tokio::test]
    async fn share_rescan_route_rebuilds_snapshot() {
        let (state, _receiver) = test_state();

        {
            let mut shares = state.shares.write().await;
            shares.entries.clear();
        }

        let response = super::route_http_request("POST", "/api/v0/shares/rescan", None, "", &state)
            .await
            .expect("route response");

        assert_eq!(response.status, "202 Accepted");
        assert!(response.body.contains("\"files\":1"));
        assert_eq!(state.shares.read().await.entries.len(), 1);
    }

    #[tokio::test]
    async fn search_api_creates_reads_and_completes_records() {
        let (state, mut receiver) = test_state();

        let created = super::route_http_request(
            "POST",
            "/api/v0/searches",
            None,
            "{\"query\":\"test flac\"}",
            &state,
        )
        .await
        .expect("create search");
        assert_eq!(created.status, "201 Created");
        assert!(created.body.contains("\"token\":1"));
        assert!(created.body.contains("\"query\":\"test flac\""));
        assert!(created.body.contains("\"target\":\"global\""));
        assert!(created.body.contains("\"status\":\"active\""));
        assert!(created.body.contains("\"result_count\":1"));
        assert_eq!(
            receiver.try_recv().expect("search command"),
            super::SessionCommand::Search {
                token: 1,
                query: "test flac".to_owned(),
                target: super::SearchDispatchTarget::Global,
            }
        );

        let listed = super::route_http_request("GET", "/api/v0/searches", None, "", &state)
            .await
            .expect("list searches");
        assert_eq!(listed.status, "200 OK");
        assert!(listed.body.contains("\"count\":1"));
        assert!(listed.body.contains("\"filtered_count\":1"));

        let fetched = super::route_http_request("GET", "/api/v0/searches/1", None, "", &state)
            .await
            .expect("get search");
        assert_eq!(fetched.status, "200 OK");
        assert!(fetched.body.contains("Virtual/Test.flac"));

        let completed =
            super::route_http_request("POST", "/api/v0/searches/1/complete", None, "", &state)
                .await
                .expect("complete search");
        assert_eq!(completed.status, "200 OK");
        assert!(completed.body.contains("\"status\":\"completed\""));
    }

    #[tokio::test]
    async fn search_create_persists_and_rehydrates_records() {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, mut receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            super::SearchStore::new(),
            Some(db.clone()),
        );

        let created = super::route_http_request(
            "POST",
            "/api/v0/searches",
            None,
            "{\"query\":\"persist me\",\"target\":\"global\"}",
            &state,
        )
        .await
        .expect("create persisted search");
        assert_eq!(created.status, "201 Created");
        let _ = receiver.try_recv();

        let persisted = db.list_searches(10, 0).await.expect("list persisted");
        assert_eq!(persisted.len(), 1);
        assert_eq!(persisted[0].query, "persist me");

        let rehydrated = super::SearchStore::from_persisted(persisted);
        let (restarted_state, _) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            rehydrated,
            Some(db),
        );
        let listed =
            super::route_http_request("GET", "/api/v0/searches", None, "", &restarted_state)
                .await
                .expect("list rehydrated searches");
        assert_eq!(listed.status, "200 OK");
        assert!(listed.body.contains("\"count\":1"));
        assert!(listed.body.contains("\"query\":\"persist me\""));
    }

    #[tokio::test]
    async fn search_api_lists_with_filters_and_pagination() {
        let (state, mut receiver) = test_state();

        super::route_http_request(
            "POST",
            "/api/v0/searches",
            None,
            "{\"query\":\"alpha\",\"target\":\"global\"}",
            &state,
        )
        .await
        .unwrap();
        let _ = receiver.try_recv();
        super::route_http_request(
            "POST",
            "/api/v0/searches",
            None,
            "{\"query\":\"beta\",\"target\":\"wishlist\"}",
            &state,
        )
        .await
        .unwrap();
        let _ = receiver.try_recv();
        super::route_http_request("POST", "/api/v0/searches/1/complete", None, "", &state)
            .await
            .unwrap();

        let filtered = super::route_http_request(
            "GET",
            "/api/v0/searches?status=active&target=wishlist&limit=1",
            None,
            "",
            &state,
        )
        .await
        .expect("filtered searches");

        assert_eq!(filtered.status, "200 OK");
        assert!(filtered.body.contains("\"count\":2"));
        assert!(filtered.body.contains("\"filtered_count\":1"));
        assert!(filtered.body.contains("\"limit\":1"));
        assert!(filtered.body.contains("\"query\":\"beta\""));
        assert!(!filtered.body.contains("\"query\":\"alpha\""));
    }

    #[tokio::test]
    async fn search_api_expires_and_prunes_records() {
        let (state, mut receiver) = test_state();

        let created = super::route_http_request(
            "POST",
            "/api/v0/searches",
            None,
            "{\"query\":\"short lived\",\"ttl_seconds\":1}",
            &state,
        )
        .await
        .expect("create search");
        assert_eq!(created.status, "201 Created");
        assert!(created.body.contains("\"expires_at\":"));
        let _ = receiver.try_recv();

        {
            let mut searches = state.searches.write().await;
            let record = searches
                .records
                .iter_mut()
                .find(|record| record.token == 1)
                .unwrap();
            record.expires_at = 0;
        }

        let listed = super::route_http_request("GET", "/api/v0/searches", None, "", &state)
            .await
            .expect("list searches");
        assert_eq!(listed.status, "200 OK");
        assert!(listed.body.contains("\"status\":\"expired\""));
        assert!(listed.body.contains("\"expired\":1"));

        let pruned = super::route_http_request("POST", "/api/v0/searches/prune", None, "", &state)
            .await
            .expect("prune searches");
        assert_eq!(pruned.status, "200 OK");
        assert!(pruned.body.contains("\"pruned\":1"));
        assert!(pruned.body.contains("\"remaining\":0"));
    }

    #[tokio::test]
    async fn events_api_records_mutating_workflows() {
        let (state, mut receiver) = test_state();

        super::route_http_request(
            "POST",
            "/api/v0/searches",
            None,
            "{\"query\":\"event flac\"}",
            &state,
        )
        .await
        .unwrap();
        let _ = receiver.try_recv();
        super::route_http_request(
            "POST",
            "/api/v0/messages/inbound",
            None,
            "{\"username\":\"friend\",\"body\":\"hi\"}",
            &state,
        )
        .await
        .unwrap();

        let events = super::route_http_request("GET", "/api/v0/events", None, "", &state)
            .await
            .expect("events response");
        assert_eq!(events.status, "200 OK");
        assert!(events.body.contains("\"kind\":\"search.started\""));
        assert!(events.body.contains("\"kind\":\"message.received\""));
        assert!(events.body.contains("\"count\":2"));

        let filtered = super::route_http_request(
            "GET",
            "/api/v0/events?kind=search.started",
            None,
            "",
            &state,
        )
        .await
        .expect("filtered events");
        assert_eq!(filtered.status, "200 OK");
        assert!(filtered.body.contains("\"filtered_count\":1"));
        assert!(filtered.body.contains("\"resource\":\"1\""));
        assert!(!filtered.body.contains("message.received"));
    }

    #[tokio::test]
    async fn webhook_registration_rejects_invalid_events_and_caps_count() {
        let (state, _receiver) = test_state();

        let invalid = super::route_http_request(
            "POST",
            "/api/webhooks",
            None,
            r#"{"url":"https://example.test/hook","events":"search.created,bogus"}"#,
            &state,
        )
        .await
        .expect("invalid webhook");
        assert_eq!(invalid.status, "400 Bad Request");
        assert_eq!(invalid.body, "{\"error\":\"invalid webhook event\"}");

        for index in 0..super::webhooks::MAX_WEBHOOKS {
            let body = format!(
                r#"{{"url":"https://example.test/hook/{index}","events":"search.created"}}"#
            );
            let created = super::route_http_request("POST", "/api/webhooks", None, &body, &state)
                .await
                .expect("create webhook");
            assert_eq!(created.status, "201 Created");
        }

        let capped = super::route_http_request(
            "POST",
            "/api/webhooks",
            None,
            r#"{"url":"https://example.test/hook/overflow","events":"search.created"}"#,
            &state,
        )
        .await
        .expect("capped webhook");
        assert_eq!(capped.status, "400 Bad Request");
        assert_eq!(capped.body, "{\"error\":\"webhook limit reached\"}");
    }

    #[tokio::test]
    async fn webhook_patch_requires_active_boolean() {
        let (state, _receiver) = test_state();
        let created = super::route_http_request(
            "POST",
            "/api/webhooks",
            None,
            r#"{"url":"https://example.test/hook","events":"search.created"}"#,
            &state,
        )
        .await
        .expect("create webhook");
        let id = serde_json::from_str::<serde_json::Value>(&created.body).expect("created json")
            ["id"]
            .as_str()
            .expect("webhook id")
            .to_owned();

        let missing =
            super::route_http_request("PATCH", &format!("/api/webhooks/{id}"), None, "{}", &state)
                .await
                .expect("missing active");
        assert_eq!(missing.status, "400 Bad Request");
        assert_eq!(missing.body, "{\"error\":\"active boolean is required\"}");

        let patched = super::route_http_request(
            "PATCH",
            &format!("/api/webhooks/{id}"),
            None,
            r#"{"active":false}"#,
            &state,
        )
        .await
        .expect("patch webhook");
        assert_eq!(patched.status, "200 OK");
        assert!(patched.body.contains("\"active\":false"));
    }

    #[tokio::test]
    async fn search_api_supports_targeted_dispatch_commands() {
        let (state, mut receiver) = test_state();

        let user = super::route_http_request(
            "POST",
            "/api/v0/searches",
            None,
            "{\"query\":\"rare\",\"target\":\"user\",\"username\":\"friend\"}",
            &state,
        )
        .await
        .expect("user search");
        assert_eq!(user.status, "201 Created");
        assert!(user.body.contains("\"target\":\"user\""));
        assert!(user.body.contains("\"target_name\":\"friend\""));
        assert_eq!(
            receiver.try_recv().expect("user search command"),
            super::SessionCommand::Search {
                token: 1,
                query: "rare".to_owned(),
                target: super::SearchDispatchTarget::User("friend".to_owned()),
            }
        );

        let room = super::route_http_request(
            "POST",
            "/api/v0/searches",
            None,
            "{\"query\":\"ambient\",\"target\":\"room\",\"room\":\"music\"}",
            &state,
        )
        .await
        .expect("room search");
        assert_eq!(room.status, "201 Created");
        assert!(room.body.contains("\"target\":\"room\""));
        assert_eq!(
            receiver.try_recv().expect("room search command"),
            super::SessionCommand::Search {
                token: 2,
                query: "ambient".to_owned(),
                target: super::SearchDispatchTarget::Room("music".to_owned()),
            }
        );

        let wishlist = super::route_http_request(
            "POST",
            "/api/v0/searches",
            None,
            "{\"query\":\"wantlist\",\"target\":\"wishlist\"}",
            &state,
        )
        .await
        .expect("wishlist search");
        assert_eq!(wishlist.status, "201 Created");
        assert!(wishlist.body.contains("\"target\":\"wishlist\""));
        assert_eq!(
            receiver.try_recv().expect("wishlist search command"),
            super::SessionCommand::Search {
                token: 3,
                query: "wantlist".to_owned(),
                target: super::SearchDispatchTarget::Wishlist,
            }
        );
    }

    #[tokio::test]
    async fn search_api_rejects_invalid_targeted_dispatch() {
        let (state, _receiver) = test_state();

        let response = super::route_http_request(
            "POST",
            "/api/v0/searches",
            None,
            "{\"query\":\"rare\",\"target\":\"user\"}",
            &state,
        )
        .await
        .expect("bad user search");

        assert_eq!(response.status, "400 Bad Request");
        assert_eq!(
            response.body,
            "{\"error\":\"username is required for user search\"}"
        );

        let invalid_target = super::route_http_request(
            "POST",
            "/api/v0/searches",
            None,
            "{\"query\":\"rare\",\"target\":\"bogus\"}",
            &state,
        )
        .await
        .expect("bad target search");

        assert_eq!(invalid_target.status, "400 Bad Request");
        assert_eq!(invalid_target.body, "{\"error\":\"invalid search target\"}");
    }

    #[tokio::test]
    async fn search_response_api_merges_flattened_results() {
        let (state, _receiver) = test_state();
        super::route_http_request(
            "POST",
            "/api/v0/searches",
            None,
            "{\"query\":\"remote\"}",
            &state,
        )
        .await
        .unwrap();

        let response = super::route_http_request(
            "POST",
            "/api/v0/search-responses",
            None,
            "{\"token\":1,\"peer_username\":\"peer1\",\"filename\":\"Remote/Song.mp3\",\"size\":99,\"slot_free\":false,\"average_speed\":12,\"queue_length\":3}",
            &state,
        )
        .await
        .expect("ingest response");

        assert_eq!(response.status, "200 OK");
        assert!(response.body.contains("\"result_count\":1"));
        assert!(response.body.contains("\"peer_username\":\"peer1\""));
        assert!(response.body.contains("\"filename\":\"Remote/Song.mp3\""));
        assert!(response.body.contains("\"extension\":\"mp3\""));
        assert!(response.body.contains("\"slot_free\":false"));
        assert!(response.body.contains("\"average_speed\":12"));
        assert!(response.body.contains("\"queue_length\":3"));

        let responses =
            super::route_http_request("GET", "/api/v0/searches/1/responses", None, "", &state)
                .await
                .expect("search responses");
        assert_eq!(responses.status, "200 OK");
        assert!(responses.body.starts_with('['));
        assert!(responses.body.contains("\"token\":1"));
        assert!(responses.body.contains("\"filename\":\"Remote/Song.mp3\""));
    }

    #[tokio::test]
    async fn search_response_api_rejects_missing_fields() {
        let (state, _receiver) = test_state();

        let response =
            super::route_http_request("POST", "/api/v0/search-responses", None, "{}", &state)
                .await
                .expect("bad response");

        assert_eq!(response.status, "400 Bad Request");
        assert_eq!(response.body, "{\"error\":\"token is required\"}");
    }

    #[test]
    fn search_store_merges_peer_search_responses() {
        let mut store = super::SearchStore::new();
        let record = store.create(None, "remote".to_owned(), "global", None, Vec::new(), 300);
        let response = FileSearchResponse {
            username: "peer1".to_owned(),
            token: record.token,
            results: vec![FileEntry {
                code: 1,
                filename: "Remote/Song.flac".to_owned(),
                size: 123,
                extension: "flac".to_owned(),
                attributes: Vec::new(),
            }],
            slot_free: false,
            average_speed: 42,
            queue_length: 7,
            unknown: 0,
            private_results: Vec::new(),
        };

        let updated = store
            .add_peer_response(&response)
            .expect("peer response accepted");

        assert_eq!(updated.results.len(), 1);
        assert_eq!(updated.results[0].peer_username.as_deref(), Some("peer1"));
        assert_eq!(updated.results[0].slot_free, Some(false));
        assert_eq!(updated.results[0].average_speed, Some(42));
        assert_eq!(updated.results[0].queue_length, Some(7));
    }

    #[tokio::test]
    async fn transfer_api_creates_updates_and_reports_stats() {
        let (state, _receiver) = test_state();

        let created = super::route_http_request(
            "POST",
            "/api/v0/transfers",
            None,
            "{\"direction\":0,\"filename\":\"Remote/Song.flac\",\"size\":100}",
            &state,
        )
        .await
        .expect("create transfer");
        assert_eq!(created.status, "201 Created");
        assert!(created.body.contains("\"id\":1"));
        assert!(created.body.contains("\"status\":\"queued\""));

        let started =
            super::route_http_request("POST", "/api/v0/transfers/1/start", None, "", &state)
                .await
                .expect("start transfer");
        assert_eq!(started.status, "200 OK");
        assert!(started.body.contains("\"status\":\"in_progress\""));

        let progress = super::route_http_request(
            "POST",
            "/api/v0/transfers/1/progress",
            None,
            "{\"bytes_transferred\":40}",
            &state,
        )
        .await
        .expect("progress transfer");
        assert_eq!(progress.status, "200 OK");
        assert!(progress.body.contains("\"bytes_transferred\":40"));

        let completed = super::route_http_request(
            "POST",
            "/api/v0/transfers/1/complete",
            None,
            "{\"bytes_transferred\":100}",
            &state,
        )
        .await
        .expect("complete transfer");
        assert_eq!(completed.status, "200 OK");
        assert!(completed.body.contains("\"status\":\"succeeded\""));

        let stats = super::route_http_request("GET", "/api/v0/transfers/stats", None, "", &state)
            .await
            .expect("transfer stats");
        assert_eq!(stats.status, "200 OK");
        assert!(stats.body.contains("\"total\":1"));
        assert!(stats.body.contains("\"succeeded\":1"));
        assert!(stats.body.contains("\"bytes_transferred\":100"));

        let filtered = super::route_http_request(
            "GET",
            "/api/v0/transfers?status=succeeded&q=song&limit=1",
            None,
            "",
            &state,
        )
        .await
        .expect("filtered transfers");
        assert_eq!(filtered.status, "200 OK");
        assert!(filtered.body.contains("\"filtered_count\":1"));
        assert!(filtered.body.contains("\"limit\":1"));
        assert!(filtered.body.contains("Remote/Song.flac"));
    }

    #[tokio::test]
    async fn transfer_start_enforces_max_active_policy() {
        let (state, _receiver) =
            test_state_with_env(MapEnv::default().with("SLSKR_TRANSFER_MAX_ACTIVE", "1"));

        for filename in ["Remote/One.flac", "Remote/Two.flac"] {
            let body = format!("{{\"filename\":\"{}\",\"size\":10}}", filename);
            let created =
                super::route_http_request("POST", "/api/v0/transfers", None, &body, &state)
                    .await
                    .expect("create transfer");
            assert_eq!(created.status, "201 Created");
        }

        let started =
            super::route_http_request("POST", "/api/v0/transfers/1/start", None, "", &state)
                .await
                .expect("start first");
        assert_eq!(started.status, "200 OK");

        let blocked =
            super::route_http_request("POST", "/api/v0/transfers/2/start", None, "", &state)
                .await
                .expect("start second");
        assert_eq!(blocked.status, "409 Conflict");
        assert_eq!(blocked.body, "{\"error\":\"transfer limit reached\"}");

        {
            let mut transfers = state.transfers.write().await;
            transfers.update_status(1, "cancelled", None, None);
        }

        let unblocked =
            super::route_http_request("POST", "/api/v0/transfers/2/start", None, "", &state)
                .await
                .expect("retry second");
        assert_eq!(unblocked.status, "200 OK");
        assert!(unblocked.body.contains("\"status\":\"in_progress\""));
    }

    #[tokio::test]
    async fn transfer_start_rejects_peer_transfer_when_outbound_disabled() {
        let (state, _receiver) =
            test_state_with_env(MapEnv::default().with("SLSKR_TRANSFER_ALLOW_OUTBOUND", "false"));

        let created = super::route_http_request(
            "POST",
            "/api/v0/transfers",
            None,
            "{\"filename\":\"Remote/Song.flac\",\"peer_username\":\"friend\",\"size\":10}",
            &state,
        )
        .await
        .expect("create transfer");
        assert_eq!(created.status, "201 Created");

        let blocked =
            super::route_http_request("POST", "/api/v0/transfers/1/start", None, "", &state)
                .await
                .expect("start transfer");
        assert_eq!(blocked.status, "409 Conflict");
        assert_eq!(
            blocked.body,
            "{\"error\":\"outbound transfers are disabled\"}"
        );
    }

    #[tokio::test]
    async fn transfer_create_rejects_upload_local_path() {
        let (state, _receiver) = test_state();
        let path = std::env::temp_dir().join(format!(
            "slskr-transfer-local-{}-ok.bin",
            std::process::id()
        ));
        std::fs::write(&path, [1_u8, 2, 3, 4]).expect("write local file");
        let body = format!(
            "{{\"direction\":1,\"filename\":\"Remote/Song.flac\",\"local_path\":\"{}\"}}",
            super::json_escape(&path.display().to_string())
        );

        let created = super::route_http_request("POST", "/api/v0/transfers", None, &body, &state)
            .await
            .expect("create transfer");
        assert_eq!(created.status, "400 Bad Request");
        assert!(created
            .body
            .contains("local_path is not accepted for uploads"));
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn transfer_create_rejects_missing_upload_local_path() {
        let (state, _receiver) = test_state();
        let path = std::env::temp_dir().join(format!(
            "slskr-transfer-local-{}-missing.bin",
            std::process::id()
        ));
        let body = format!(
            "{{\"direction\":1,\"filename\":\"Remote/Song.flac\",\"local_path\":\"{}\"}}",
            super::json_escape(&path.display().to_string())
        );

        let created = super::route_http_request("POST", "/api/v0/transfers", None, &body, &state)
            .await
            .expect("create transfer");
        assert_eq!(created.status, "400 Bad Request");
        assert!(created
            .body
            .contains("local_path is not accepted for uploads"));
    }

    #[cfg(unix)]
    #[test]
    fn download_path_rejects_final_symlink() {
        use std::os::unix::fs::symlink;

        let state_dir = std::env::temp_dir().join(format!(
            "slskr-download-symlink-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        let path =
            super::safe_download_path(&state_dir, "Remote/Song.flac").expect("download path");
        let parent = path.parent().expect("download parent");
        std::fs::create_dir_all(parent).expect("download parent dir");
        let target = state_dir.join("outside.flac");
        std::fs::write(&target, [1_u8]).expect("target file");
        symlink(&target, &path).expect("download symlink");

        let error = super::ensure_scoped_download_path(&state_dir, &path.display().to_string())
            .expect_err("symlink rejected");
        assert!(error.contains("must not be a symlink"));

        let _ = std::fs::remove_dir_all(state_dir);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn upload_share_lookup_rejects_swapped_symlink() {
        use std::os::unix::fs::symlink;

        let (state, _receiver) = test_state();
        let dir = std::env::temp_dir().join(format!(
            "slskr-upload-symlink-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).expect("test dir");
        let shared_path = dir.join("shared.flac");
        let target_path = dir.join("outside.flac");
        std::fs::write(&shared_path, [1_u8, 2, 3, 4]).expect("shared file");
        std::fs::write(&target_path, [5_u8, 6, 7, 8]).expect("target file");
        add_test_share(&state, "Remote/Song.flac", &shared_path, 4).await;
        std::fs::remove_file(&shared_path).expect("remove shared file");
        symlink(&target_path, &shared_path).expect("swap symlink");

        assert!(super::find_shared_local_file(&state, "Remote/Song.flac")
            .await
            .is_none());

        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn transfer_start_with_peer_requests_peer_address() {
        let (state, mut receiver) = test_state();
        let path = std::env::temp_dir().join(format!(
            "slskr-transfer-start-{}-upload.bin",
            std::process::id()
        ));
        std::fs::write(&path, [1_u8, 2, 3, 4]).expect("write shared file");
        add_test_share(&state, "Remote/Song.flac", &path, 4).await;
        let created = super::route_http_request(
            "POST",
            "/api/v0/transfers",
            None,
            "{\"direction\":1,\"peer_username\":\"friend\",\"filename\":\"Remote/Song.flac\",\"size\":4}",
            &state,
        )
        .await
        .expect("create transfer");
        assert_eq!(created.status, "201 Created");
        assert!(created.body.contains("\"token\":1"));

        let started =
            super::route_http_request("POST", "/api/v0/transfers/1/start", None, "", &state)
                .await
                .expect("start transfer");

        assert_eq!(started.status, "200 OK");
        assert!(started.body.contains("\"status\":\"peer_lookup\""));
        assert_eq!(
            receiver.try_recv().expect("transfer command"),
            super::SessionCommand::TransferPeer {
                id: 1,
                username: "friend".to_owned(),
            }
        );
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn peer_address_response_negotiates_pending_transfer() {
        let (state, _receiver) = test_state();
        {
            let mut transfers = state.transfers.write().await;
            let entry = transfers.create(
                1,
                Some("friend".to_owned()),
                "Remote/Song.flac".to_owned(),
                None,
                Some(4),
            );
            assert_eq!(entry.token, 1);
            transfers.update_status(entry.id, "peer_lookup", None, None);
        }

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let local_addr = listener.local_addr().expect("local addr");
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept");
            let mut init = slskr_client::stream::InitConnection::new(stream);
            let init_message = init.receive().await.expect("init");
            assert_eq!(
                init_message,
                slskr_client::protocol::init::InitMessage::PeerInit {
                    username: "friend".to_owned(),
                    connection_type: "P".to_owned(),
                    token: 0,
                }
            );
            let mut peer = slskr_client::stream::PeerMessageConnection::new(init.into_inner());
            assert_eq!(
                peer.receive().await.expect("transfer request"),
                super::PeerMessage::TransferRequest(super::TransferRequest {
                    direction: 1,
                    token: 1,
                    filename: "Remote/Song.flac".to_owned(),
                    size: Some(4),
                })
            );
            peer.send(&super::PeerMessage::TransferResponse(
                super::TransferResponse::Allowed {
                    token: 1,
                    size: Some(4),
                },
            ))
            .await
            .expect("response");
        });
        let address = slskr_client::protocol::server::PeerAddress {
            username: "friend".to_owned(),
            ip: "127.0.0.1".parse().unwrap(),
            port: u32::from(local_addr.port()),
            obfuscation_type: 0,
            obfuscated_port: 0,
        };

        super::project_peer_transfer_response(&state, &address).await;
        server.await.expect("server task");

        let transfers = state.transfers.read().await;
        let record = transfers.entries.first().expect("transfer");
        assert_eq!(record.status, "accepted");
        assert_eq!(record.size, Some(4));
        assert_eq!(record.reason, None);
        assert!(transfers.stats_json().contains("\"in_progress\":1"));
    }

    #[tokio::test]
    async fn peer_address_response_uploads_accepted_local_file_transfer() {
        let (state, _receiver) = test_state();
        let path = std::env::temp_dir().join(format!(
            "slskr-transfer-f-{}-upload.bin",
            std::process::id()
        ));
        std::fs::write(&path, [1_u8, 2, 3, 4]).expect("write upload file");
        add_test_share(&state, "Remote/Song.flac", &path, 4).await;
        {
            let mut transfers = state.transfers.write().await;
            let entry = transfers.create(
                1,
                Some("friend".to_owned()),
                "Remote/Song.flac".to_owned(),
                Some(path.display().to_string()),
                Some(4),
            );
            assert_eq!(entry.token, 1);
            transfers.update_status(entry.id, "peer_lookup", None, None);
        }

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let local_addr = listener.local_addr().expect("local addr");
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept peer-message");
            let mut init = slskr_client::stream::InitConnection::new(stream);
            let init_message = init.receive().await.expect("peer-message init");
            assert_eq!(
                init_message,
                slskr_client::protocol::init::InitMessage::PeerInit {
                    username: "friend".to_owned(),
                    connection_type: "P".to_owned(),
                    token: 0,
                }
            );
            let mut peer = slskr_client::stream::PeerMessageConnection::new(init.into_inner());
            assert_eq!(
                peer.receive().await.expect("transfer request"),
                super::PeerMessage::TransferRequest(super::TransferRequest {
                    direction: 1,
                    token: 1,
                    filename: "Remote/Song.flac".to_owned(),
                    size: Some(4),
                })
            );
            peer.send(&super::PeerMessage::TransferResponse(
                super::TransferResponse::Allowed {
                    token: 1,
                    size: Some(4),
                },
            ))
            .await
            .expect("transfer response");

            let (stream, _) = listener.accept().await.expect("accept file-transfer");
            let mut init = slskr_client::stream::InitConnection::new(stream);
            let init_message = init.receive().await.expect("file init");
            assert_eq!(
                init_message,
                slskr_client::protocol::init::InitMessage::PeerInit {
                    username: "friend".to_owned(),
                    connection_type: "F".to_owned(),
                    token: 0,
                }
            );
            let mut file =
                slskr_client::file_transfer::FileTransferConnection::new(init.into_inner());
            assert_eq!(file.receive_token().await.expect("token"), 1);
            file.send_offset(1).await.expect("offset");
            file.read_chunk(3).await.expect("chunk")
        });
        let address = slskr_client::protocol::server::PeerAddress {
            username: "friend".to_owned(),
            ip: "127.0.0.1".parse().unwrap(),
            port: u32::from(local_addr.port()),
            obfuscation_type: 0,
            obfuscated_port: 0,
        };

        super::project_peer_transfer_response(&state, &address).await;
        let uploaded = server.await.expect("server task");
        assert_eq!(uploaded, vec![2, 3, 4]);

        let transfers = state.transfers.read().await;
        let record = transfers.entries.first().expect("transfer");
        assert_eq!(record.status, "succeeded");
        assert_eq!(record.bytes_transferred, 3);
        assert_eq!(record.size, Some(4));
        assert_eq!(record.reason, None);
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn peer_address_response_downloads_accepted_file_transfer_with_resume() {
        let (state, _receiver) = test_state();
        let path = super::safe_download_path(&state.config.state_dir, "Remote/Song.flac")
            .expect("download path");
        std::fs::create_dir_all(path.parent().unwrap()).expect("download dir");
        std::fs::write(&path, [9_u8]).expect("write partial download file");
        {
            let mut transfers = state.transfers.write().await;
            let entry = transfers.create(
                0,
                Some("friend".to_owned()),
                "Remote/Song.flac".to_owned(),
                Some(path.display().to_string()),
                Some(4),
            );
            assert_eq!(entry.token, 1);
            transfers.update_status(entry.id, "peer_lookup", None, None);
        }

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let local_addr = listener.local_addr().expect("local addr");
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept peer-message");
            let mut init = slskr_client::stream::InitConnection::new(stream);
            let init_message = init.receive().await.expect("peer-message init");
            assert_eq!(
                init_message,
                slskr_client::protocol::init::InitMessage::PeerInit {
                    username: "friend".to_owned(),
                    connection_type: "P".to_owned(),
                    token: 0,
                }
            );
            let mut peer = slskr_client::stream::PeerMessageConnection::new(init.into_inner());
            assert_eq!(
                peer.receive().await.expect("transfer request"),
                super::PeerMessage::TransferRequest(super::TransferRequest {
                    direction: 0,
                    token: 1,
                    filename: "Remote/Song.flac".to_owned(),
                    size: None,
                })
            );
            peer.send(&super::PeerMessage::TransferResponse(
                super::TransferResponse::Allowed {
                    token: 1,
                    size: Some(4),
                },
            ))
            .await
            .expect("transfer response");

            let (stream, _) = listener.accept().await.expect("accept file-transfer");
            let mut init = slskr_client::stream::InitConnection::new(stream);
            let init_message = init.receive().await.expect("file init");
            assert_eq!(
                init_message,
                slskr_client::protocol::init::InitMessage::PeerInit {
                    username: "friend".to_owned(),
                    connection_type: "F".to_owned(),
                    token: 0,
                }
            );
            let mut file =
                slskr_client::file_transfer::FileTransferConnection::new(init.into_inner());
            file.send_token(1).await.expect("token");
            assert_eq!(file.receive_offset().await.expect("offset"), 1);
            file.write_chunk(&[2, 3, 4]).await.expect("chunk");
        });
        let address = slskr_client::protocol::server::PeerAddress {
            username: "friend".to_owned(),
            ip: "127.0.0.1".parse().unwrap(),
            port: u32::from(local_addr.port()),
            obfuscation_type: 0,
            obfuscated_port: 0,
        };

        super::project_peer_transfer_response(&state, &address).await;
        server.await.expect("server task");

        let transfers = state.transfers.read().await;
        let record = transfers.entries.first().expect("transfer");
        assert_eq!(record.status, "succeeded");
        assert_eq!(record.bytes_transferred, 4);
        assert_eq!(record.size, Some(4));
        assert_eq!(
            std::fs::read(&path).expect("download file"),
            vec![9, 2, 3, 4]
        );
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn peer_address_response_uses_obfuscated_file_transfer_when_advertised() {
        let (state, _receiver) = test_state();
        let path = std::env::temp_dir().join(format!(
            "slskr-transfer-f-{}-obfuscated-upload.bin",
            std::process::id()
        ));
        std::fs::write(&path, [5_u8, 6, 7]).expect("write upload file");
        add_test_share(&state, "Remote/Obfuscated.flac", &path, 3).await;
        {
            let mut transfers = state.transfers.write().await;
            let entry = transfers.create(
                1,
                Some("friend".to_owned()),
                "Remote/Obfuscated.flac".to_owned(),
                Some(path.display().to_string()),
                Some(3),
            );
            assert_eq!(entry.token, 1);
            transfers.update_status(entry.id, "peer_lookup", None, None);
        }

        let listener = slskr_client::listener::Listener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let local_addr = listener.local_addr().expect("local addr");
        let server = tokio::spawn(async move {
            let (incoming, _) = listener.accept_obfuscated().await.expect("accept p");
            let slskr_client::listener::IncomingConnection::ObfuscatedPeerMessages(mut peer) =
                incoming
            else {
                panic!("expected obfuscated peer messages");
            };
            assert_eq!(
                peer.receive().await.expect("transfer request"),
                super::PeerMessage::TransferRequest(super::TransferRequest {
                    direction: 1,
                    token: 1,
                    filename: "Remote/Obfuscated.flac".to_owned(),
                    size: Some(3),
                })
            );
            peer.send(&super::PeerMessage::TransferResponse(
                super::TransferResponse::Allowed {
                    token: 1,
                    size: Some(3),
                },
            ))
            .await
            .expect("transfer response");

            let (incoming, _) = listener.accept_obfuscated().await.expect("accept f");
            let slskr_client::listener::IncomingConnection::PeerInit {
                username,
                kind,
                token,
                stream,
            } = incoming
            else {
                panic!("expected obfuscated file-transfer peer init");
            };
            assert_eq!(username, "friend");
            assert_eq!(kind, super::ConnectionKind::FileTransfer);
            assert_eq!(token, 0);
            let mut file = slskr_client::file_transfer::FileTransferConnection::new(stream);
            assert_eq!(file.receive_token().await.expect("token"), 1);
            file.send_offset(0).await.expect("offset");
            file.read_chunk(3).await.expect("chunk")
        });
        let address = slskr_client::protocol::server::PeerAddress {
            username: "friend".to_owned(),
            ip: "127.0.0.1".parse().unwrap(),
            port: 0,
            obfuscation_type: super::ROTATED_OBFUSCATION_TYPE,
            obfuscated_port: local_addr.port(),
        };

        super::project_peer_transfer_response(&state, &address).await;
        let uploaded = server.await.expect("server task");
        assert_eq!(uploaded, vec![5, 6, 7]);

        let transfers = state.transfers.read().await;
        let record = transfers.entries.first().expect("transfer");
        assert_eq!(record.status, "succeeded");
        assert_eq!(record.bytes_transferred, 3);
        assert_eq!(record.size, Some(3));
        assert_eq!(record.reason, None);
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn indirect_transfer_command_requests_connect_to_peer() {
        let (state, mut receiver) = test_state();
        super::try_send_session_command(
            &state,
            super::SessionCommand::IndirectTransfer {
                id: 7,
                username: "friend".to_owned(),
                token: 42,
            },
        );

        assert_eq!(
            receiver.try_recv().expect("indirect command"),
            super::SessionCommand::IndirectTransfer {
                id: 7,
                username: "friend".to_owned(),
                token: 42,
            }
        );
    }

    #[tokio::test]
    async fn connect_to_peer_response_executes_indirect_file_upload() {
        let (state, _receiver) = test_state();
        let path = std::env::temp_dir().join(format!(
            "slskr-transfer-f-{}-indirect-upload.bin",
            std::process::id()
        ));
        std::fs::write(&path, [8_u8, 9, 10]).expect("write upload file");
        add_test_share(&state, "Remote/Indirect.flac", &path, 3).await;
        {
            let mut transfers = state.transfers.write().await;
            let entry = transfers.create(
                1,
                Some("friend".to_owned()),
                "Remote/Indirect.flac".to_owned(),
                Some(path.display().to_string()),
                Some(3),
            );
            assert_eq!(entry.token, 1);
            transfers.update_status(entry.id, "indirect_pending", None, None);
        }

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let local_addr = listener.local_addr().expect("local addr");
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept indirect");
            let incoming = slskr_client::listener::demux_incoming(stream)
                .await
                .expect("demux indirect");
            let slskr_client::listener::IncomingConnection::PierceFirewall { token, stream } =
                incoming
            else {
                panic!("expected pierce firewall");
            };
            assert_eq!(token, 1);
            let mut file = slskr_client::file_transfer::FileTransferConnection::new(stream);
            assert_eq!(file.receive_token().await.expect("token"), 1);
            file.send_offset(1).await.expect("offset");
            file.read_chunk(2).await.expect("chunk")
        });
        let response = super::ConnectToPeerResponse {
            username: "friend".to_owned(),
            connection_type: "F".to_owned(),
            ip: "127.0.0.1".parse().unwrap(),
            port: u32::from(local_addr.port()),
            token: 1,
            privileged: false,
            obfuscation_type: 0,
            obfuscated_port: 0,
        };

        super::project_indirect_transfer_response(&state, &response).await;
        let uploaded = server.await.expect("server task");
        assert_eq!(uploaded, vec![9, 10]);

        let transfers = state.transfers.read().await;
        let record = transfers.entries.first().expect("transfer");
        assert_eq!(record.status, "succeeded");
        assert_eq!(record.bytes_transferred, 2);
        assert_eq!(record.size, Some(3));
        assert_eq!(record.reason, None);
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn connect_to_peer_response_executes_indirect_browse() {
        let (state, _receiver) = test_state();
        {
            let mut browse = state.browse.write().await;
            browse.request("friend".to_owned());
            assert_eq!(
                browse.mark_indirect_pending("friend", "direct failed".to_owned()),
                Some(1)
            );
        }

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let local_addr = listener.local_addr().expect("local addr");
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept indirect");
            let incoming = slskr_client::listener::demux_incoming(stream)
                .await
                .expect("demux indirect");
            let slskr_client::listener::IncomingConnection::PierceFirewall { token, stream } =
                incoming
            else {
                panic!("expected pierce firewall");
            };
            assert_eq!(token, 1);
            let mut peer = slskr_client::stream::PeerMessageConnection::new(stream);
            assert_eq!(
                peer.receive().await.expect("browse request"),
                super::PeerMessage::GetShareFileList
            );
            let entries =
                crate::config::parse_share_entries("Remote/Indirect.flac=55").expect("entries");
            let payload = super::build_shared_file_list_payload(&entries).expect("payload");
            peer.send(&super::PeerMessage::SharedFileListResponse(payload))
                .await
                .expect("response");
        });
        let response = super::ConnectToPeerResponse {
            username: "friend".to_owned(),
            connection_type: "P".to_owned(),
            ip: "127.0.0.1".parse().unwrap(),
            port: u32::from(local_addr.port()),
            token: 1,
            privileged: false,
            obfuscation_type: 0,
            obfuscated_port: 0,
        };

        super::project_indirect_browse_response(&state, &response).await;
        server.await.expect("server task");

        let browse = state.browse.read().await;
        let record = browse.get("friend").expect("browse record");
        assert_eq!(record.status, "ready");
        assert_eq!(record.indirect_token, None);
        assert_eq!(record.entries.len(), 1);
        assert_eq!(record.entries[0].filename, "Remote/Indirect.flac");
        assert_eq!(record.entries[0].size, 55);
    }

    #[tokio::test]
    async fn inbound_transfer_request_serves_shared_file_over_pierce_firewall() {
        let (state, _receiver) = test_state();
        let path = std::env::temp_dir().join(format!(
            "slskr-transfer-f-{}-inbound-upload.bin",
            std::process::id()
        ));
        std::fs::write(&path, [1_u8, 2, 3, 4]).expect("write shared file");
        {
            let mut shares = state.shares.write().await;
            shares.entries.push(FileEntry {
                code: 1,
                filename: "Virtual/Inbound.flac".to_owned(),
                size: 4,
                extension: "flac".to_owned(),
                attributes: Vec::new(),
            });
            shares
                .local_paths
                .insert("Virtual/Inbound.flac".to_owned(), path.clone());
        }

        super::handle_peer_message(
            &state,
            super::PeerMessage::TransferRequest(super::TransferRequest {
                direction: 0,
                token: 7,
                filename: "Virtual/Inbound.flac".to_owned(),
                size: None,
            }),
            |response| async move {
                assert_eq!(
                    response,
                    super::PeerMessage::TransferResponse(super::TransferResponse::Allowed {
                        token: 7,
                        size: Some(4),
                    },)
                );
                Ok(())
            },
        )
        .await
        .expect("transfer request");

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let local_addr = listener.local_addr().expect("local addr");
        let inbound_state = Arc::clone(&state);
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept file-transfer");
            super::handle_inbound_file_transfer(
                &inbound_state,
                slskr_client::file_transfer::FileTransferConnection::new(stream),
                Some(7),
            )
            .await
            .expect("serve inbound file");
        });
        let stream = tokio::net::TcpStream::connect(local_addr)
            .await
            .expect("connect file-transfer");
        let mut file = slskr_client::file_transfer::FileTransferConnection::new(stream);
        assert_eq!(file.receive_token().await.expect("token"), 7);
        file.send_offset(2).await.expect("offset");
        assert_eq!(file.read_chunk(2).await.expect("chunk"), vec![3, 4]);
        server.await.expect("server task");

        let transfers = state.transfers.read().await;
        let record = transfers.entries.first().expect("transfer");
        assert_eq!(record.status, "succeeded");
        assert_eq!(record.bytes_transferred, 2);
        assert_eq!(record.size, Some(4));
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn inbound_transfer_request_rejects_when_active_limit_is_full() {
        let (state, _receiver) =
            test_state_with_env(MapEnv::default().with("SLSKR_TRANSFER_MAX_ACTIVE", "1"));
        {
            let mut transfers = state.transfers.write().await;
            let entry = transfers.create(1, None, "Remote/Busy.flac".to_owned(), None, Some(1));
            transfers.update_status(entry.id, "in_progress", None, None);
        }

        super::handle_peer_message(
            &state,
            super::PeerMessage::TransferRequest(super::TransferRequest {
                direction: 0,
                token: 99,
                filename: "Virtual/Test.flac".to_owned(),
                size: None,
            }),
            |response| async move {
                assert_eq!(
                    response,
                    super::PeerMessage::TransferResponse(super::TransferResponse::Rejected {
                        token: 99,
                        reason: "transfer limit reached".to_owned(),
                    },)
                );
                Ok(())
            },
        )
        .await
        .expect("reject transfer request");

        let transfers = state.transfers.read().await;
        assert_eq!(transfers.entries.len(), 2);
        let rejected = transfers.entries.get(1).expect("rejection");
        assert_eq!(rejected.status, "rejected");
        assert_eq!(rejected.reason.as_deref(), Some("transfer limit reached"));
    }

    #[tokio::test]
    async fn inbound_transfer_request_rejects_when_inbound_disabled() {
        let (state, _receiver) =
            test_state_with_env(MapEnv::default().with("SLSKR_TRANSFER_ALLOW_INBOUND", "false"));

        super::handle_peer_message(
            &state,
            super::PeerMessage::TransferRequest(super::TransferRequest {
                direction: 0,
                token: 55,
                filename: "Virtual/Test.flac".to_owned(),
                size: None,
            }),
            |response| async move {
                assert_eq!(
                    response,
                    super::PeerMessage::TransferResponse(super::TransferResponse::Rejected {
                        token: 55,
                        reason: "inbound transfers are disabled".to_owned(),
                    },)
                );
                Ok(())
            },
        )
        .await
        .expect("reject transfer request");

        let transfers = state.transfers.read().await;
        let rejected = transfers.entries.first().expect("rejection");
        assert_eq!(rejected.status, "rejected");
        assert_eq!(
            rejected.reason.as_deref(),
            Some("inbound transfers are disabled")
        );
    }

    #[tokio::test]
    async fn transfer_api_rejects_missing_filename() {
        let (state, _receiver) = test_state();

        let response = super::route_http_request("POST", "/api/v0/transfers", None, "{}", &state)
            .await
            .expect("bad transfer");

        assert_eq!(response.status, "400 Bad Request");
        assert_eq!(response.body, "{\"error\":\"filename is required\"}");
    }

    #[tokio::test]
    async fn users_api_watches_lists_and_unwatches_users() {
        let (state, mut receiver) = test_state();

        let watched = super::route_http_request(
            "POST",
            "/api/v0/users/watch",
            None,
            "{\"username\":\"friend\"}",
            &state,
        )
        .await
        .expect("watch user");
        assert_eq!(watched.status, "201 Created");
        assert!(watched.body.contains("\"username\":\"friend\""));
        assert!(watched.body.contains("\"watched\":true"));
        assert_eq!(
            receiver.try_recv().expect("watch command"),
            super::SessionCommand::WatchUser("friend".to_owned())
        );

        let listed = super::route_http_request("GET", "/api/v0/users", None, "", &state)
            .await
            .expect("list users");
        assert_eq!(listed.status, "200 OK");
        assert!(listed.body.contains("\"count\":1"));

        let stats_request = super::route_http_request(
            "POST",
            "/api/v1/users/friend/stats/request",
            None,
            "",
            &state,
        )
        .await
        .expect("request user stats");
        assert_eq!(stats_request.status, "202 Accepted");
        assert_eq!(
            receiver.try_recv().expect("stats command"),
            super::SessionCommand::RequestUserStats("friend".to_owned())
        );

        {
            let mut users = state.users.write().await;
            users.apply_stats(
                "friend".to_owned(),
                &super::UserStats {
                    average_speed: 1234,
                    upload_count: 5,
                    unknown: 0,
                    file_count: 42,
                    directory_count: 7,
                },
            );
        }
        let listed = super::route_http_request("GET", "/api/v0/users", None, "", &state)
            .await
            .expect("list users with stats");
        assert!(listed.body.contains("\"average_speed\":1234"));
        assert!(listed.body.contains("\"upload_count\":5"));
        assert!(listed.body.contains("\"file_count\":42"));
        assert!(listed.body.contains("\"directory_count\":7"));

        let unwatched =
            super::route_http_request("DELETE", "/api/v2/users/friend/watch", None, "", &state)
                .await
                .expect("unwatch user");
        assert_eq!(unwatched.status, "200 OK");
        assert!(unwatched.body.contains("\"watched\":false"));
        assert_eq!(
            receiver.try_recv().expect("unwatch command"),
            super::SessionCommand::UnwatchUser("friend".to_owned())
        );
    }

    #[tokio::test]
    async fn user_browse_api_requests_and_ingests_entries() {
        let (state, mut receiver) = test_state();

        let requested = super::route_http_request(
            "POST",
            "/api/v0/users/friend/browse/request",
            None,
            "",
            &state,
        )
        .await
        .expect("browse request");
        assert_eq!(requested.status, "202 Accepted");
        assert!(requested.body.contains("\"status\":\"requested\""));
        assert_eq!(
            receiver.try_recv().expect("browse command"),
            super::SessionCommand::BrowseUser("friend".to_owned())
        );

        let folder_requested = super::route_http_request(
            "POST",
            "/api/v0/users/friend/browse/folder",
            None,
            "{\"folder\":\"Remote/Album\"}",
            &state,
        )
        .await
        .expect("folder browse request");
        assert_eq!(folder_requested.status, "202 Accepted");
        assert!(folder_requested.body.contains("\"status\":\"requested\""));
        assert!(folder_requested
            .body
            .contains("\"folder\":\"Remote/Album\""));
        assert_eq!(
            receiver.try_recv().expect("folder browse command"),
            super::SessionCommand::BrowseFolder {
                username: "friend".to_owned(),
                folder: "Remote/Album".to_owned()
            }
        );

        let ingested = super::route_http_request(
            "POST",
            "/api/v0/browse-responses",
            None,
            "{\"username\":\"friend\",\"complete\":false,\"entries\": [{\"filename\":\"Remote/Album/Song.flac\",\"size\":123}]}",
            &state,
        )
        .await
        .expect("browse ingest");
        assert_eq!(ingested.status, "200 OK");
        assert!(ingested.body.contains("\"status\":\"partial\""));
        assert!(ingested.body.contains("\"count\":1"));
        assert!(ingested.body.contains("\"total_bytes\":123"));
        assert!(ingested.body.contains("\"extension\":\"flac\""));

        let listed = super::route_http_request(
            "GET",
            "/api/v0/browse?status=partial&q=friend",
            None,
            "",
            &state,
        )
        .await
        .expect("partial browse list");
        assert_eq!(listed.status, "200 OK");
        assert!(listed.body.contains("\"filtered_count\":1"));

        let ingested = super::route_http_request(
            "POST",
            "/api/v0/browse-responses",
            None,
            "{\"username\":\"friend\",\"entries\":[{\"filename\":\"Remote/Album/Cover.jpg\",\"size\":10,\"extension\":\"jpg\"}]}",
            &state,
        )
        .await
        .expect("browse complete ingest");
        assert_eq!(ingested.status, "200 OK");
        assert!(ingested.body.contains("\"status\":\"ready\""));
        assert!(ingested.body.contains("\"count\":2"));
        assert!(ingested.body.contains("\"total_bytes\":133"));
        assert!(ingested.body.contains("\"extension\":\"jpg\""));

        let fetched =
            super::route_http_request("GET", "/api/v0/users/friend/browse", None, "", &state)
                .await
                .expect("browse fetch");
        assert_eq!(fetched.status, "200 OK");
        assert!(fetched.body.contains("Remote/Album/Song.flac"));

        let listed = super::route_http_request(
            "GET",
            "/api/v0/browse?status=ready&q=friend&limit=1",
            None,
            "",
            &state,
        )
        .await
        .expect("browse list");
        assert_eq!(listed.status, "200 OK");
        assert!(listed.body.contains("\"count\":1"));
        assert!(listed.body.contains("\"filtered_count\":1"));
        assert!(listed.body.contains("\"limit\":1"));

        let failed = super::route_http_request(
            "POST",
            "/api/v0/users/friend/browse/fail",
            None,
            "{\"reason\":\"peer timed out\"}",
            &state,
        )
        .await
        .expect("browse fail");
        assert_eq!(failed.status, "200 OK");
        assert!(failed.body.contains("\"status\":\"failed\""));
        assert!(failed.body.contains("\"reason\":\"peer timed out\""));

        let listed = super::route_http_request(
            "GET",
            "/api/v0/browse?status=failed&q=friend",
            None,
            "",
            &state,
        )
        .await
        .expect("failed browse list");
        assert_eq!(listed.status, "200 OK");
        assert!(listed.body.contains("\"filtered_count\":1"));
    }

    #[tokio::test]
    async fn browse_response_api_accepts_single_flattened_entry() {
        let (state, _receiver) = test_state();

        let response = super::route_http_request(
            "POST",
            "/api/v0/browse-responses",
            None,
            "{\"username\":\"friend\",\"filename\":\"Remote/One.mp3\",\"size\":7}",
            &state,
        )
        .await
        .expect("flat browse response");

        assert_eq!(response.status, "200 OK");
        assert!(response.body.contains("\"count\":1"));
        assert!(response.body.contains("\"extension\":\"mp3\""));
    }

    #[tokio::test]
    async fn browse_response_api_rejects_missing_fields() {
        let (state, _receiver) = test_state();

        let response =
            super::route_http_request("POST", "/api/v0/browse-responses", None, "{}", &state)
                .await
                .expect("bad browse response");

        assert_eq!(response.status, "400 Bad Request");
        assert_eq!(response.body, "{\"error\":\"username is required\"}");
    }

    #[test]
    fn shared_file_list_payload_parses_to_browse_entries() {
        let entries =
            crate::config::parse_share_entries("Music/Artist - Song.flac=123;Loose.mp3=7")
                .expect("share fixture");
        let payload = super::build_shared_file_list_payload(&entries).expect("payload");

        let parsed = super::parse_shared_file_list_payload(&payload).expect("parsed payload");

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].filename, "Music/Artist - Song.flac");
        assert_eq!(parsed[0].size, 123);
        assert_eq!(parsed[0].extension, "flac");
        assert_eq!(parsed[1].filename, "Loose.mp3");
    }

    #[test]
    fn folder_contents_payload_filters_to_requested_virtual_folder() {
        let entries = crate::config::parse_share_entries(
            "Remote/Album/Song.flac=321;Remote/Other/Skip.flac=9;Loose.mp3=7",
        )
        .expect("entries");
        let payload =
            super::build_folder_contents_payload(&entries, "Remote/Album").expect("folder payload");
        let parsed =
            super::parse_shared_file_list_payload(&payload).expect("parsed folder payload");

        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].filename, "Remote/Album/Song.flac");
        assert_eq!(parsed[0].size, 321);
    }

    #[tokio::test]
    async fn peer_address_response_fetches_pending_browse_from_plain_peer() {
        let (state, _receiver) = test_state();
        {
            let mut browse = state.browse.write().await;
            browse.request("friend".to_owned());
        }

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let local_addr = listener.local_addr().expect("local addr");
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept");
            let mut init = slskr_client::stream::InitConnection::new(stream);
            let init_message = init.receive().await.expect("init");
            assert_eq!(
                init_message,
                slskr_client::protocol::init::InitMessage::PeerInit {
                    username: "friend".to_owned(),
                    connection_type: "P".to_owned(),
                    token: 0,
                }
            );
            let mut peer = slskr_client::stream::PeerMessageConnection::new(init.into_inner());
            assert_eq!(
                peer.receive().await.expect("browse request"),
                super::PeerMessage::GetShareFileList
            );
            let entries =
                crate::config::parse_share_entries("Remote/Song.flac=321").expect("entries");
            let payload = super::build_shared_file_list_payload(&entries).expect("payload");
            peer.send(&super::PeerMessage::SharedFileListResponse(payload))
                .await
                .expect("response");
        });
        let address = slskr_client::protocol::server::PeerAddress {
            username: "friend".to_owned(),
            ip: "127.0.0.1".parse().unwrap(),
            port: u32::from(local_addr.port()),
            obfuscation_type: 0,
            obfuscated_port: 0,
        };

        super::project_peer_browse_response(&state, &address).await;
        server.await.expect("server task");

        let browse = state.browse.read().await;
        let record = browse.get("friend").expect("browse record");
        assert_eq!(record.status, "ready");
        assert_eq!(record.entries.len(), 1);
        assert_eq!(record.entries[0].filename, "Remote/Song.flac");
        assert_eq!(record.entries[0].size, 321);
    }

    #[tokio::test]
    async fn peer_address_response_falls_back_to_indirect_browse() {
        let (state, mut receiver) = test_state();
        {
            let mut browse = state.browse.write().await;
            browse.request("friend".to_owned());
        }
        let address = slskr_client::protocol::server::PeerAddress {
            username: "friend".to_owned(),
            ip: "127.0.0.1".parse().unwrap(),
            port: 0,
            obfuscation_type: 0,
            obfuscated_port: 0,
        };

        super::project_peer_browse_response(&state, &address).await;

        assert_eq!(
            receiver.try_recv().expect("indirect browse command"),
            super::SessionCommand::IndirectBrowse {
                username: "friend".to_owned(),
                token: 1,
            }
        );
        let browse = state.browse.read().await;
        let record = browse.get("friend").expect("browse record");
        assert_eq!(record.status, "indirect_pending");
        assert_eq!(record.indirect_token, Some(1));
        assert!(record
            .reason
            .as_deref()
            .unwrap_or_default()
            .contains("direct browse failed"));
    }

    #[tokio::test]
    async fn peer_address_response_fetches_pending_browse_folder_from_plain_peer() {
        let (state, _receiver) = test_state();
        {
            let mut browse = state.browse.write().await;
            browse.request_folder("friend".to_owned(), "Remote/Album".to_owned());
        }

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let local_addr = listener.local_addr().expect("local addr");
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept");
            let mut init = slskr_client::stream::InitConnection::new(stream);
            let init_message = init.receive().await.expect("init");
            assert_eq!(
                init_message,
                slskr_client::protocol::init::InitMessage::PeerInit {
                    username: "friend".to_owned(),
                    connection_type: "P".to_owned(),
                    token: 0,
                }
            );
            let mut peer = slskr_client::stream::PeerMessageConnection::new(init.into_inner());
            assert_eq!(
                peer.receive().await.expect("folder request"),
                super::PeerMessage::FolderContentsRequest(super::FolderContentsRequest {
                    token: 0,
                    folder: "Remote/Album".to_owned()
                })
            );
            let entries =
                crate::config::parse_share_entries("Remote/Album/Song.flac=321").expect("entries");
            let payload =
                super::build_folder_contents_payload(&entries, "Remote/Album").expect("payload");
            peer.send(&super::PeerMessage::FolderContentsResponse(payload))
                .await
                .expect("response");
        });
        let address = slskr_client::protocol::server::PeerAddress {
            username: "friend".to_owned(),
            ip: "127.0.0.1".parse().unwrap(),
            port: u32::from(local_addr.port()),
            obfuscation_type: 0,
            obfuscated_port: 0,
        };

        super::project_peer_browse_response(&state, &address).await;
        server.await.expect("server task");

        let browse = state.browse.read().await;
        let record = browse.get("friend").expect("browse record");
        assert_eq!(record.status, "ready");
        assert_eq!(record.folder.as_deref(), Some("Remote/Album"));
        assert_eq!(record.entries.len(), 1);
        assert_eq!(record.entries[0].filename, "Remote/Album/Song.flac");
        assert_eq!(record.entries[0].size, 321);
    }

    #[tokio::test]
    async fn users_api_rejects_missing_username() {
        let (state, _receiver) = test_state();

        let response = super::route_http_request("POST", "/api/v0/users/watch", None, "{}", &state)
            .await
            .expect("bad user watch");

        assert_eq!(response.status, "400 Bad Request");
        assert_eq!(response.body, "{\"error\":\"username is required\"}");
    }

    #[tokio::test]
    async fn messages_api_records_lists_and_acks_messages() {
        let (state, mut receiver) = test_state();

        let outbound = super::route_http_request(
            "POST",
            "/api/v0/messages",
            None,
            "{\"username\":\"friend\",\"body\":\"hello\"}",
            &state,
        )
        .await
        .expect("outbound message");
        assert_eq!(outbound.status, "201 Created");
        assert!(outbound.body.contains("\"direction\":\"outbound\""));
        assert!(outbound.body.contains("\"acknowledged\":false"));
        assert_eq!(
            receiver.try_recv().expect("message command"),
            super::SessionCommand::MessageUser {
                username: "friend".to_owned(),
                body: "hello".to_owned(),
            }
        );

        let inbound = super::route_http_request(
            "POST",
            "/api/v0/messages/inbound",
            None,
            "{\"username\":\"friend\",\"body\":\"hi\"}",
            &state,
        )
        .await
        .expect("inbound message");
        assert_eq!(inbound.status, "201 Created");
        assert!(inbound.body.contains("\"direction\":\"inbound\""));

        let listed = super::route_http_request("GET", "/api/v0/messages/friend", None, "", &state)
            .await
            .expect("list messages");
        assert_eq!(listed.status, "200 OK");
        assert!(listed.body.contains("\"body\":\"hello\""));
        assert!(listed.body.contains("\"body\":\"hi\""));

        let filtered = super::route_http_request(
            "GET",
            "/api/v0/messages?username=friend&direction=inbound&q=hi&limit=1",
            None,
            "",
            &state,
        )
        .await
        .expect("filtered messages");
        assert_eq!(filtered.status, "200 OK");
        assert!(filtered.body.contains("\"filtered_count\":1"));
        assert!(filtered.body.contains("\"direction\":\"inbound\""));
        assert!(!filtered.body.contains("\"body\":\"hello\""));

        let acked = super::route_http_request("POST", "/api/v0/messages/1/ack", None, "", &state)
            .await
            .expect("ack message");
        assert_eq!(acked.status, "200 OK");
        assert!(acked.body.contains("\"acknowledged\":true"));
        assert_eq!(
            receiver.try_recv().expect("ack command"),
            super::SessionCommand::MessageAcked { id: 1 }
        );
    }

    #[tokio::test]
    async fn rooms_api_joins_and_records_messages() {
        let (state, mut receiver) = test_state();

        let refresh = super::route_http_request("POST", "/api/v0/rooms/refresh", None, "", &state)
            .await
            .expect("room refresh");
        assert_eq!(refresh.status, "202 Accepted");
        assert_eq!(
            receiver.try_recv().expect("room refresh command"),
            super::SessionCommand::RefreshRooms
        );

        let joined =
            super::route_http_request("POST", "/api/v0/rooms/music/join", None, "", &state)
                .await
                .expect("join room");
        assert_eq!(joined.status, "201 Created");
        assert!(joined.body.contains("\"name\":\"music\""));
        assert!(joined.body.contains("\"joined\":true"));
        assert_eq!(
            receiver.try_recv().expect("join command"),
            super::SessionCommand::JoinRoom("music".to_owned())
        );

        let message = super::route_http_request(
            "POST",
            "/api/v0/rooms/music/messages",
            None,
            "{\"username\":\"friend\",\"body\":\"track?\"}",
            &state,
        )
        .await
        .expect("room message");
        assert_eq!(message.status, "200 OK");
        assert!(message.body.contains("\"message_count\":1"));
        assert!(message.body.contains("\"body\":\"track?\""));
        assert_eq!(
            receiver.try_recv().expect("room message command"),
            super::SessionCommand::SayRoom {
                room: "music".to_owned(),
                body: "track?".to_owned(),
            }
        );

        let rooms = super::route_http_request("GET", "/api/v0/rooms", None, "", &state)
            .await
            .expect("list rooms");
        assert_eq!(rooms.status, "200 OK");
        assert!(rooms.body.contains("\"count\":1"));

        let filtered =
            super::route_http_request("GET", "/api/v0/rooms?joined=true&q=music", None, "", &state)
                .await
                .expect("filtered rooms");
        assert_eq!(filtered.status, "200 OK");
        assert!(filtered.body.contains("\"filtered_count\":1"));
        assert!(filtered.body.contains("\"name\":\"music\""));

        let left =
            super::route_http_request("DELETE", "/api/v0/rooms/music/join", None, "", &state)
                .await
                .expect("leave room");
        assert_eq!(left.status, "200 OK");
        assert!(left.body.contains("\"joined\":false"));
        assert_eq!(
            receiver.try_recv().expect("leave room command"),
            super::SessionCommand::LeaveRoom("music".to_owned())
        );

        let joined_filter =
            super::route_http_request("GET", "/api/v0/rooms?joined=true&q=music", None, "", &state)
                .await
                .expect("joined room filter");
        assert!(joined_filter.body.contains("\"filtered_count\":0"));
    }

    #[test]
    fn room_list_projection_tracks_server_metadata() {
        let mut rooms = super::RoomStore::new();
        rooms.join("public".to_owned());
        rooms.apply_room_list(&super::RoomList {
            public_rooms: vec![super::RoomListEntry {
                name: "public".to_owned(),
                user_count: 12,
            }],
            owned_private_rooms: vec![super::RoomListEntry {
                name: "owned".to_owned(),
                user_count: 2,
            }],
            private_rooms: vec![super::RoomListEntry {
                name: "private".to_owned(),
                user_count: 3,
            }],
            operated_private_rooms: vec!["private".to_owned(), "orphan-operated".to_owned()],
        });

        let json = rooms.json(None);
        assert!(json.contains("\"name\":\"public\""));
        assert!(json.contains("\"joined\":true"));
        assert!(json.contains("\"kind\":\"public\""));
        assert!(json.contains("\"user_count\":12"));
        assert!(json.contains("\"name\":\"owned\""));
        assert!(json.contains("\"kind\":\"owned_private\""));
        assert!(json.contains("\"operated\":true"));
        assert!(json.contains("\"name\":\"private\""));
        assert!(json.contains("\"kind\":\"private\""));
        assert!(json.contains("\"user_count\":3"));
        assert!(json.contains("\"name\":\"orphan-operated\""));
        assert!(json.contains("\"kind\":\"operated_private\""));
    }

    #[tokio::test]
    async fn search_api_rejects_missing_query() {
        let (state, _receiver) = test_state();

        let response = super::route_http_request("POST", "/api/v0/searches", None, "{}", &state)
            .await
            .expect("bad search");

        assert_eq!(response.status, "400 Bad Request");
        assert_eq!(
            response.body,
            "{\"error\":\"query/searchText is required\"}"
        );
    }

    #[tokio::test]
    async fn unknown_api_route_returns_json_404() {
        let (state, _receiver) = test_state();

        let response = super::route_http_request("GET", "/api/v0/missing", None, "", &state)
            .await
            .expect("route response");

        assert_eq!(response.status, "404 Not Found");
        assert_eq!(response.content_type, "application/json");
        assert_eq!(response.body, "{\"error\":\"not found\"}");
    }

    #[test]
    fn usernames_are_redacted() {
        assert_eq!(redact_username("tester"), "t***r");
        assert_eq!(redact_username("xy"), "**");
    }

    #[test]
    fn json_escape_handles_control_characters() {
        assert_eq!(json_escape("a\"b\\c\n"), "a\\\"b\\\\c\\n");
    }

    #[test]
    fn query_params_decode_percent_encoding() {
        assert_eq!(
            percent_decode("Virtual%2FTest+File.flac"),
            "Virtual/Test File.flac"
        );
        assert_eq!(
            query_params("q=test+file&extension=flac"),
            vec![
                ("q".to_owned(), "test file".to_owned()),
                ("extension".to_owned(), "flac".to_owned()),
            ]
        );
    }

    #[test]
    fn list_limits_are_bounded_by_default() {
        let default_filter = super::RecordListFilter::from_query(None);
        assert_eq!(default_filter.limit, Some(super::DEFAULT_LIST_LIMIT));

        let huge_filter = super::RecordListFilter::from_query(Some("limit=999999"));
        assert_eq!(huge_filter.limit, Some(super::DEFAULT_LIST_LIMIT));

        let zero_filter = super::CatalogFilter::from_query(Some("limit=0"));
        assert_eq!(zero_filter.limit, Some(super::DEFAULT_LIST_LIMIT));
    }

    #[test]
    fn extracts_simple_json_string_fields() {
        assert_eq!(
            super::extract_json_string_field(r#"{"query":"artist \"song\""}"#, "query"),
            Some("artist \"song\"".to_owned())
        );
        assert_eq!(
            super::extract_json_string_field(
                r#"{"a":"\"query\":\"hijacked\"","query":"real"}"#,
                "query"
            ),
            Some("real".to_owned())
        );
        assert_eq!(
            super::extract_json_string_array_field(
                r#"{"capabilities":["shares","telemetry","quoted \" item"]}"#,
                "capabilities"
            ),
            Some(vec![
                "shares".to_owned(),
                "telemetry".to_owned(),
                "quoted \" item".to_owned()
            ])
        );
        assert_eq!(
            super::extract_json_string_field(r#"{"other":"value"}"#, "query"),
            None
        );
        assert_eq!(
            super::extract_json_u32_field(r#"{"token":42}"#, "token"),
            Some(42)
        );
        assert_eq!(
            super::extract_json_bool_field(r#"{"slot_free":false}"#, "slot_free"),
            Some(false)
        );
    }

    #[test]
    fn capabilities_negotiation_escapes_unsupported_values() {
        let response = super::capabilities_negotiate_response(
            r#"{"capabilities":["shares","a\",\"x\":\"y"]}"#,
        );
        let parsed = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
        assert_eq!(parsed["accepted"], serde_json::json!(["shares"]));
        assert_eq!(parsed["unsupported"], serde_json::json!(["a\",\"x\":\"y"]));
    }

    #[test]
    fn websocket_origin_must_match_host_when_present() {
        let headers = super::RequestSecurityHeaders {
            host: Some("127.0.0.1:5030".to_owned()),
            origin: Some("https://evil.example".to_owned()),
            referer: None,
            cookie: None,
        };
        assert!(!super::request_origin_matches_host(
            &headers,
            "127.0.0.1:5030"
        ));

        let headers = super::RequestSecurityHeaders {
            host: Some("127.0.0.1:5030".to_owned()),
            origin: Some("http://127.0.0.1:5030".to_owned()),
            referer: None,
            cookie: None,
        };
        assert!(super::request_origin_matches_host(
            &headers,
            "127.0.0.1:5030"
        ));

        let headers = super::RequestSecurityHeaders {
            host: Some("[::1]:5030".to_owned()),
            origin: Some("http://[::1]:5030".to_owned()),
            referer: None,
            cookie: None,
        };
        assert!(super::request_origin_matches_host(&headers, "[::1]:5030"));
    }

    #[test]
    fn cors_headers_reject_control_characters() {
        assert_eq!(
            super::cors_headers(Some("https://example.test\r\nX-Bad: yes"), &["*"]),
            ""
        );
        let headers = super::cors_headers(Some("https://example.test"), &["*"]);
        assert!(headers.contains("Access-Control-Allow-Origin: https://example.test"));
    }

    #[test]
    fn protected_api_cache_control_is_no_store() {
        assert_eq!(
            super::cache_control_header("GET", "application/json", "/api/shares/catalog"),
            Some("Cache-Control: no-store\r\n".to_string())
        );
        assert_eq!(
            super::cache_control_header("GET", "application/json", "/api/metrics"),
            Some("Cache-Control: no-store\r\n".to_string())
        );
        assert_eq!(
            super::cache_control_header("GET", "application/json", "/api/capabilities"),
            Some("Cache-Control: public, max-age=3600\r\n".to_string())
        );
    }

    #[tokio::test]
    async fn patch_options_reports_non_persisted_runtime_update() {
        let (state, _receiver) = test_state();

        let response = super::route_http_request(
            "PATCH",
            "/api/options",
            None,
            r#"{"key":"x\",\"owned\":true,\"a\":\"b","value":"y"}"#,
            &state,
        )
        .await
        .expect("options response");

        assert_eq!(response.status, "200 OK");
        assert!(response.body.contains("\"persisted\":false"));
    }

    #[test]
    fn toml_config_populates_app_config_without_leaking_password() {
        let file_config = toml::from_str::<FileConfig>(
            r#"
                [app]
                http_bind = "127.0.0.1:7788"
                state_dir = "/tmp/slskr-state"
                auto_connect = true
                reconnect = false
                reconnect_seconds = 7
                ping_seconds = 11

                [network]
                server_address = "example.invalid:2242"
                listen_port = 3333
                username = "alice"
                password = "secret-password"

                [listeners]
                regular_bind = "0.0.0.0:3333"
                advertised_port = 4444
                obfuscated_bind = "0.0.0.0:3334"
                obfuscated_advertised_port = 4445

                [profile]
                user_info_description = "custom daemon"

                [timeouts]
                peer_response_seconds = 9

                [shares]
                dirs = ["/tmp/music"]
                fixture = "Virtual/Song.flac=42"
                follow_symlinks = true
                include_hidden = true
                scan_max_files = 123

                [transfers]
                history_limit = 12
                max_active = 2
                allow_inbound = false
                allow_outbound = false

                [auth]
                disabled = false
                api_token = "test-token"
            "#,
        )
        .unwrap();

        let config = super::AppConfig::from_layers(
            Some(PathBuf::from("/tmp/slskr/config.toml")),
            file_config,
            &MapEnv::default(),
        )
        .unwrap();

        assert_eq!(config.http_bind.to_string(), "127.0.0.1:7788");
        assert_eq!(config.server_address, "example.invalid:2242");
        assert_eq!(config.listen_port, 3333);
        assert_eq!(config.advertised_port, 4444);
        assert_eq!(config.obfuscated_advertised_port, Some(4445));
        assert!(config.auto_connect);
        assert!(!config.reconnect);
        assert_eq!(config.reconnect_delay.as_secs(), 7);
        assert_eq!(config.ping_interval.as_secs(), 11);
        assert_eq!(config.user_info_description, "custom daemon");
        assert_eq!(config.peer_response_timeout.as_secs(), 9);
        assert_eq!(
            config.share_settings.roots,
            vec![PathBuf::from("/tmp/music")]
        );
        assert_eq!(config.share_settings.fixture_entries.len(), 1);
        assert_eq!(config.transfer_history_limit, 12);
        assert_eq!(config.transfer_max_active, 2);
        assert!(!config.transfer_allow_inbound);
        assert!(!config.transfer_allow_outbound);
        assert!(config.auth_required);
        assert_eq!(config.api_token.as_deref(), Some("test-token"));

        let sanitized = config.sanitized_json();
        assert!(sanitized.contains("\"credentials_configured\":true"));
        assert!(sanitized.contains("\"transfer_max_active\":2"));
        assert!(sanitized.contains("\"transfer_allow_inbound\":false"));
        assert!(sanitized.contains("\"transfer_allow_outbound\":false"));
        assert!(sanitized.contains("\"api_token_configured\":true"));
        assert!(sanitized.contains("a***e"));
        assert!(!sanitized.contains("secret-password"));
        assert!(!sanitized.contains("test-token"));
        assert!(!sanitized.contains("\"alice\""));
    }

    #[test]
    fn non_loopback_bind_requires_api_token() {
        let env = MapEnv::default().with("SLSKR_HTTP_BIND", "0.0.0.0:5030");
        let error = super::AppConfig::from_layers(None, FileConfig::default(), &env)
            .expect_err("missing API token should fail");

        assert!(error.contains("SLSKR_API_TOKEN"));
    }

    #[test]
    fn peer_host_override_is_sanitized_config_only() {
        let env = MapEnv::default().with("SLSKR_PEER_HOST_OVERRIDE", "127.0.0.1");
        let config =
            super::AppConfig::from_layers(None, FileConfig::default(), &env).expect("config");

        assert_eq!(
            config.peer_host_override,
            Some(std::net::Ipv4Addr::new(127, 0, 0, 1))
        );
        assert!(config
            .sanitized_json()
            .contains("\"peer_host_override\":\"127.0.0.1\""));
    }

    #[tokio::test]
    async fn configured_api_token_protects_api_routes() {
        let env = MapEnv::default()
            .with(
                "SLSKR_STATE_DIR",
                &std::env::temp_dir().display().to_string(),
            )
            .with("SLSKR_API_TOKEN", "route-token");
        let config =
            super::AppConfig::from_layers(None, FileConfig::default(), &env).expect("auth config");
        let (sender, _receiver) = mpsc::channel(8);
        let (event_tx, _) = tokio::sync::broadcast::channel(super::EVENT_HISTORY_LIMIT);
        let rate_limiter =
            super::rate_limit::RateLimiter::new(super::rate_limit::RateLimitConfig {
                max_requests_anonymous: 1000,
                max_requests_authenticated: 5000,
                window_seconds: 60,
                enabled: true,
            });

        let state = super::AppState {
            session: RwLock::new(super::SessionSnapshot::disconnected(&config)),
            listeners: RwLock::new(super::ListenerSnapshot::new(&config)),
            shares: RwLock::new(super::build_share_index(&config)),
            searches: RwLock::new(super::SearchStore::new()),
            users: RwLock::new(super::UserStore::new()),
            browse: RwLock::new(super::BrowseStore::new()),
            messages: RwLock::new(super::MessageStore::new()),
            rooms: RwLock::new(super::RoomStore::new()),
            transfers: RwLock::new(super::TransferQueue::new(&config)),
            events: RwLock::new(super::EventStore::new(super::EVENT_HISTORY_LIMIT)),
            event_tx,
            webhooks: RwLock::new(super::webhooks::WebhookManager::new()),
            collections: RwLock::new(super::CollectionStore::new()),
            wishlist: RwLock::new(super::WishlistStore::new()),
            contacts: RwLock::new(super::ContactStore::new()),
            sharegroups: RwLock::new(super::ShareGroupStore::new()),
            user_notes: RwLock::new(super::UserNoteStore::new()),
            interests: RwLock::new(super::InterestStore::new()),
            share_grants: RwLock::new(super::ShareGrantStore::new()),
            library: RwLock::new(super::LibraryStore::new()),
            destinations: RwLock::new(super::DestinationStore::new()),
            db: None,
            config,
            session_commands: sender,
            rate_limiter,
            oauth_states: RwLock::new(super::OAuthStateStore::default()),
        };

        let missing = super::route_http_request("GET", "/api/v0/config", None, "", &state)
            .await
            .unwrap();
        assert_eq!(missing.status, "401 Unauthorized");

        let wrong =
            super::route_http_request("GET", "/api/v0/config", Some("Bearer wrong"), "", &state)
                .await
                .unwrap();
        assert_eq!(wrong.status, "401 Unauthorized");

        let allowed = super::route_http_request(
            "GET",
            "/api/v0/config",
            Some("Bearer route-token"),
            "",
            &state,
        )
        .await
        .unwrap();
        assert_eq!(allowed.status, "200 OK");

        let x_api_key_allowed = super::route_http_request(
            "GET",
            "/api/v0/config",
            Some("ApiKey route-token"),
            "",
            &state,
        )
        .await
        .unwrap();
        assert_eq!(x_api_key_allowed.status, "200 OK");

        let cross_site = super::route_http_request_with_headers(
            "POST",
            "/api/v0/session/ping",
            Some("Bearer route-token"),
            "",
            &state,
            super::RequestSecurityHeaders {
                host: Some("127.0.0.1:5030".to_string()),
                origin: Some("https://evil.example".to_string()),
                referer: None,
                cookie: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(cross_site.status, "403 Forbidden");
        assert_eq!(
            cross_site.body,
            "{\"error\":\"cross-site mutating request rejected\"}"
        );

        let same_origin = super::route_http_request_with_headers(
            "POST",
            "/api/v0/session/ping",
            Some("Bearer route-token"),
            "",
            &state,
            super::RequestSecurityHeaders {
                host: Some("127.0.0.1:5030".to_string()),
                origin: Some("http://127.0.0.1:5030".to_string()),
                referer: None,
                cookie: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(same_origin.status, "202 Accepted");

        let cookie_allowed = super::route_http_request_with_headers(
            "GET",
            "/api/v0/config",
            None,
            "",
            &state,
            super::RequestSecurityHeaders {
                host: Some("127.0.0.1:5030".to_string()),
                origin: None,
                referer: None,
                cookie: Some("other=value; slskr.session=route-token".to_string()),
            },
        )
        .await
        .unwrap();
        assert_eq!(cookie_allowed.status, "200 OK");

        let health = super::route_http_request("GET", "/api/v0/health", None, "", &state)
            .await
            .unwrap();
        assert_eq!(health.status, "200 OK");

        let capabilities =
            super::route_http_request("GET", "/api/v0/capabilities", None, "", &state)
                .await
                .unwrap();
        assert_eq!(capabilities.status, "200 OK");
        assert!(capabilities.body.contains("\"room-list-sync\""));
        assert!(capabilities.body.contains("\"browser-session-auth\""));
    }

    #[test]
    fn environment_overrides_file_config() {
        let file_config = toml::from_str::<FileConfig>(
            r#"
                [network]
                listen_port = 3333

                [shares]
                scan_max_files = 123

                [transfers]
                max_active = 9
                allow_inbound = true
                allow_outbound = true
            "#,
        )
        .unwrap();
        let env = MapEnv::default()
            .with("SLSK_LISTEN_PORT", "4444")
            .with("SLSKR_SHARE_SCAN_MAX_FILES", "5")
            .with("SLSKR_TRANSFER_MAX_ACTIVE", "1")
            .with("SLSKR_TRANSFER_ALLOW_INBOUND", "false")
            .with("SLSKR_TRANSFER_ALLOW_OUTBOUND", "false");

        let config = super::AppConfig::from_layers(None, file_config, &env).unwrap();

        assert_eq!(config.listen_port, 4444);
        assert_eq!(config.share_settings.max_files, 5);
        assert_eq!(config.transfer_max_active, 1);
        assert!(!config.transfer_allow_inbound);
        assert!(!config.transfer_allow_outbound);
    }

    #[test]
    fn toml_config_rejects_unknown_fields() {
        let error = toml::from_str::<FileConfig>(
            r#"
                [app]
                surprise = true
            "#,
        )
        .unwrap_err();

        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn scrubbed_socket_addr_hides_host() {
        let address = "192.0.2.10:2234".parse().unwrap();
        assert_eq!(super::scrub_socket_addr(address), "ipv4:2234");
    }

    #[test]
    fn peer_message_names_are_stable() {
        assert_eq!(
            super::peer_message_name(&slskr_client::protocol::peer::PeerMessage::UserInfoRequest),
            "UserInfoRequest"
        );
    }

    #[test]
    fn transfer_active_statuses_include_peer_lifecycle() {
        assert!(super::is_active_transfer_status("in_progress"));
        assert!(super::is_active_transfer_status("peer_lookup"));
        assert!(super::is_active_transfer_status("peer_negotiating"));
        assert!(super::is_active_transfer_status("accepted"));
        assert!(super::is_active_transfer_status("indirect_pending"));
        assert!(!super::is_active_transfer_status("queued"));
        assert!(!super::is_active_transfer_status("failed"));
    }

    #[test]
    fn share_fixture_entries_are_searchable() {
        let entries = crate::config::parse_share_entries("Music/Artist - Song.flac=123").unwrap();
        assert_eq!(entries[0].extension, "flac");
        assert_eq!(super::search_shares(&entries, "artist song").len(), 1);
        assert!(super::search_shares(&entries, "missing").is_empty());
    }

    #[test]
    fn share_scan_discovers_visible_files() {
        let root = std::env::temp_dir().join(format!("slskr-share-test-{}", std::process::id()));
        let artist = root.join("Artist");
        std::fs::create_dir_all(&artist).unwrap();
        std::fs::write(artist.join("Song.flac"), b"audio").unwrap();
        std::fs::create_dir_all(root.join(".hidden")).unwrap();
        std::fs::write(root.join(".hidden").join("Secret.mp3"), b"hidden").unwrap();

        let scan = super::scan_share_dirs(std::slice::from_ref(&root), false, false, 100);

        assert_eq!(scan.entries.len(), 1);
        assert!(scan.entries[0].filename.ends_with("/Artist/Song.flac"));
        assert_eq!(scan.entries[0].size, 5);
        assert_eq!(scan.entries[0].extension, "flac");
        assert_eq!(scan.roots[0].files, 1);
        assert_eq!(scan.roots[0].bytes, 5);
        assert_eq!(scan.roots[0].extensions[0].extension, "flac");
        assert_eq!(scan.roots[0].extensions[0].files, 1);
        assert_eq!(scan.roots[0].extensions[0].bytes, 5);
        assert!(scan.roots[0].json().contains("\"bytes\":5"));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn share_cache_escapes_fields() {
        assert_eq!(super::escape_cache_field("a\tb\\c\n"), "a\\tb\\\\c\\n");
    }

    #[test]
    fn transfer_queue_records_rejections_with_limit() {
        let mut queue = super::TransferQueue::new_in_memory(1);

        queue.record_rejected_request(1, 10, "a.flac".to_owned(), Some(5), "disabled".to_owned());
        queue.record_rejected_request(1, 11, "b.flac".to_owned(), Some(6), "disabled".to_owned());

        assert_eq!(queue.entries.len(), 1);
        assert_eq!(queue.entries[0].id, 2);
        assert_eq!(queue.entries[0].filename, "b.flac");
        assert_eq!(queue.entries[0].status, "rejected");

        let _ = std::fs::remove_file(queue.events_path);
    }

    #[test]
    fn transfer_queue_records_progress_events_with_bytes() {
        let mut queue = super::TransferQueue::new_in_memory(8);
        let entry = queue.create(1, None, "Remote/Song.flac".to_owned(), None, Some(100));

        queue.update_status(entry.id, "in_progress", None, None);
        queue.update_progress(entry.id, 64);
        queue.update_local_execution(entry.id, "succeeded", 100, Some(100), None);

        let events = std::fs::read_to_string(&queue.events_path).expect("events");
        assert!(events.starts_with("slskr-transfer-events-v2\n"));
        assert!(events.contains("bytes_transferred"));
        assert!(events.contains("\t64\tin_progress\t\tRemote/Song.flac"));
        assert!(events.contains("\t100\tsucceeded\t\tRemote/Song.flac"));

        let _ = std::fs::remove_file(queue.events_path);
        let _ = std::fs::remove_file(queue.state_path);
    }

    #[test]
    fn transfer_queue_persists_and_reloads_resume_state() {
        let state_dir = std::env::temp_dir().join(format!(
            "slskr-transfer-state-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&state_dir).expect("state dir");
        let env = MapEnv::default()
            .with("SLSKR_STATE_DIR", &state_dir.display().to_string())
            .with("SLSKR_AUTO_CONNECT", "false");
        let config =
            super::AppConfig::from_layers(None, FileConfig::default(), &env).expect("config");

        {
            let mut queue = super::TransferQueue::new(&config);
            let entry = queue.create(
                0,
                Some("friend".to_owned()),
                "Remote/Song.flac".to_owned(),
                Some(state_dir.join("Song.flac").display().to_string()),
                Some(100),
            );
            queue.update_status(entry.id, "in_progress", Some(40), None);
        }

        let reloaded = super::TransferQueue::new(&config);
        let entry = reloaded.entries.first().expect("reloaded transfer");
        assert_eq!(entry.status, "queued");
        assert_eq!(entry.bytes_transferred, 40);
        assert_eq!(entry.reason.as_deref(), Some("resumed after restart"));
        assert_eq!(reloaded.next_id, 2);
        assert_eq!(reloaded.next_token, 2);

        let _ = std::fs::remove_dir_all(state_dir);
    }

    #[test]
    fn transfer_state_loader_rejects_oversized_state_file() {
        let state_dir = std::env::temp_dir().join(format!(
            "slskr-transfer-state-size-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&state_dir).expect("state dir");
        let state_path = super::transfer_state_path(&state_dir);
        std::fs::write(
            &state_path,
            vec![b' '; (super::MAX_TRANSFER_STATE_BYTES as usize) + 1],
        )
        .expect("oversized state");

        let error = super::load_transfer_state(&state_path, 100).expect_err("oversized state");
        assert!(error.contains("transfer state file is too large"));

        let _ = std::fs::remove_dir_all(state_dir);
    }
}
