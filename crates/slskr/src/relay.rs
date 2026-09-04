//! State and credential primitives for the relay controller/agent protocol.
//!
//! The HTTP controller routes are only safe when a request is tied to the
//! registered agent connection that was issued the request token.  Keep that
//! state separate from the compatibility-facing `RelayState` fields so the
//! latter can continue to be persisted as the small runtime projection it has
//! always been.

#![allow(clippy::too_many_arguments)]

use std::collections::BTreeMap;
use std::fs;
use std::net::IpAddr;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use aes::Aes256;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use cbc::cipher::{block_padding::Pkcs7, BlockModeEncrypt, KeyIvInit};
use ring::{hmac, pbkdf2};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx_core::{query::query, row::Row};
use sqlx_sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

use crate::config::{ControllerProfile, RelaySettings};

const CHALLENGE_TTL_SECONDS: u64 = 10;
const REQUEST_TTL_SECONDS: u64 = 5 * 60;
const DOWNLOAD_TTL_SECONDS: u64 = 10 * 60;
const MAX_RELAY_PENDING_REQUESTS: usize = 4_096;
pub(crate) const MAX_RELAY_SHARE_ENTRIES: usize = 131_072;
const MAX_RELAY_SHARE_MANIFEST_BYTES: u64 = 64 * 1024 * 1024;
const MAX_RELAY_SHARE_UPLOAD_RECORDS: usize = 4_096;
const MAX_RELAY_SHARE_FILENAME_BYTES: usize = 16 * 1024;
const MAX_MULTIPART_PARTS: usize = 16;
const MAX_MULTIPART_HEADER_BYTES: usize = 64 * 1024;
const MAX_MULTIPART_PARAMETER_BYTES: usize = 16 * 1024;
pub(crate) const MAX_RELAY_SHARE_METADATA_BYTES: usize = 64 * 1024 * 1024;
pub(crate) const HUB_OUTBOUND_QUEUE_CAPACITY: usize = 64;
const PBKDF2_ITERATIONS: NonZeroU32 = match NonZeroU32::new(1_000) {
    Some(value) => value,
    None => unreachable!(),
};

type Aes256CbcEnc = cbc::Encryptor<Aes256>;
const BASE62_ALPHABET: &[u8; 62] =
    b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CredentialScheme {
    LegacyAesBase62,
    NativeHmacBase64,
}

pub(crate) fn credential_scheme(target: ControllerProfile) -> CredentialScheme {
    match target {
        ControllerProfile::Legacy => CredentialScheme::LegacyAesBase62,
        ControllerProfile::Native => CredentialScheme::NativeHmacBase64,
    }
}

/// SignalR is a process-local transport in the oracle.  Keep the equivalent
/// connection senders outside the persisted compatibility projection: they
/// are live sockets, not state that can be serialized or restored.
static HUB_CONNECTIONS: OnceLock<Mutex<BTreeMap<String, mpsc::Sender<String>>>> = OnceLock::new();

/// A controller request for an agent file is completed by a later multipart
/// HTTP request.  The sender is deliberately process-local: a live stream
/// waiter cannot be persisted or reconstructed after a restart.
type FileUploadWaiters = Mutex<BTreeMap<String, oneshot::Sender<Result<UploadedFile, String>>>>;
static FILE_UPLOAD_WAITERS: OnceLock<FileUploadWaiters> = OnceLock::new();

/// File-info and stream failures are callbacks on the same authenticated hub
/// connection as the successful upload response.  Keep those waiters process
/// local for the same reason as the upload stream waiters.
type FileInfoWaiters = Mutex<BTreeMap<String, oneshot::Sender<Result<FileInfo, String>>>>;
static FILE_INFO_WAITERS: OnceLock<FileInfoWaiters> = OnceLock::new();

/// Share-manifest updates are read/modify/write operations.  Serialize them
/// so concurrent agent uploads cannot overwrite one another's records.
static SHARE_MANIFEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn hub_connections() -> &'static Mutex<BTreeMap<String, mpsc::Sender<String>>> {
    HUB_CONNECTIONS.get_or_init(|| Mutex::new(BTreeMap::new()))
}

fn file_upload_waiters() -> &'static FileUploadWaiters {
    FILE_UPLOAD_WAITERS.get_or_init(|| Mutex::new(BTreeMap::new()))
}

fn file_info_waiters() -> &'static FileInfoWaiters {
    FILE_INFO_WAITERS.get_or_init(|| Mutex::new(BTreeMap::new()))
}

fn share_manifest_lock() -> &'static Mutex<()> {
    SHARE_MANIFEST_LOCK.get_or_init(|| Mutex::new(()))
}

pub(crate) fn register_hub_connection(connection_id: String, sender: mpsc::Sender<String>) {
    if let Ok(mut connections) = hub_connections().lock() {
        connections.insert(connection_id, sender);
    }
}

pub(crate) fn unregister_hub_connection(connection_id: &str) {
    if let Ok(mut connections) = hub_connections().lock() {
        connections.remove(connection_id);
    }
}

/// Push a SignalR invocation to a connected agent. A false return means the
/// agent is not connected, its writer queue is closed, or its bounded queue is
/// full. Dropping a message under backpressure is preferable to allowing a
/// stalled relay agent to grow controller memory without a limit.
pub(crate) fn send_hub_invocation(
    runtime: &RuntimeState,
    agent_name: &str,
    target: &str,
    arguments: Vec<Value>,
) -> bool {
    let Some(connection_id) = runtime.connection_for_agent(agent_name) else {
        return false;
    };
    let Ok(mut connections) = hub_connections().lock() else {
        return false;
    };
    let Some(sender) = connections.get(connection_id).cloned() else {
        return false;
    };
    let message = serde_json::json!({
        "type": 1,
        "target": target,
        "arguments": arguments,
    })
    .to_string();
    match sender.try_send(message) {
        Ok(()) => true,
        Err(mpsc::error::TrySendError::Full(_)) => false,
        Err(mpsc::error::TrySendError::Closed(_)) => {
            connections.remove(connection_id);
            false
        }
    }
}

