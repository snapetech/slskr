/// Database persistence layer for slskr
///
/// SQLite-backed durable storage using sqlx for async operations.
/// Provides full persistence for searches, transfers, messages, and user stats.
use serde::{Deserialize, Serialize};
use sqlx_core::{
    from_row::FromRow, query::query, query_as::query_as, row::Row, sql_str::AssertSqlSafe, Error,
};
use sqlx_sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions, SqliteRow};
use std::collections::BTreeMap;
#[cfg(unix)]
use std::fs::OpenOptions;
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const SQLITE_PARAMETER_CHUNK: usize = 900;

#[cfg(unix)]
fn prepare_private_database_file(db_path: &str) -> std::io::Result<()> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(db_path)?;
    if !file.metadata()?.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "database path must be a regular file",
        ));
    }
    file.set_permissions(std::fs::Permissions::from_mode(0o600))
}

fn sql_placeholders(count: usize) -> String {
    (0..count).map(|_| "?").collect::<Vec<_>>().join(", ")
}

fn sql_value_rows(row_count: usize, column_count: usize) -> String {
    let row = format!("({})", sql_placeholders(column_count));
    std::iter::repeat_n(row, row_count)
        .collect::<Vec<_>>()
        .join(", ")
}

/// Search record for persistence
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchRecord {
    pub id: String,
    pub query: String,
    pub status: String,
    pub result_count: i64,
    pub created_at: i64,
    pub completed_at: Option<i64>,
    pub room: Option<String>,
    pub target: Option<String>,
    pub fallback_attempts: i64,
}

/// Search result row for persistence.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchResultRecord {
    pub id: i64,
    pub search_id: String,
    pub peer_username: Option<String>,
    pub filename: String,
    pub size: i64,
    pub extension: String,
    pub bit_rate: Option<i64>,
    pub sample_rate: Option<i64>,
    pub bit_depth: Option<i64>,
    pub length_seconds: Option<i64>,
    pub locked: bool,
    pub slot_free: Option<bool>,
    pub average_speed: Option<i64>,
    pub queue_length: Option<i64>,
    pub created_at: i64,
}

/// One complete search projection to write inside a larger search transition.
/// The public identity is stored separately because the protocol token and
/// the controller-facing identifier have different compatibility contracts.
pub struct SearchWrite {
    pub record: SearchRecord,
    pub external_id: String,
    pub results: Vec<SearchResultRecord>,
}

/// Transfer record for persistence
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransferRecord {
    pub id: String,
    pub direction: String,
    pub filename: String,
    pub peer_username: String,
    pub filesize: i64,
    pub progress: i64,
    pub status: String,
    pub started_at: i64,
    pub completed_at: Option<i64>,
    pub request_id: Option<String>,
    pub wishlist_item_id: Option<String>,
    pub request_name: Option<String>,
    pub destination_directory: Option<String>,
    pub local_path: Option<String>,
    pub batch_id: Option<String>,
    pub reason: Option<String>,
    pub bit_rate: Option<i64>,
    pub sample_rate: Option<i64>,
    pub bit_depth: Option<i64>,
    pub length_seconds: Option<i64>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub title: Option<String>,
    pub track_number: Option<i64>,
    pub year: Option<i64>,
    pub attempts: i64,
    pub auto_replace_attempts: i64,
    pub next_attempt_at: Option<i64>,
    /// Monotonic per-transfer revision used to reject delayed snapshots.
    pub updated_at_ms: i64,
}

/// Durable transfer-batch metadata. The legacy profile stores batches
/// in its Transfers database separately from the associated transfer rows;
/// keeping that boundary here prevents the controller-feature JSON file from
/// becoming the source of truth for the legacy compatibility profile.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransferBatchRecord {
    pub id: String,
    pub search_id: Option<String>,
    pub username: String,
    pub direction: i64,
    pub created_at: String,
    pub options_json: Option<String>,
}

/// Durable HashDb row using the core columns shared by the frozen native profile
/// schema and slskR's content-discovery model.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HashDbRecord {
    pub flac_key: String,
    pub byte_hash: String,
    pub size: i64,
    pub first_seen_at: i64,
    pub last_updated_at: i64,
    pub seq_id: i64,
    pub use_count: i64,
    pub full_file_hash: String,
    pub musicbrainz_id: String,
    pub file_sha256: String,
}

/// Durable key/value state for HashDb cursors and backfill progress.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HashDbStateRecord {
    pub key: String,
    pub value: Option<String>,
}

/// Durable overlay/Soulseek traffic counters used by the native profile fairness
/// guard projection.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TrafficTotalsRecord {
    pub overlay_upload_bytes: i64,
    pub overlay_download_bytes: i64,
    pub soulseek_upload_bytes: i64,
    pub soulseek_download_bytes: i64,
}

/// Transfer transition/progress event record for persistence
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransferEventRecord {
    pub id: i64,
    pub transfer_id: String,
    pub direction: String,
    pub token: i64,
    pub filename: String,
    pub peer_username: Option<String>,
    pub filesize: i64,
    pub progress: i64,
    pub status: String,
    pub reason: Option<String>,
    pub created_at: i64,
    /// Orders rapid transitions that share the same wall-clock second.
    pub updated_at_ms: i64,
}

/// Share index file record for persistence
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShareFileRecord {
    pub filename: String,
    pub size: i64,
    pub extension: String,
    pub root_label: String,
    pub local_path: Option<String>,
    pub attributes_json: String,
    pub updated_at: i64,
}

/// Runtime event record for persistence
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventRecord {
    pub id: i64,
    pub kind: String,
    pub resource: String,
    pub detail: Option<String>,
    pub created_at: i64,
}

/// Message record for persistence
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MessageRecord {
    pub id: String,
    pub username: String,
    pub content: String,
    pub direction: String,
    pub read: bool,
    pub created_at: i64,
    #[serde(default)]
    pub source_id: Option<i64>,
    #[serde(default)]
    pub source_timestamp: Option<i64>,
    #[serde(default)]
    pub was_replayed: bool,
}

