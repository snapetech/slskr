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
use cbc::cipher::{block_padding::Pkcs7, BlockEncryptMut, KeyIvInit};
use ring::{hmac, pbkdf2};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx_core::{query::query, row::Row};
use sqlx_sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use tokio::sync::{mpsc::UnboundedSender, oneshot};
use uuid::Uuid;

use crate::config::{ControllerCompatibilityTarget, RelaySettings};

const CHALLENGE_TTL_SECONDS: u64 = 10;
const REQUEST_TTL_SECONDS: u64 = 5 * 60;
const DOWNLOAD_TTL_SECONDS: u64 = 10 * 60;
const PBKDF2_ITERATIONS: NonZeroU32 = match NonZeroU32::new(1_000) {
    Some(value) => value,
    None => unreachable!(),
};

type Aes256CbcEnc = cbc::Encryptor<Aes256>;
const BASE62_ALPHABET: &[u8; 62] =
    b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CredentialScheme {
    SlskdAesBase62,
    SlskdnHmacBase64,
}

pub(crate) fn credential_scheme(target: ControllerCompatibilityTarget) -> CredentialScheme {
    match target {
        ControllerCompatibilityTarget::Slskd => CredentialScheme::SlskdAesBase62,
        ControllerCompatibilityTarget::Slskdn => CredentialScheme::SlskdnHmacBase64,
    }
}

/// SignalR is a process-local transport in the oracle.  Keep the equivalent
/// connection senders outside the persisted compatibility projection: they
/// are live sockets, not state that can be serialized or restored.
static HUB_CONNECTIONS: OnceLock<Mutex<BTreeMap<String, UnboundedSender<String>>>> =
    OnceLock::new();

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

fn hub_connections() -> &'static Mutex<BTreeMap<String, UnboundedSender<String>>> {
    HUB_CONNECTIONS.get_or_init(|| Mutex::new(BTreeMap::new()))
}

fn file_upload_waiters() -> &'static FileUploadWaiters {
    FILE_UPLOAD_WAITERS.get_or_init(|| Mutex::new(BTreeMap::new()))
}

fn file_info_waiters() -> &'static FileInfoWaiters {
    FILE_INFO_WAITERS.get_or_init(|| Mutex::new(BTreeMap::new()))
}

pub(crate) fn register_hub_connection(connection_id: String, sender: UnboundedSender<String>) {
    if let Ok(mut connections) = hub_connections().lock() {
        connections.insert(connection_id, sender);
    }
}

pub(crate) fn unregister_hub_connection(connection_id: &str) {
    if let Ok(mut connections) = hub_connections().lock() {
        connections.remove(connection_id);
    }
}

/// Push a SignalR invocation to a connected agent.  A false return means the
/// agent is no longer connected or its writer queue has closed.
pub(crate) fn send_hub_invocation(
    runtime: &RuntimeState,
    agent_name: &str,
    target: &str,
    arguments: Vec<Value>,
) -> bool {
    let Some(connection_id) = runtime.connection_for_agent(agent_name) else {
        return false;
    };
    let Ok(connections) = hub_connections().lock() else {
        return false;
    };
    let Some(sender) = connections.get(connection_id) else {
        return false;
    };
    sender
        .send(
            serde_json::json!({
                "type": 1,
                "target": target,
                "arguments": arguments,
            })
            .to_string(),
        )
        .is_ok()
}