#[derive(Debug)]
pub(crate) struct UploadedFile {
    pub filename: String,
    pub file: fs::File,
    pub path: PathBuf,
    pub length: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FileInfo {
    pub exists: bool,
    pub length: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CompletedShareUpload {
    pub agent_name: String,
    pub share_count: usize,
    pub shares: Vec<RemoteShare>,
    pub database_path: PathBuf,
    pub completed_at: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RemoteShare {
    pub filename: String,
    pub size: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct PersistedShareUpload {
    token: String,
    agent_name: String,
    shares: Vec<RemoteShare>,
    database_path: PathBuf,
    completed_at: u64,
}

const OLD_SHARE_DATABASE_TABLES: &[&str] = &[
    "version",
    "scans",
    "directories",
    "filenames",
    "filenames_data",
    "filenames_idx",
    "filenames_content",
    "filenames_docsize",
    "filenames_config",
    "files",
];

const CURRENT_SHARE_DATABASE_TABLES: &[&str] = &[
    "version",
    "scans",
    "directories",
    "filenames",
    "filenames_data",
    "filenames_idx",
    "filenames_content",
    "filenames_docsize",
    "filenames_config",
    "files",
    "content_items",
];

/// Write the SQLite share repository uploaded by a relay agent.  The two
/// frozen profiles intentionally have different repository schemas: the legacy
/// profile uses the ten-table repository, while the native profile adds
/// moderation columns and the content-items table.
pub(crate) async fn write_share_database(
    path: &Path,
    target: ControllerProfile,
    shares: &[RemoteShare],
) -> Result<(), String> {
    validate_share_entries(shares)?;
    validate_share_database_path(path)?;
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .map_err(|error| format!("relay share database open failed: {error}"))?;
    let result = async {
        let statements = [
            "CREATE TABLE version (a INTEGER PRIMARY KEY)",
            "CREATE TABLE scans (timestamp INTEGER PRIMARY KEY, options TEXT NOT NULL, end INTEGER DEFAULT NULL, suspect INTEGER DEFAULT 0)",
            "CREATE TABLE directories (name TEXT PRIMARY KEY, timestamp INTEGER NOT NULL)",
            "CREATE VIRTUAL TABLE filenames USING fts5(maskedFilename)",
        ];
        for statement in statements {
            query(statement)
                .execute(&pool)
                .await
                .map_err(|error| format!("relay share database schema failed: {error}"))?;
        }
        let files_sql = match target {
            ControllerProfile::Legacy => {
                "CREATE TABLE files (maskedFilename TEXT PRIMARY KEY, originalFilename TEXT NOT NULL, size BIGINT NOT NULL, touchedAt TEXT NOT NULL, code INTEGER DEFAULT 1 NOT NULL, extension TEXT, attributeJson TEXT NOT NULL, timestamp INTEGER NOT NULL)"
            }
            ControllerProfile::Native => {
                "CREATE TABLE files (maskedFilename TEXT PRIMARY KEY, originalFilename TEXT NOT NULL, size BIGINT NOT NULL, touchedAt TEXT NOT NULL, code INTEGER DEFAULT 1 NOT NULL, extension TEXT, attributeJson TEXT NOT NULL, timestamp INTEGER NOT NULL, isBlocked INTEGER DEFAULT 0 NOT NULL, isQuarantined INTEGER DEFAULT 0 NOT NULL, moderationReason TEXT)"
            }
        };
        query(files_sql)
            .execute(&pool)
            .await
            .map_err(|error| format!("relay share database file schema failed: {error}"))?;
        if target == ControllerProfile::Native {
            query(
                "CREATE TABLE content_items (contentId TEXT PRIMARY KEY, domain TEXT NOT NULL, workId TEXT, maskedFilename TEXT NOT NULL, isAdvertisable INTEGER DEFAULT 0 NOT NULL, moderationReason TEXT, checkedAt INTEGER NOT NULL, FOREIGN KEY(maskedFilename) REFERENCES files(maskedFilename) ON DELETE CASCADE)",
            )
            .execute(&pool)
            .await
            .map_err(|error| format!("relay share database content schema failed: {error}"))?;
            query("CREATE INDEX idx_content_items_filename ON content_items(maskedFilename)")
                .execute(&pool)
                .await
                .map_err(|error| format!("relay share database content index failed: {error}"))?;
            query("CREATE INDEX idx_content_items_advertisable ON content_items(isAdvertisable)")
                .execute(&pool)
                .await
                .map_err(|error| format!("relay share database content index failed: {error}"))?;
        }

        let timestamp = crate::unix_timestamp() as i64;
        query("INSERT INTO version (a) VALUES (?)")
            .bind(1_i64)
            .execute(&pool)
            .await
            .map_err(|error| format!("relay share database version failed: {error}"))?;
        query("INSERT INTO scans (timestamp, options, end) VALUES (?, ?, ?)")
            .bind(timestamp)
            .bind("{}")
            .bind(timestamp)
            .execute(&pool)
            .await
            .map_err(|error| format!("relay share database scan failed: {error}"))?;

        for share in shares {
            let size = i64::try_from(share.size)
                .map_err(|_| "relay share size exceeds SQLite integer range".to_owned())?;
            let mut components = share.filename.rsplitn(2, '/');
            let _file_name = components.next();
            let parent = components.next().unwrap_or_default();
            if !parent.is_empty() {
                let mut prefix = String::new();
                for component in parent.split('/') {
                    if component.is_empty() {
                        continue;
                    }
                    if !prefix.is_empty() {
                        prefix.push('/');
                    }
                    prefix.push_str(component);
                    query("INSERT OR IGNORE INTO directories (name, timestamp) VALUES (?, ?)")
                        .bind(&prefix)
                        .bind(timestamp)
                        .execute(&pool)
                        .await
                        .map_err(|error| {
                            format!("relay share database directory failed: {error}")
                        })?;
                }
            }
            let extension = share
                .filename
                .rsplit_once('.')
                .filter(|(prefix, suffix)| !prefix.contains('/') && !suffix.is_empty())
                .map(|(_, suffix)| suffix.to_lowercase())
                .unwrap_or_default();
            match target {
                ControllerProfile::Legacy => {
                    query("INSERT INTO files (maskedFilename, originalFilename, size, touchedAt, code, extension, attributeJson, timestamp) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
                        .bind(&share.filename)
                        .bind(&share.filename)
                        .bind(size)
                        .bind("1970-01-01")
                        .bind(1_i64)
                        .bind(&extension)
                        .bind("[]")
                        .bind(timestamp)
                        .execute(&pool)
                        .await
                        .map_err(|error| format!("relay share database file failed: {error}"))?;
                }
                ControllerProfile::Native => {
                    query("INSERT INTO files (maskedFilename, originalFilename, size, touchedAt, code, extension, attributeJson, timestamp, isBlocked, isQuarantined, moderationReason) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
                        .bind(&share.filename)
                        .bind(&share.filename)
                        .bind(size)
                        .bind("1970-01-01")
                        .bind(1_i64)
                        .bind(&extension)
                        .bind("[]")
                        .bind(timestamp)
                        .bind(0_i64)
                        .bind(0_i64)
                        .bind::<Option<&str>>(None)
                        .execute(&pool)
                        .await
                        .map_err(|error| format!("relay share database file failed: {error}"))?;
                }
            }
            query("INSERT INTO filenames (maskedFilename) VALUES (?)")
                .bind(&share.filename)
                .execute(&pool)
                .await
                .map_err(|error| format!("relay share database index failed: {error}"))?;
        }
        Ok(())
    }
    .await;
    pool.close().await;
    result
}

/// Read and minimally validate a frozen relay share repository, returning the
/// files that the controller can use for remote content resolution.
pub(crate) async fn read_share_database(
    path: &Path,
    target: ControllerProfile,
) -> Result<Vec<RemoteShare>, String> {
    validate_share_database_path(path)?;
    let options = SqliteConnectOptions::new()
        .filename(path)
        .read_only(true)
        .create_if_missing(false);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .map_err(|error| format!("relay share database validation open failed: {error}"))?;
    let result = async {
        let rows = query("SELECT name FROM sqlite_master WHERE type = 'table'")
            .fetch_all(&pool)
            .await
            .map_err(|error| format!("relay share database schema read failed: {error}"))?;
        let mut tables = rows
            .iter()
            .filter_map(|row| row.try_get::<String, _>("name").ok())
            .collect::<Vec<_>>();
        tables.sort();
        let mut expected = match target {
            ControllerProfile::Legacy => OLD_SHARE_DATABASE_TABLES.to_vec(),
            ControllerProfile::Native => CURRENT_SHARE_DATABASE_TABLES.to_vec(),
        };
        expected.sort_unstable();
        if tables != expected {
            return Err(format!(
                "relay share database schema does not match {}",
                match target {
                    ControllerProfile::Legacy => "slskd",
                    ControllerProfile::Native => "native",
                }
            ));
        }
        let row_limit = i64::try_from(MAX_RELAY_SHARE_ENTRIES + 1)
            .expect("relay share entry limit fits in SQLite integer");
        let rows = query("SELECT maskedFilename, size FROM files ORDER BY maskedFilename LIMIT ?")
            .bind(row_limit)
            .fetch_all(&pool)
            .await
            .map_err(|error| format!("relay share database files read failed: {error}"))?;
        if rows.len() > MAX_RELAY_SHARE_ENTRIES {
            return Err(format!(
                "relay share database contains more than {MAX_RELAY_SHARE_ENTRIES} files"
            ));
        }
        let mut shares = Vec::with_capacity(rows.len());
        for row in rows {
            let share = RemoteShare {
                filename: row
                    .try_get::<String, _>("maskedFilename")
                    .map_err(|error| format!("relay share filename is invalid: {error}"))?,
                size: row
                    .try_get::<i64, _>("size")
                    .map_err(|error| format!("relay share size is invalid: {error}"))?
                    .try_into()
                    .map_err(|_| "relay share size is negative".to_owned())?,
            };
            validate_share_entries(std::slice::from_ref(&share))?;
            shares.push(share);
        }
        Ok(shares)
    }
    .await;
    pool.close().await;
    result
}

fn validate_share_database_path(path: &Path) -> Result<(), String> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(format!(
                "relay share database path metadata read failed: {error}"
            ));
        }
    };
    if metadata.file_type().is_symlink() {
        return Err("relay share database path must not be a symlink".to_owned());
    }
    if !metadata.is_file() {
        return Err("relay share database path must be a regular file".to_owned());
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MultipartPart<'a> {
    pub name: String,
    pub filename: Option<String>,
    pub data: &'a [u8],
}

/// Parse the multipart form used by the frozen relay controller without
/// converting file bytes to UTF-8.  The normal API body parser is text-based,
/// but relay uploads are allowed to contain arbitrary audio/database bytes.
pub(crate) fn parse_multipart<'a>(
    body: &'a [u8],
    content_type: Option<&str>,
) -> Result<Vec<MultipartPart<'a>>, String> {
    let boundary = content_type
        .and_then(|value| {
            let mut parts = value.split(';');
            let media_type = parts.next()?.trim();
            if !media_type.eq_ignore_ascii_case("multipart/form-data") {
                return None;
            }
            parts.find_map(|part| {
                let (name, value) = part.trim().split_once('=')?;
                if !name.trim().eq_ignore_ascii_case("boundary") {
                    return None;
                }
                let value = value.trim().trim_matches('"');
                (!value.is_empty()).then(|| value.to_owned())
            })
        })
        .ok_or_else(|| "multipart/form-data boundary is missing".to_owned())?;
    if boundary.len() > 70
        || boundary
            .bytes()
            .any(|byte| byte.is_ascii_control() || byte == b' ')
    {
        return Err("multipart boundary is invalid".to_owned());
    }
    let marker = format!("--{boundary}").into_bytes();
    let mut cursor = 0;
    let mut parts = Vec::new();
    while let Some(start) = find_multipart_boundary(body, &marker, cursor) {
        cursor = start + marker.len();
        if body.get(cursor..cursor + 2) == Some(b"--") {
            break;
        }
        if body.get(cursor..cursor + 2) != Some(b"\r\n") {
            return Err("multipart boundary delimiter is invalid".to_owned());
        }
        cursor += 2;
        let Some(next) = find_multipart_boundary(body, &marker, cursor) else {
            return Err("multipart closing boundary is missing".to_owned());
        };
        let mut part_end = next;
        if part_end >= 2 && body[part_end - 2..part_end] == *b"\r\n" {
            part_end -= 2;
        }
        let Some(header_end) = find_bytes(&body[cursor..part_end], b"\r\n\r\n", 0) else {
            return Err("multipart part headers are missing".to_owned());
        };
        if header_end > MAX_MULTIPART_HEADER_BYTES {
            return Err(format!(
                "multipart part headers exceed {MAX_MULTIPART_HEADER_BYTES} bytes"
            ));
        }
        let header_text = std::str::from_utf8(&body[cursor..cursor + header_end])
            .map_err(|_| "multipart part headers are not valid UTF-8".to_owned())?;
        let disposition = header_text.lines().find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.trim()
                .eq_ignore_ascii_case("content-disposition")
                .then(|| value.trim())
        });
        let Some(disposition) = disposition else {
            return Err("multipart Content-Disposition is missing".to_owned());
        };
        if parts.len() >= MAX_MULTIPART_PARTS {
            return Err(format!(
                "multipart body contains more than {MAX_MULTIPART_PARTS} parts"
            ));
        }
        let field_name = multipart_parameter(disposition, "name")
            .ok_or_else(|| "multipart field name is missing".to_owned())?;
        let filename = multipart_parameter(disposition, "filename");
        let data_start = cursor + header_end + 4;
        parts.push(MultipartPart {
            name: field_name,
            filename,
            data: &body[data_start..part_end],
        });
        cursor = next;
    }
    if parts.is_empty() {
        return Err("multipart body contains no parts".to_owned());
    }
    Ok(parts)
}