/// User statistics record
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserStatsRecord {
    pub username: String,
    pub uploads: i64,
    pub downloads: i64,
    pub total_uploaded: i64,
    pub total_downloaded: i64,
    pub watched: bool,
    pub last_seen: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

/// User projection record for persistence
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserProjectionRecord {
    pub username: String,
    pub watched: bool,
    pub status: Option<String>,
    pub average_speed: Option<i64>,
    pub upload_count: Option<i64>,
    pub file_count: Option<i64>,
    pub directory_count: Option<i64>,
    pub updated_at: i64,
}

/// Room subscription record
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoomRecord {
    pub name: String,
    pub owner: Option<String>,
    pub subscribed: bool,
    pub joined_at: i64,
    pub last_activity: i64,
}

/// User note record for persistence
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserNoteRecord {
    pub id: String,
    pub username: String,
    pub note: String,
    pub color: String,
    pub icon: String,
    pub is_high_priority: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Soulseek interest record for persistence
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InterestRecord {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub created_at: i64,
}

/// Security ban record for persistence
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecurityBanRecord {
    pub kind: String,
    pub value: String,
    pub created_at: i64,
    pub reason: String,
    pub expires_at: i64,
    pub is_permanent: bool,
}

/// Wishlist item record for persistence
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WishlistItemRecord {
    pub id: String,
    pub artist: String,
    pub title: String,
    pub kind: String,
    pub filter: String,
    pub enabled: bool,
    pub auto_download: bool,
    pub max_results: i64,
    pub max_downloads: Option<i64>,
    pub last_viewed_at: Option<i64>,
    pub last_searched_at: Option<i64>,
    pub last_match_count: i64,
    pub last_visible_hit_count: i64,
    pub last_hidden_locked_hit_count: i64,
    pub last_filtered_out_hit_count: i64,
    pub last_ignored_result_hit_count: i64,
    pub last_response_count: i64,
    pub total_search_count: i64,
    pub total_download_count: i64,
    pub last_search_id: Option<String>,
    pub lidarr_album_id: Option<i64>,
    pub lidarr_track_id: Option<i64>,
    pub lidarr_track_count: Option<i64>,
    pub lidarr_duration_seconds: Option<i64>,
    pub lidarr_release_disambiguation: Option<String>,
    pub added_at: i64,
}

/// Persisted per-wishlist peer-directory suppression rule.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WishlistIgnoredResultRecord {
    pub id: String,
    pub wishlist_item_id: String,
    pub username: String,
    pub directory: String,
    pub created_at: i64,
}

/// Contact record for persistence
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContactRecord {
    pub id: String,
    pub username: String,
    pub online: bool,
    pub status: String,
    pub free_upload_slots: Option<i64>,
    pub queue_length: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Share grant record for persistence
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShareGrantRecord {
    pub id: String,
    pub collection_id: String,
    pub username: String,
    pub shared_at: i64,
    pub permissions: String,
}

/// Durable delegated-share token verifier. The raw bearer token is never stored.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShareAccessTokenRecord {
    pub token_digest: String,
    pub grant_id: String,
    pub expires_at: i64,
}

/// Share group record for persistence
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShareGroupRecord {
    pub id: String,
    pub name: String,
    pub description: String,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Share group member record for persistence
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShareGroupMemberRecord {
    pub group_id: String,
    pub username: String,
    pub added_at: i64,
}

/// Collection record for persistence
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CollectionRecord {
    pub id: String,
    pub owner_user_id: String,
    pub name: String,
    pub description: String,
    pub collection_type: String,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Collection item record for persistence
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CollectionItemRecord {
    pub id: String,
    pub collection_id: String,
    pub content_id: String,
    pub artist: String,
    pub title: String,
    pub kind: String,
    pub file_name: String,
    pub album: String,
    pub content_hash: String,
    pub added_at: i64,
    pub position: i64,
}

/// Library item record for persistence
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LibraryItemRecord {
    pub id: String,
    pub artist: String,
    pub title: String,
    pub kind: String,
    pub created_at: i64,
}

/// Destination record for persistence
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DestinationRecord {
    pub id: String,
    pub name: String,
    pub path: String,
    pub is_default: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Now-playing record for persistence
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NowPlayingRecord {
    pub username: String,
    pub artist: String,
    pub title: String,
    pub updated_at: i64,
}

/// Browse cache record for persistence
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BrowseRecord {
    pub username: String,
    pub status: String,
    pub entries_json: String,
    pub reason: Option<String>,
    pub folder: Option<String>,
    pub indirect_token: Option<i64>,
    pub requested_at: Option<i64>,
    pub updated_at: i64,
}

/// Runtime compatibility state record for persistence
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuntimeCompatRecord {
    pub id: String,
    pub application_restart_requested: bool,
    pub gc_runs: i64,
    pub autoreplace_enabled: bool,
    pub relay_enabled: bool,
    pub relay_agent_enabled: bool,
    pub bridge_running: bool,
    pub bridge_config_updates: i64,
    pub options_updates: i64,
    pub options_yaml_uploads: i64,
    pub options_yaml_validations: i64,
    pub profile_invites_created: i64,
    pub cache_warm_runs: i64,
    pub backfill_runs: i64,
    pub songid_runs: i64,
    pub songid_run_records_json: String,
    pub lidarr_sync_runs: i64,
    pub lidarr_manual_imports: i64,
    pub updated_at: i64,
}

/// Pending OAuth state record for persistence
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OAuthStateRecord {
    pub state: String,
    pub provider: String,
    pub redirect_uri: String,
    pub created_at: i64,
    pub expires_at: i64,
}

/// Webhook configuration record for persistence
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WebhookRecord {
    pub id: String,
    pub url: String,
    pub events: String, // JSON-encoded array of event types
    pub secret: String,
    pub active: bool,
    pub created_at: i64,
    pub last_triggered: Option<i64>,
    pub retry_count: i32,
    pub max_retries: i32,
    pub timeout_seconds: i32,
}

/// Webhook delivery log record for persistence
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WebhookLogRecord {
    pub id: String,
    pub webhook_id: String,
    pub event: String,
    pub correlation_id: String,
    pub status: String,       // success, failed, timeout, etc.
    pub request_body: String, // JSON payload sent
    pub response_status: Option<i32>,
    pub response_body: Option<String>,
    pub error_message: Option<String>,
    pub attempt: i32,
    pub timestamp: i64,
}

impl<'r> FromRow<'r, SqliteRow> for SearchRecord {
    fn from_row(row: &'r SqliteRow) -> Result<Self, Error> {
        Ok(Self {
            id: row.try_get("id")?,
            query: row.try_get("query")?,
            status: row.try_get("status")?,
            result_count: row.try_get("result_count")?,
            created_at: row.try_get("created_at")?,
            completed_at: row.try_get("completed_at")?,
            room: row.try_get("room")?,
            target: row.try_get("target")?,
            fallback_attempts: row.try_get("fallback_attempts")?,
        })
    }
}

impl<'r> FromRow<'r, SqliteRow> for SearchResultRecord {
    fn from_row(row: &'r SqliteRow) -> Result<Self, Error> {
        Ok(Self {
            id: row.try_get("id")?,
            search_id: row.try_get("search_id")?,
            peer_username: row.try_get("peer_username")?,
            filename: row.try_get("filename")?,
            size: row.try_get("size")?,
            extension: row.try_get("extension")?,
            bit_rate: row.try_get("bit_rate")?,
            sample_rate: row.try_get("sample_rate")?,
            bit_depth: row.try_get("bit_depth")?,
            length_seconds: row.try_get("length_seconds")?,
            locked: row.try_get("locked")?,
            slot_free: row.try_get("slot_free")?,
            average_speed: row.try_get("average_speed")?,
            queue_length: row.try_get("queue_length")?,
            created_at: row.try_get("created_at")?,
        })
    }
}

impl<'r> FromRow<'r, SqliteRow> for TransferRecord {
    fn from_row(row: &'r SqliteRow) -> Result<Self, Error> {
        Ok(Self {
            id: row.try_get("id")?,
            direction: row.try_get("direction")?,
            filename: row.try_get("filename")?,
            peer_username: row.try_get("peer_username")?,
            filesize: row.try_get("filesize")?,
            progress: row.try_get("progress")?,
            status: row.try_get("status")?,
            started_at: row.try_get("started_at")?,
            completed_at: row.try_get("completed_at")?,
            request_id: row.try_get("request_id")?,
            wishlist_item_id: row.try_get("wishlist_item_id")?,
            request_name: row.try_get("request_name")?,
            destination_directory: row.try_get("destination_directory")?,
            local_path: row.try_get("local_path")?,
            batch_id: row.try_get("batch_id")?,
            reason: row.try_get("reason")?,
            bit_rate: row.try_get("bit_rate")?,
            sample_rate: row.try_get("sample_rate")?,
            bit_depth: row.try_get("bit_depth")?,
            length_seconds: row.try_get("length_seconds")?,
            artist: row.try_get("artist")?,
            album: row.try_get("album")?,
            title: row.try_get("title")?,
            track_number: row.try_get("track_number")?,
            year: row.try_get("year")?,
            attempts: row.try_get("attempts")?,
            auto_replace_attempts: row.try_get("auto_replace_attempts")?,
            next_attempt_at: row.try_get("next_attempt_at")?,
            updated_at_ms: row.try_get("updated_at_ms")?,
        })
    }
}

impl<'r> FromRow<'r, SqliteRow> for TransferBatchRecord {
    fn from_row(row: &'r SqliteRow) -> Result<Self, Error> {
        Ok(Self {
            id: row.try_get("Id")?,
            search_id: row.try_get("SearchId")?,
            username: row.try_get("Username")?,
            direction: row.try_get("Direction")?,
            created_at: row.try_get("CreatedAt")?,
            options_json: row.try_get("Options")?,
        })
    }
}

impl<'r> FromRow<'r, SqliteRow> for HashDbRecord {
    fn from_row(row: &'r SqliteRow) -> Result<Self, Error> {
        Ok(Self {
            flac_key: row.try_get("flac_key")?,
            byte_hash: row.try_get("byte_hash")?,
            size: row.try_get("size")?,
            first_seen_at: row.try_get("first_seen_at")?,
            last_updated_at: row.try_get("last_updated_at")?,
            seq_id: row.try_get::<Option<i64>, _>("seq_id")?.unwrap_or_default(),
            use_count: row.try_get::<Option<i64>, _>("use_count")?.unwrap_or(1),
            full_file_hash: row
                .try_get::<Option<String>, _>("full_file_hash")?
                .unwrap_or_default(),
            musicbrainz_id: row
                .try_get::<Option<String>, _>("musicbrainz_id")?
                .unwrap_or_default(),
            file_sha256: row
                .try_get::<Option<String>, _>("file_sha256")?
                .unwrap_or_default(),
        })
    }
}

impl<'r> FromRow<'r, SqliteRow> for HashDbStateRecord {
    fn from_row(row: &'r SqliteRow) -> Result<Self, Error> {
        Ok(Self {
            key: row.try_get("key")?,
            value: row.try_get("value")?,
        })
    }
}

impl<'r> FromRow<'r, SqliteRow> for TrafficTotalsRecord {
    fn from_row(row: &'r SqliteRow) -> Result<Self, Error> {
        Ok(Self {
            overlay_upload_bytes: row.try_get("overlay_upload_bytes")?,
            overlay_download_bytes: row.try_get("overlay_download_bytes")?,
            soulseek_upload_bytes: row.try_get("soulseek_upload_bytes")?,
            soulseek_download_bytes: row.try_get("soulseek_download_bytes")?,
        })
    }
}

impl<'r> FromRow<'r, SqliteRow> for TransferEventRecord {
    fn from_row(row: &'r SqliteRow) -> Result<Self, Error> {
        Ok(Self {
            id: row.try_get("id")?,
            transfer_id: row.try_get("transfer_id")?,
            direction: row.try_get("direction")?,
            token: row.try_get("token")?,
            filename: row.try_get("filename")?,
            peer_username: row.try_get("peer_username")?,
            filesize: row.try_get("filesize")?,
            progress: row.try_get("progress")?,
            status: row.try_get("status")?,
            reason: row.try_get("reason")?,
            created_at: row.try_get("created_at")?,
            updated_at_ms: row.try_get("updated_at_ms")?,
        })
    }
}

impl<'r> FromRow<'r, SqliteRow> for ShareFileRecord {
    fn from_row(row: &'r SqliteRow) -> Result<Self, Error> {
        Ok(Self {
            filename: row.try_get("filename")?,
            size: row.try_get("size")?,
            extension: row.try_get("extension")?,
            root_label: row.try_get("root_label")?,
            local_path: row.try_get("local_path")?,
            attributes_json: row.try_get("attributes_json")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

impl<'r> FromRow<'r, SqliteRow> for EventRecord {
    fn from_row(row: &'r SqliteRow) -> Result<Self, Error> {
        Ok(Self {
            id: row.try_get("id")?,
            kind: row.try_get("kind")?,
            resource: row.try_get("resource")?,
            detail: row.try_get("detail")?,
            created_at: row.try_get("created_at")?,
        })
    }
}

impl<'r> FromRow<'r, SqliteRow> for MessageRecord {
    fn from_row(row: &'r SqliteRow) -> Result<Self, Error> {
        Ok(Self {
            id: row.try_get("id")?,
            username: row.try_get("username")?,
            content: row.try_get("content")?,
            direction: row.try_get("direction")?,
            read: row.try_get("read")?,
            created_at: row.try_get("created_at")?,
            source_id: row.try_get("source_id").unwrap_or(None),
            source_timestamp: row.try_get("source_timestamp").unwrap_or(None),
            was_replayed: row.try_get("was_replayed").unwrap_or(false),
        })
    }
}

impl<'r> FromRow<'r, SqliteRow> for UserStatsRecord {
    fn from_row(row: &'r SqliteRow) -> Result<Self, Error> {
        Ok(Self {
            username: row.try_get("username")?,
            uploads: row.try_get("uploads")?,
            downloads: row.try_get("downloads")?,
            total_uploaded: row.try_get("total_uploaded")?,
            total_downloaded: row.try_get("total_downloaded")?,
            watched: row.try_get("watched")?,
            last_seen: row.try_get("last_seen")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

impl<'r> FromRow<'r, SqliteRow> for UserProjectionRecord {
    fn from_row(row: &'r SqliteRow) -> Result<Self, Error> {
        Ok(Self {
            username: row.try_get("username")?,
            watched: row.try_get("watched")?,
            status: row.try_get("status")?,
            average_speed: row.try_get("average_speed")?,
            upload_count: row.try_get("upload_count")?,
            file_count: row.try_get("file_count")?,
            directory_count: row.try_get("directory_count")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

impl<'r> FromRow<'r, SqliteRow> for RoomRecord {
    fn from_row(row: &'r SqliteRow) -> Result<Self, Error> {
        Ok(Self {
            name: row.try_get("name")?,
            owner: row.try_get("owner")?,
            subscribed: row.try_get("subscribed")?,
            joined_at: row.try_get("joined_at")?,
            last_activity: row.try_get("last_activity")?,
        })
    }
}

impl<'r> FromRow<'r, SqliteRow> for UserNoteRecord {
    fn from_row(row: &'r SqliteRow) -> Result<Self, Error> {
        Ok(Self {
            id: row.try_get("id")?,
            username: row.try_get("username")?,
            note: row.try_get("note")?,
            color: row.try_get("color")?,
            icon: row.try_get("icon")?,
            is_high_priority: row.try_get("is_high_priority")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

impl<'r> FromRow<'r, SqliteRow> for InterestRecord {
    fn from_row(row: &'r SqliteRow) -> Result<Self, Error> {
        Ok(Self {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            kind: row.try_get("kind")?,
            created_at: row.try_get("created_at")?,
        })
    }
}

impl<'r> FromRow<'r, SqliteRow> for SecurityBanRecord {
    fn from_row(row: &'r SqliteRow) -> Result<Self, Error> {
        Ok(Self {
            kind: row.try_get("kind")?,
            value: row.try_get("value")?,
            created_at: row.try_get("created_at")?,
            reason: row.try_get("reason")?,
            expires_at: row.try_get("expires_at")?,
            is_permanent: row.try_get("is_permanent")?,
        })
    }
}

impl<'r> FromRow<'r, SqliteRow> for WishlistItemRecord {
    fn from_row(row: &'r SqliteRow) -> Result<Self, Error> {
        Ok(Self {
            id: row.try_get("id")?,
            artist: row.try_get("artist")?,
            title: row.try_get("title")?,
            kind: row.try_get("kind")?,
            filter: row.try_get("filter")?,
            enabled: row.try_get("enabled")?,
            auto_download: row.try_get("auto_download")?,
            max_results: row.try_get("max_results")?,
            max_downloads: row.try_get("max_downloads")?,
            last_viewed_at: row.try_get("last_viewed_at")?,
            last_searched_at: row.try_get("last_searched_at")?,
            last_match_count: row.try_get("last_match_count")?,
            last_visible_hit_count: row.try_get("last_visible_hit_count")?,
            last_hidden_locked_hit_count: row.try_get("last_hidden_locked_hit_count")?,
            last_filtered_out_hit_count: row.try_get("last_filtered_out_hit_count")?,
            last_ignored_result_hit_count: row.try_get("last_ignored_result_hit_count")?,
            last_response_count: row.try_get("last_response_count")?,
            total_search_count: row.try_get("total_search_count")?,
            total_download_count: row.try_get("total_download_count")?,
            last_search_id: row.try_get("last_search_id")?,
            lidarr_album_id: row.try_get("lidarr_album_id")?,
            lidarr_track_id: row.try_get("lidarr_track_id")?,
            lidarr_track_count: row.try_get("lidarr_track_count")?,
            lidarr_duration_seconds: row.try_get("lidarr_duration_seconds")?,
            lidarr_release_disambiguation: row.try_get("lidarr_release_disambiguation")?,
            added_at: row.try_get("added_at")?,
        })
    }
}

impl<'r> FromRow<'r, SqliteRow> for WishlistIgnoredResultRecord {
    fn from_row(row: &'r SqliteRow) -> Result<Self, Error> {
        Ok(Self {
            id: row.try_get("id")?,
            wishlist_item_id: row.try_get("wishlist_item_id")?,
            username: row.try_get("username")?,
            directory: row.try_get("directory")?,
            created_at: row.try_get("created_at")?,
        })
    }
}

impl<'r> FromRow<'r, SqliteRow> for ContactRecord {
    fn from_row(row: &'r SqliteRow) -> Result<Self, Error> {
        Ok(Self {
            id: row.try_get("id")?,
            username: row.try_get("username")?,
            online: row.try_get("online")?,
            status: row.try_get("status")?,
            free_upload_slots: row.try_get("free_upload_slots")?,
            queue_length: row.try_get("queue_length")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

impl<'r> FromRow<'r, SqliteRow> for ShareGrantRecord {
    fn from_row(row: &'r SqliteRow) -> Result<Self, Error> {
        Ok(Self {
            id: row.try_get("id")?,
            collection_id: row.try_get("collection_id")?,
            username: row.try_get("username")?,
            shared_at: row.try_get("shared_at")?,
            permissions: row.try_get("permissions")?,
        })
    }
}

impl<'r> FromRow<'r, SqliteRow> for ShareAccessTokenRecord {
    fn from_row(row: &'r SqliteRow) -> Result<Self, Error> {
        Ok(Self {
            token_digest: row.try_get("token_digest")?,
            grant_id: row.try_get("grant_id")?,
            expires_at: row.try_get("expires_at")?,
        })
    }
}

impl<'r> FromRow<'r, SqliteRow> for ShareGroupRecord {
    fn from_row(row: &'r SqliteRow) -> Result<Self, Error> {
        Ok(Self {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            description: row.try_get("description")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

impl<'r> FromRow<'r, SqliteRow> for ShareGroupMemberRecord {
    fn from_row(row: &'r SqliteRow) -> Result<Self, Error> {
        Ok(Self {
            group_id: row.try_get("group_id")?,
            username: row.try_get("username")?,
            added_at: row.try_get("added_at")?,
        })
    }
}

impl<'r> FromRow<'r, SqliteRow> for CollectionRecord {
    fn from_row(row: &'r SqliteRow) -> Result<Self, Error> {
        Ok(Self {
            id: row.try_get("id")?,
            owner_user_id: row.try_get("owner_user_id")?,
            name: row.try_get("name")?,
            description: row.try_get("description")?,
            collection_type: row.try_get("collection_type")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

impl<'r> FromRow<'r, SqliteRow> for CollectionItemRecord {
    fn from_row(row: &'r SqliteRow) -> Result<Self, Error> {
        Ok(Self {
            id: row.try_get("id")?,
            collection_id: row.try_get("collection_id")?,
            content_id: row.try_get("content_id")?,
            artist: row.try_get("artist")?,
            title: row.try_get("title")?,
            kind: row.try_get("kind")?,
            file_name: row.try_get("file_name")?,
            album: row.try_get("album")?,
            content_hash: row.try_get("content_hash")?,
            added_at: row.try_get("added_at")?,
            position: row.try_get("position")?,
        })
    }
}

impl<'r> FromRow<'r, SqliteRow> for LibraryItemRecord {
    fn from_row(row: &'r SqliteRow) -> Result<Self, Error> {
        Ok(Self {
            id: row.try_get("id")?,
            artist: row.try_get("artist")?,
            title: row.try_get("title")?,
            kind: row.try_get("kind")?,
            created_at: row.try_get("created_at")?,
        })
    }
}

impl<'r> FromRow<'r, SqliteRow> for DestinationRecord {
    fn from_row(row: &'r SqliteRow) -> Result<Self, Error> {
        Ok(Self {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            path: row.try_get("path")?,
            is_default: row.try_get("is_default")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

impl<'r> FromRow<'r, SqliteRow> for NowPlayingRecord {
    fn from_row(row: &'r SqliteRow) -> Result<Self, Error> {
        Ok(Self {
            username: row.try_get("username")?,
            artist: row.try_get("artist")?,
            title: row.try_get("title")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

impl<'r> FromRow<'r, SqliteRow> for BrowseRecord {
    fn from_row(row: &'r SqliteRow) -> Result<Self, Error> {
        Ok(Self {
            username: row.try_get("username")?,
            status: row.try_get("status")?,
            entries_json: row.try_get("entries_json")?,
            reason: row.try_get("reason")?,
            folder: row.try_get("folder")?,
            indirect_token: row.try_get("indirect_token")?,
            requested_at: row.try_get("requested_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

impl<'r> FromRow<'r, SqliteRow> for RuntimeCompatRecord {
    fn from_row(row: &'r SqliteRow) -> Result<Self, Error> {
        Ok(Self {
            id: row.try_get("id")?,
            application_restart_requested: row.try_get("application_restart_requested")?,
            gc_runs: row.try_get("gc_runs")?,
            autoreplace_enabled: row.try_get("autoreplace_enabled")?,
            relay_enabled: row.try_get("relay_enabled")?,
            relay_agent_enabled: row.try_get("relay_agent_enabled")?,
            bridge_running: row.try_get("bridge_running")?,
            bridge_config_updates: row.try_get("bridge_config_updates")?,
            options_updates: row.try_get("options_updates")?,
            options_yaml_uploads: row.try_get("options_yaml_uploads")?,
            options_yaml_validations: row.try_get("options_yaml_validations")?,
            profile_invites_created: row.try_get("profile_invites_created")?,
            cache_warm_runs: row.try_get("cache_warm_runs")?,
            backfill_runs: row.try_get("backfill_runs")?,
            songid_runs: row.try_get("songid_runs")?,
            songid_run_records_json: row
                .try_get("songid_run_records_json")
                .unwrap_or_else(|_| "[]".to_owned()),
            lidarr_sync_runs: row.try_get("lidarr_sync_runs")?,
            lidarr_manual_imports: row.try_get("lidarr_manual_imports")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

impl<'r> FromRow<'r, SqliteRow> for OAuthStateRecord {
    fn from_row(row: &'r SqliteRow) -> Result<Self, Error> {
        Ok(Self {
            state: row.try_get("state")?,
            provider: row.try_get("provider")?,
            redirect_uri: row.try_get("redirect_uri")?,
            created_at: row.try_get("created_at")?,
            expires_at: row.try_get("expires_at")?,
        })
    }
}

impl<'r> FromRow<'r, SqliteRow> for WebhookRecord {
    fn from_row(row: &'r SqliteRow) -> Result<Self, Error> {
        Ok(Self {
            id: row.try_get("id")?,
            url: row.try_get("url")?,
            events: row.try_get("events")?,
            secret: row.try_get("secret")?,
            active: row.try_get("active")?,
            created_at: row.try_get("created_at")?,
            last_triggered: row.try_get("last_triggered")?,
            retry_count: row.try_get("retry_count")?,
            max_retries: row.try_get("max_retries")?,
            timeout_seconds: row.try_get("timeout_seconds")?,
        })
    }
}

impl<'r> FromRow<'r, SqliteRow> for WebhookLogRecord {
    fn from_row(row: &'r SqliteRow) -> Result<Self, Error> {
        Ok(Self {
            id: row.try_get("id")?,
            webhook_id: row.try_get("webhook_id")?,
            event: row.try_get("event")?,
            correlation_id: row.try_get("correlation_id")?,
            status: row.try_get("status")?,
            request_body: row.try_get("request_body")?,
            response_status: row.try_get("response_status")?,
            response_body: row.try_get("response_body")?,
            error_message: row.try_get("error_message")?,
            attempt: row.try_get("attempt")?,
            timestamp: row.try_get("timestamp")?,
        })
    }
}

/// SQLite-backed database manager
#[derive(Clone)]
pub struct DatabaseManager {
    pool: SqlitePool,
}

impl std::fmt::Debug for DatabaseManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DatabaseManager").finish()
    }
}

impl DatabaseManager {
    #[cfg(any(test, feature = "bounded-differential"))]
    pub async fn close_for_test(&self) {
        self.pool.close().await;
    }

    /// Executes an arbitrary raw SQL statement, bypassing every typed
    /// store method. Test-only: used to inject deliberately corrupt data
    /// (values a normal insert path could never produce, thanks to
    /// SQLite's weak column typing) so a differential test can prove the
    /// real rehydration path fails cleanly instead of panicking.
    #[cfg(any(test, feature = "bounded-differential"))]
    pub async fn execute_raw_for_test(&self, sql: &str) -> Result<(), Box<dyn std::error::Error>> {
        // sqlx's `query()` ties its statement cache key to a `'static`
        // str; leaking a small owned copy here is fine for a test-only
        // helper called a handful of times per test run.
        let sql: &'static str = Box::leak(sql.to_owned().into_boxed_str());
        query(sql).execute(&self.pool).await?;
        Ok(())
    }

    #[cfg(any(test, feature = "bounded-differential"))]
    pub async fn fail_oauth_delete_for_test(&self) -> Result<(), Box<dyn std::error::Error>> {
        query(
            r#"
            CREATE TRIGGER fail_oauth_delete
            BEFORE DELETE ON oauth_states
            BEGIN
                SELECT RAISE(ABORT, 'forced OAuth delete failure');
            END
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Create new database manager with SQLite backend
    pub async fn new(db_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        #[cfg(unix)]
        prepare_private_database_file(db_path)?;

        let connect_options = SqliteConnectOptions::new()
            .filename(db_path)
            .create_if_missing(true)
            // SQLite has a single-writer lock.  The controller records HTTP,
            // daemon, and protocol events concurrently, so let SQLite wait
            // briefly for the writer instead of surfacing transient lock
            // errors as a false session failure.
            .busy_timeout(Duration::from_secs(30));

        let pool = SqlitePoolOptions::new()
            // A single connection makes file-backed writes obey SQLite's
            // single-writer model and keeps event journaling deterministic
            // under concurrent Web UI/API traffic.
            .max_connections(1)
            .connect_with(connect_options)
            .await?;

        let manager = DatabaseManager { pool };
        manager.initialize().await?;
        Ok(manager)
    }

    /// In-memory database for testing.
    ///
    /// Pinned to a single pooled connection: sqlx keeps a `:memory:`
    /// database alive across a multi-connection pool via SQLite's shared
    /// cache mode, and shared cache mode's `SQLITE_LOCKED_SHAREDCACHE`
    /// error on concurrent cross-connection table writes is NOT retried
    /// by `busy_timeout` (that only covers `SQLITE_BUSY`, a documented
    /// SQLite limitation) -- multiple real concurrent writers would
    /// otherwise see spurious "database table is locked" errors that
    /// can't happen against a real single-writer file-backed database.
    /// One connection makes the pool serialize concurrent transactions
    /// by queuing the acquire instead of racing two live connections.
    pub async fn in_memory() -> Result<Self, Box<dyn std::error::Error>> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await?;
        let manager = DatabaseManager { pool };
        manager.initialize().await?;
        Ok(manager)
    }

    /// Initialize database schema
    async fn initialize(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Create searches table
        query(
            r#"
            CREATE TABLE IF NOT EXISTS searches (
                id TEXT PRIMARY KEY,
                query TEXT NOT NULL,
                status TEXT NOT NULL,
                result_count INTEGER DEFAULT 0,
                created_at INTEGER NOT NULL,
                completed_at INTEGER,
                room TEXT,
                target TEXT
                , fallback_attempts INTEGER NOT NULL DEFAULT 0
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        query(
            r#"
            CREATE TABLE IF NOT EXISTS search_identities (
                search_id TEXT PRIMARY KEY,
                external_id TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create durable search result table
        query(
            r#"
            CREATE TABLE IF NOT EXISTS search_results (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                search_id TEXT NOT NULL,
                peer_username TEXT,
                filename TEXT NOT NULL,
                size INTEGER NOT NULL,
                extension TEXT NOT NULL,
                bit_rate INTEGER,
                sample_rate INTEGER,
                bit_depth INTEGER,
                length_seconds INTEGER,
                locked INTEGER NOT NULL,
                slot_free INTEGER,
                average_speed INTEGER,
                queue_length INTEGER,
                created_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create transfers table
        query(
            r#"
            CREATE TABLE IF NOT EXISTS transfers (
                id TEXT PRIMARY KEY,
                direction TEXT NOT NULL,
                filename TEXT NOT NULL,
                peer_username TEXT NOT NULL,
                filesize INTEGER NOT NULL,
                progress INTEGER DEFAULT 0,
                status TEXT NOT NULL,
                started_at INTEGER NOT NULL,
                completed_at INTEGER
                , request_id TEXT
                , wishlist_item_id TEXT
                , request_name TEXT
                , destination_directory TEXT
                , local_path TEXT
                , batch_id TEXT
                , reason TEXT
                , bit_rate INTEGER
                , sample_rate INTEGER
                , bit_depth INTEGER
                , length_seconds INTEGER
                , artist TEXT
                , album TEXT
                , title TEXT
                , track_number INTEGER
                , year INTEGER
                , attempts INTEGER NOT NULL DEFAULT 1
                , auto_replace_attempts INTEGER NOT NULL DEFAULT 0
                , next_attempt_at INTEGER
                , updated_at_ms INTEGER NOT NULL DEFAULT 0
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create the durable controller-compatible transfer batch table. This is
        // intentionally separate from the generic controller feature store:
        // batch reads must fail with the Transfers database, and batch rows
        // must survive a process restart alongside their transfer records.
        query(
            r#"
            CREATE TABLE IF NOT EXISTS Batches (
                Id TEXT NOT NULL CONSTRAINT PK_Batches PRIMARY KEY,
                SearchId TEXT,
                Username TEXT,
                Direction INTEGER NOT NULL,
                CreatedAt TEXT NOT NULL,
                Options TEXT
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        query("CREATE INDEX IF NOT EXISTS IDX_Batches_SearchId ON Batches (SearchId)")
            .execute(&self.pool)
            .await?;

        // Core HashDb and cursor state tables.  The controller cache remains
        // useful for projections, but these tables are the durable source of
        // truth for hash entries and progress when SQLite persistence is on.
        query(
            r#"
            CREATE TABLE IF NOT EXISTS HashDb (
                flac_key TEXT PRIMARY KEY,
                byte_hash TEXT NOT NULL,
                size INTEGER NOT NULL,
                meta_flags INTEGER,
                first_seen_at INTEGER NOT NULL,
                last_updated_at INTEGER NOT NULL,
                seq_id INTEGER,
                use_count INTEGER DEFAULT 1,
                full_file_hash TEXT,
                musicbrainz_id TEXT,
                file_sha256 TEXT
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        query(
            r#"
            CREATE TABLE IF NOT EXISTS HashDbState (
                key TEXT PRIMARY KEY,
                value TEXT
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        query(
            r#"
            CREATE TABLE IF NOT EXISTS TrafficStats (
                key TEXT PRIMARY KEY,
                overlay_upload_bytes INTEGER NOT NULL DEFAULT 0,
                overlay_download_bytes INTEGER NOT NULL DEFAULT 0,
                soulseek_upload_bytes INTEGER NOT NULL DEFAULT 0,
                soulseek_download_bytes INTEGER NOT NULL DEFAULT 0,
                updated_at INTEGER NOT NULL DEFAULT 0
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        for statement in [
            "CREATE INDEX IF NOT EXISTS idx_hashdb_size ON HashDb(size)",
            "CREATE INDEX IF NOT EXISTS idx_hashdb_seq ON HashDb(seq_id)",
            "CREATE INDEX IF NOT EXISTS idx_hashdb_hash ON HashDb(byte_hash)",
        ] {
            query(statement).execute(&self.pool).await?;
        }

        // Create durable transfer event trail table
        query(
            r#"
            CREATE TABLE IF NOT EXISTS transfer_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                transfer_id TEXT NOT NULL,
                direction TEXT NOT NULL,
                token INTEGER NOT NULL,
                filename TEXT NOT NULL,
                peer_username TEXT,
                filesize INTEGER NOT NULL,
                progress INTEGER NOT NULL,
                status TEXT NOT NULL,
                reason TEXT,
                created_at INTEGER NOT NULL,
                updated_at_ms INTEGER NOT NULL DEFAULT 0
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        query(
            r#"
            CREATE TABLE IF NOT EXISTS transfer_tombstones (
                id TEXT PRIMARY KEY,
                deleted_at_ms INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create durable share index table
        query(
            r#"
            CREATE TABLE IF NOT EXISTS share_files (
                filename TEXT PRIMARY KEY,
                size INTEGER NOT NULL,
                extension TEXT NOT NULL,
                root_label TEXT NOT NULL,
                local_path TEXT,
                attributes_json TEXT NOT NULL DEFAULT '[]',
                updated_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create durable runtime event log table
        query(
            r#"
            CREATE TABLE IF NOT EXISTS events (
                id INTEGER PRIMARY KEY,
                kind TEXT NOT NULL,
                resource TEXT NOT NULL,
                detail TEXT,
                created_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create messages table
        query(
            r#"
            CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                username TEXT NOT NULL,
                content TEXT NOT NULL,
                direction TEXT NOT NULL,
                read INTEGER DEFAULT 0,
                created_at INTEGER NOT NULL,
                source_id INTEGER,
                source_timestamp INTEGER,
                was_replayed INTEGER DEFAULT 0
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        for statement in [
            "ALTER TABLE messages ADD COLUMN source_id INTEGER",
            "ALTER TABLE messages ADD COLUMN source_timestamp INTEGER",
            "ALTER TABLE messages ADD COLUMN was_replayed INTEGER DEFAULT 0",
        ] {
            if let Err(error) = query(statement).execute(&self.pool).await {
                if !error.to_string().contains("duplicate column name") {
                    return Err(error.into());
                }
            }
        }

        // Create user stats table
        query(
            r#"
            CREATE TABLE IF NOT EXISTS user_stats (
                username TEXT PRIMARY KEY,
                uploads INTEGER DEFAULT 0,
                downloads INTEGER DEFAULT 0,
                total_uploaded INTEGER DEFAULT 0,
                total_downloaded INTEGER DEFAULT 0,
                watched INTEGER DEFAULT 0,
                last_seen INTEGER,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create user projection table
        query(
            r#"
            CREATE TABLE IF NOT EXISTS user_records (
                username TEXT PRIMARY KEY,
                watched INTEGER DEFAULT 0,
                status TEXT,
                average_speed INTEGER,
                upload_count INTEGER,
                file_count INTEGER,
                directory_count INTEGER,
                updated_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create rooms table
        query(
            r#"
            CREATE TABLE IF NOT EXISTS rooms (
                name TEXT PRIMARY KEY,
                owner TEXT,
                subscribed INTEGER DEFAULT 0,
                joined_at INTEGER NOT NULL,
                last_activity INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create user notes table
        query(
            r#"
            CREATE TABLE IF NOT EXISTS user_notes (
                id TEXT PRIMARY KEY,
                username TEXT NOT NULL,
                note TEXT NOT NULL,
                color TEXT NOT NULL DEFAULT '',
                icon TEXT NOT NULL DEFAULT '',
                is_high_priority INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create interests table
        query(
            r#"
            CREATE TABLE IF NOT EXISTS interests (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                kind TEXT NOT NULL,
                created_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create security bans table
        query(
            r#"
            CREATE TABLE IF NOT EXISTS security_bans (
                kind TEXT NOT NULL,
                value TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                reason TEXT NOT NULL DEFAULT 'Manual ban',
                expires_at INTEGER NOT NULL DEFAULT 0,
                is_permanent INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (kind, value)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create wishlist items table
        query(
            r#"
            CREATE TABLE IF NOT EXISTS wishlist_items (
                id TEXT PRIMARY KEY,
                artist TEXT NOT NULL,
                title TEXT NOT NULL,
                kind TEXT NOT NULL,
                filter TEXT NOT NULL DEFAULT '',
                enabled INTEGER NOT NULL DEFAULT 1,
                auto_download INTEGER NOT NULL DEFAULT 0,
                max_results INTEGER NOT NULL DEFAULT 100,
                max_downloads INTEGER,
                last_viewed_at INTEGER,
                last_searched_at INTEGER,
                last_match_count INTEGER NOT NULL DEFAULT 0,
                last_visible_hit_count INTEGER NOT NULL DEFAULT 0,
                last_hidden_locked_hit_count INTEGER NOT NULL DEFAULT 0,
                last_filtered_out_hit_count INTEGER NOT NULL DEFAULT 0,
                last_ignored_result_hit_count INTEGER NOT NULL DEFAULT 0,
                last_response_count INTEGER NOT NULL DEFAULT 0,
                total_search_count INTEGER NOT NULL DEFAULT 0,
                total_download_count INTEGER NOT NULL DEFAULT 0,
                last_search_id TEXT,
                lidarr_album_id INTEGER,
                lidarr_track_id INTEGER,
                lidarr_track_count INTEGER,
                lidarr_duration_seconds INTEGER,
                lidarr_release_disambiguation TEXT,
                added_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        query(
            r#"
            CREATE TABLE IF NOT EXISTS wishlist_ignored_results (
                id TEXT PRIMARY KEY,
                wishlist_item_id TEXT NOT NULL,
                username TEXT COLLATE NOCASE NOT NULL,
                directory TEXT COLLATE NOCASE NOT NULL,
                created_at INTEGER NOT NULL,
                UNIQUE (wishlist_item_id, username, directory),
                FOREIGN KEY (wishlist_item_id) REFERENCES wishlist_items(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create contacts table
        query(
            r#"
            CREATE TABLE IF NOT EXISTS contacts (
                id TEXT PRIMARY KEY,
                username TEXT NOT NULL,
                online INTEGER DEFAULT 0,
                status TEXT NOT NULL,
                free_upload_slots INTEGER,
                queue_length INTEGER,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create share grants table
        query(
            r#"
            CREATE TABLE IF NOT EXISTS share_grants (
                id TEXT PRIMARY KEY,
                collection_id TEXT NOT NULL,
                username TEXT NOT NULL,
                shared_at INTEGER NOT NULL,
                permissions TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        query(
            r#"
            CREATE TABLE IF NOT EXISTS share_access_tokens (
                token_digest TEXT PRIMARY KEY,
                grant_id TEXT NOT NULL,
                expires_at INTEGER NOT NULL,
                FOREIGN KEY (grant_id) REFERENCES share_grants(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create share groups table
        query(
            r#"
            CREATE TABLE IF NOT EXISTS share_groups (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create share group members table
        query(
            r#"
            CREATE TABLE IF NOT EXISTS share_group_members (
                group_id TEXT NOT NULL,
                username TEXT NOT NULL,
                added_at INTEGER NOT NULL,
                PRIMARY KEY (group_id, username),
                FOREIGN KEY (group_id) REFERENCES share_groups(id)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create collections table
        query(
            r#"
            CREATE TABLE IF NOT EXISTS collections (
                id TEXT PRIMARY KEY,
                owner_user_id TEXT NOT NULL DEFAULT '',
                name TEXT NOT NULL,
                description TEXT NOT NULL,
                collection_type TEXT NOT NULL DEFAULT 'ShareList',
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create collection items table
        query(
            r#"
            CREATE TABLE IF NOT EXISTS collection_items (
                id TEXT PRIMARY KEY,
                collection_id TEXT NOT NULL,
                content_id TEXT NOT NULL,
                artist TEXT NOT NULL,
                title TEXT NOT NULL,
                kind TEXT NOT NULL,
                file_name TEXT NOT NULL DEFAULT '',
                album TEXT NOT NULL DEFAULT '',
                content_hash TEXT NOT NULL DEFAULT '',
                added_at INTEGER NOT NULL,
                position INTEGER NOT NULL,
                FOREIGN KEY (collection_id) REFERENCES collections(id)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create library items table
        query(
            r#"
            CREATE TABLE IF NOT EXISTS library_items (
                id TEXT PRIMARY KEY,
                artist TEXT NOT NULL,
                title TEXT NOT NULL,
                kind TEXT NOT NULL,
                created_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create destinations table
        query(
            r#"
            CREATE TABLE IF NOT EXISTS destinations (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                path TEXT NOT NULL,
                is_default INTEGER DEFAULT 0,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create now-playing table
        query(
            r#"
            CREATE TABLE IF NOT EXISTS now_playing (
                username TEXT PRIMARY KEY,
                artist TEXT NOT NULL,
                title TEXT NOT NULL,
                updated_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create browse cache table
        query(
            r#"
            CREATE TABLE IF NOT EXISTS browse_records (
                username TEXT PRIMARY KEY,
                status TEXT NOT NULL,
                entries_json TEXT NOT NULL,
                reason TEXT,
                folder TEXT,
                indirect_token INTEGER,
                requested_at INTEGER,
                updated_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create runtime compatibility singleton state table
        query(
            r#"
            CREATE TABLE IF NOT EXISTS runtime_compat_state (
                id TEXT PRIMARY KEY,
                application_restart_requested INTEGER DEFAULT 0,
                gc_runs INTEGER NOT NULL,
                autoreplace_enabled INTEGER DEFAULT 0,
                relay_enabled INTEGER DEFAULT 0,
                relay_agent_enabled INTEGER DEFAULT 0,
                bridge_running INTEGER DEFAULT 0,
                bridge_config_updates INTEGER NOT NULL,
                options_updates INTEGER NOT NULL DEFAULT 0,
                options_yaml_uploads INTEGER NOT NULL DEFAULT 0,
                options_yaml_validations INTEGER NOT NULL DEFAULT 0,
                profile_invites_created INTEGER NOT NULL,
                cache_warm_runs INTEGER NOT NULL,
                backfill_runs INTEGER NOT NULL,
                songid_runs INTEGER NOT NULL,
                songid_run_records_json TEXT NOT NULL DEFAULT '[]',
                lidarr_sync_runs INTEGER NOT NULL,
                lidarr_manual_imports INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        self.ensure_runtime_compat_columns().await?;
        self.ensure_search_columns().await?;
        self.ensure_security_ban_columns().await?;
        self.ensure_wishlist_item_columns().await?;
        self.ensure_collection_columns().await?;
        self.ensure_collection_item_columns().await?;
        self.ensure_user_note_columns().await?;
        self.ensure_transfer_columns().await?;
        self.ensure_share_file_columns().await?;

        // Create pending OAuth states table
        query(
            r#"
            CREATE TABLE IF NOT EXISTS oauth_states (
                state TEXT PRIMARY KEY,
                provider TEXT NOT NULL,
                redirect_uri TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                expires_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create webhooks table
        query(
            r#"
            CREATE TABLE IF NOT EXISTS webhooks (
                id TEXT PRIMARY KEY,
                url TEXT NOT NULL,
                events TEXT NOT NULL,
                secret TEXT NOT NULL,
                active INTEGER DEFAULT 1,
                created_at INTEGER NOT NULL,
                last_triggered INTEGER,
                retry_count INTEGER DEFAULT 0,
                max_retries INTEGER DEFAULT 3,
                timeout_seconds INTEGER DEFAULT 30
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create webhook logs table
        query(
            r#"
            CREATE TABLE IF NOT EXISTS webhook_logs (
                id TEXT PRIMARY KEY,
                webhook_id TEXT NOT NULL,
                event TEXT NOT NULL,
                correlation_id TEXT NOT NULL,
                status TEXT NOT NULL,
                request_body TEXT NOT NULL,
                response_status INTEGER,
                response_body TEXT,
                error_message TEXT,
                attempt INTEGER DEFAULT 1,
                timestamp INTEGER NOT NULL,
                FOREIGN KEY (webhook_id) REFERENCES webhooks(id)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        query(
            r#"
            CREATE TABLE IF NOT EXISTS wishlist_scheduler_state (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                next_index INTEGER NOT NULL DEFAULT 0,
                server_interval_seconds INTEGER,
                updated_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        query(
            r#"
            CREATE TABLE IF NOT EXISTS distributed_tree_state (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                branch_level INTEGER NOT NULL DEFAULT 0,
                branch_root TEXT NOT NULL,
                parent_username TEXT,
                updated_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        query(
            r#"
            CREATE TABLE IF NOT EXISTS distributed_children (
                username TEXT PRIMARY KEY,
                depth INTEGER NOT NULL DEFAULT 0,
                updated_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create indices for common queries
        query("CREATE INDEX IF NOT EXISTS idx_searches_created ON searches(created_at DESC)")
            .execute(&self.pool)
            .await?;

        query("CREATE INDEX IF NOT EXISTS idx_search_results_search ON search_results(search_id)")
            .execute(&self.pool)
            .await?;

        // `id` is an INTEGER PRIMARY KEY alias for rowid. SQLite already
        // appends rowid to the existing search_id index, so this explicit
        // composite index duplicates storage and write work without helping
        // the ordered search-result pages.
        query("DROP INDEX IF EXISTS idx_search_results_search_id")
            .execute(&self.pool)
            .await?;

        query("CREATE INDEX IF NOT EXISTS idx_transfers_started ON transfers(started_at DESC)")
            .execute(&self.pool)
            .await?;

        query(
            "CREATE INDEX IF NOT EXISTS idx_transfers_status_started ON transfers(status, started_at DESC)",
        )
        .execute(&self.pool)
        .await?;

        query(
            "CREATE INDEX IF NOT EXISTS idx_transfer_events_created ON transfer_events(created_at DESC)",
        )
        .execute(&self.pool)
        .await?;

        query(
            "CREATE INDEX IF NOT EXISTS idx_transfer_events_transfer ON transfer_events(transfer_id)",
        )
        .execute(&self.pool)
        .await?;

        query(
            "CREATE INDEX IF NOT EXISTS idx_transfer_events_order ON transfer_events(transfer_id, updated_at_ms DESC, id DESC)",
        )
        .execute(&self.pool)
        .await?;

        query("CREATE INDEX IF NOT EXISTS idx_share_files_root ON share_files(root_label)")
            .execute(&self.pool)
            .await?;

        query("CREATE INDEX IF NOT EXISTS idx_share_files_extension ON share_files(extension)")
            .execute(&self.pool)
            .await?;

        query("CREATE INDEX IF NOT EXISTS idx_events_created ON events(created_at DESC)")
            .execute(&self.pool)
            .await?;

        query("CREATE INDEX IF NOT EXISTS idx_events_kind ON events(kind)")
            .execute(&self.pool)
            .await?;

        query("CREATE INDEX IF NOT EXISTS idx_messages_username ON messages(username)")
            .execute(&self.pool)
            .await?;

        query("CREATE INDEX IF NOT EXISTS idx_messages_created ON messages(created_at DESC)")
            .execute(&self.pool)
            .await?;

        query(
            "CREATE INDEX IF NOT EXISTS idx_messages_username_created ON messages(username, created_at DESC)",
        )
        .execute(&self.pool)
        .await?;

        query("CREATE INDEX IF NOT EXISTS idx_webhooks_active ON webhooks(active)")
            .execute(&self.pool)
            .await?;

        query("CREATE INDEX IF NOT EXISTS idx_oauth_states_expires ON oauth_states(expires_at)")
            .execute(&self.pool)
            .await?;

        query("CREATE INDEX IF NOT EXISTS idx_user_notes_username ON user_notes(username)")
            .execute(&self.pool)
            .await?;

        query("CREATE INDEX IF NOT EXISTS idx_interests_kind ON interests(kind)")
            .execute(&self.pool)
            .await?;

        query(
            "CREATE INDEX IF NOT EXISTS idx_wishlist_items_added ON wishlist_items(added_at DESC)",
        )
        .execute(&self.pool)
        .await?;

        query(
            "CREATE INDEX IF NOT EXISTS idx_wishlist_ignored_item ON wishlist_ignored_results(wishlist_item_id, created_at DESC)",
        )
        .execute(&self.pool)
        .await?;

        query("CREATE INDEX IF NOT EXISTS idx_contacts_username ON contacts(username)")
            .execute(&self.pool)
            .await?;

        query(
            "CREATE INDEX IF NOT EXISTS idx_share_grants_collection ON share_grants(collection_id)",
        )
        .execute(&self.pool)
        .await?;

        query(
            "CREATE INDEX IF NOT EXISTS idx_share_access_tokens_grant_expiry ON share_access_tokens(grant_id, expires_at)",
        )
        .execute(&self.pool)
        .await?;

        query("CREATE INDEX IF NOT EXISTS idx_share_group_members_username ON share_group_members(username)")
            .execute(&self.pool)
            .await?;

        query("CREATE INDEX IF NOT EXISTS idx_collection_items_collection ON collection_items(collection_id, position)")
            .execute(&self.pool)
            .await?;

        query("CREATE INDEX IF NOT EXISTS idx_library_items_artist ON library_items(artist)")
            .execute(&self.pool)
            .await?;

        query(
            "CREATE INDEX IF NOT EXISTS idx_library_items_created ON library_items(created_at DESC)",
        )
        .execute(&self.pool)
        .await?;

        query("CREATE INDEX IF NOT EXISTS idx_destinations_default ON destinations(is_default)")
            .execute(&self.pool)
            .await?;

        query("CREATE INDEX IF NOT EXISTS idx_now_playing_updated ON now_playing(updated_at DESC)")
            .execute(&self.pool)
            .await?;

        query("CREATE INDEX IF NOT EXISTS idx_browse_records_status ON browse_records(status, updated_at DESC)")
            .execute(&self.pool)
            .await?;

        query("CREATE INDEX IF NOT EXISTS idx_webhook_logs_webhook ON webhook_logs(webhook_id)")
            .execute(&self.pool)
            .await?;

        query("CREATE INDEX IF NOT EXISTS idx_webhook_logs_queued ON webhook_logs(status) WHERE status = 'queued'")
            .execute(&self.pool)
            .await?;

        query(
            "CREATE INDEX IF NOT EXISTS idx_webhook_logs_webhook_timestamp ON webhook_logs(webhook_id, timestamp DESC)",
        )
        .execute(&self.pool)
        .await?;

        query(
            "CREATE INDEX IF NOT EXISTS idx_webhook_logs_timestamp ON webhook_logs(timestamp DESC)",
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn ensure_runtime_compat_columns(&self) -> Result<(), Box<dyn std::error::Error>> {
        for statement in [
            "ALTER TABLE runtime_compat_state ADD COLUMN options_updates INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE runtime_compat_state ADD COLUMN options_yaml_uploads INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE runtime_compat_state ADD COLUMN options_yaml_validations INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE runtime_compat_state ADD COLUMN songid_run_records_json TEXT NOT NULL DEFAULT '[]'",
        ] {
            if let Err(error) = query(statement).execute(&self.pool).await {
                let message = error.to_string();
                if !message.contains("duplicate column name") {
                    return Err(Box::new(error));
                }
            }
        }
        Ok(())
    }

    async fn ensure_search_columns(&self) -> Result<(), Box<dyn std::error::Error>> {
        for statement in [
            "ALTER TABLE searches ADD COLUMN fallback_attempts INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE search_results ADD COLUMN bit_rate INTEGER",
            "ALTER TABLE search_results ADD COLUMN sample_rate INTEGER",
            "ALTER TABLE search_results ADD COLUMN bit_depth INTEGER",
            "ALTER TABLE search_results ADD COLUMN length_seconds INTEGER",
        ] {
            if let Err(error) = query(statement).execute(&self.pool).await {
                if !error.to_string().contains("duplicate column name") {
                    return Err(Box::new(error));
                }
            }
        }
        Ok(())
    }

    async fn ensure_security_ban_columns(&self) -> Result<(), Box<dyn std::error::Error>> {
        for statement in [
            "ALTER TABLE security_bans ADD COLUMN reason TEXT NOT NULL DEFAULT 'Manual ban'",
            "ALTER TABLE security_bans ADD COLUMN expires_at INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE security_bans ADD COLUMN is_permanent INTEGER NOT NULL DEFAULT 0",
        ] {
            if let Err(error) = query(statement).execute(&self.pool).await {
                if !error.to_string().contains("duplicate column name") {
                    return Err(Box::new(error));
                }
            }
        }
        query("UPDATE security_bans SET expires_at = created_at + 3600 WHERE expires_at = 0")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn ensure_wishlist_item_columns(&self) -> Result<(), Box<dyn std::error::Error>> {
        for statement in [
            "ALTER TABLE wishlist_items ADD COLUMN filter TEXT NOT NULL DEFAULT ''",
            "ALTER TABLE wishlist_items ADD COLUMN enabled INTEGER NOT NULL DEFAULT 1",
            "ALTER TABLE wishlist_items ADD COLUMN auto_download INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE wishlist_items ADD COLUMN max_results INTEGER NOT NULL DEFAULT 100",
            "ALTER TABLE wishlist_items ADD COLUMN max_downloads INTEGER",
            "ALTER TABLE wishlist_items ADD COLUMN last_viewed_at INTEGER",
            "ALTER TABLE wishlist_items ADD COLUMN last_searched_at INTEGER",
            "ALTER TABLE wishlist_items ADD COLUMN last_match_count INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE wishlist_items ADD COLUMN last_visible_hit_count INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE wishlist_items ADD COLUMN last_hidden_locked_hit_count INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE wishlist_items ADD COLUMN last_filtered_out_hit_count INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE wishlist_items ADD COLUMN last_ignored_result_hit_count INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE wishlist_items ADD COLUMN last_response_count INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE wishlist_items ADD COLUMN total_search_count INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE wishlist_items ADD COLUMN total_download_count INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE wishlist_items ADD COLUMN last_search_id TEXT",
            "ALTER TABLE wishlist_items ADD COLUMN lidarr_album_id INTEGER",
            "ALTER TABLE wishlist_items ADD COLUMN lidarr_track_id INTEGER",
            "ALTER TABLE wishlist_items ADD COLUMN lidarr_track_count INTEGER",
            "ALTER TABLE wishlist_items ADD COLUMN lidarr_duration_seconds INTEGER",
            "ALTER TABLE wishlist_items ADD COLUMN lidarr_release_disambiguation TEXT",
        ] {
            if let Err(error) = query(statement).execute(&self.pool).await {
                let message = error.to_string();
                if !message.contains("duplicate column name") {
                    return Err(Box::new(error));
                }
            }
        }
        Ok(())
    }

    async fn ensure_collection_columns(&self) -> Result<(), Box<dyn std::error::Error>> {
        for statement in [
            "ALTER TABLE collections ADD COLUMN owner_user_id TEXT NOT NULL DEFAULT ''",
            "ALTER TABLE collections ADD COLUMN collection_type TEXT NOT NULL DEFAULT 'ShareList'",
        ] {
            if let Err(error) = query(statement).execute(&self.pool).await {
                if !error.to_string().contains("duplicate column name") {
                    return Err(Box::new(error));
                }
            }
        }
        Ok(())
    }

    async fn ensure_collection_item_columns(&self) -> Result<(), Box<dyn std::error::Error>> {
        for statement in [
            "ALTER TABLE collection_items ADD COLUMN file_name TEXT NOT NULL DEFAULT ''",
            "ALTER TABLE collection_items ADD COLUMN album TEXT NOT NULL DEFAULT ''",
            "ALTER TABLE collection_items ADD COLUMN content_hash TEXT NOT NULL DEFAULT ''",
        ] {
            if let Err(error) = query(statement).execute(&self.pool).await {
                if !error.to_string().contains("duplicate column name") {
                    return Err(Box::new(error));
                }
            }
        }
        Ok(())
    }

    async fn ensure_user_note_columns(&self) -> Result<(), Box<dyn std::error::Error>> {
        for statement in [
            "ALTER TABLE user_notes ADD COLUMN color TEXT NOT NULL DEFAULT ''",
            "ALTER TABLE user_notes ADD COLUMN icon TEXT NOT NULL DEFAULT ''",
            "ALTER TABLE user_notes ADD COLUMN is_high_priority INTEGER NOT NULL DEFAULT 0",
        ] {
            if let Err(error) = query(statement).execute(&self.pool).await {
                if !error.to_string().contains("duplicate column name") {
                    return Err(error.into());
                }
            }
        }
        Ok(())
    }

    async fn ensure_transfer_columns(&self) -> Result<(), Box<dyn std::error::Error>> {
        for statement in [
            "ALTER TABLE transfers ADD COLUMN request_id TEXT",
            "ALTER TABLE transfers ADD COLUMN wishlist_item_id TEXT",
            "ALTER TABLE transfers ADD COLUMN request_name TEXT",
            "ALTER TABLE transfers ADD COLUMN destination_directory TEXT",
            "ALTER TABLE transfers ADD COLUMN local_path TEXT",
            "ALTER TABLE transfers ADD COLUMN batch_id TEXT",
            "ALTER TABLE transfers ADD COLUMN reason TEXT",
            "ALTER TABLE transfers ADD COLUMN bit_rate INTEGER",
            "ALTER TABLE transfers ADD COLUMN sample_rate INTEGER",
            "ALTER TABLE transfers ADD COLUMN bit_depth INTEGER",
            "ALTER TABLE transfers ADD COLUMN length_seconds INTEGER",
            "ALTER TABLE transfers ADD COLUMN artist TEXT",
            "ALTER TABLE transfers ADD COLUMN album TEXT",
            "ALTER TABLE transfers ADD COLUMN title TEXT",
            "ALTER TABLE transfers ADD COLUMN track_number INTEGER",
            "ALTER TABLE transfers ADD COLUMN year INTEGER",
            "ALTER TABLE transfers ADD COLUMN attempts INTEGER NOT NULL DEFAULT 1",
            "ALTER TABLE transfers ADD COLUMN auto_replace_attempts INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE transfers ADD COLUMN next_attempt_at INTEGER",
            "ALTER TABLE transfers ADD COLUMN updated_at_ms INTEGER NOT NULL DEFAULT 0",
        ] {
            if let Err(error) = query(statement).execute(&self.pool).await {
                if !error.to_string().contains("duplicate column name") {
                    return Err(Box::new(error));
                }
            }
        }
        if let Err(error) =
            query("ALTER TABLE transfer_events ADD COLUMN updated_at_ms INTEGER NOT NULL DEFAULT 0")
                .execute(&self.pool)
                .await
        {
            if !error.to_string().contains("duplicate column name") {
                return Err(Box::new(error));
            }
        }
        query("CREATE INDEX IF NOT EXISTS idx_transfers_request ON transfers(request_id, started_at DESC)")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn ensure_share_file_columns(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Err(error) =
            query("ALTER TABLE share_files ADD COLUMN attributes_json TEXT NOT NULL DEFAULT '[]'")
                .execute(&self.pool)
                .await
        {
            if !error.to_string().contains("duplicate column name") {
                return Err(error.into());
            }
        }
        Ok(())
    }
}

#[path = "persistence_activity.rs"]
mod activity;
#[path = "persistence_collaboration.rs"]
mod collaboration;
#[path = "persistence_distributed.rs"]
mod distributed;
#[path = "persistence_hashdb.rs"]
mod hashdb;
#[path = "persistence_library_runtime.rs"]
mod library_runtime;
#[path = "persistence_oauth.rs"]
mod oauth;
#[path = "persistence_people.rs"]
mod people;
#[path = "persistence_search.rs"]
mod search;
#[path = "persistence_transfer_batches.rs"]
mod transfer_batches;
#[path = "persistence_transfers.rs"]
mod transfers;
#[path = "persistence_webhooks.rs"]
mod webhooks;
#[path = "persistence_wishlist.rs"]
mod wishlist;

fn nonnegative_database_count(value: i64) -> Result<u64, std::num::TryFromIntError> {
    u64::try_from(value)
}

/// Database statistics
#[derive(Clone, Debug, Serialize)]
pub struct DatabaseStats {
    pub search_count: u64,
    pub search_result_count: u64,
    pub transfer_count: u64,
    pub transfer_event_count: u64,
    pub share_file_count: u64,
    pub event_count: u64,
    pub message_count: u64,
    pub user_count: u64,
    pub user_projection_count: u64,
    pub room_count: u64,
    pub user_note_count: u64,
    pub interest_count: u64,
    pub security_ban_count: u64,
    pub wishlist_count: u64,
    pub contact_count: u64,
    pub share_grant_count: u64,
    pub share_access_token_count: u64,
    pub share_group_count: u64,
    pub share_group_member_count: u64,
    pub collection_count: u64,
    pub collection_item_count: u64,
    pub library_item_count: u64,
    pub destination_count: u64,
    pub now_playing_count: u64,
    pub browse_count: u64,
    pub runtime_state_count: u64,
    pub oauth_state_count: u64,
    pub webhook_count: u64,
    pub webhook_log_count: u64,
}

#[cfg(test)]
#[path = "persistence_tests.rs"]
mod tests;
