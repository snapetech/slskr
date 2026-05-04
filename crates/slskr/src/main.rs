#![allow(dead_code, unused_imports)]

mod batch;
mod caching;
mod config;
mod logging;
mod rate_limit;
mod utils;
mod storage;
mod routing;
mod websocket;

use std::{
    collections::BTreeMap,
    env, fs,
    net::{SocketAddr, SocketAddrV4},
    path::{Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

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
    version::{
        CLIENT_MAJOR_VERSION, CLIENT_MINOR_VERSION, CLIENT_NAME,
    },
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{mpsc, RwLock},
    time::{self, Duration, Instant},
};

use crate::config::{
    json_bool_option, json_escape, json_option, json_u32_option,
    json_u64_option, json_usize_option, parse_share_entries, AppConfig,
};
use crate::utils::*;

use config::redact_username;

const TRANSFER_PROGRESS_CHUNK_BYTES: usize = 64 * 1024;
const DEFAULT_SEARCH_TTL_SECONDS: u64 = 300;
const EVENT_HISTORY_LIMIT: usize = 500;

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

    fn root_files_json(&self, label: &str, query: Option<&str>) -> Option<String> {
        let root = self.roots.iter().find(|root| root.label == label)?;
        let filter = CatalogFilter::from_query(query);
        let prefix = format!("{label}/").to_ascii_lowercase();
        let mut entries = self
            .entries
            .iter()
            .filter(|entry| entry.filename.to_ascii_lowercase().starts_with(&prefix))
            .filter(|entry| filter.matches(entry))
            .collect::<Vec<_>>();
        entries.sort_by(|left, right| left.filename.cmp(&right.filename));
        let filtered_count = entries.len();
        let total_bytes = entries.iter().map(|entry| entry.size).sum::<u64>();
        let files = entries
            .into_iter()
            .skip(filter.offset)
            .take(filter.limit.unwrap_or(usize::MAX))
            .map(|entry| {
                let path = entry
                    .filename
                    .split_once('/')
                    .map_or(entry.filename.as_str(), |(_, path)| path);
                format!(
                    "{{\"path\":\"{}\",\"virtual_path\":\"{}\",\"size\":{},\"extension\":\"{}\",\"attribute_count\":{}}}",
                    json_escape(path),
                    json_escape(&entry.filename),
                    entry.size,
                    json_escape(&entry.extension),
                    entry.attributes.len()
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        Some(format!(
            "{{\"root\":{},\"files\":[{}],\"count\":{},\"filtered_count\":{},\"total_bytes\":{},\"offset\":{},\"limit\":{},\"updated_at\":{}}}",
            root.json(),
            files,
            root.files,
            filtered_count,
            total_bytes,
            filter.offset,
            json_usize_option(filter.limit),
            self.updated_at
        ))
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
        let mut filter = Self::default();
        for (name, value) in query_params(query.unwrap_or_default()) {
            match name.as_str() {
                "q" => filter.q = non_empty(value.to_ascii_lowercase()),
                "prefix" => filter.prefix = non_empty(value),
                "extension" => {
                    filter.extension =
                        non_empty(value.trim_start_matches('.').to_ascii_lowercase());
                }
                "limit" => filter.limit = value.parse::<usize>().ok(),
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
}

#[derive(Clone, Debug)]
struct SearchRecord {
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
            "{{\"token\":{},\"query\":\"{}\",\"target\":\"{}\",\"target_name\":{},\"status\":\"{}\",\"result_count\":{},\"results\":[{}],\"expires_at\":{},\"created_at\":{},\"updated_at\":{}}}",
            self.token,
            json_escape(&self.query),
            self.target,
            json_option(self.target_name.as_deref()),
            self.status,
            self.results.len(),
            results,
            self.expires_at,
            self.created_at,
            self.updated_at
        )
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

    fn create(
        &mut self,
        query: String,
        target: &'static str,
        target_name: Option<String>,
        results: Vec<FileEntry>,
        ttl_seconds: u64,
    ) -> SearchRecord {
        let now = unix_timestamp();
        let record = SearchRecord {
            token: self.next_token,
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

    fn add_result(&mut self, token: u32, result: SearchResultEntry) -> Option<SearchRecord> {
        let record = self
            .records
            .iter_mut()
            .find(|record| record.token == token)?;
        record.results.push(result);
        record.updated_at = unix_timestamp();
        Some(record.clone())
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
        let mut filter = Self::default();
        for (name, value) in query_params(query.unwrap_or_default()) {
            match name.as_str() {
                "q" => filter.q = non_empty(value.to_ascii_lowercase()),
                "status" => filter.status = non_empty(value),
                "target" => filter.target = non_empty(value),
                "direction" => filter.direction = non_empty(value),
                "username" => filter.username = non_empty(value),
                "joined" => filter.joined = parse_bool_value(&value),
                "kind" => filter.kind = non_empty(value),
                "limit" => filter.limit = value.parse::<usize>().ok(),
                "offset" => filter.offset = value.parse::<usize>().unwrap_or(0),
                _ => {}
            }
        }
        filter
    }
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

    fn summary_json(&self) -> String {
        format!(
            "{{\"total\":{},\"history_limit\":{},\"next_id\":{}}}",
            self.records.len(),
            self.history_limit,
            self.next_id
        )
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
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
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

    fn get(&self, id: u64) -> Option<TransferEntry> {
        self.entries.iter().find(|entry| entry.id == id).cloned()
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

    fn summary_json(&self) -> String {
        format!(
            "{{\"regular_accepts\":{},\"obfuscated_accepts\":{},\"peer_messages\":{},\"obfuscated_peer_messages\":{},\"file_transfers\":{},\"distributed\":{},\"peer_inits\":{},\"pierce_firewalls\":{},\"user_info_requests\":{},\"share_list_requests\":{},\"file_search_requests\":{},\"transfer_rejections\":{},\"errors\":{},\"updated_at\":{}}}",
            self.regular_accepts,
            self.obfuscated_accepts,
            self.peer_messages,
            self.obfuscated_peer_messages,
            self.file_transfers,
            self.distributed,
            self.peer_inits,
            self.pierce_firewalls,
            self.user_info_requests,
            self.share_list_requests,
            self.file_search_requests,
            self.transfer_rejections,
            self.errors,
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

#[derive(Debug, Deserialize)]
struct BrowseResponseBody {
    username: Option<String>,
    filename: Option<String>,
    size: Option<u64>,
    extension: Option<String>,
    complete: Option<bool>,
    entries: Option<Vec<BrowseResponseBodyEntry>>,
}

#[derive(Debug, Deserialize)]
struct BrowseResponseBodyEntry {
    filename: String,
    size: u64,
    extension: Option<String>,
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
            .collect::<Vec<_>>();
        let filtered_count = records.len();
        let records = records
            .into_iter()
            .skip(filter.offset)
            .take(filter.limit.unwrap_or(usize::MAX))
            .map(RoomRecord::json)
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
    session_commands: mpsc::Sender<SessionCommand>,
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

impl SearchDispatchTarget {
    fn from_body(body: &str) -> Result<Self, String> {
        match extract_json_string_field(body, "target")
            .unwrap_or_else(|| "global".to_owned())
            .as_str()
        {
            "global" => Ok(Self::Global),
            "wishlist" => Ok(Self::Wishlist),
            "user" => extract_json_string_field(body, "username")
                .filter(|value| !value.trim().is_empty())
                .map(Self::User)
                .ok_or_else(|| "username is required for user search".to_owned()),
            "room" => extract_json_string_field(body, "room")
                .filter(|value| !value.trim().is_empty())
                .map(Self::Room)
                .ok_or_else(|| "room is required for room search".to_owned()),
            _ => Err("target must be global, user, room, or wishlist".to_owned()),
        }
    }

    const fn label(&self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::User(_) => "user",
            Self::Room(_) => "room",
            Self::Wishlist => "wishlist",
        }
    }

    fn name(&self) -> Option<&str> {
        match self {
            Self::User(username) | Self::Room(username) => Some(username),
            Self::Global | Self::Wishlist => None,
        }
    }
}

use routing::HttpResponse;

async fn route_http_request_with_headers(
    method: &str,
    path: &str,
    authorization: Option<&str>,
    body: &str,
    state: &AppState,
    headers: RequestSecurityHeaders<'_>,
) -> Result<HttpResponse, String> {
    let route = routing::parse_route(method, path);

    if let Err(err) = routing::check_route_auth(&state.config, method, route.normalized_path, authorization, &headers) {
        return Ok(if err == "forbidden" {
            routing::forbidden_response("cross-site mutating request rejected")
        } else {
            routing::unauthorized_response()
        });
    }

    match (method, route.normalized_path) {
        ("GET", "/") => Ok(index_html_response()),
        ("GET", "/api/health") => Ok(health_response()),
        ("GET", "/api/version") => Ok(version_response()),
        ("GET", "/api/capabilities") => Ok(capabilities_response()),
        ("POST", "/api/capabilities/negotiate") => Ok(capabilities_negotiate_response(body)),
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
        ("GET", "/api/shares") => {
            let shares = state.shares.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: shares.json(),
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
            let root = shares.roots.iter()
                .find(|r| r.label == folder);
            
            if root.is_none() {
                drop(shares);
                return Ok(routing::not_found_response());
            }
            
            let root = root.unwrap();
            
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
        ("GET", path) if search_token_path(route.normalized_path, "").is_some() => {
            let token = search_token_path(route.normalized_path, "").unwrap();
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
            let query = match extract_json_string_field(body, "query") {
                Some(q) => q,
                None => return Ok(routing::bad_request_response("query is required")),
            };
            
            let target_str = extract_json_string_field(body, "target").unwrap_or_else(|| "global".to_string());
            let username_opt = extract_json_string_field(body, "username");
            let room_opt = extract_json_string_field(body, "room");
            
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
            
            let static_target: &'static str = Box::leak(target_str.clone().into_boxed_str());
            let record = searches.create(query.clone(), static_target, target_name, matching_results, 300);
            let token = record.token;
            drop(searches);
            
            record_event(state, "search.started", format!("{}", token), None).await;
            
            let dispatch_target = match target_str.as_str() {
                "user" => SearchDispatchTarget::User(username_opt.unwrap()),
                "room" => SearchDispatchTarget::Room(room_opt.unwrap()),
                "wishlist" => SearchDispatchTarget::Wishlist,
                _ => SearchDispatchTarget::Global,
            };
            
            send_session_command(state, SessionCommand::Search { token, query, target: dispatch_target }).await.ok();
            
            Ok(routing::created_response(record.json()))
        }
        
        ("POST", path) if search_token_path(route.normalized_path, "/complete").is_some() => {
            let token = search_token_path(route.normalized_path, "/complete").unwrap();
            let mut searches = state.searches.write().await;
            if let Some(record) = searches.complete(token) {
                let body_json = record.json();
                drop(searches);
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
                    extension: filename.as_ref().and_then(|f| f.split('.').last().map(|s| s.to_string())).unwrap_or_default(),
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
            let filename = match extract_json_string_field(body, "filename") {
                Some(f) => f,
                None => return Ok(routing::bad_request_response("filename is required")),
            };
            
            let direction = extract_json_u32_field(body, "direction").unwrap_or(0);
            let peer_username = extract_json_string_field(body, "peer_username");
            let local_path = extract_json_string_field(body, "local_path");
            let size = extract_json_u64_field(body, "size");
            
            let mut transfers = state.transfers.write().await;
            let entry = transfers.create(direction, peer_username, filename, local_path, size);
            drop(transfers);
            
            Ok(routing::created_response(entry.json()))
        }
        
        ("POST", path) if transfer_action_path(route.normalized_path).is_some() => {
            if let Some((id, action)) = transfer_action_path(route.normalized_path) {
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
                            // If local_path is present, try to read metadata
                            if let Some(ref local_path) = entry.local_path {
                                match fs::metadata(local_path) {
                                    Ok(metadata) if metadata.is_file() => {
                                        entry.size = Some(metadata.len());
                                        entry.bytes_transferred = metadata.len();
                                        entry.status = "succeeded".to_owned();
                                    }
                                    _ => {
                                        entry.status = "failed".to_owned();
                                        entry.reason = Some("local path metadata failed".to_string());
                                        entry.bytes_transferred = 0;
                                    }
                                }
                            } else {
                                entry.status = "in_progress".to_owned();
                            }
                            
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
                        entry.status = status_str;
                        entry.updated_at = unix_timestamp();
                        let json_response = entry.json();
                        drop(transfers);
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
            let record = messages.add(username.clone(), Box::leak("outbound".to_string().into_boxed_str()), message_body.clone());
            drop(messages);
            
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
            let record = messages.add(username, Box::leak("inbound".to_string().into_boxed_str()), message_body);
            drop(messages);
            
            record_event(state, "message.received", "messages", Some(format!("id={}", record.id))).await;
            
            Ok(routing::created_response(record.json()))
        }
        
        ("POST", path) if message_ack_path(route.normalized_path).is_some() => {
            let id = message_ack_path(route.normalized_path).unwrap();
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
         
         ("GET", path) if messages_user_path(route.normalized_path).is_some() => {
            let username = messages_user_path(route.normalized_path).unwrap();
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
        
        ("POST", path) if room_join_path(route.normalized_path).is_some() => {
            let room_name = room_join_path(route.normalized_path).unwrap();
            let mut rooms = state.rooms.write().await;
            let record = rooms.join(room_name.to_string());
            drop(rooms);
            
            send_session_command(state, SessionCommand::JoinRoom(room_name.to_string())).await.ok();
            
            Ok(routing::created_response(record.json()))
        }
        
        ("DELETE", path) if room_join_path(route.normalized_path).is_some() => {
            let room_name = room_join_path(route.normalized_path).unwrap();
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
        
        ("POST", path) if room_messages_path(route.normalized_path).is_some() => {
            let room_name = room_messages_path(route.normalized_path).unwrap();
            let username = extract_json_string_field(body, "username").unwrap_or_else(|| "unknown".to_string());
            let message_body = extract_json_string_field(body, "body").unwrap_or_default();
            
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
        
        ("DELETE", path) if user_watch_path(route.normalized_path).is_some() => {
            let username = user_watch_path(route.normalized_path).unwrap();
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
        
        ("POST", path) if user_stats_request_path(route.normalized_path).is_some() => {
            let username = user_stats_request_path(route.normalized_path).unwrap();
            send_session_command(state, SessionCommand::RequestUserStats(username.to_string())).await.ok();
            Ok(routing::accepted_response(format!("{{\"username\":\"{}\"}}", json_escape(username))))
        }
        
        ("POST", path) if user_browse_request_path(route.normalized_path).is_some() => {
            let username = user_browse_request_path(route.normalized_path).unwrap();
            
            let mut browse = state.browse.write().await;
            let record = browse.request(username.to_string());
            drop(browse);
            
            send_session_command(state, SessionCommand::BrowseUser(username.to_string())).await.ok();
            
            Ok(routing::accepted_response(record.json()))
        }
        
        ("POST", path) if user_browse_folder_path(route.normalized_path).is_some() => {
            let username = user_browse_folder_path(route.normalized_path).unwrap();
            let folder = extract_json_string_field(body, "folder").unwrap_or_default();
            
            let mut browse = state.browse.write().await;
            let record = browse.request_folder(username.to_string(), folder.clone());
            drop(browse);
            
            send_session_command(state, SessionCommand::BrowseFolder { username: username.to_string(), folder }).await.ok();
            
            Ok(routing::accepted_response(record.json()))
        }
        
        ("POST", path) if user_browse_fail_path(route.normalized_path).is_some() => {
            let username = user_browse_fail_path(route.normalized_path).unwrap();
            let reason = extract_json_string_field(body, "reason").unwrap_or_default();
            
            let mut browse = state.browse.write().await;
            if let Some(r) = browse.records.iter_mut().find(|b| b.username == username) {
                r.status = Box::leak("failed".to_string().into_boxed_str());
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
            
            // Parse entries array - find the array in the JSON and manually parse objects
            let mut entries = Vec::new();
            if let Some(entries_start) = body.find("\"entries\":[") {
                let after_bracket = entries_start + 11; // len("\"entries\":[")
                if let Some(array_end) = body[after_bracket..].find(']') {
                    let array_content = &body[after_bracket..after_bracket + array_end];
                    
                    // Split by }, but be careful with nested structures
                    let mut current_obj_start = 0;
                    let mut brace_depth = 0;
                    for (i, ch) in array_content.chars().enumerate() {
                        if ch == '{' {
                            if brace_depth == 0 {
                                current_obj_start = i;
                            }
                            brace_depth += 1;
                        } else if ch == '}' {
                            brace_depth -= 1;
                            if brace_depth == 0 {
                                let obj_str = &array_content[current_obj_start..=i];
                                if let Some(filename) = extract_json_string_field(obj_str, "filename") {
                                    let size = extract_json_u64_field(obj_str, "size").unwrap_or(0);
                                    let extension = extract_json_string_field(obj_str, "extension")
                                        .unwrap_or_else(|| {
                                            filename.split('.').last().unwrap_or("").to_string()
                                        });
                                    entries.push(BrowseEntry {
                                        filename,
                                        size,
                                        extension,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            
            // Fallback for single entry format (backward compatibility)
            if entries.is_empty() {
                if let Some(filename) = extract_json_string_field(body, "filename") {
                    let size = extract_json_u64_field(body, "size").unwrap_or(0);
                    let extension = extract_json_string_field(body, "extension")
                        .unwrap_or_else(|| {
                            filename.split('.').last().unwrap_or("").to_string()
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
                    Ok(routing::not_found_response())
                }
            } else {
                Ok(routing::not_found_response())
            }
        }
        _ => Ok(routing::not_found_response()),
    }
}

fn index_html_response() -> HttpResponse {
    HttpResponse {
        status: "200 OK",
        content_type: "text/html; charset=utf-8",
        body: index_html(),
    }
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
            "{{\"name\":\"{}\",\"major\":{},\"minor\":{}}}",
            CLIENT_NAME, CLIENT_MAJOR_VERSION, CLIENT_MINOR_VERSION
        ),
    }
}

fn capabilities_response() -> HttpResponse {
    HttpResponse {
        status: "200 OK",
        content_type: "application/json",
        body: r#"{"api_version":"v0","client_version":"0.1","supports":["login","peers","shares","searches","transfers","users","messages","rooms","room-list-sync","browser-session-auth"]}"#.to_owned(),
    }
}

fn capabilities_negotiate_response(body: &str) -> HttpResponse {
    let server_capabilities = vec!["shares", "telemetry"];
    
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
    
    let accepted_json = accepted.iter()
        .map(|s| format!("\"{}\"", s))
        .collect::<Vec<_>>()
        .join(",");
    let unsupported_json = unsupported.iter()
        .map(|s| format!("\"{}\"", s))
        .collect::<Vec<_>>()
        .join(",");
    let server_caps_json = server_capabilities.iter()
        .map(|s| format!("\"{}\"", s))
        .collect::<Vec<_>>()
        .join(",");
    
    let response_body = format!(
        "{{\"accepted\":[{}],\"unsupported\":[{}],\"server_capabilities\":[{}]}}",
        accepted_json,
        unsupported_json,
        server_caps_json
    );
    
    HttpResponse {
        status: "200 OK",
        content_type: "application/json",
        body: response_body,
    }
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
    let key = format!("\"{}\"", field);
    let after_key = json_field_after_key(body, &key)?;
    let after_colon = after_key.trim_start().strip_prefix(':')?.trim_start();
    let end = after_colon.find([',', '}']).unwrap_or(after_colon.len());
    after_colon[..end].trim().parse().ok()
}

fn extract_json_bool_field(body: &str, field: &str) -> Option<bool> {
    let key = format!("\"{}\"", field);
    let after_key = json_field_after_key(body, &key)?;
    let after_colon = after_key.trim_start().strip_prefix(':')?.trim_start();
    let end = after_colon.find([',', '}']).unwrap_or(after_colon.len());
    let value = after_colon[..end].trim();
    match value {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn extract_json_u64_field(body: &str, field: &str) -> Option<u64> {
    let key = format!("\"{}\"", field);
    let after_key = json_field_after_key(body, &key)?;
    let after_colon = after_key.trim_start().strip_prefix(':')?.trim_start();
    let end = after_colon.find([',', '}']).unwrap_or(after_colon.len());
    after_colon[..end].trim().parse().ok()
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
    let state = Arc::new(AppState {
        session: RwLock::new(SessionSnapshot::disconnected(&config)),
        listeners: RwLock::new(ListenerSnapshot::new(&config)),
        shares: RwLock::new(share_index),
        searches: RwLock::new(SearchStore::new()),
        users: RwLock::new(UserStore::new()),
        browse: RwLock::new(BrowseStore::new()),
        messages: RwLock::new(MessageStore::new()),
        rooms: RwLock::new(RoomStore::new()),
        transfers: RwLock::new(TransferQueue::new(&config)),
        events: RwLock::new(EventStore::new(EVENT_HISTORY_LIMIT)),
        config,
        session_commands,
    });
    spawn_session_manager(Arc::clone(&state), session_receiver);
    spawn_configured_listeners(Arc::clone(&state));

    if state.config.auto_connect {
        send_session_command(&state, SessionCommand::Connect).await?;
    }

    let listener = TcpListener::bind(address)
        .await
        .map_err(|error| format!("failed to bind {address}: {error}"))?;
    println!("slskr listening on http://{address}");

    loop {
        let (stream, _) = listener
            .accept()
            .await
            .map_err(|error| format!("accept failed: {error}"))?;
        let state = Arc::clone(&state);
        tokio::spawn(async move {
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
                let receive_result =
                    time::timeout(Duration::from_millis(250), active_session.receive()).await;
                match receive_result {
                    Ok(Ok(message)) => {
                        project_server_message(&state, active_session, &message).await;
                        update_session(&state, |snapshot| {
                            snapshot.state = "connected";
                            snapshot.server_messages_seen += 1;
                            snapshot.last_server_message = Some(server_message_name(&message).to_string());
                        })
                        .await;
                    }
                    Ok(Err(error)) => {
                        session = None;
                        update_session(&state, |snapshot| {
                            snapshot.state = "error";
                            snapshot.last_error = Some(format!("server receive failed: {error}"));
                            snapshot.supporter = None;
                            snapshot.connected_at = None;
                        })
                        .await;
                        reconnect_requested = state.config.reconnect;
                    }
                    Err(_) => {}
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
    let metadata = fs::metadata(&local_path).ok()?;
    metadata.is_file().then_some(SharedLocalFile {
        local_path,
        size: metadata.len(),
    })
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
    let bytes = fs::read(local_path).map_err(|error| format!("local file read failed: {error}"))?;
    let size = u64::try_from(bytes.len()).map_err(|_| "local file is too large".to_owned())?;
    let offset = upload_file_with_progress(state, transfer, connection, &bytes, size).await?;
    Ok((size.saturating_sub(offset), size))
}

async fn upload_file_with_progress(
    state: &AppState,
    transfer: &TransferEntry,
    connection: &mut slskr_client::file_transfer::FileTransferConnection<TcpStream>,
    bytes: &[u8],
    size: u64,
) -> Result<u64, String> {
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
    if start > bytes.len() {
        return Err(format!(
            "transfer offset {offset} exceeds local file size {size}"
        ));
    }

    let mut sent = 0_u64;
    for chunk in bytes[start..].chunks(TRANSFER_PROGRESS_CHUNK_BYTES) {
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
    let path = PathBuf::from(local_path);
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .map_err(|error| format!("download directory create failed: {error}"))?;
    }
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
            SocketAddr::V4(SocketAddrV4::new(address.ip, port)),
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
    let stream = time::timeout(
        state.config.peer_response_timeout,
        TcpStream::connect(SocketAddr::V4(SocketAddrV4::new(
            address.ip,
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
    let stream = time::timeout(
        state.config.peer_response_timeout,
        TcpStream::connect(SocketAddr::V4(SocketAddrV4::new(response.ip, port))),
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
    let stream = time::timeout(
        state.config.peer_response_timeout,
        TcpStream::connect(SocketAddr::V4(SocketAddrV4::new(response.ip, port))),
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
    if address.obfuscation_type == ROTATED_OBFUSCATION_TYPE && address.obfuscated_port != 0 {
        match browse_obfuscated_peer(
            SocketAddr::V4(SocketAddrV4::new(address.ip, address.obfuscated_port)),
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
            SocketAddr::V4(SocketAddrV4::new(address.ip, port)),
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
    if address.obfuscation_type == ROTATED_OBFUSCATION_TYPE && address.obfuscated_port != 0 {
        match send_obfuscated_peer_message_request(
            SocketAddr::V4(SocketAddrV4::new(address.ip, address.obfuscated_port)),
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
            SocketAddr::V4(SocketAddrV4::new(address.ip, port)),
            username,
            message,
            state.config.peer_response_timeout,
        )
        .await;
    }

    Err(obfuscated_error.unwrap_or_else(|| "peer did not advertise a peer-message port".to_owned()))
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
    events.record(kind, resource, detail);
}

async fn handle_http_connection(mut stream: TcpStream, state: Arc<AppState>) -> Result<(), String> {
    let request_timer = logging::start_timer();
    let remote_addr = stream.peer_addr().ok();
    
    let mut buffer = [0_u8; 4096];
    let bytes_read = stream
        .read(&mut buffer)
        .await
        .map_err(|error| format!("read failed: {error}"))?;
    let request = std::str::from_utf8(&buffer[..bytes_read])
        .map_err(|error| format!("request was not UTF-8: {error}"))?;
    let (method, path) = parse_route(request);
    let (normalized_path, query) = split_request_target(path);
    let authorization = authorization_header(request);
    let headers = RequestSecurityHeaders::from_request(request);
    let body = request_body(request);
    
    // Log request
    let req_log = logging::HttpRequestLog {
        method: method.to_string(),
        path: normalized_path.to_string(),
        query: query.map(|q| q.to_string()),
        remote_addr: remote_addr.map(|addr| addr.to_string()),
        timestamp: logging::format_timestamp(),
    };
    
    let response =
        route_http_request_with_headers(method, path, authorization, body, &state, headers).await?;

    let response_text = format!(
        "HTTP/1.1 {}\r\ncontent-type: {}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
        response.status,
        response.content_type,
        response.body.len(),
        response.body
    );
    
    // Log response
    let resp_log = logging::HttpResponseLog {
        status_code: logging::status_code_from_string(response.status),
        content_length: response.body.len(),
        duration_ms: logging::elapsed_ms(request_timer),
        error: None,
    };
    
    let trans_log = logging::HttpTransactionLog {
        request: req_log,
        response: resp_log,
    };
    let log_config = logging::LogConfig::from_env();
    logging::log_transaction(&log_config, &trans_log);
    
    stream
        .write_all(response_text.as_bytes())
        .await
        .map_err(|error| format!("write failed: {error}"))?;
    Ok(())
}



pub fn index_html() -> String {
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
      document.cookie = `slskr.session=${encoded}; Path=/; SameSite=Strict`;
    };
    const clearSessionCookie = () => {
      document.cookie = "slskr.session=; Path=/; SameSite=Strict; Max-Age=0";
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
        path::PathBuf,
        sync::Arc,
        time::{SystemTime, UNIX_EPOCH},
    };

    use slskr_client::protocol::peer::{FileEntry, FileSearchResponse};
    use tokio::sync::{mpsc, RwLock};

    use crate::utils::{normalize_api_path, parse_route, percent_decode, query_params, split_request_target};
    use crate::config::{ConfigEnv, FileConfig, json_escape, redact_username};

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
        let state = Arc::new(super::AppState {
            session: RwLock::new(super::SessionSnapshot::disconnected(&config)),
            listeners: RwLock::new(super::ListenerSnapshot::new(&config)),
            shares: RwLock::new(share_index),
            searches: RwLock::new(super::SearchStore::new()),
            users: RwLock::new(super::UserStore::new()),
            browse: RwLock::new(super::BrowseStore::new()),
            messages: RwLock::new(super::MessageStore::new()),
            rooms: RwLock::new(super::RoomStore::new()),
            transfers: RwLock::new(super::TransferQueue::new(&config)),
            events: RwLock::new(super::EventStore::new(super::EVENT_HISTORY_LIMIT)),
            config,
            session_commands: sender,
        });
        (state, receiver)
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
                !response.body.contains("secret"),
                "{path}: {}",
                response.body
            );
        }
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
    async fn root_dashboard_exposes_core_controls() {
        let (state, _receiver) = test_state();

        let response = super::route_http_request("GET", "/", None, "", &state)
            .await
            .expect("dashboard response");

        assert_eq!(response.status, "200 OK");
        assert_eq!(response.content_type, "text/html; charset=utf-8");
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
        let record = store.create(
            "remote".to_owned(),
            "global",
            None,
            Vec::new(),
            super::DEFAULT_SEARCH_TTL_SECONDS,
        );
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
            "{\"direction\":1,\"filename\":\"Remote/Song.flac\",\"size\":100}",
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
    async fn transfer_start_executes_local_path_metadata() {
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
        assert_eq!(created.status, "201 Created");

        let started =
            super::route_http_request("POST", "/api/v0/transfers/1/start", None, "", &state)
                .await
                .expect("start transfer");

        assert_eq!(started.status, "200 OK");
        assert!(started.body.contains("\"status\":\"succeeded\""));
        assert!(started.body.contains("\"bytes_transferred\":4"));
        assert!(started.body.contains("\"size\":4"));
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn transfer_start_fails_missing_local_path() {
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
        assert_eq!(created.status, "201 Created");

        let started =
            super::route_http_request("POST", "/api/v0/transfers/1/start", None, "", &state)
                .await
                .expect("start transfer");

        assert_eq!(started.status, "200 OK");
        assert!(started.body.contains("\"status\":\"failed\""));
        assert!(started.body.contains("\"bytes_transferred\":0"));
        assert!(started.body.contains("local path metadata failed"));
    }

    #[tokio::test]
    async fn transfer_start_with_peer_requests_peer_address() {
        let (state, mut receiver) = test_state();
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
        let record = transfers.get(1).expect("transfer");
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
        let record = transfers.get(1).expect("transfer");
        assert_eq!(record.status, "succeeded");
        assert_eq!(record.bytes_transferred, 3);
        assert_eq!(record.size, Some(4));
        assert_eq!(record.reason, None);
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn peer_address_response_downloads_accepted_file_transfer_with_resume() {
        let (state, _receiver) = test_state();
        let path = std::env::temp_dir().join(format!(
            "slskr-transfer-f-{}-download.bin",
            std::process::id()
        ));
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
        let record = transfers.get(1).expect("transfer");
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
        let record = transfers.get(1).expect("transfer");
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
        let record = transfers.get(1).expect("transfer");
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
            let entries = super::parse_share_entries("Remote/Indirect.flac=55").expect("entries");
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
        let record = transfers.get(1).expect("transfer");
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
        let rejected = transfers.get(2).expect("rejection");
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
        let rejected = transfers.get(1).expect("rejection");
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
            "/api/v0/users/friend/stats/request",
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
            super::route_http_request("DELETE", "/api/v0/users/friend/watch", None, "", &state)
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
            "{\"username\":\"friend\",\"complete\":false,\"entries\":[{\"filename\":\"Remote/Album/Song.flac\",\"size\":123}]}",
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
        let entries = super::parse_share_entries("Music/Artist - Song.flac=123;Loose.mp3=7")
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
        let entries = super::parse_share_entries(
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
            let entries = super::parse_share_entries("Remote/Song.flac=321").expect("entries");
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
                super::parse_share_entries("Remote/Album/Song.flac=321").expect("entries");
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
        assert_eq!(response.body, "{\"error\":\"query is required\"}");
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
    fn extracts_simple_json_string_fields() {
        assert_eq!(
            super::extract_json_string_field(r#"{"query":"artist \"song\""}"#, "query"),
            Some("artist \"song\"".to_owned())
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
            config,
            session_commands: sender,
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

        let cross_site = super::route_http_request_with_headers(
            "POST",
            "/api/v0/session/ping",
            Some("Bearer route-token"),
            "",
            &state,
            super::RequestSecurityHeaders {
                host: Some("127.0.0.1:5030"),
                origin: Some("https://evil.example"),
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
                host: Some("127.0.0.1:5030"),
                origin: Some("http://127.0.0.1:5030"),
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
                host: Some("127.0.0.1:5030"),
                origin: None,
                referer: None,
                cookie: Some("other=value; slskr.session=route-token"),
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
        let entries = super::parse_share_entries("Music/Artist - Song.flac=123").unwrap();
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
        let entry = reloaded.get(1).expect("reloaded transfer");
        assert_eq!(entry.status, "queued");
        assert_eq!(entry.bytes_transferred, 40);
        assert_eq!(entry.reason.as_deref(), Some("resumed after restart"));
        assert_eq!(reloaded.next_id, 2);
        assert_eq!(reloaded.next_token, 2);

        let _ = std::fs::remove_dir_all(state_dir);
    }
}