fn multipart_parameter(disposition: &str, parameter: &str) -> Option<String> {
    disposition.split(';').skip(1).find_map(|part| {
        let (name, value) = part.trim().split_once('=')?;
        if !name.trim().eq_ignore_ascii_case(parameter) {
            return None;
        }
        let value = value.trim();
        let value = value
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'))
            .unwrap_or(value);
        (value.len() <= MAX_MULTIPART_PARAMETER_BYTES && !value.is_empty())
            .then(|| value.to_owned())
    })
}

fn find_bytes(haystack: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    haystack
        .get(from..)?
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|offset| from + offset)
}

fn find_multipart_boundary(haystack: &[u8], marker: &[u8], from: usize) -> Option<usize> {
    let mut search_from = from;
    while let Some(start) = find_bytes(haystack, marker, search_from) {
        let at_line_start =
            start == 0 || (start >= 2 && haystack.get(start - 2..start) == Some(b"\r\n"));
        let suffix = haystack.get(start + marker.len()..start + marker.len() + 2);
        if at_line_start && matches!(suffix, Some(b"--") | Some(b"\r\n")) {
            return Some(start);
        }
        search_from = start.saturating_add(1);
    }
    None
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Challenge {
    token: String,
    expires_at: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AgentRegistration {
    connection_id: String,
    remote_ip: IpAddr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PendingRequest {
    connection_id: String,
    filename: Option<String>,
    start_offset: u64,
    expires_at: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RuntimeState {
    challenges: BTreeMap<String, Challenge>,
    registered_agents: BTreeMap<String, AgentRegistration>,
    pending_downloads: BTreeMap<String, PendingRequest>,
    pending_file_info: BTreeMap<String, PendingRequest>,
    pending_file_uploads: BTreeMap<String, PendingRequest>,
    pending_share_uploads: BTreeMap<String, PendingRequest>,
    completed_share_uploads: BTreeMap<String, CompletedShareUpload>,
    agent_shares: BTreeMap<String, Vec<RemoteShare>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AuthorizedDownload {
    pub filename: String,
    pub agent_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AuthorizedUpload {
    pub agent_name: String,
    pub filename: Option<String>,
}

impl RuntimeState {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Rehydrate accepted remote share repositories after a controller
    /// restart.  Live connection/request state is intentionally not restored;
    /// agents must reconnect for that.  The durable share projection remains
    /// available for content-id resolution as soon as the controller starts.
    pub(crate) async fn restore_persisted_share_uploads(
        &mut self,
        incoming_directory: &Path,
        target: ControllerProfile,
    ) -> Result<(), String> {
        let manifest_path = incoming_directory.join("manifest.json");
        let Some(records) = read_share_manifest(&manifest_path)? else {
            return Ok(());
        };
        for record in records {
            let expected_filename = format!("share-{}.db", record.token.replace('-', ""));
            if record.agent_name.trim().is_empty()
                || Uuid::parse_str(&record.token).is_err()
                || record.database_path.parent() != Some(incoming_directory)
                || record
                    .database_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    != Some(expected_filename.as_str())
            {
                continue;
            }
            let Ok(shares) = read_share_database(&record.database_path, target).await else {
                continue;
            };
            let key = record.token.clone();
            self.agent_shares
                .insert(record.agent_name.clone(), shares.clone());
            self.completed_share_uploads.insert(
                key,
                CompletedShareUpload {
                    agent_name: record.agent_name,
                    share_count: shares.len(),
                    shares,
                    database_path: record.database_path,
                    completed_at: record.completed_at,
                },
            );
            self.prune_completed_share_uploads();
        }
        Ok(())
    }

    pub(crate) fn prune(&mut self, now: u64) {
        self.challenges
            .retain(|_, challenge| challenge.expires_at > now);
        self.pending_downloads
            .retain(|_, request| request.expires_at > now);
        let expired_info = self
            .pending_file_info
            .iter()
            .filter(|(_, request)| request.expires_at <= now)
            .map(|(token, _)| token.clone())
            .collect::<Vec<_>>();
        self.pending_file_info
            .retain(|_, request| request.expires_at > now);
        if let Ok(mut waiters) = file_info_waiters().lock() {
            for token in expired_info {
                if let Some(sender) = waiters.remove(&token) {
                    let _ = sender.send(Err("relay file-info request expired".to_owned()));
                }
            }
        }
        let expired_uploads = self
            .pending_file_uploads
            .iter()
            .filter(|(_, request)| request.expires_at <= now)
            .map(|(token, _)| token.clone())
            .collect::<Vec<_>>();
        self.pending_file_uploads
            .retain(|_, request| request.expires_at > now);
        if let Ok(mut waiters) = file_upload_waiters().lock() {
            for token in expired_uploads {
                if let Some(sender) = waiters.remove(&token) {
                    let _ = sender.send(Err("relay file upload request expired".to_owned()));
                }
            }
        }
        self.pending_share_uploads
            .retain(|_, request| request.expires_at > now);
    }

    fn prune_completed_share_uploads(&mut self) {
        let excess = self
            .completed_share_uploads
            .len()
            .saturating_sub(MAX_RELAY_SHARE_UPLOAD_RECORDS);
        if excess == 0 {
            return;
        }
        let mut oldest = self
            .completed_share_uploads
            .iter()
            .map(|(token, upload)| (upload.completed_at, token.clone()))
            .collect::<Vec<_>>();
        oldest.sort_unstable();
        for (_, token) in oldest.into_iter().take(excess) {
            self.completed_share_uploads.remove(&token);
        }
    }

    /// Issue the short-lived challenge sent by the relay hub after a new
    /// agent connection is established.
    pub(crate) fn issue_challenge(&mut self, connection_id: &str, now: u64) -> String {
        self.prune(now);
        let token = Uuid::new_v4().simple().to_string();
        self.challenges.insert(
            connection_id.to_owned(),
            Challenge {
                token: token.clone(),
                expires_at: now.saturating_add(CHALLENGE_TTL_SECONDS),
            },
        );
        token
    }

    /// Authenticate and bind an agent to its connection.  This mirrors the
    /// oracle's hub checks: configured instance name, configured source CIDR,
    /// then PBKDF2/HMAC challenge verification.
    pub(crate) fn authenticate_agent(
        &mut self,
        settings: &RelaySettings,
        scheme: CredentialScheme,
        connection_id: &str,
        agent_name: &str,
        credential: &str,
        remote_ip: IpAddr,
        now: u64,
    ) -> bool {
        self.prune(now);
        let Some(challenge) = self.challenges.get(connection_id) else {
            return false;
        };
        let Some(agent) = settings
            .agents
            .values()
            .find(|agent| agent.instance_name == agent_name)
        else {
            return false;
        };
        let allowed_ip = agent
            .cidr
            .split(',')
            .map(str::trim)
            .filter(|cidr| !cidr.is_empty())
            .filter_map(|cidr| crate::config::TrustedProxyCidr::parse(cidr).ok())
            .any(|cidr| cidr.contains(remote_ip));
        if !allowed_ip
            || !verify_credential(
                scheme,
                &agent.secret,
                agent_name,
                &challenge.token,
                credential,
            )
        {
            self.registered_agents
                .retain(|_, registration| registration.connection_id != connection_id);
            return false;
        }

        self.challenges.remove(connection_id);
        self.registered_agents.insert(
            agent_name.to_owned(),
            AgentRegistration {
                connection_id: connection_id.to_owned(),
                remote_ip,
            },
        );
        true
    }

    pub(crate) fn deregister_connection(&mut self, connection_id: &str) {
        self.challenges.remove(connection_id);
        let removed_agents = self
            .registered_agents
            .iter()
            .filter(|(_, registration)| registration.connection_id == connection_id)
            .map(|(name, _)| name.clone())
            .collect::<Vec<_>>();
        self.registered_agents
            .retain(|_, registration| registration.connection_id != connection_id);
        for agent_name in removed_agents {
            self.agent_shares.remove(&agent_name);
        }
        self.pending_downloads
            .retain(|_, request| request.connection_id != connection_id);
        let removed_info = self
            .pending_file_info
            .iter()
            .filter(|(_, request)| request.connection_id == connection_id)
            .map(|(token, _)| token.clone())
            .collect::<Vec<_>>();
        self.pending_file_info
            .retain(|_, request| request.connection_id != connection_id);
        if let Ok(mut waiters) = file_info_waiters().lock() {
            for token in removed_info {
                waiters.remove(&token);
            }
        }
        let removed_uploads = self
            .pending_file_uploads
            .iter()
            .filter(|(_, request)| request.connection_id == connection_id)
            .map(|(token, _)| token.clone())
            .collect::<Vec<_>>();
        self.pending_file_uploads
            .retain(|_, request| request.connection_id != connection_id);
        if let Ok(mut waiters) = file_upload_waiters().lock() {
            for token in removed_uploads {
                waiters.remove(&token);
            }
        }
        self.pending_share_uploads
            .retain(|_, request| request.connection_id != connection_id);
    }

    pub(crate) fn issue_share_upload_token(
        &mut self,
        agent_name: &str,
        now: u64,
    ) -> Option<String> {
        self.prune(now);
        if self.pending_share_uploads.len() >= MAX_RELAY_PENDING_REQUESTS {
            return None;
        }
        let connection_id = self
            .registered_agents
            .get(agent_name)?
            .connection_id
            .clone();
        let token = Uuid::new_v4().to_string();
        self.pending_share_uploads.insert(
            token.clone(),
            PendingRequest {
                connection_id,
                filename: None,
                start_offset: 0,
                expires_at: now.saturating_add(REQUEST_TTL_SECONDS),
            },
        );
        Some(token)
    }

    #[cfg(any(test, feature = "bounded-differential"))]
    #[allow(dead_code)]
    pub(crate) fn issue_file_upload_token(
        &mut self,
        agent_name: &str,
        filename: &str,
        token: Uuid,
        now: u64,
    ) -> bool {
        let Some(connection_id) = self
            .registered_agents
            .get(agent_name)
            .map(|registration| registration.connection_id.clone())
        else {
            return false;
        };
        self.prune(now);
        if self.pending_file_uploads.len() >= MAX_RELAY_PENDING_REQUESTS {
            return false;
        }
        self.pending_file_uploads.insert(
            token.to_string(),
            PendingRequest {
                connection_id,
                filename: Some(filename.to_owned()),
                start_offset: 0,
                expires_at: now.saturating_add(REQUEST_TTL_SECONDS),
            },
        );
        true
    }

    pub(crate) fn issue_download_tokens(
        &mut self,
        filename: &str,
        now: u64,
    ) -> Vec<(String, String)> {
        self.prune(now);
        if self
            .pending_downloads
            .len()
            .saturating_add(self.registered_agents.len())
            > MAX_RELAY_PENDING_REQUESTS
        {
            return Vec::new();
        }
        self.registered_agents
            .iter()
            .map(|(agent_name, registration)| {
                let token = Uuid::new_v4().to_string();
                self.pending_downloads.insert(
                    token.clone(),
                    PendingRequest {
                        connection_id: registration.connection_id.clone(),
                        filename: Some(filename.to_owned()),
                        start_offset: 0,
                        expires_at: now.saturating_add(DOWNLOAD_TTL_SECONDS),
                    },
                );
                (agent_name.clone(), token)
            })
            .collect()
    }

    pub(crate) fn cancel_download(&mut self, token: &str) {
        self.pending_downloads.remove(token);
    }

    pub(crate) fn validate_download(
        &mut self,
        settings: &RelaySettings,
        scheme: CredentialScheme,
        token: Uuid,
        credential: &str,
        now: u64,
    ) -> Option<AuthorizedDownload> {
        self.prune(now);
        let request = self.pending_downloads.get(&token.to_string())?;
        let (agent_name, _) = self.agent_for_connection(&request.connection_id)?;
        let configured = settings
            .agents
            .values()
            .find(|configured| configured.instance_name == agent_name)?;
        verify_credential(
            scheme,
            &configured.secret,
            agent_name,
            &token.to_string(),
            credential,
        )
        .then(|| AuthorizedDownload {
            filename: request.filename.clone().unwrap_or_default(),
            agent_name: agent_name.to_owned(),
        })
    }

    pub(crate) fn validate_file_upload(
        &mut self,
        settings: &RelaySettings,
        scheme: CredentialScheme,
        token: Uuid,
        filename: &str,
        credential: &str,
        now: u64,
    ) -> Option<AuthorizedUpload> {
        self.validate_one_use_request(scheme, settings, token, filename, credential, now, false)
    }

    pub(crate) fn validate_share_upload(
        &mut self,
        settings: &RelaySettings,
        scheme: CredentialScheme,
        token: Uuid,
        credential: &str,
        now: u64,
    ) -> Option<AuthorizedUpload> {
        self.validate_one_use_request(scheme, settings, token, "", credential, now, true)
    }

    /// Start the controller-side file-info workflow used before a stream is
    /// opened.  The authenticated agent answers through `ReturnFileInfo`.
    pub(crate) fn begin_file_info(
        &mut self,
        agent_name: &str,
        filename: &str,
        now: u64,
    ) -> Option<(Uuid, oneshot::Receiver<Result<FileInfo, String>>)> {
        self.prune(now);
        let connection_id = self
            .registered_agents
            .get(agent_name)
            .map(|registration| registration.connection_id.clone())?;
        if self.pending_file_info.len() >= MAX_RELAY_PENDING_REQUESTS {
            return None;
        }
        let token = Uuid::new_v4();
        self.pending_file_info.insert(
            token.to_string(),
            PendingRequest {
                connection_id,
                filename: Some(filename.to_owned()),
                start_offset: 0,
                expires_at: now.saturating_add(REQUEST_TTL_SECONDS),
            },
        );
        let (sender, receiver) = oneshot::channel();
        let Ok(mut waiters) = file_info_waiters().lock() else {
            self.pending_file_info.remove(&token.to_string());
            return None;
        };
        waiters.insert(token.to_string(), sender);
        Some((token, receiver))
    }

    pub(crate) fn complete_file_info(
        &mut self,
        connection_id: &str,
        token: Uuid,
        exists: bool,
        length: u64,
    ) -> bool {
        let key = token.to_string();
        let Some(request) = self.pending_file_info.get(&key) else {
            return false;
        };
        if request.connection_id != connection_id {
            return false;
        }
        self.pending_file_info.remove(&key);
        let sender = file_info_waiters()
            .lock()
            .ok()
            .and_then(|mut waiters| waiters.remove(&key));
        sender.is_some_and(|sender| sender.send(Ok(FileInfo { exists, length })).is_ok())
    }

    pub(crate) fn fail_file_info(
        &mut self,
        connection_id: &str,
        token: Uuid,
        error: String,
    ) -> bool {
        let key = token.to_string();
        let Some(request) = self.pending_file_info.get(&key) else {
            return false;
        };
        if request.connection_id != connection_id {
            return false;
        }
        self.pending_file_info.remove(&key);
        let sender = file_info_waiters()
            .lock()
            .ok()
            .and_then(|mut waiters| waiters.remove(&key));
        sender.is_some_and(|sender| sender.send(Err(error)).is_ok())
    }

    pub(crate) fn cancel_file_info(&mut self, token: Uuid) {
        let key = token.to_string();
        self.pending_file_info.remove(&key);
        if let Ok(mut waiters) = file_info_waiters().lock() {
            waiters.remove(&key);
        }
    }

    /// Start the controller side of the file-stream workflow.  The returned
    /// token is sent to the authenticated agent over SignalR and the receiver
    /// is completed by the matching multipart HTTP upload.
    pub(crate) fn begin_file_stream(
        &mut self,
        agent_name: &str,
        filename: &str,
        start_offset: u64,
        now: u64,
    ) -> Option<(Uuid, oneshot::Receiver<Result<UploadedFile, String>>)> {
        self.prune(now);
        let connection_id = self
            .registered_agents
            .get(agent_name)
            .map(|registration| registration.connection_id.clone())?;
        if self.pending_file_uploads.len() >= MAX_RELAY_PENDING_REQUESTS {
            return None;
        }
        let token = Uuid::new_v4();
        self.pending_file_uploads.insert(
            token.to_string(),
            PendingRequest {
                connection_id,
                filename: Some(filename.to_owned()),
                start_offset,
                expires_at: now.saturating_add(REQUEST_TTL_SECONDS),
            },
        );
        let (sender, receiver) = oneshot::channel();
        let Ok(mut waiters) = file_upload_waiters().lock() else {
            self.pending_file_uploads.remove(&token.to_string());
            return None;
        };
        waiters.insert(token.to_string(), sender);
        Some((token, receiver))
    }

    pub(crate) fn cancel_file_stream(&mut self, token: Uuid) {
        self.pending_file_uploads.remove(&token.to_string());
        if let Ok(mut waiters) = file_upload_waiters().lock() {
            waiters.remove(&token.to_string());
        }
    }

    pub(crate) fn complete_file_stream(token: Uuid, file: UploadedFile) -> bool {
        let sender = file_upload_waiters()
            .lock()
            .ok()
            .and_then(|mut waiters| waiters.remove(&token.to_string()));
        sender.is_some_and(|sender| sender.send(Ok(file)).is_ok())
    }

    /// Fail a validated upload after the HTTP body could not be persisted.
    /// The one-use request token has already been consumed at that point, so
    /// the normal connection-bound failure path cannot be used to wake the
    /// controller stream waiter.
    pub(crate) fn fail_file_stream_token(token: Uuid, error: String) -> bool {
        let sender = file_upload_waiters()
            .lock()
            .ok()
            .and_then(|mut waiters| waiters.remove(&token.to_string()));
        sender.is_some_and(|sender| sender.send(Err(error)).is_ok())
    }

    pub(crate) fn fail_file_stream(
        &mut self,
        connection_id: &str,
        token: Uuid,
        error: String,
    ) -> bool {
        let key = token.to_string();
        let Some(request) = self.pending_file_uploads.get(&key) else {
            return false;
        };
        if request.connection_id != connection_id {
            return false;
        }
        self.pending_file_uploads.remove(&key);
        let sender = file_upload_waiters()
            .lock()
            .ok()
            .and_then(|mut waiters| waiters.remove(&key));
        sender.is_some_and(|sender| sender.send(Err(error)).is_ok())
    }

    pub(crate) fn record_share_upload(
        &mut self,
        token: Uuid,
        agent_name: String,
        share_count: usize,
        shares: Vec<RemoteShare>,
        database_path: PathBuf,
        completed_at: u64,
    ) -> Result<(), String> {
        validate_share_entries(&shares)?;
        if share_count != shares.len() {
            return Err(format!(
                "relay share count {share_count} does not match {} entries",
                shares.len()
            ));
        }
        let completed = CompletedShareUpload {
            agent_name: agent_name.clone(),
            share_count,
            shares: shares.clone(),
            database_path: database_path.clone(),
            completed_at,
        };
        persist_share_manifest(
            &database_path,
            &PersistedShareUpload {
                token: token.to_string(),
                agent_name: agent_name.clone(),
                shares: shares.clone(),
                database_path: database_path.clone(),
                completed_at,
            },
        )?;
        self.agent_shares.insert(agent_name.clone(), shares.clone());
        self.completed_share_uploads
            .insert(token.to_string(), completed);
        self.prune_completed_share_uploads();
        Ok(())
    }

    pub(crate) fn remote_file_for_agent(
        &self,
        agent_name: &str,
        content_id: &str,
    ) -> Option<(String, u64)> {
        self.agent_shares.get(agent_name)?.iter().find_map(|share| {
            (share.filename == content_id
                || crate::stable_content_hash(&share.filename, share.size).to_string()
                    == content_id)
                .then(|| (share.filename.clone(), share.size))
        })
    }

    fn validate_one_use_request(
        &mut self,
        scheme: CredentialScheme,
        settings: &RelaySettings,
        token: Uuid,
        filename: &str,
        credential: &str,
        now: u64,
        share: bool,
    ) -> Option<AuthorizedUpload> {
        self.prune(now);
        let key = token.to_string();
        let request = if share {
            self.pending_share_uploads.get(&key)
        } else {
            self.pending_file_uploads.get(&key)
        }?;
        let request = request.clone();
        let agent_name = self
            .agent_for_connection(&request.connection_id)
            .map(|(name, _)| name.to_owned())?;
        let configured = settings
            .agents
            .values()
            .find(|configured| configured.instance_name == agent_name)?;
        let filename_matches = share
            || request
                .filename
                .as_deref()
                .is_some_and(|expected| expected == filename);
        let valid = filename_matches
            && verify_credential(scheme, &configured.secret, &agent_name, &key, credential);
        if share {
            self.pending_share_uploads.remove(&key);
        } else {
            self.pending_file_uploads.remove(&key);
        }
        valid.then_some(AuthorizedUpload {
            agent_name,
            filename: request.filename,
        })
    }

    fn agent_for_connection(&self, connection_id: &str) -> Option<(&str, &AgentRegistration)> {
        self.registered_agents
            .iter()
            .find(|(_, registration)| registration.connection_id == connection_id)
            .map(|(name, registration)| (name.as_str(), registration))
    }

    pub(crate) fn connection_for_agent(&self, agent_name: &str) -> Option<&str> {
        self.registered_agents
            .get(agent_name)
            .map(|registration| registration.connection_id.as_str())
    }

    pub(crate) fn registered_agent_name(&self, connection_id: &str) -> Option<&str> {
        self.agent_for_connection(connection_id)
            .map(|(name, _)| name)
    }

    #[cfg(any(test, feature = "bounded-differential"))]
    #[allow(dead_code)]
    pub(crate) fn registered_agent_remote_ip(&self, agent_name: &str) -> Option<IpAddr> {
        self.registered_agents
            .get(agent_name)
            .map(|registration| registration.remote_ip)
    }
}

fn persist_share_manifest(
    database_path: &Path,
    record: &PersistedShareUpload,
) -> Result<(), String> {
    let Some(incoming_directory) = database_path.parent() else {
        return Ok(());
    };
    if incoming_directory
        .file_name()
        .and_then(|name| name.to_str())
        != Some("incoming")
        || incoming_directory
            .parent()
            .and_then(|parent| parent.file_name())
            .and_then(|name| name.to_str())
            != Some("relay")
    {
        return Ok(());
    }
    let _manifest_guard = share_manifest_lock()
        .lock()
        .map_err(|_| "relay share manifest lock is poisoned".to_owned())?;
    let manifest_path = incoming_directory.join("manifest.json");
    let mut records = read_share_manifest(&manifest_path)?.unwrap_or_default();
    records.retain(|existing| existing.token != record.token);
    records.push(record.clone());
    let bytes = serde_json::to_vec_pretty(&records)
        .map_err(|error| format!("relay share manifest serialization failed: {error}"))?;
    if bytes.len() as u64 > MAX_RELAY_SHARE_MANIFEST_BYTES {
        return Err(format!(
            "relay share manifest exceeds {MAX_RELAY_SHARE_MANIFEST_BYTES} bytes"
        ));
    }
    let temporary_path =
        incoming_directory.join(format!(".manifest-{}.json.tmp", Uuid::new_v4().simple()));
    let write_result = (|| {
        use std::io::Write;

        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut temporary = options
            .open(&temporary_path)
            .map_err(|error| format!("relay share manifest write failed: {error}"))?;
        temporary
            .write_all(&bytes)
            .and_then(|()| temporary.sync_all())
            .map_err(|error| format!("relay share manifest write failed: {error}"))?;
        drop(temporary);
        fs::rename(&temporary_path, &manifest_path)
            .map_err(|error| format!("relay share manifest replace failed: {error}"))?;
        sync_manifest_directory(incoming_directory)
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }
    write_result?;
    Ok(())
}

fn read_share_manifest(path: &Path) -> Result<Option<Vec<PersistedShareUpload>>, String> {
    let Some(bytes) = read_bounded_file(path, MAX_RELAY_SHARE_MANIFEST_BYTES)
        .map_err(|error| format!("relay share manifest read failed: {error}"))?
    else {
        return Ok(None);
    };
    let records = serde_json::from_slice::<Vec<PersistedShareUpload>>(&bytes)
        .map_err(|error| format!("relay share manifest parse failed: {error}"))?;
    if records.len() > MAX_RELAY_SHARE_UPLOAD_RECORDS {
        return Err(format!(
            "relay share manifest contains more than {MAX_RELAY_SHARE_UPLOAD_RECORDS} uploads"
        ));
    }
    for record in &records {
        validate_share_entries(&record.shares)?;
    }
    Ok(Some(records))
}

fn validate_share_entries(shares: &[RemoteShare]) -> Result<(), String> {
    if shares.len() > MAX_RELAY_SHARE_ENTRIES {
        return Err(format!(
            "relay share list contains more than {MAX_RELAY_SHARE_ENTRIES} files"
        ));
    }
    if shares.iter().any(|share| {
        share.filename.trim().is_empty()
            || share.filename.len() > MAX_RELAY_SHARE_FILENAME_BYTES
            || share.filename.chars().any(char::is_control)
    }) {
        return Err(format!(
            "relay share filename exceeds {MAX_RELAY_SHARE_FILENAME_BYTES} bytes or contains invalid characters"
        ));
    }
    if shares
        .iter()
        .any(|share| i64::try_from(share.size).is_err())
    {
        return Err("relay share size exceeds SQLite integer range".to_owned());
    }
    Ok(())
}

fn read_bounded_file(path: &Path, max_bytes: u64) -> std::io::Result<Option<Vec<u8>>> {
    use std::io::Read;

    #[cfg(not(unix))]
    {
        let metadata = match fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        if metadata.file_type().is_symlink() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "path must not be a symlink",
            ));
        }
    }

    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = match options.open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "path must be a regular file",
        ));
    }
    if metadata.len() > max_bytes {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("file exceeds {max_bytes} bytes"),
        ));
    }
    let mut bytes = Vec::new();
    file.take(max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > max_bytes {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("file exceeds {max_bytes} bytes"),
        ));
    }
    Ok(Some(bytes))
}