#[derive(Debug)]
pub(crate) struct UploadedFile {
    pub filename: String,
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
/// frozen controller targets intentionally have different repository schemas:
/// slskd uses the legacy ten-table repository, while slskdN adds moderation
/// columns and the content-items table.
pub(crate) async fn write_share_database(
    path: &Path,
    target: ControllerCompatibilityTarget,
    shares: &[RemoteShare],
) -> Result<(), String> {
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
            ControllerCompatibilityTarget::Slskd => {
                "CREATE TABLE files (maskedFilename TEXT PRIMARY KEY, originalFilename TEXT NOT NULL, size BIGINT NOT NULL, touchedAt TEXT NOT NULL, code INTEGER DEFAULT 1 NOT NULL, extension TEXT, attributeJson TEXT NOT NULL, timestamp INTEGER NOT NULL)"
            }
            ControllerCompatibilityTarget::Slskdn => {
                "CREATE TABLE files (maskedFilename TEXT PRIMARY KEY, originalFilename TEXT NOT NULL, size BIGINT NOT NULL, touchedAt TEXT NOT NULL, code INTEGER DEFAULT 1 NOT NULL, extension TEXT, attributeJson TEXT NOT NULL, timestamp INTEGER NOT NULL, isBlocked INTEGER DEFAULT 0 NOT NULL, isQuarantined INTEGER DEFAULT 0 NOT NULL, moderationReason TEXT)"
            }
        };
        query(files_sql)
            .execute(&pool)
            .await
            .map_err(|error| format!("relay share database file schema failed: {error}"))?;
        if target == ControllerCompatibilityTarget::Slskdn {
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
                ControllerCompatibilityTarget::Slskd => {
                    query("INSERT INTO files (maskedFilename, originalFilename, size, touchedAt, code, extension, attributeJson, timestamp) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
                        .bind(&share.filename)
                        .bind(&share.filename)
                        .bind(share.size as i64)
                        .bind("1970-01-01")
                        .bind(1_i64)
                        .bind(&extension)
                        .bind("[]")
                        .bind(timestamp)
                        .execute(&pool)
                        .await
                        .map_err(|error| format!("relay share database file failed: {error}"))?;
                }
                ControllerCompatibilityTarget::Slskdn => {
                    query("INSERT INTO files (maskedFilename, originalFilename, size, touchedAt, code, extension, attributeJson, timestamp, isBlocked, isQuarantined, moderationReason) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
                        .bind(&share.filename)
                        .bind(&share.filename)
                        .bind(share.size as i64)
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
    target: ControllerCompatibilityTarget,
) -> Result<Vec<RemoteShare>, String> {
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
            ControllerCompatibilityTarget::Slskd => OLD_SHARE_DATABASE_TABLES.to_vec(),
            ControllerCompatibilityTarget::Slskdn => CURRENT_SHARE_DATABASE_TABLES.to_vec(),
        };
        expected.sort_unstable();
        if tables != expected {
            return Err(format!(
                "relay share database schema does not match {}",
                match target {
                    ControllerCompatibilityTarget::Slskd => "slskd",
                    ControllerCompatibilityTarget::Slskdn => "slskdN",
                }
            ));
        }
        let rows = query("SELECT maskedFilename, size FROM files ORDER BY maskedFilename")
            .fetch_all(&pool)
            .await
            .map_err(|error| format!("relay share database files read failed: {error}"))?;
        rows.iter()
            .map(|row| {
                Ok(RemoteShare {
                    filename: row
                        .try_get::<String, _>("maskedFilename")
                        .map_err(|error| format!("relay share filename is invalid: {error}"))?,
                    size: row
                        .try_get::<i64, _>("size")
                        .map_err(|error| format!("relay share size is invalid: {error}"))?
                        .try_into()
                        .map_err(|_| "relay share size is negative".to_owned())?,
                })
            })
            .collect()
    }
    .await;
    pool.close().await;
    result
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MultipartPart {
    pub name: String,
    pub filename: Option<String>,
    pub data: Vec<u8>,
}

/// Parse the multipart form used by the frozen relay controller without
/// converting file bytes to UTF-8.  The normal API body parser is text-based,
/// but relay uploads are allowed to contain arbitrary audio/database bytes.
pub(crate) fn parse_multipart(
    body: &[u8],
    content_type: Option<&str>,
) -> Result<Vec<MultipartPart>, String> {
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
    while let Some(start) = find_bytes(body, &marker, cursor) {
        cursor = start + marker.len();
        if body.get(cursor..cursor + 2) == Some(b"--") {
            break;
        }
        if body.get(cursor..cursor + 2) == Some(b"\r\n") {
            cursor += 2;
        }
        let Some(next) = find_bytes(body, &marker, cursor) else {
            return Err("multipart closing boundary is missing".to_owned());
        };
        let mut part_end = next;
        if part_end >= 2 && body[part_end - 2..part_end] == *b"\r\n" {
            part_end -= 2;
        }
        let Some(header_end) = find_bytes(&body[cursor..part_end], b"\r\n\r\n", 0) else {
            return Err("multipart part headers are missing".to_owned());
        };
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
        let field_name = multipart_parameter(disposition, "name")
            .ok_or_else(|| "multipart field name is missing".to_owned())?;
        let filename = multipart_parameter(disposition, "filename");
        let data_start = cursor + header_end + 4;
        parts.push(MultipartPart {
            name: field_name,
            filename,
            data: body[data_start..part_end].to_vec(),
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
        (!value.is_empty()).then(|| value.to_owned())
    })
}

fn find_bytes(haystack: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    haystack
        .get(from..)?
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|offset| from + offset)
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
        target: ControllerCompatibilityTarget,
    ) -> Result<(), String> {
        let manifest_path = incoming_directory.join("manifest.json");
        let bytes = match fs::read(&manifest_path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => {
                return Err(format!("relay share manifest read failed: {error}"));
            }
        };
        let records = serde_json::from_slice::<Vec<PersistedShareUpload>>(&bytes)
            .map_err(|error| format!("relay share manifest is invalid: {error}"))?;
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

    #[cfg(test)]
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

    #[cfg(test)]
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
    let manifest_path = incoming_directory.join("manifest.json");
    let mut records = match fs::read(&manifest_path) {
        Ok(bytes) => serde_json::from_slice::<Vec<PersistedShareUpload>>(&bytes)
            .map_err(|error| format!("relay share manifest parse failed: {error}"))?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(error) => return Err(format!("relay share manifest read failed: {error}")),
    };
    records.retain(|existing| existing.token != record.token);
    records.push(record.clone());
    let bytes = serde_json::to_vec_pretty(&records)
        .map_err(|error| format!("relay share manifest serialization failed: {error}"))?;
    let temporary_path = manifest_path.with_extension("json.tmp");
    fs::write(&temporary_path, bytes)
        .map_err(|error| format!("relay share manifest write failed: {error}"))?;
    fs::rename(&temporary_path, &manifest_path)
        .map_err(|error| format!("relay share manifest replace failed: {error}"))?;
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
        .encrypt_padded_vec_mut::<Pkcs7>(token.as_bytes());
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
        CredentialScheme::SlskdAesBase62 => credential_for_aes(secret, agent_name, token),
        CredentialScheme::SlskdnHmacBase64 => credential_for_hmac(secret, agent_name, token),
    }
}

pub(crate) fn credential_for_target(
    target: ControllerCompatibilityTarget,
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

#[cfg(test)]
pub(crate) fn credential_for_test(secret: &str, agent_name: &str, token: &str) -> String {
    credential_for_aes(secret, agent_name, token)
}

#[cfg(test)]
mod tests {
    use super::*;

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
            CredentialScheme::SlskdAesBase62,
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
            CredentialScheme::SlskdAesBase62,
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
    fn credential_uses_oracle_compatible_slskdn_pbkdf2_hmac_and_base64() {
        assert_eq!(
            credential_for_target(
                ControllerCompatibilityTarget::Slskdn,
                "0123456789abcdef",
                "edge-one",
                "00000000-0000-0000-0000-000000000001",
            ),
            "1n/Y+pCmvHtSlQQgafVp4X5Xl/LrDsj+sHrAcCwSOqw="
        );
        assert!(verify_credential(
            CredentialScheme::SlskdnHmacBase64,
            "0123456789abcdef",
            "edge-one",
            "00000000-0000-0000-0000-000000000001",
            "1n/Y+pCmvHtSlQQgafVp4X5Xl/LrDsj+sHrAcCwSOqw=",
        ));
        assert!(!verify_credential(
            CredentialScheme::SlskdnHmacBase64,
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
        for (index, target) in [
            ControllerCompatibilityTarget::Slskd,
            ControllerCompatibilityTarget::Slskdn,
        ]
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
        write_share_database(
            &database_path,
            ControllerCompatibilityTarget::Slskdn,
            &shares,
        )
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
            .restore_persisted_share_uploads(&incoming, ControllerCompatibilityTarget::Slskdn)
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
}