#[cfg(unix)]
fn sync_manifest_directory(path: &Path) -> Result<(), String> {
    fs::File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("relay share manifest directory sync failed: {error}"))
}

#[cfg(not(unix))]
fn sync_manifest_directory(_path: &Path) -> Result<(), String> {
    Ok(())
}

fn derive_relay_key(secret: &str, agent_name: &str) -> [u8; 48] {
    let mut key = [0_u8; 48];
    pbkdf2::derive(
        pbkdf2::PBKDF2_HMAC_SHA256,
        PBKDF2_ITERATIONS,
        agent_name.as_bytes(),
        secret.as_bytes(),
        &mut key,
    );
    key
}

fn base62_encode(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }

    let leading_zeroes = bytes
        .iter()
        .take_while(|byte| **byte == 0)
        .count()
        .min(bytes.len().saturating_sub(1));
    let mut source = bytes
        .iter()
        .map(|byte| u32::from(*byte))
        .collect::<Vec<_>>();
    let mut digits = Vec::new();
    while !source.is_empty() {
        let mut quotient = Vec::with_capacity(source.len());
        let mut remainder = 0_u32;
        for value in source {
            let accumulator = value + remainder * 256;
            let digit = accumulator / 62;
            remainder = accumulator % 62;
            if !quotient.is_empty() || digit > 0 {
                quotient.push(digit);
            }
        }
        digits.push(BASE62_ALPHABET[remainder as usize] as char);
        source = quotient;
    }
    digits.reverse();
    format!(
        "{}{}",
        "0".repeat(leading_zeroes),
        digits.into_iter().collect::<String>()
    )
}

fn credential_for_aes(secret: &str, agent_name: &str, token: &str) -> String {
    let key = derive_relay_key(secret, agent_name);
    let encrypted = Aes256CbcEnc::new_from_slices(&key[..32], &key[32..])
        .expect("relay credential key and IV have fixed valid lengths")
        .encrypt_padded_vec::<Pkcs7>(token.as_bytes());
    base62_encode(&encrypted)
}

fn credential_for_hmac(secret: &str, agent_name: &str, token: &str) -> String {
    let key = derive_relay_key(secret, agent_name);
    let signing_key = hmac::Key::new(hmac::HMAC_SHA256, &key);
    BASE64.encode(hmac::sign(&signing_key, token.as_bytes()).as_ref())
}

pub(crate) fn credential_for_with_scheme(
    scheme: CredentialScheme,
    secret: &str,
    agent_name: &str,
    token: &str,
) -> String {
    match scheme {
        CredentialScheme::LegacyAesBase62 => credential_for_aes(secret, agent_name, token),
        CredentialScheme::NativeHmacBase64 => credential_for_hmac(secret, agent_name, token),
    }
}

pub(crate) fn credential_for_target(
    target: ControllerProfile,
    secret: &str,
    agent_name: &str,
    token: &str,
) -> String {
    credential_for_with_scheme(credential_scheme(target), secret, agent_name, token)
}

fn verify_credential(
    scheme: CredentialScheme,
    secret: &str,
    agent_name: &str,
    token: &str,
    credential: &str,
) -> bool {
    credential_for_with_scheme(scheme, secret, agent_name, token) == credential
}

#[cfg(any(test, feature = "bounded-differential"))]
pub(crate) fn credential_for_test(secret: &str, agent_name: &str, token: &str) -> String {
    credential_for_aes(secret, agent_name, token)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn multipart_fixture(part_count: usize) -> Vec<u8> {
        let mut body = Vec::new();
        for index in 0..part_count {
            body.extend_from_slice(
                format!(
                    "--boundary\r\nContent-Disposition: form-data; name=\"part{index}\"\r\n\r\n"
                )
                .as_bytes(),
            );
            body.extend_from_slice(b"payload\r\n");
        }
        body.extend_from_slice(b"--boundary--\r\n");
        body
    }

    #[test]
    fn multipart_parser_borrows_binary_payloads() {
        let body = b"--boundary\r\nContent-Disposition: form-data; name=\"database\"; filename=\"shares.db\"\r\n\r\n\0\xffpayload\r\n--boundary--\r\n";
        let parts = parse_multipart(body, Some("multipart/form-data; boundary=boundary"))
            .expect("binary multipart payload must parse");
        let payload = b"\0\xffpayload";
        let payload_offset = body
            .windows(payload.len())
            .position(|window| window == payload)
            .expect("payload offset");

        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].name, "database");
        assert_eq!(parts[0].filename.as_deref(), Some("shares.db"));
        assert_eq!(parts[0].data, payload);
        assert!(std::ptr::eq(
            parts[0].data.as_ptr(),
            body[payload_offset..].as_ptr()
        ));
    }

    #[test]
    fn multipart_parser_keeps_boundary_like_binary_payloads() {
        let payload = b"\0binary--boundaryXpayload";
        let mut body =
            b"--boundary\r\nContent-Disposition: form-data; name=\"database\"; filename=\"shares.db\"\r\n\r\n"
                .to_vec();
        body.extend_from_slice(payload);
        body.extend_from_slice(b"\r\n--boundary--\r\n");

        let parts = parse_multipart(&body, Some("multipart/form-data; boundary=boundary"))
            .expect("boundary-like bytes in binary payload must not terminate the part");

        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].data, payload);
    }

    #[test]
    fn multipart_parser_rejects_excessive_part_count() {
        let body = multipart_fixture(MAX_MULTIPART_PARTS + 1);
        let error = parse_multipart(&body, Some("multipart/form-data; boundary=boundary"))
            .expect_err("multipart part count must be bounded");
        assert!(error.contains("more than 16 parts"), "{error}");
    }

    #[test]
    fn multipart_parser_rejects_oversized_part_headers() {
        let oversized_header = "x".repeat(MAX_MULTIPART_HEADER_BYTES + 1);
        let body = format!(
            "--boundary\r\nX-Relay-Metadata: {oversized_header}\r\nContent-Disposition: form-data; name=\"file\"\r\n\r\npayload\r\n--boundary--\r\n"
        );
        let error = parse_multipart(
            body.as_bytes(),
            Some("multipart/form-data; boundary=boundary"),
        )
        .expect_err("multipart part headers must be bounded");
        assert!(error.contains("headers exceed"), "{error}");
    }

    #[test]
    fn multipart_parameters_are_bounded() {
        let oversized = "x".repeat(MAX_MULTIPART_PARAMETER_BYTES + 1);
        let disposition = format!("form-data; name=\"{oversized}\"");
        assert!(multipart_parameter(&disposition, "name").is_none());
    }

    fn credential(secret: &str, agent_name: &str, token: &str) -> String {
        super::credential_for_test(secret, agent_name, token)
    }

    #[test]
    fn credential_uses_oracle_compatible_pbkdf2_aes_and_base62() {
        assert_eq!(
            credential(
                "0123456789abcdef",
                "edge-one",
                "00000000-0000-0000-0000-000000000001"
            ),
            "1EZXSJxy9aWLtOiBHIVJSQ4hCRoZ6ICplJM3c3sAUyar3MzDvSuVQnQV1pDoVMo2E"
        );
        assert!(verify_credential(
            CredentialScheme::LegacyAesBase62,
            "0123456789abcdef",
            "edge-one",
            "00000000-0000-0000-0000-000000000001",
            &credential(
                "0123456789abcdef",
                "edge-one",
                "00000000-0000-0000-0000-000000000001"
            )
        ));
        assert!(!verify_credential(
            CredentialScheme::LegacyAesBase62,
            "0123456789abcdef",
            "edge-one",
            "00000000-0000-0000-0000-000000000001",
            &credential(
                "0123456789abcdef",
                "edge-one",
                "00000000-0000-0000-0000-000000000002"
            )
        ));
    }

    #[test]
    fn credential_uses_oracle_compatible_native_pbkdf2_hmac_and_base64() {
        assert_eq!(
            credential_for_target(
                ControllerProfile::Native,
                "0123456789abcdef",
                "edge-one",
                "00000000-0000-0000-0000-000000000001",
            ),
            "1n/Y+pCmvHtSlQQgafVp4X5Xl/LrDsj+sHrAcCwSOqw="
        );
        assert!(verify_credential(
            CredentialScheme::NativeHmacBase64,
            "0123456789abcdef",
            "edge-one",
            "00000000-0000-0000-0000-000000000001",
            "1n/Y+pCmvHtSlQQgafVp4X5Xl/LrDsj+sHrAcCwSOqw=",
        ));
        assert!(!verify_credential(
            CredentialScheme::NativeHmacBase64,
            "0123456789abcdef",
            "edge-one",
            "00000000-0000-0000-0000-000000000001",
            "1EZXSJxy9aWLtOiBHIVJSQ4hCRoZ6ICplJM3c3sAUyar3MzDvSuVQnQV1pDoVMo2E",
        ));
    }

    #[tokio::test]
    async fn relay_share_database_round_trips_both_target_schemas() {
        let shares = vec![RemoteShare {
            filename: "Remote/Agent.flac".to_owned(),
            size: 6,
        }];
        for (index, target) in [ControllerProfile::Legacy, ControllerProfile::Native]
            .into_iter()
            .enumerate()
        {
            let path = std::env::temp_dir().join(format!(
                "slskr-relay-share-test-{index}-{}.db",
                Uuid::new_v4().simple()
            ));
            write_share_database(&path, target, &shares)
                .await
                .expect("write relay share database");
            assert_eq!(
                read_share_database(&path, target)
                    .await
                    .expect("read relay share database"),
                shares
            );
            assert_eq!(
                tokio::fs::read(&path)
                    .await
                    .expect("read relay share database bytes")
                    .get(..16),
                Some(b"SQLite format 3\0".as_slice())
            );
            tokio::fs::remove_file(path)
                .await
                .expect("remove relay share database");
        }
    }

    #[tokio::test]
    async fn relay_share_database_rejects_sizes_outside_sqlite_range() {
        let path = std::env::temp_dir().join(format!(
            "slskr-relay-share-size-limit-{}.db",
            Uuid::new_v4().simple()
        ));
        let error = write_share_database(
            &path,
            ControllerProfile::Native,
            &[RemoteShare {
                filename: "Remote/Too-Large.flac".to_owned(),
                size: u64::MAX,
            }],
        )
        .await
        .expect_err("unrepresentable relay share size must be rejected");
        assert!(error.contains("SQLite integer range"), "{error}");
        assert!(!path.exists());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn relay_share_database_rejects_symlink_paths() {
        use std::os::unix::fs::symlink;

        let root = std::env::temp_dir().join(format!(
            "slskr-relay-share-symlink-{}",
            Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&root).expect("create relay share symlink fixture");
        let target = root.join("target.db");
        let linked = root.join("linked.db");
        let shares = [RemoteShare {
            filename: "Remote/Target.flac".to_owned(),
            size: 1,
        }];
        write_share_database(&target, ControllerProfile::Native, &shares)
            .await
            .expect("write relay share symlink target");
        symlink(&target, &linked).expect("create relay share database symlink");

        let read_error = read_share_database(&linked, ControllerProfile::Native)
            .await
            .expect_err("read must reject relay share database symlink");
        assert!(read_error.contains("must not be a symlink"), "{read_error}");
        let write_error = write_share_database(&linked, ControllerProfile::Native, &shares)
            .await
            .expect_err("write must reject relay share database symlink");
        assert!(
            write_error.contains("must not be a symlink"),
            "{write_error}"
        );

        std::fs::remove_dir_all(root).expect("remove relay share symlink fixture");
    }

    #[tokio::test]
    async fn persisted_relay_share_upload_rehydrates_remote_file_lookup() {
        let root =
            std::env::temp_dir().join(format!("slskr-relay-rehydrate-{}", Uuid::new_v4().simple()));
        let incoming = root.join("relay").join("incoming");
        std::fs::create_dir_all(&incoming).expect("create relay incoming directory");
        let token = Uuid::new_v4();
        let database_path = incoming.join(format!("share-{}.db", token.simple()));
        let shares = vec![RemoteShare {
            filename: "Remote/Restarted.flac".to_owned(),
            size: 42,
        }];
        write_share_database(&database_path, ControllerProfile::Native, &shares)
            .await
            .expect("write persisted relay database");
        let mut state = RuntimeState::new();
        state
            .record_share_upload(
                token,
                "edge-restarted".to_owned(),
                shares.len(),
                shares.clone(),
                database_path.clone(),
                123,
            )
            .expect("record persisted relay share upload");
        let mut restored = RuntimeState::new();
        restored
            .restore_persisted_share_uploads(&incoming, ControllerProfile::Native)
            .await
            .expect("restore persisted relay database");
        assert_eq!(
            restored.remote_file_for_agent("edge-restarted", "Remote/Restarted.flac"),
            Some(("Remote/Restarted.flac".to_owned(), 42))
        );
        assert_eq!(state.completed_share_uploads.len(), 1);
        std::fs::remove_dir_all(root).expect("remove relay rehydration fixture");
    }

    #[tokio::test]
    async fn relay_share_manifest_failure_does_not_publish_upload() {
        let root = std::env::temp_dir().join(format!(
            "slskr-relay-manifest-failure-{}",
            Uuid::new_v4().simple()
        ));
        let incoming = root.join("relay").join("incoming");
        std::fs::create_dir_all(&incoming).expect("create relay incoming directory");
        std::fs::write(incoming.join("manifest.json"), b"not-json")
            .expect("write malformed relay manifest");
        let token = Uuid::new_v4();
        let database_path = incoming.join(format!("share-{}.db", token.simple()));
        let mut state = RuntimeState::new();
        let error = state
            .record_share_upload(
                token,
                "edge-failed".to_owned(),
                1,
                vec![RemoteShare {
                    filename: "Remote/Failed.flac".to_owned(),
                    size: 7,
                }],
                database_path,
                123,
            )
            .expect_err("malformed manifest must reject publication");
        assert!(error.contains("manifest parse failed"));
        assert!(state.agent_shares.is_empty());
        assert!(state.completed_share_uploads.is_empty());
        std::fs::remove_dir_all(root).expect("remove relay manifest fixture");
    }

    #[test]
    fn relay_share_manifest_rejects_excessive_upload_records() {
        let root = std::env::temp_dir().join(format!(
            "slskr-relay-manifest-record-limit-{}",
            Uuid::new_v4().simple()
        ));
        let incoming = root.join("relay").join("incoming");
        std::fs::create_dir_all(&incoming).expect("create relay incoming directory");
        let record = PersistedShareUpload {
            token: Uuid::new_v4().to_string(),
            agent_name: "edge-limit".to_owned(),
            shares: Vec::new(),
            database_path: incoming.join("share-limit.db"),
            completed_at: 1,
        };
        let records = vec![record.clone(); MAX_RELAY_SHARE_UPLOAD_RECORDS + 1];
        std::fs::write(
            incoming.join("manifest.json"),
            serde_json::to_vec(&records).expect("serialize oversized manifest"),
        )
        .expect("write oversized manifest");

        let error = persist_share_manifest(&record.database_path, &record)
            .expect_err("oversized manifest must be rejected");
        assert!(error.contains("contains more than"), "{error}");
        std::fs::remove_dir_all(root).expect("remove oversized manifest fixture");
    }

    #[test]
    fn relay_share_record_rejects_excessive_file_count() {
        let shares = vec![
            RemoteShare {
                filename: "file.flac".to_owned(),
                size: 1,
            };
            MAX_RELAY_SHARE_ENTRIES + 1
        ];
        let mut state = RuntimeState::new();
        let error = state
            .record_share_upload(
                Uuid::new_v4(),
                "edge-limit".to_owned(),
                shares.len(),
                shares,
                std::env::temp_dir().join("not-published.db"),
                1,
            )
            .expect_err("oversized relay share record must be rejected");
        assert!(error.contains("contains more than"), "{error}");
    }

    #[test]
    fn relay_share_record_rejects_inconsistent_count() {
        let mut state = RuntimeState::new();
        let error = state
            .record_share_upload(
                Uuid::new_v4(),
                "edge-count".to_owned(),
                1,
                Vec::new(),
                std::env::temp_dir().join("not-published.db"),
                1,
            )
            .expect_err("relay share count mismatch must be rejected");
        assert!(error.contains("does not match"), "{error}");
    }

    #[test]
    fn relay_pending_request_maps_have_hard_capacity() {
        let now = 100;
        let mut state = RuntimeState::new();
        state.registered_agents.insert(
            "edge-limit".to_owned(),
            AgentRegistration {
                connection_id: "connection-limit".to_owned(),
                remote_ip: "127.0.0.1".parse().expect("test relay address"),
            },
        );
        let pending = PendingRequest {
            connection_id: "connection-limit".to_owned(),
            filename: Some("file.flac".to_owned()),
            start_offset: 0,
            expires_at: now + 1,
        };
        state.pending_downloads = (0..MAX_RELAY_PENDING_REQUESTS)
            .map(|index| (format!("download-{index}"), pending.clone()))
            .collect();
        state.pending_file_info = (0..MAX_RELAY_PENDING_REQUESTS)
            .map(|index| (format!("info-{index}"), pending.clone()))
            .collect();
        state.pending_file_uploads = (0..MAX_RELAY_PENDING_REQUESTS)
            .map(|index| (format!("upload-{index}"), pending.clone()))
            .collect();
        state.pending_share_uploads = (0..MAX_RELAY_PENDING_REQUESTS)
            .map(|index| (format!("share-{index}"), pending.clone()))
            .collect();

        assert!(state.issue_download_tokens("file.flac", now).is_empty());
        assert!(state.issue_share_upload_token("edge-limit", now).is_none());
        assert!(state
            .begin_file_info("edge-limit", "file.flac", now)
            .is_none());
        assert!(state
            .begin_file_stream("edge-limit", "file.flac", 0, now)
            .is_none());
    }

    #[test]
    fn completed_relay_share_upload_history_has_hard_capacity() {
        let mut state = RuntimeState::new();
        for index in 0..=MAX_RELAY_SHARE_UPLOAD_RECORDS {
            let token = format!("token-{index:04}");
            state.completed_share_uploads.insert(
                token,
                CompletedShareUpload {
                    agent_name: "edge-limit".to_owned(),
                    share_count: 0,
                    shares: Vec::new(),
                    database_path: PathBuf::from("share.db"),
                    completed_at: index as u64,
                },
            );
        }

        state.prune_completed_share_uploads();

        assert_eq!(
            state.completed_share_uploads.len(),
            MAX_RELAY_SHARE_UPLOAD_RECORDS
        );
        assert!(!state.completed_share_uploads.contains_key("token-0000"));
        assert!(state
            .completed_share_uploads
            .contains_key(&format!("token-{MAX_RELAY_SHARE_UPLOAD_RECORDS:04}")));
    }

    #[test]
    fn concurrent_relay_share_manifest_updates_retain_both_records() {
        let root = std::env::temp_dir().join(format!(
            "slskr-relay-manifest-concurrent-{}",
            Uuid::new_v4().simple()
        ));
        let incoming = root.join("relay").join("incoming");
        std::fs::create_dir_all(&incoming).expect("create concurrent manifest directory");
        let first_token = Uuid::new_v4();
        let second_token = Uuid::new_v4();
        let first_path = incoming.join(format!("share-{}.db", first_token.simple()));
        let second_path = incoming.join(format!("share-{}.db", second_token.simple()));
        let first = PersistedShareUpload {
            token: first_token.to_string(),
            agent_name: "edge-one".to_owned(),
            shares: vec![RemoteShare {
                filename: "one.flac".to_owned(),
                size: 1,
            }],
            database_path: first_path.clone(),
            completed_at: 1,
        };
        let second = PersistedShareUpload {
            token: second_token.to_string(),
            agent_name: "edge-two".to_owned(),
            shares: vec![RemoteShare {
                filename: "two.flac".to_owned(),
                size: 2,
            }],
            database_path: second_path.clone(),
            completed_at: 2,
        };
        let first_token_text = first.token.clone();
        let second_token_text = second.token.clone();
        let first_thread = std::thread::spawn(move || persist_share_manifest(&first_path, &first));
        let second_thread =
            std::thread::spawn(move || persist_share_manifest(&second_path, &second));
        first_thread
            .join()
            .expect("first manifest writer must not panic")
            .expect("first manifest update");
        second_thread
            .join()
            .expect("second manifest writer must not panic")
            .expect("second manifest update");

        let bytes = std::fs::read(incoming.join("manifest.json")).expect("read manifest");
        let records = serde_json::from_slice::<Vec<PersistedShareUpload>>(&bytes)
            .expect("parse concurrent manifest");
        assert_eq!(records.len(), 2);
        assert!(records
            .iter()
            .any(|record| record.token == first_token_text));
        assert!(records
            .iter()
            .any(|record| record.token == second_token_text));
        std::fs::remove_dir_all(root).expect("remove concurrent manifest fixture");
    }
}
