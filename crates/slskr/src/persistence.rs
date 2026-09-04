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
                created_at INTEGER NOT NULL
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

        query("CREATE INDEX IF NOT EXISTS idx_transfers_started ON transfers(started_at DESC)")
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
        ] {
            if let Err(error) = query(statement).execute(&self.pool).await {
                if !error.to_string().contains("duplicate column name") {
                    return Err(Box::new(error));
                }
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

    // ========================================================================
    // OAuth State Operations
    // ========================================================================

    /// Insert or update a pending OAuth state.
    pub async fn upsert_oauth_state(
        &self,
        record: &OAuthStateRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query(
            r#"
            INSERT INTO oauth_states (state, provider, redirect_uri, created_at, expires_at)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT(state) DO UPDATE SET
                provider = excluded.provider,
                redirect_uri = excluded.redirect_uri,
                created_at = excluded.created_at,
                expires_at = excluded.expires_at
            "#,
        )
        .bind(&record.state)
        .bind(&record.provider)
        .bind(&record.redirect_uri)
        .bind(record.created_at)
        .bind(record.expires_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Delete a pending OAuth state after consumption.
    pub async fn delete_oauth_state(&self, state: &str) -> Result<(), Box<dyn std::error::Error>> {
        query("DELETE FROM oauth_states WHERE state = ?")
            .bind(state)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Delete expired pending OAuth states.
    pub async fn delete_expired_oauth_states(
        &self,
        now: i64,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        let result = query("DELETE FROM oauth_states WHERE expires_at <= ?")
            .bind(now)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// List non-expired pending OAuth states.
    pub async fn list_oauth_states(
        &self,
        now: i64,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<OAuthStateRecord>, Box<dyn std::error::Error>> {
        let records = query_as::<_, OAuthStateRecord>(
            r#"
            SELECT state, provider, redirect_uri, created_at, expires_at
            FROM oauth_states
            WHERE expires_at > ?
            ORDER BY created_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(now)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    // ========================================================================
    // Search Operations
    // ========================================================================

    /// Insert search record
    pub async fn insert_search(
        &self,
        record: &SearchRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query(
            r#"
            INSERT OR REPLACE INTO searches (id, query, status, result_count, created_at, completed_at, room, target, fallback_attempts)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&record.id)
        .bind(&record.query)
        .bind(&record.status)
        .bind(record.result_count)
        .bind(record.created_at)
        .bind(record.completed_at)
        .bind(&record.room)
        .bind(&record.target)
        .bind(record.fallback_attempts)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Persist the stable public identifier associated with a protocol token.
    pub async fn upsert_search_identity(
        &self,
        search_id: &str,
        external_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query("INSERT OR REPLACE INTO search_identities (search_id, external_id) VALUES (?, ?)")
            .bind(search_id)
            .bind(external_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Persist a search projection, its stable public identity, and all of
    /// its result rows as one durable unit. Search updates are emitted from
    /// the in-memory store only after this operation succeeds, so a failed
    /// result batch must not leave behind a half-written search or identity.
    pub async fn persist_search(
        &self,
        record: &SearchRecord,
        external_id: &str,
        results: &[SearchResultRecord],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut transaction = self.pool.begin().await?;
        query(
            r#"
            INSERT OR REPLACE INTO searches (id, query, status, result_count, created_at, completed_at, room, target, fallback_attempts)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&record.id)
        .bind(&record.query)
        .bind(&record.status)
        .bind(record.result_count)
        .bind(record.created_at)
        .bind(record.completed_at)
        .bind(&record.room)
        .bind(&record.target)
        .bind(record.fallback_attempts)
        .execute(&mut *transaction)
        .await?;

        query("INSERT OR REPLACE INTO search_identities (search_id, external_id) VALUES (?, ?)")
            .bind(&record.id)
            .bind(external_id)
            .execute(&mut *transaction)
            .await?;

        query("DELETE FROM search_results WHERE search_id = ?")
            .bind(&record.id)
            .execute(&mut *transaction)
            .await?;
        for batch in results.chunks(SQLITE_PARAMETER_CHUNK / 14) {
            let statement = format!(
                r#"
                INSERT INTO search_results
                (search_id, peer_username, filename, size, extension, bit_rate, sample_rate, bit_depth, length_seconds, locked, slot_free, average_speed, queue_length, created_at)
                VALUES {}
                "#,
                sql_value_rows(batch.len(), 14)
            );
            let mut insert = query(AssertSqlSafe(statement));
            for result in batch {
                insert = insert
                    .bind(&record.id)
                    .bind(&result.peer_username)
                    .bind(&result.filename)
                    .bind(result.size)
                    .bind(&result.extension)
                    .bind(result.bit_rate)
                    .bind(result.sample_rate)
                    .bind(result.bit_depth)
                    .bind(result.length_seconds)
                    .bind(result.locked)
                    .bind(result.slot_free)
                    .bind(result.average_speed)
                    .bind(result.queue_length)
                    .bind(result.created_at);
            }
            insert.execute(&mut *transaction).await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    /// Atomically apply search upserts and evictions. This keeps an in-memory
    /// SearchStore transition and its durable projection from diverging when
    /// a result batch, identity write, or eviction fails partway through.
    pub async fn persist_search_changes(
        &self,
        upserts: &[SearchWrite],
        deletes: &[String],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut transaction = self.pool.begin().await?;
        for write in upserts {
            query(
                r#"
                INSERT OR REPLACE INTO searches (id, query, status, result_count, created_at, completed_at, room, target, fallback_attempts)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&write.record.id)
            .bind(&write.record.query)
            .bind(&write.record.status)
            .bind(write.record.result_count)
            .bind(write.record.created_at)
            .bind(write.record.completed_at)
            .bind(&write.record.room)
            .bind(&write.record.target)
            .bind(write.record.fallback_attempts)
            .execute(&mut *transaction)
            .await?;

            query(
                "INSERT OR REPLACE INTO search_identities (search_id, external_id) VALUES (?, ?)",
            )
            .bind(&write.record.id)
            .bind(&write.external_id)
            .execute(&mut *transaction)
            .await?;

            query("DELETE FROM search_results WHERE search_id = ?")
                .bind(&write.record.id)
                .execute(&mut *transaction)
                .await?;
            for batch in write.results.chunks(SQLITE_PARAMETER_CHUNK / 14) {
                let statement = format!(
                    r#"
                    INSERT INTO search_results
                    (search_id, peer_username, filename, size, extension, bit_rate, sample_rate, bit_depth, length_seconds, locked, slot_free, average_speed, queue_length, created_at)
                    VALUES {}
                    "#,
                    sql_value_rows(batch.len(), 14)
                );
                let mut insert = query(AssertSqlSafe(statement));
                for result in batch {
                    insert = insert
                        .bind(&write.record.id)
                        .bind(&result.peer_username)
                        .bind(&result.filename)
                        .bind(result.size)
                        .bind(&result.extension)
                        .bind(result.bit_rate)
                        .bind(result.sample_rate)
                        .bind(result.bit_depth)
                        .bind(result.length_seconds)
                        .bind(result.locked)
                        .bind(result.slot_free)
                        .bind(result.average_speed)
                        .bind(result.queue_length)
                        .bind(result.created_at);
                }
                insert.execute(&mut *transaction).await?;
            }
        }
        // Apply evictions after upserts. A capacity-full create can expire
        // and evict the same old record; the eviction must win in that case.
        for id in deletes {
            query("DELETE FROM search_results WHERE search_id = ?")
                .bind(id)
                .execute(&mut *transaction)
                .await?;
            query("DELETE FROM searches WHERE id = ?")
                .bind(id)
                .execute(&mut *transaction)
                .await?;
            query("DELETE FROM search_identities WHERE search_id = ?")
                .bind(id)
                .execute(&mut *transaction)
                .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    /// Load stable public identifiers associated with protocol tokens.
    pub async fn list_search_identities(
        &self,
    ) -> Result<BTreeMap<String, String>, Box<dyn std::error::Error>> {
        let rows = query("SELECT search_id, external_id FROM search_identities")
            .fetch_all(&self.pool)
            .await?;
        let mut identities = BTreeMap::new();
        for row in rows {
            identities.insert(row.try_get("search_id")?, row.try_get("external_id")?);
        }
        Ok(identities)
    }

    /// Get search record
    pub async fn get_search(
        &self,
        id: &str,
    ) -> Result<Option<SearchRecord>, Box<dyn std::error::Error>> {
        let record = query_as::<_, SearchRecord>(
            "SELECT id, query, status, result_count, created_at, completed_at, room, target, fallback_attempts FROM searches WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(record)
    }

    /// List recent searches
    pub async fn list_searches(
        &self,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<SearchRecord>, Box<dyn std::error::Error>> {
        let records = query_as::<_, SearchRecord>(
            "SELECT id, query, status, result_count, created_at, completed_at, room, target, fallback_attempts FROM searches ORDER BY created_at DESC LIMIT ? OFFSET ?"
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    /// Update search status
    pub async fn update_search_status(
        &self,
        id: &str,
        status: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query("UPDATE searches SET status = ? WHERE id = ?")
            .bind(status)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Update search results
    pub async fn update_search_results(
        &self,
        id: &str,
        count: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query("UPDATE searches SET result_count = ? WHERE id = ?")
            .bind(count as i64)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Replace persisted result rows for one search.
    pub async fn replace_search_results(
        &self,
        search_id: &str,
        records: &[SearchResultRecord],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut transaction = self.pool.begin().await?;
        let delete = query("DELETE FROM search_results WHERE search_id = ?")
            .bind(search_id)
            .execute(&mut *transaction)
            .await;
        if let Err(error) = delete {
            let _ = transaction.rollback().await;
            return Err(error.into());
        }
        for batch in records.chunks(SQLITE_PARAMETER_CHUNK / 14) {
            let statement = format!(
                r#"
                INSERT INTO search_results
                (search_id, peer_username, filename, size, extension, bit_rate, sample_rate, bit_depth, length_seconds, locked, slot_free, average_speed, queue_length, created_at)
                VALUES {}
                "#,
                sql_value_rows(batch.len(), 14)
            );
            let mut insert = query(AssertSqlSafe(statement));
            for record in batch {
                insert = insert
                    .bind(search_id)
                    .bind(&record.peer_username)
                    .bind(&record.filename)
                    .bind(record.size)
                    .bind(&record.extension)
                    .bind(record.bit_rate)
                    .bind(record.sample_rate)
                    .bind(record.bit_depth)
                    .bind(record.length_seconds)
                    .bind(record.locked)
                    .bind(record.slot_free)
                    .bind(record.average_speed)
                    .bind(record.queue_length)
                    .bind(record.created_at);
            }
            if let Err(error) = insert.execute(&mut *transaction).await {
                let _ = transaction.rollback().await;
                return Err(error.into());
            }
        }
        transaction.commit().await?;
        Ok(())
    }

    /// List persisted search result rows.
    pub async fn list_search_results(
        &self,
        search_id: Option<&str>,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<SearchResultRecord>, Box<dyn std::error::Error>> {
        let records = if let Some(search_id) = search_id {
            query_as::<_, SearchResultRecord>(
                r#"
                SELECT id, search_id, peer_username, filename, size, extension, bit_rate, sample_rate, bit_depth, length_seconds, locked, slot_free, average_speed, queue_length, created_at
                FROM search_results
                WHERE search_id = ?
                ORDER BY id
                LIMIT ? OFFSET ?
                "#,
            )
            .bind(search_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        } else {
            query_as::<_, SearchResultRecord>(
                r#"
                SELECT id, search_id, peer_username, filename, size, extension, bit_rate, sample_rate, bit_depth, length_seconds, locked, slot_free, average_speed, queue_length, created_at
                FROM search_results
                ORDER BY search_id, id
                LIMIT ? OFFSET ?
                "#,
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        };
        Ok(records)
    }

    /// Delete a search record
    pub async fn delete_search(&self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut transaction = self.pool.begin().await?;
        query("DELETE FROM search_results WHERE search_id = ?")
            .bind(id)
            .execute(&mut *transaction)
            .await?;
        query("DELETE FROM searches WHERE id = ?")
            .bind(id)
            .execute(&mut *transaction)
            .await?;
        query("DELETE FROM search_identities WHERE search_id = ?")
            .bind(id)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(())
    }

    /// Atomically delete a batch of search records and their projections.
    pub async fn delete_searches(&self, ids: &[String]) -> Result<(), Box<dyn std::error::Error>> {
        let mut transaction = self.pool.begin().await?;
        for id in ids {
            query("DELETE FROM search_results WHERE search_id = ?")
                .bind(id)
                .execute(&mut *transaction)
                .await?;
            query("DELETE FROM searches WHERE id = ?")
                .bind(id)
                .execute(&mut *transaction)
                .await?;
            query("DELETE FROM search_identities WHERE search_id = ?")
                .bind(id)
                .execute(&mut *transaction)
                .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    /// Delete all search records
    pub async fn delete_all_searches(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut transaction = self.pool.begin().await?;
        query("DELETE FROM search_results")
            .execute(&mut *transaction)
            .await?;
        query("DELETE FROM searches")
            .execute(&mut *transaction)
            .await?;
        query("DELETE FROM search_identities")
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(())
    }

    // ========================================================================
    // Transfer Operations
    // ========================================================================

    /// Insert a durable transfer batch.  The primary key deliberately remains
    /// database-enforced so concurrent callers cannot create the same batch
    /// twice.
    pub async fn insert_transfer_batch(
        &self,
        record: &TransferBatchRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query(
            r#"
            INSERT INTO Batches (Id, SearchId, Username, Direction, CreatedAt, Options)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&record.id)
        .bind(&record.search_id)
        .bind(&record.username)
        .bind(record.direction)
        .bind(&record.created_at)
        .bind(&record.options_json)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Read one durable transfer batch without hydrating its associated
    /// transfer rows.
    pub async fn get_transfer_batch(
        &self,
        id: &str,
    ) -> Result<Option<TransferBatchRecord>, Box<dyn std::error::Error>> {
        let record = query_as::<_, TransferBatchRecord>(
            "SELECT Id, SearchId, Username, Direction, CreatedAt, Options FROM Batches WHERE Id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(record)
    }

    /// Update durable transfer-batch metadata for lifecycle migrations and
    /// administrative maintenance.
    pub async fn update_transfer_batch(
        &self,
        record: &TransferBatchRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query(
            r#"
            UPDATE Batches
            SET SearchId = ?, Username = ?, Direction = ?, CreatedAt = ?, Options = ?
            WHERE Id = ?
            "#,
        )
        .bind(&record.search_id)
        .bind(&record.username)
        .bind(record.direction)
        .bind(&record.created_at)
        .bind(&record.options_json)
        .bind(&record.id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Delete one durable transfer batch and return whether a row existed.
    pub async fn delete_transfer_batch(
        &self,
        id: &str,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let result = query("DELETE FROM Batches WHERE Id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Replace the durable HashDb snapshot and its latest-sequence cursor in
    /// one transaction.  Callers can therefore never restart with a row set
    /// whose cursor points past the rows that were committed.
    pub async fn replace_hash_db_snapshot(
        &self,
        records: &[HashDbRecord],
        latest_seq: i64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut transaction = self.pool.begin().await?;
        query("DELETE FROM HashDb")
            .execute(&mut *transaction)
            .await?;
        for batch in records.chunks(SQLITE_PARAMETER_CHUNK / 10) {
            let statement = format!(
                r#"
                INSERT INTO HashDb
                    (flac_key, byte_hash, size, first_seen_at, last_updated_at, seq_id, use_count, full_file_hash, musicbrainz_id, file_sha256)
                VALUES {}
                "#,
                sql_value_rows(batch.len(), 10)
            );
            let mut insert = query(AssertSqlSafe(statement));
            for record in batch {
                insert = insert
                    .bind(&record.flac_key)
                    .bind(&record.byte_hash)
                    .bind(record.size)
                    .bind(record.first_seen_at)
                    .bind(record.last_updated_at)
                    .bind(record.seq_id)
                    .bind(record.use_count)
                    .bind(&record.full_file_hash)
                    .bind(&record.musicbrainz_id)
                    .bind(&record.file_sha256);
            }
            insert.execute(&mut *transaction).await?;
        }
        query(
            r#"
            INSERT INTO HashDbState (key, value) VALUES ('latest_seq', ?)
            ON CONFLICT(key) DO UPDATE SET value = excluded.value
            "#,
        )
        .bind(latest_seq.to_string())
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(())
    }

    /// Read all durable HashDb rows in sequence order for startup rehydration.
    pub async fn list_hash_db_entries(
        &self,
    ) -> Result<Vec<HashDbRecord>, Box<dyn std::error::Error>> {
        Ok(query_as::<_, HashDbRecord>(
            "SELECT flac_key, byte_hash, size, first_seen_at, last_updated_at, seq_id, use_count, full_file_hash, musicbrainz_id, file_sha256 FROM HashDb ORDER BY seq_id, flac_key",
        )
        .fetch_all(&self.pool)
        .await?)
    }

    /// Read one durable HashDb entry.
    pub async fn get_hash_db_entry(
        &self,
        flac_key: &str,
    ) -> Result<Option<HashDbRecord>, Box<dyn std::error::Error>> {
        Ok(query_as::<_, HashDbRecord>(
            "SELECT flac_key, byte_hash, size, first_seen_at, last_updated_at, seq_id, use_count, full_file_hash, musicbrainz_id, file_sha256 FROM HashDb WHERE flac_key = ?",
        )
        .bind(flac_key)
        .fetch_optional(&self.pool)
        .await?)
    }

    /// Insert or update one HashDb entry for targeted lifecycle operations.
    pub async fn upsert_hash_db_entry(
        &self,
        record: &HashDbRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query(
            r#"
            INSERT INTO HashDb
                (flac_key, byte_hash, size, first_seen_at, last_updated_at, seq_id, use_count, full_file_hash, musicbrainz_id, file_sha256)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(flac_key) DO UPDATE SET
                byte_hash = excluded.byte_hash,
                size = excluded.size,
                first_seen_at = excluded.first_seen_at,
                last_updated_at = excluded.last_updated_at,
                seq_id = excluded.seq_id,
                use_count = excluded.use_count,
                full_file_hash = excluded.full_file_hash,
                musicbrainz_id = excluded.musicbrainz_id,
                file_sha256 = excluded.file_sha256
            "#,
        )
        .bind(&record.flac_key)
        .bind(&record.byte_hash)
        .bind(record.size)
        .bind(record.first_seen_at)
        .bind(record.last_updated_at)
        .bind(record.seq_id)
        .bind(record.use_count)
        .bind(&record.full_file_hash)
        .bind(&record.musicbrainz_id)
        .bind(&record.file_sha256)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Delete one HashDb entry and report whether it existed.
    pub async fn delete_hash_db_entry(
        &self,
        flac_key: &str,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let result = query("DELETE FROM HashDb WHERE flac_key = ?")
            .bind(flac_key)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Read one HashDb key/value state record.
    pub async fn get_hash_db_state(
        &self,
        key: &str,
    ) -> Result<Option<HashDbStateRecord>, Box<dyn std::error::Error>> {
        Ok(
            query_as::<_, HashDbStateRecord>("SELECT key, value FROM HashDbState WHERE key = ?")
                .bind(key)
                .fetch_optional(&self.pool)
                .await?,
        )
    }

    /// Insert or replace one HashDb key/value state record.
    pub async fn upsert_hash_db_state(
        &self,
        record: &HashDbStateRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query(
            r#"
            INSERT INTO HashDbState (key, value) VALUES (?, ?)
            ON CONFLICT(key) DO UPDATE SET value = excluded.value
            "#,
        )
        .bind(&record.key)
        .bind(&record.value)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Read the durable global overlay/Soulseek traffic counters.  A missing
    /// row is the same neutral zero state returned by the frozen HashDb
    /// service before any traffic has been accounted.
    pub async fn get_traffic_totals(
        &self,
    ) -> Result<TrafficTotalsRecord, Box<dyn std::error::Error>> {
        Ok(query_as::<_, TrafficTotalsRecord>(
            "SELECT overlay_upload_bytes, overlay_download_bytes, soulseek_upload_bytes, soulseek_download_bytes FROM TrafficStats WHERE key = 'global'",
        )
        .fetch_optional(&self.pool)
        .await?
        .unwrap_or_default())
    }

    /// Add bytes to the durable global traffic counters used by fairness.
    pub async fn add_traffic(
        &self,
        overlay_upload_bytes: i64,
        overlay_download_bytes: i64,
        soulseek_upload_bytes: i64,
        soulseek_download_bytes: i64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| i64::try_from(duration.as_secs()).unwrap_or(i64::MAX))
            .unwrap_or_default();
        query(
            r#"
            INSERT INTO TrafficStats
                (key, overlay_upload_bytes, overlay_download_bytes, soulseek_upload_bytes, soulseek_download_bytes, updated_at)
            VALUES ('global', ?, ?, ?, ?, ?)
            ON CONFLICT(key) DO UPDATE SET
                overlay_upload_bytes = TrafficStats.overlay_upload_bytes + excluded.overlay_upload_bytes,
                overlay_download_bytes = TrafficStats.overlay_download_bytes + excluded.overlay_download_bytes,
                soulseek_upload_bytes = TrafficStats.soulseek_upload_bytes + excluded.soulseek_upload_bytes,
                soulseek_download_bytes = TrafficStats.soulseek_download_bytes + excluded.soulseek_download_bytes,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(overlay_upload_bytes)
        .bind(overlay_download_bytes)
        .bind(soulseek_upload_bytes)
        .bind(soulseek_download_bytes)
        .bind(updated_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Delete one HashDb key/value state record and report whether it existed.
    pub async fn delete_hash_db_state(
        &self,
        key: &str,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let result = query("DELETE FROM HashDbState WHERE key = ?")
            .bind(key)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Insert transfer record
    pub async fn insert_transfer(
        &self,
        record: &TransferRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query(
            r#"
            INSERT OR REPLACE INTO transfers (id, direction, filename, peer_username, filesize, progress, status, started_at, completed_at, request_id, wishlist_item_id, request_name, destination_directory, local_path, batch_id, reason, bit_rate, sample_rate, bit_depth, length_seconds, artist, album, title, track_number, year, attempts, auto_replace_attempts, next_attempt_at)
            VALUES (
                ?, ?, ?, ?, ?, ?, ?, ?,
                ?, ?, ?, ?, ?, ?, ?, ?,
                ?, ?, ?, ?, ?, ?, ?, ?,
                ?, ?, ?, ?
            )
            "#
        )
        .bind(&record.id)
        .bind(&record.direction)
        .bind(&record.filename)
        .bind(&record.peer_username)
        .bind(record.filesize)
        .bind(record.progress)
        .bind(&record.status)
        .bind(record.started_at)
            .bind(record.completed_at)
            .bind(&record.request_id)
            .bind(&record.wishlist_item_id)
            .bind(&record.request_name)
        .bind(&record.destination_directory)
        .bind(&record.local_path)
        .bind(&record.batch_id)
        .bind(&record.reason)
        .bind(record.bit_rate)
        .bind(record.sample_rate)
        .bind(record.bit_depth)
        .bind(record.length_seconds)
        .bind(&record.artist)
        .bind(&record.album)
        .bind(&record.title)
        .bind(record.track_number)
        .bind(record.year)
        .bind(record.attempts)
        .bind(record.auto_replace_attempts)
        .bind(record.next_attempt_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Get transfer record
    pub async fn get_transfer(
        &self,
        id: &str,
    ) -> Result<Option<TransferRecord>, Box<dyn std::error::Error>> {
        let record = query_as::<_, TransferRecord>(
            "SELECT id, direction, filename, peer_username, filesize, progress, status, started_at, completed_at, request_id, wishlist_item_id, request_name, destination_directory, local_path, batch_id, reason, bit_rate, sample_rate, bit_depth, length_seconds, artist, album, title, track_number, year, attempts, auto_replace_attempts, next_attempt_at FROM transfers WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(record)
    }

    /// List transfers with optional status filter
    pub async fn list_transfers(
        &self,
        status: Option<&str>,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<TransferRecord>, Box<dyn std::error::Error>> {
        let records = if let Some(status) = status {
            query_as::<_, TransferRecord>(
                "SELECT id, direction, filename, peer_username, filesize, progress, status, started_at, completed_at, request_id, wishlist_item_id, request_name, destination_directory, local_path, batch_id, reason, bit_rate, sample_rate, bit_depth, length_seconds, artist, album, title, track_number, year, attempts, auto_replace_attempts, next_attempt_at FROM transfers WHERE status = ? ORDER BY started_at DESC LIMIT ? OFFSET ?"
            )
            .bind(status)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        } else {
            query_as::<_, TransferRecord>(
                "SELECT id, direction, filename, peer_username, filesize, progress, status, started_at, completed_at, request_id, wishlist_item_id, request_name, destination_directory, local_path, batch_id, reason, bit_rate, sample_rate, bit_depth, length_seconds, artist, album, title, track_number, year, attempts, auto_replace_attempts, next_attempt_at FROM transfers ORDER BY started_at DESC LIMIT ? OFFSET ?"
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        };
        Ok(records)
    }

    /// Update transfer progress
    pub async fn update_transfer_progress(
        &self,
        id: &str,
        progress: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query("UPDATE transfers SET progress = ? WHERE id = ?")
            .bind(i64::try_from(progress).unwrap_or(i64::MAX))
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Delete transfer record
    pub async fn delete_transfer(&self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        query("DELETE FROM transfers WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Delete a set of transfer records atomically.
    pub async fn delete_transfers(&self, ids: &[String]) -> Result<(), Box<dyn std::error::Error>> {
        if ids.is_empty() {
            return Ok(());
        }
        let mut transaction = self.pool.begin().await?;
        for chunk in ids.chunks(SQLITE_PARAMETER_CHUNK) {
            let mut statement = query(AssertSqlSafe(format!(
                "DELETE FROM transfers WHERE id IN ({})",
                sql_placeholders(chunk.len())
            )));
            for id in chunk {
                statement = statement.bind(id);
            }
            statement.execute(&mut *transaction).await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    /// Roll back transfers that were staged but never dispatched, including their event trail.
    pub async fn rollback_staged_transfers(
        &self,
        ids: &[String],
    ) -> Result<(), Box<dyn std::error::Error>> {
        if ids.is_empty() {
            return Ok(());
        }
        let mut transaction = self.pool.begin().await?;
        for chunk in ids.chunks(SQLITE_PARAMETER_CHUNK) {
            let placeholders = sql_placeholders(chunk.len());
            let mut event_statement = query(AssertSqlSafe(format!(
                "DELETE FROM transfer_events WHERE transfer_id IN ({placeholders})"
            )));
            for id in chunk {
                event_statement = event_statement.bind(id);
            }
            event_statement.execute(&mut *transaction).await?;

            let mut transfer_statement = query(AssertSqlSafe(format!(
                "DELETE FROM transfers WHERE id IN ({placeholders})"
            )));
            for id in chunk {
                transfer_statement = transfer_statement.bind(id);
            }
            transfer_statement.execute(&mut *transaction).await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    /// Append a transfer transition/progress event.
    pub async fn insert_transfer_event(
        &self,
        record: &TransferEventRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query(
            r#"
            INSERT INTO transfer_events
                (transfer_id, direction, token, filename, peer_username, filesize, progress, status, reason, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&record.transfer_id)
        .bind(&record.direction)
        .bind(record.token)
        .bind(&record.filename)
        .bind(&record.peer_username)
        .bind(record.filesize)
        .bind(record.progress)
        .bind(&record.status)
        .bind(&record.reason)
        .bind(record.created_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Atomically persist transfer projections and their corresponding events.
    pub async fn insert_transfer_records_with_events(
        &self,
        records: &[(TransferRecord, TransferEventRecord)],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut transaction = self.pool.begin().await?;
        for (transfer, event) in records {
            let transfer_insert = query(
                r#"
                INSERT OR REPLACE INTO transfers (id, direction, filename, peer_username, filesize, progress, status, started_at, completed_at, request_id, wishlist_item_id, request_name, destination_directory, local_path, batch_id, reason, bit_rate, sample_rate, bit_depth, length_seconds, artist, album, title, track_number, year, attempts, auto_replace_attempts, next_attempt_at)
                VALUES (
                    ?, ?, ?, ?, ?, ?, ?, ?,
                    ?, ?, ?, ?, ?, ?, ?, ?,
                    ?, ?, ?, ?, ?, ?, ?, ?,
                    ?, ?, ?, ?
                )
                "#,
            )
            .bind(&transfer.id)
            .bind(&transfer.direction)
            .bind(&transfer.filename)
            .bind(&transfer.peer_username)
            .bind(transfer.filesize)
            .bind(transfer.progress)
            .bind(&transfer.status)
            .bind(transfer.started_at)
            .bind(transfer.completed_at)
            .bind(&transfer.request_id)
            .bind(&transfer.wishlist_item_id)
            .bind(&transfer.request_name)
            .bind(&transfer.destination_directory)
            .bind(&transfer.local_path)
            .bind(&transfer.batch_id)
            .bind(&transfer.reason)
            .bind(transfer.bit_rate)
            .bind(transfer.sample_rate)
            .bind(transfer.bit_depth)
            .bind(transfer.length_seconds)
            .bind(&transfer.artist)
            .bind(&transfer.album)
            .bind(&transfer.title)
            .bind(transfer.track_number)
            .bind(transfer.year)
            .bind(transfer.attempts)
            .bind(transfer.auto_replace_attempts)
            .bind(transfer.next_attempt_at)
            .execute(&mut *transaction)
            .await;
            if let Err(error) = transfer_insert {
                let _ = transaction.rollback().await;
                return Err(error.into());
            }
            let event_insert = query(
                r#"
                INSERT INTO transfer_events
                    (transfer_id, direction, token, filename, peer_username, filesize, progress, status, reason, created_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&event.transfer_id)
            .bind(&event.direction)
            .bind(event.token)
            .bind(&event.filename)
            .bind(&event.peer_username)
            .bind(event.filesize)
            .bind(event.progress)
            .bind(&event.status)
            .bind(&event.reason)
            .bind(event.created_at)
            .execute(&mut *transaction)
            .await;
            if let Err(error) = event_insert {
                let _ = transaction.rollback().await;
                return Err(error.into());
            }
        }
        transaction.commit().await?;
        Ok(())
    }

    /// List recent transfer transition/progress events.
    pub async fn list_transfer_events(
        &self,
        transfer_id: Option<&str>,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<TransferEventRecord>, Box<dyn std::error::Error>> {
        let records = if let Some(transfer_id) = transfer_id {
            query_as::<_, TransferEventRecord>(
                r#"
                SELECT id, transfer_id, direction, token, filename, peer_username, filesize, progress, status, reason, created_at
                FROM transfer_events
                WHERE transfer_id = ?
                ORDER BY id DESC
                LIMIT ? OFFSET ?
                "#,
            )
            .bind(transfer_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        } else {
            query_as::<_, TransferEventRecord>(
                r#"
                SELECT id, transfer_id, direction, token, filename, peer_username, filesize, progress, status, reason, created_at
                FROM transfer_events
                ORDER BY id DESC
                LIMIT ? OFFSET ?
                "#,
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        };
        Ok(records)
    }

    // ========================================================================
    // Share Index Operations
    // ========================================================================

    /// Replace the durable share index snapshot.
    pub async fn replace_share_files(
        &self,
        records: &[ShareFileRecord],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut tx = self.pool.begin().await?;
        query("DELETE FROM share_files").execute(&mut *tx).await?;
        for batch in records.chunks(SQLITE_PARAMETER_CHUNK / 7) {
            let statement = format!(
                r#"
                INSERT OR REPLACE INTO share_files
                (filename, size, extension, root_label, local_path, attributes_json, updated_at)
                VALUES {}
                "#,
                sql_value_rows(batch.len(), 7)
            );
            let mut insert = query(AssertSqlSafe(statement));
            for record in batch {
                insert = insert
                    .bind(&record.filename)
                    .bind(record.size)
                    .bind(&record.extension)
                    .bind(&record.root_label)
                    .bind(&record.local_path)
                    .bind(&record.attributes_json)
                    .bind(record.updated_at);
            }
            insert.execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// List durable share index records.
    pub async fn list_share_files(
        &self,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<ShareFileRecord>, Box<dyn std::error::Error>> {
        let records = query_as::<_, ShareFileRecord>(
            r#"
            SELECT filename, size, extension, root_label, local_path, attributes_json, updated_at
            FROM share_files
            ORDER BY filename ASC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    /// Insert or update a runtime event record.
    pub async fn insert_event(
        &self,
        record: &EventRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query(
            r#"
            INSERT OR REPLACE INTO events (id, kind, resource, detail, created_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(record.id)
        .bind(&record.kind)
        .bind(&record.resource)
        .bind(&record.detail)
        .bind(record.created_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Insert an event and enforce its retention limit atomically.
    pub async fn insert_event_and_prune(
        &self,
        record: &EventRecord,
        history_limit: i32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut transaction = self.pool.begin().await?;
        query(
            r#"
            INSERT OR REPLACE INTO events (id, kind, resource, detail, created_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(record.id)
        .bind(&record.kind)
        .bind(&record.resource)
        .bind(&record.detail)
        .bind(record.created_at)
        .execute(&mut *transaction)
        .await?;
        query(
            r#"
            DELETE FROM events
            WHERE id NOT IN (
                SELECT id FROM events ORDER BY id DESC LIMIT ?
            )
            "#,
        )
        .bind(history_limit)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(())
    }

    /// List recent persisted runtime event records in ascending id order.
    pub async fn list_events(
        &self,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<EventRecord>, Box<dyn std::error::Error>> {
        let records = query_as::<_, EventRecord>(
            r#"
            SELECT id, kind, resource, detail, created_at
            FROM (
                SELECT id, kind, resource, detail, created_at
                FROM events
                ORDER BY id DESC
                LIMIT ? OFFSET ?
            )
            ORDER BY id ASC
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    /// Prune persisted events beyond the configured history limit.
    pub async fn prune_events(
        &self,
        history_limit: i32,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        let result = query(
            r#"
            DELETE FROM events
            WHERE id NOT IN (
                SELECT id FROM events ORDER BY id DESC LIMIT ?
            )
            "#,
        )
        .bind(history_limit)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    // ========================================================================
    // Message Operations
    // ========================================================================

    /// Insert message record
    pub async fn insert_message(
        &self,
        record: &MessageRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query(
            r#"
            INSERT INTO messages (id, username, content, direction, read, created_at, source_id, source_timestamp, was_replayed)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&record.id)
        .bind(&record.username)
        .bind(&record.content)
        .bind(&record.direction)
        .bind(record.read as i32)
        .bind(record.created_at)
        .bind(record.source_id)
        .bind(record.source_timestamp)
        .bind(record.was_replayed as i32)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Insert multiple message records atomically.
    pub async fn insert_messages(
        &self,
        records: &[MessageRecord],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut transaction = self.pool.begin().await?;
        let result = async {
            for batch in records.chunks(SQLITE_PARAMETER_CHUNK / 9) {
                let statement = format!(
                    r#"
                    INSERT INTO messages (id, username, content, direction, read, created_at, source_id, source_timestamp, was_replayed)
                    VALUES {}
                    "#,
                    sql_value_rows(batch.len(), 9)
                );
                let mut insert = query(AssertSqlSafe(statement));
                for record in batch {
                    insert = insert
                        .bind(&record.id)
                        .bind(&record.username)
                        .bind(&record.content)
                        .bind(&record.direction)
                        .bind(record.read as i32)
                        .bind(record.created_at)
                        .bind(record.source_id)
                        .bind(record.source_timestamp)
                        .bind(record.was_replayed as i32);
                }
                insert.execute(&mut *transaction).await?;
            }
            Ok::<(), sqlx_core::Error>(())
        }
        .await;
        if let Err(error) = result {
            transaction.rollback().await?;
            return Err(error.into());
        }
        transaction.commit().await?;
        Ok(())
    }

    /// List messages from user
    pub async fn list_messages_from_user(
        &self,
        username: &str,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<MessageRecord>, Box<dyn std::error::Error>> {
        let records = query_as::<_, MessageRecord>(
            "SELECT id, username, content, direction, read, created_at FROM messages WHERE username = ? ORDER BY created_at DESC LIMIT ? OFFSET ?"
        )
        .bind(username)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    /// List recent messages across all users
    pub async fn list_messages(
        &self,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<MessageRecord>, Box<dyn std::error::Error>> {
        let records = query_as::<_, MessageRecord>(
            "SELECT id, username, content, direction, read, created_at FROM messages ORDER BY created_at DESC LIMIT ? OFFSET ?"
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    /// Mark message as read
    pub async fn mark_message_read(&self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        query("UPDATE messages SET read = 1 WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Mark multiple messages as read atomically.
    pub async fn mark_messages_read(
        &self,
        ids: &[String],
    ) -> Result<(), Box<dyn std::error::Error>> {
        if ids.is_empty() {
            return Ok(());
        }
        let mut transaction = self.pool.begin().await?;
        for chunk in ids.chunks(SQLITE_PARAMETER_CHUNK) {
            let mut statement = query(AssertSqlSafe(format!(
                "UPDATE messages SET read = 1 WHERE id IN ({})",
                sql_placeholders(chunk.len())
            )));
            for id in chunk {
                statement = statement.bind(id);
            }
            statement.execute(&mut *transaction).await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    /// Delete every persisted message in a user's conversation.
    pub async fn delete_messages_from_user(
        &self,
        username: &str,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        let result = query("DELETE FROM messages WHERE username = ?")
            .bind(username)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    // ========================================================================
    // User Statistics Operations
    // ========================================================================

    /// Get or create user stats
    pub async fn get_user_stats(
        &self,
        username: &str,
    ) -> Result<Option<UserStatsRecord>, Box<dyn std::error::Error>> {
        let record = query_as::<_, UserStatsRecord>(
            "SELECT username, uploads, downloads, total_uploaded, total_downloaded, watched, last_seen, created_at, updated_at FROM user_stats WHERE username = ?"
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;
        Ok(record)
    }

    /// Update user stats
    pub async fn update_user_stats(
        &self,
        username: &str,
        uploads: i64,
        downloads: i64,
        total_uploaded: i64,
        total_downloaded: i64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
        query(
            "INSERT OR REPLACE INTO user_stats (username, uploads, downloads, total_uploaded, total_downloaded, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(username)
        .bind(uploads)
        .bind(downloads)
        .bind(total_uploaded)
        .bind(total_downloaded)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Mark user as watched
    pub async fn set_user_watched(
        &self,
        username: &str,
        watched: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query("UPDATE user_stats SET watched = ? WHERE username = ?")
            .bind(watched as i32)
            .bind(username)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// List watched users
    pub async fn list_watched_users(
        &self,
    ) -> Result<Vec<UserStatsRecord>, Box<dyn std::error::Error>> {
        let records = query_as::<_, UserStatsRecord>(
            "SELECT username, uploads, downloads, total_uploaded, total_downloaded, watched, last_seen, created_at, updated_at FROM user_stats WHERE watched = 1 ORDER BY username"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    /// Insert or update a user projection record.
    pub async fn upsert_user_projection(
        &self,
        record: &UserProjectionRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query(
            r#"
            INSERT INTO user_records (
                username, watched, status, average_speed, upload_count,
                file_count, directory_count, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(username) DO UPDATE SET
                watched = excluded.watched,
                status = excluded.status,
                average_speed = excluded.average_speed,
                upload_count = excluded.upload_count,
                file_count = excluded.file_count,
                directory_count = excluded.directory_count,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(&record.username)
        .bind(record.watched)
        .bind(&record.status)
        .bind(record.average_speed)
        .bind(record.upload_count)
        .bind(record.file_count)
        .bind(record.directory_count)
        .bind(record.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// List persisted user projection records.
    pub async fn list_user_projections(
        &self,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<UserProjectionRecord>, Box<dyn std::error::Error>> {
        let records = query_as::<_, UserProjectionRecord>(
            r#"
            SELECT username, watched, status, average_speed, upload_count,
                   file_count, directory_count, updated_at
            FROM user_records
            ORDER BY updated_at DESC, username
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    // ========================================================================
    // Room Operations
    // ========================================================================

    /// Subscribe to room
    pub async fn subscribe_room(
        &self,
        name: &str,
        owner: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
        query(
            "INSERT OR REPLACE INTO rooms (name, owner, subscribed, joined_at, last_activity) VALUES (?, ?, 1, ?, ?)"
        )
        .bind(name)
        .bind(owner)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Unsubscribe from room
    pub async fn unsubscribe_room(&self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        query("UPDATE rooms SET subscribed = 0 WHERE name = ?")
            .bind(name)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// List subscribed rooms
    pub async fn list_subscribed_rooms(
        &self,
    ) -> Result<Vec<RoomRecord>, Box<dyn std::error::Error>> {
        let records = query_as::<_, RoomRecord>(
            "SELECT name, owner, subscribed, joined_at, last_activity FROM rooms WHERE subscribed = 1 ORDER BY name"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    // ========================================================================
    // User Note, Interest, and Security Operations
    // ========================================================================

    /// Insert or update a user note record.
    pub async fn upsert_user_note(
        &self,
        record: &UserNoteRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query(
            r#"
            INSERT OR REPLACE INTO user_notes
                (id, username, note, color, icon, is_high_priority, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&record.id)
        .bind(&record.username)
        .bind(&record.note)
        .bind(&record.color)
        .bind(&record.icon)
        .bind(record.is_high_priority)
        .bind(record.created_at)
        .bind(record.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Delete a user note.
    pub async fn delete_user_note(&self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        query("DELETE FROM user_notes WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// List persisted user notes.
    pub async fn list_user_notes(
        &self,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<UserNoteRecord>, Box<dyn std::error::Error>> {
        let records = query_as::<_, UserNoteRecord>(
            "SELECT id, username, note, color, icon, is_high_priority, created_at, updated_at FROM user_notes ORDER BY updated_at DESC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    /// Insert or update an interest record.
    pub async fn upsert_interest(
        &self,
        record: &InterestRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query(
            r#"
            INSERT OR REPLACE INTO interests (id, name, kind, created_at)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(&record.id)
        .bind(&record.name)
        .bind(&record.kind)
        .bind(record.created_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Delete an interest record.
    pub async fn delete_interest(&self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        query("DELETE FROM interests WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// List persisted interests.
    pub async fn list_interests(
        &self,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<InterestRecord>, Box<dyn std::error::Error>> {
        let records = query_as::<_, InterestRecord>(
            "SELECT id, name, kind, created_at FROM interests ORDER BY created_at DESC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    /// Insert or update a security ban.
    pub async fn upsert_security_ban(
        &self,
        record: &SecurityBanRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query(
            r#"
            INSERT OR REPLACE INTO security_bans
                (kind, value, created_at, reason, expires_at, is_permanent)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&record.kind)
        .bind(&record.value)
        .bind(record.created_at)
        .bind(&record.reason)
        .bind(record.expires_at)
        .bind(record.is_permanent)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Delete a security ban.
    pub async fn delete_security_ban(
        &self,
        kind: &str,
        value: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query("DELETE FROM security_bans WHERE kind = ? AND value = ?")
            .bind(kind)
            .bind(value)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// List persisted security bans.
    pub async fn list_security_bans(
        &self,
    ) -> Result<Vec<SecurityBanRecord>, Box<dyn std::error::Error>> {
        let records = query_as::<_, SecurityBanRecord>(
            "SELECT kind, value, created_at, reason, expires_at, is_permanent FROM security_bans ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    /// Insert or update a wishlist item.
    pub async fn upsert_wishlist_item(
        &self,
        record: &WishlistItemRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query(
            r#"
            INSERT INTO wishlist_items
                (id, artist, title, kind, filter, enabled, auto_download, max_results,
                 max_downloads, last_viewed_at, last_searched_at, last_match_count,
                 last_visible_hit_count, last_hidden_locked_hit_count,
                 last_filtered_out_hit_count, last_ignored_result_hit_count,
                 last_response_count, total_search_count, total_download_count,
                 last_search_id, lidarr_album_id, lidarr_track_id, lidarr_track_count,
                 lidarr_duration_seconds, lidarr_release_disambiguation, added_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                artist = excluded.artist,
                title = excluded.title,
                kind = excluded.kind,
                filter = excluded.filter,
                enabled = excluded.enabled,
                auto_download = excluded.auto_download,
                max_results = excluded.max_results,
                max_downloads = excluded.max_downloads,
                last_viewed_at = excluded.last_viewed_at,
                last_searched_at = excluded.last_searched_at,
                last_match_count = excluded.last_match_count,
                last_visible_hit_count = excluded.last_visible_hit_count,
                last_hidden_locked_hit_count = excluded.last_hidden_locked_hit_count,
                last_filtered_out_hit_count = excluded.last_filtered_out_hit_count,
                last_ignored_result_hit_count = excluded.last_ignored_result_hit_count,
                last_response_count = excluded.last_response_count,
                total_search_count = excluded.total_search_count,
                total_download_count = excluded.total_download_count,
                last_search_id = excluded.last_search_id,
                lidarr_album_id = excluded.lidarr_album_id,
                lidarr_track_id = excluded.lidarr_track_id,
                lidarr_track_count = excluded.lidarr_track_count,
                lidarr_duration_seconds = excluded.lidarr_duration_seconds,
                lidarr_release_disambiguation = excluded.lidarr_release_disambiguation,
                added_at = excluded.added_at
            "#,
        )
        .bind(&record.id)
        .bind(&record.artist)
        .bind(&record.title)
        .bind(&record.kind)
        .bind(&record.filter)
        .bind(record.enabled)
        .bind(record.auto_download)
        .bind(record.max_results)
        .bind(record.max_downloads)
        .bind(record.last_viewed_at)
        .bind(record.last_searched_at)
        .bind(record.last_match_count)
        .bind(record.last_visible_hit_count)
        .bind(record.last_hidden_locked_hit_count)
        .bind(record.last_filtered_out_hit_count)
        .bind(record.last_ignored_result_hit_count)
        .bind(record.last_response_count)
        .bind(record.total_search_count)
        .bind(record.total_download_count)
        .bind(&record.last_search_id)
        .bind(record.lidarr_album_id)
        .bind(record.lidarr_track_id)
        .bind(record.lidarr_track_count)
        .bind(record.lidarr_duration_seconds)
        .bind(&record.lidarr_release_disambiguation)
        .bind(record.added_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Insert or update wishlist items atomically.
    pub async fn upsert_wishlist_items(
        &self,
        records: &[WishlistItemRecord],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut transaction = self.pool.begin().await?;
        for batch in records.chunks(SQLITE_PARAMETER_CHUNK / 26) {
            let values = std::iter::repeat_n(
                "(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                batch.len(),
            )
            .collect::<Vec<_>>()
            .join(", ");
            let statement = format!(
                r#"
                INSERT INTO wishlist_items
                    (id, artist, title, kind, filter, enabled, auto_download, max_results,
                     max_downloads, last_viewed_at, last_searched_at, last_match_count,
                    last_visible_hit_count, last_hidden_locked_hit_count,
                    last_filtered_out_hit_count, last_ignored_result_hit_count,
                    last_response_count, total_search_count, total_download_count,
                    last_search_id, lidarr_album_id, lidarr_track_id, lidarr_track_count,
                    lidarr_duration_seconds, lidarr_release_disambiguation, added_at)
                VALUES {values}
                ON CONFLICT(id) DO UPDATE SET
                    artist = excluded.artist,
                    title = excluded.title,
                    kind = excluded.kind,
                    filter = excluded.filter,
                    enabled = excluded.enabled,
                    auto_download = excluded.auto_download,
                    max_results = excluded.max_results,
                    max_downloads = excluded.max_downloads,
                    last_viewed_at = excluded.last_viewed_at,
                    last_searched_at = excluded.last_searched_at,
                    last_match_count = excluded.last_match_count,
                    last_visible_hit_count = excluded.last_visible_hit_count,
                    last_hidden_locked_hit_count = excluded.last_hidden_locked_hit_count,
                    last_filtered_out_hit_count = excluded.last_filtered_out_hit_count,
                    last_ignored_result_hit_count = excluded.last_ignored_result_hit_count,
                    last_response_count = excluded.last_response_count,
                    total_search_count = excluded.total_search_count,
                    total_download_count = excluded.total_download_count,
                    last_search_id = excluded.last_search_id,
                    lidarr_album_id = excluded.lidarr_album_id,
                    lidarr_track_id = excluded.lidarr_track_id,
                    lidarr_track_count = excluded.lidarr_track_count,
                    lidarr_duration_seconds = excluded.lidarr_duration_seconds,
                    lidarr_release_disambiguation = excluded.lidarr_release_disambiguation,
                    added_at = excluded.added_at
                "#
            );
            // The only dynamic fragment is a fixed placeholder tuple repeated for the bounded
            // batch length; every record value remains a bind parameter.
            let mut statement = query(AssertSqlSafe(statement));
            for record in batch {
                statement = statement
                    .bind(&record.id)
                    .bind(&record.artist)
                    .bind(&record.title)
                    .bind(&record.kind)
                    .bind(&record.filter)
                    .bind(record.enabled)
                    .bind(record.auto_download)
                    .bind(record.max_results)
                    .bind(record.max_downloads)
                    .bind(record.last_viewed_at)
                    .bind(record.last_searched_at)
                    .bind(record.last_match_count)
                    .bind(record.last_visible_hit_count)
                    .bind(record.last_hidden_locked_hit_count)
                    .bind(record.last_filtered_out_hit_count)
                    .bind(record.last_ignored_result_hit_count)
                    .bind(record.last_response_count)
                    .bind(record.total_search_count)
                    .bind(record.total_download_count)
                    .bind(&record.last_search_id)
                    .bind(record.lidarr_album_id)
                    .bind(record.lidarr_track_id)
                    .bind(record.lidarr_track_count)
                    .bind(record.lidarr_duration_seconds)
                    .bind(&record.lidarr_release_disambiguation)
                    .bind(record.added_at);
            }
            statement.execute(&mut *transaction).await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    /// Delete a wishlist item.
    pub async fn delete_wishlist_item(&self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut transaction = self.pool.begin().await?;
        query("DELETE FROM wishlist_ignored_results WHERE wishlist_item_id = ?")
            .bind(id)
            .execute(&mut *transaction)
            .await?;
        query("DELETE FROM wishlist_items WHERE id = ?")
            .bind(id)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(())
    }

    /// List persisted wishlist items.
    pub async fn list_wishlist_items(
        &self,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<WishlistItemRecord>, Box<dyn std::error::Error>> {
        let records = query_as::<_, WishlistItemRecord>(
            "SELECT id, artist, title, kind, filter, enabled, auto_download, max_results, max_downloads, last_viewed_at, last_searched_at, last_match_count, last_visible_hit_count, last_hidden_locked_hit_count, last_filtered_out_hit_count, last_ignored_result_hit_count, last_response_count, total_search_count, total_download_count, last_search_id, lidarr_album_id, lidarr_track_id, lidarr_track_count, lidarr_duration_seconds, lidarr_release_disambiguation, added_at FROM wishlist_items ORDER BY added_at DESC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    /// Persist an ignored wishlist rule and its suppressed search snapshots atomically.
    pub async fn upsert_wishlist_ignored_result_and_searches(
        &self,
        rule: &WishlistIgnoredResultRecord,
        searches: &[(SearchRecord, Vec<SearchResultRecord>)],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut transaction = self.pool.begin().await?;
        query(
            r#"
            INSERT OR REPLACE INTO wishlist_ignored_results
                (id, wishlist_item_id, username, directory, created_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&rule.id)
        .bind(&rule.wishlist_item_id)
        .bind(&rule.username)
        .bind(&rule.directory)
        .bind(rule.created_at)
        .execute(&mut *transaction)
        .await?;
        for (search, results) in searches {
            query(
                r#"
                INSERT OR REPLACE INTO searches
                    (id, query, status, result_count, created_at, completed_at, room, target)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&search.id)
            .bind(&search.query)
            .bind(&search.status)
            .bind(search.result_count)
            .bind(search.created_at)
            .bind(search.completed_at)
            .bind(&search.room)
            .bind(&search.target)
            .execute(&mut *transaction)
            .await?;
            query("DELETE FROM search_results WHERE search_id = ?")
                .bind(&search.id)
                .execute(&mut *transaction)
                .await?;
            for result in results {
                query(
                    r#"
                    INSERT INTO search_results
                        (search_id, peer_username, filename, size, extension, bit_rate, sample_rate, bit_depth, length_seconds, locked, slot_free, average_speed, queue_length, created_at)
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                )
                .bind(&search.id)
                .bind(&result.peer_username)
                .bind(&result.filename)
                .bind(result.size)
                .bind(&result.extension)
                .bind(result.bit_rate)
                .bind(result.sample_rate)
                .bind(result.bit_depth)
                .bind(result.length_seconds)
                .bind(result.locked)
                .bind(result.slot_free)
                .bind(result.average_speed)
                .bind(result.queue_length)
                .bind(result.created_at)
                .execute(&mut *transaction)
                .await?;
            }
        }
        transaction.commit().await?;
        Ok(())
    }

    /// Remove one ignored result rule, scoped to its wishlist item.
    pub async fn delete_wishlist_ignored_result(
        &self,
        wishlist_item_id: &str,
        id: &str,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        let result =
            query("DELETE FROM wishlist_ignored_results WHERE wishlist_item_id = ? AND id = ?")
                .bind(wishlist_item_id)
                .bind(id)
                .execute(&self.pool)
                .await?;
        Ok(result.rows_affected())
    }

    /// List ignored result rules for one wishlist item, newest first.
    pub async fn list_wishlist_ignored_results(
        &self,
        wishlist_item_id: &str,
    ) -> Result<Vec<WishlistIgnoredResultRecord>, Box<dyn std::error::Error>> {
        let records = query_as::<_, WishlistIgnoredResultRecord>(
            r#"
            SELECT id, wishlist_item_id, username, directory, created_at
            FROM wishlist_ignored_results
            WHERE wishlist_item_id = ?
            ORDER BY created_at DESC, id DESC
            "#,
        )
        .bind(wishlist_item_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    /// List all ignored result rules for daemon rehydration.
    pub async fn list_all_wishlist_ignored_results(
        &self,
    ) -> Result<Vec<WishlistIgnoredResultRecord>, Box<dyn std::error::Error>> {
        let records = query_as::<_, WishlistIgnoredResultRecord>(
            r#"
            SELECT id, wishlist_item_id, username, directory, created_at
            FROM wishlist_ignored_results
            ORDER BY created_at DESC, id DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    /// Insert or update a contact.
    pub async fn upsert_contact(
        &self,
        record: &ContactRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query(
            r#"
            INSERT OR REPLACE INTO contacts (id, username, online, status, free_upload_slots, queue_length, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&record.id)
        .bind(&record.username)
        .bind(record.online as i32)
        .bind(&record.status)
        .bind(record.free_upload_slots)
        .bind(record.queue_length)
        .bind(record.created_at)
        .bind(record.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Delete a contact.
    pub async fn delete_contact(&self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        query("DELETE FROM contacts WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// List persisted contacts.
    pub async fn list_contacts(
        &self,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<ContactRecord>, Box<dyn std::error::Error>> {
        let records = query_as::<_, ContactRecord>(
            "SELECT id, username, online, status, free_upload_slots, queue_length, created_at, updated_at FROM contacts ORDER BY updated_at DESC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    /// Insert or update a share grant.
    pub async fn upsert_share_grant(
        &self,
        record: &ShareGrantRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query(
            r#"
            INSERT INTO share_grants (id, collection_id, username, shared_at, permissions)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                collection_id = excluded.collection_id,
                username = excluded.username,
                shared_at = excluded.shared_at,
                permissions = excluded.permissions
            "#,
        )
        .bind(&record.id)
        .bind(&record.collection_id)
        .bind(&record.username)
        .bind(record.shared_at)
        .bind(&record.permissions)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Delete a share grant.
    pub async fn delete_share_grant(&self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut transaction = self.pool.begin().await?;
        query("DELETE FROM share_access_tokens WHERE grant_id = ?")
            .bind(id)
            .execute(&mut *transaction)
            .await?;
        query("DELETE FROM share_grants WHERE id = ?")
            .bind(id)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(())
    }

    /// List persisted share grants.
    pub async fn list_share_grants(
        &self,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<ShareGrantRecord>, Box<dyn std::error::Error>> {
        let records = query_as::<_, ShareGrantRecord>(
            "SELECT id, collection_id, username, shared_at, permissions FROM share_grants ORDER BY shared_at DESC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    /// Persist a delegated-share verifier. Callers must provide only a digest.
    pub async fn upsert_share_access_token(
        &self,
        record: &ShareAccessTokenRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if record.token_digest.len() != 64
            || !record
                .token_digest
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "share access token verifier must be a SHA-256 hex digest",
            )
            .into());
        }
        let result = query(
            r#"
            INSERT INTO share_access_tokens (token_digest, grant_id, expires_at)
            SELECT ?, ?, ?
            WHERE EXISTS (SELECT 1 FROM share_grants WHERE id = ?)
            ON CONFLICT(token_digest) DO UPDATE SET
                grant_id = excluded.grant_id,
                expires_at = excluded.expires_at
            "#,
        )
        .bind(&record.token_digest)
        .bind(&record.grant_id)
        .bind(record.expires_at)
        .bind(&record.grant_id)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() != 1 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "share access token grant is unavailable",
            )
            .into());
        }
        Ok(())
    }

    /// Delete expired delegated-share verifiers.
    pub async fn delete_expired_share_access_tokens(
        &self,
        now: i64,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        let result = query("DELETE FROM share_access_tokens WHERE expires_at <= ?")
            .bind(now)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// List unexpired delegated-share verifiers without exposing raw tokens.
    pub async fn list_share_access_tokens(
        &self,
        now: i64,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<ShareAccessTokenRecord>, Box<dyn std::error::Error>> {
        let records = query_as::<_, ShareAccessTokenRecord>(
            "SELECT token_digest, grant_id, expires_at FROM share_access_tokens WHERE expires_at > ? ORDER BY expires_at ASC LIMIT ? OFFSET ?",
        )
        .bind(now)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    /// Insert or update a share group.
    pub async fn upsert_share_group(
        &self,
        record: &ShareGroupRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query(
            r#"
            INSERT OR REPLACE INTO share_groups (id, name, description, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&record.id)
        .bind(&record.name)
        .bind(&record.description)
        .bind(record.created_at)
        .bind(record.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Replace a share group and its complete membership snapshot atomically.
    pub async fn replace_share_group(
        &self,
        record: &ShareGroupRecord,
        members: &[ShareGroupMemberRecord],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut transaction = self.pool.begin().await?;
        let result = async {
            query(
                r#"
                INSERT OR REPLACE INTO share_groups (id, name, description, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?)
                "#,
            )
            .bind(&record.id)
            .bind(&record.name)
            .bind(&record.description)
            .bind(record.created_at)
            .bind(record.updated_at)
            .execute(&mut *transaction)
            .await?;
            query("DELETE FROM share_group_members WHERE group_id = ?")
                .bind(&record.id)
                .execute(&mut *transaction)
                .await?;
            for member in members {
                query(
                    r#"
                    INSERT INTO share_group_members (group_id, username, added_at)
                    VALUES (?, ?, ?)
                    "#,
                )
                .bind(&member.group_id)
                .bind(&member.username)
                .bind(member.added_at)
                .execute(&mut *transaction)
                .await?;
            }
            Ok::<(), sqlx_core::Error>(())
        }
        .await;
        if let Err(error) = result {
            transaction.rollback().await?;
            return Err(error.into());
        }
        transaction.commit().await?;
        Ok(())
    }

    /// Delete a share group and its members.
    pub async fn delete_share_group(&self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut transaction = self.pool.begin().await?;
        query("DELETE FROM share_group_members WHERE group_id = ?")
            .bind(id)
            .execute(&mut *transaction)
            .await?;
        query("DELETE FROM share_groups WHERE id = ?")
            .bind(id)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(())
    }

    /// List persisted share groups.
    pub async fn list_share_groups(
        &self,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<ShareGroupRecord>, Box<dyn std::error::Error>> {
        let records = query_as::<_, ShareGroupRecord>(
            "SELECT id, name, description, created_at, updated_at FROM share_groups ORDER BY updated_at DESC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    /// Insert or update a share group member.
    pub async fn upsert_share_group_member(
        &self,
        record: &ShareGroupMemberRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query(
            r#"
            INSERT OR REPLACE INTO share_group_members (group_id, username, added_at)
            VALUES (?, ?, ?)
            "#,
        )
        .bind(&record.group_id)
        .bind(&record.username)
        .bind(record.added_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Delete a share group member.
    pub async fn delete_share_group_member(
        &self,
        group_id: &str,
        username: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query("DELETE FROM share_group_members WHERE group_id = ? AND username = ?")
            .bind(group_id)
            .bind(username)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// List persisted share group members.
    pub async fn list_share_group_members(
        &self,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<ShareGroupMemberRecord>, Box<dyn std::error::Error>> {
        let records = query_as::<_, ShareGroupMemberRecord>(
            "SELECT group_id, username, added_at FROM share_group_members ORDER BY added_at DESC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    /// Insert or update a collection.
    pub async fn upsert_collection(
        &self,
        record: &CollectionRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query(
            r#"
            INSERT OR REPLACE INTO collections
                (id, owner_user_id, name, description, collection_type, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&record.id)
        .bind(&record.owner_user_id)
        .bind(&record.name)
        .bind(&record.description)
        .bind(&record.collection_type)
        .bind(record.created_at)
        .bind(record.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Persist a collection and its exact ordered item snapshot atomically.
    pub async fn replace_collection(
        &self,
        record: &CollectionRecord,
        items: &[CollectionItemRecord],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut transaction = self.pool.begin().await?;
        let result = async {
            query(
                r#"
                INSERT INTO collections
                    (id, owner_user_id, name, description, collection_type, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(id) DO UPDATE SET
                    owner_user_id = excluded.owner_user_id,
                    name = excluded.name,
                    description = excluded.description,
                    collection_type = excluded.collection_type,
                    created_at = excluded.created_at,
                    updated_at = excluded.updated_at
                "#,
            )
            .bind(&record.id)
            .bind(&record.owner_user_id)
            .bind(&record.name)
            .bind(&record.description)
            .bind(&record.collection_type)
            .bind(record.created_at)
            .bind(record.updated_at)
            .execute(&mut *transaction)
            .await?;
            query("DELETE FROM collection_items WHERE collection_id = ?")
                .bind(&record.id)
                .execute(&mut *transaction)
                .await?;
            for item in items {
                query(
                    r#"
                    INSERT INTO collection_items
                        (id, collection_id, content_id, artist, title, kind, file_name, album, content_hash, added_at, position)
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                )
                .bind(&item.id)
                .bind(&item.collection_id)
                .bind(&item.content_id)
                .bind(&item.artist)
                .bind(&item.title)
                .bind(&item.kind)
                .bind(&item.file_name)
                .bind(&item.album)
                .bind(&item.content_hash)
                .bind(item.added_at)
                .bind(item.position)
                .execute(&mut *transaction)
                .await?;
            }
            Ok::<(), sqlx_core::Error>(())
        }
        .await;
        if let Err(error) = result {
            transaction.rollback().await?;
            return Err(error.into());
        }
        transaction.commit().await?;
        Ok(())
    }

    /// Delete a collection, its items, and its access grants atomically.
    pub async fn delete_collection(&self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut transaction = self.pool.begin().await?;
        query(
            "DELETE FROM share_access_tokens WHERE grant_id IN (SELECT id FROM share_grants WHERE collection_id = ?)",
        )
        .bind(id)
        .execute(&mut *transaction)
        .await?;
        query("DELETE FROM share_grants WHERE collection_id = ?")
            .bind(id)
            .execute(&mut *transaction)
            .await?;
        query("DELETE FROM collection_items WHERE collection_id = ?")
            .bind(id)
            .execute(&mut *transaction)
            .await?;
        query("DELETE FROM collections WHERE id = ?")
            .bind(id)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(())
    }

    /// List persisted collections.
    pub async fn list_collections(
        &self,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<CollectionRecord>, Box<dyn std::error::Error>> {
        let records = query_as::<_, CollectionRecord>(
            "SELECT id, owner_user_id, name, description, collection_type, created_at, updated_at FROM collections ORDER BY updated_at DESC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    /// Insert or update a collection item.
    pub async fn upsert_collection_item(
        &self,
        record: &CollectionItemRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query(
            r#"
            INSERT OR REPLACE INTO collection_items
                (id, collection_id, content_id, artist, title, kind, file_name, album, content_hash, added_at, position)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&record.id)
        .bind(&record.collection_id)
        .bind(&record.content_id)
        .bind(&record.artist)
        .bind(&record.title)
        .bind(&record.kind)
        .bind(&record.file_name)
        .bind(&record.album)
        .bind(&record.content_hash)
        .bind(record.added_at)
        .bind(record.position)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Delete a collection item.
    pub async fn delete_collection_item(&self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        query("DELETE FROM collection_items WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// List persisted collection items.
    pub async fn list_collection_items(
        &self,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<CollectionItemRecord>, Box<dyn std::error::Error>> {
        let records = query_as::<_, CollectionItemRecord>(
            "SELECT id, collection_id, content_id, artist, title, kind, file_name, album, content_hash, added_at, position FROM collection_items ORDER BY collection_id, position, added_at LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    /// Insert or update a library item.
    pub async fn upsert_library_item(
        &self,
        record: &LibraryItemRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query(
            r#"
            INSERT OR REPLACE INTO library_items (id, artist, title, kind, created_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&record.id)
        .bind(&record.artist)
        .bind(&record.title)
        .bind(&record.kind)
        .bind(record.created_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Upsert a library item and runtime compatibility state atomically.
    pub async fn upsert_library_item_and_runtime_compat_state(
        &self,
        library: &LibraryItemRecord,
        runtime: &RuntimeCompatRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut transaction = self.pool.begin().await?;
        query(
            r#"
            INSERT OR REPLACE INTO library_items (id, artist, title, kind, created_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&library.id)
        .bind(&library.artist)
        .bind(&library.title)
        .bind(&library.kind)
        .bind(library.created_at)
        .execute(&mut *transaction)
        .await?;
        query(
            r#"
            INSERT OR REPLACE INTO runtime_compat_state
            (id, application_restart_requested, gc_runs, autoreplace_enabled, relay_enabled,
             relay_agent_enabled, bridge_running, bridge_config_updates, profile_invites_created,
             options_updates, options_yaml_uploads, options_yaml_validations, cache_warm_runs,
             backfill_runs, songid_runs, songid_run_records_json, lidarr_sync_runs,
             lidarr_manual_imports, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&runtime.id)
        .bind(runtime.application_restart_requested)
        .bind(runtime.gc_runs)
        .bind(runtime.autoreplace_enabled)
        .bind(runtime.relay_enabled)
        .bind(runtime.relay_agent_enabled)
        .bind(runtime.bridge_running)
        .bind(runtime.bridge_config_updates)
        .bind(runtime.profile_invites_created)
        .bind(runtime.options_updates)
        .bind(runtime.options_yaml_uploads)
        .bind(runtime.options_yaml_validations)
        .bind(runtime.cache_warm_runs)
        .bind(runtime.backfill_runs)
        .bind(runtime.songid_runs)
        .bind(&runtime.songid_run_records_json)
        .bind(runtime.lidarr_sync_runs)
        .bind(runtime.lidarr_manual_imports)
        .bind(runtime.updated_at)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(())
    }

    /// Insert or update library items atomically.
    pub async fn upsert_library_items(
        &self,
        records: &[LibraryItemRecord],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut transaction = self.pool.begin().await?;
        for batch in records.chunks(SQLITE_PARAMETER_CHUNK / 5) {
            let statement = format!(
                r#"
                INSERT OR REPLACE INTO library_items (id, artist, title, kind, created_at)
                VALUES {}
                "#,
                sql_value_rows(batch.len(), 5)
            );
            let mut insert = query(AssertSqlSafe(statement));
            for record in batch {
                insert = insert
                    .bind(&record.id)
                    .bind(&record.artist)
                    .bind(&record.title)
                    .bind(&record.kind)
                    .bind(record.created_at);
            }
            insert.execute(&mut *transaction).await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    /// Delete a library item.
    pub async fn delete_library_item(&self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        query("DELETE FROM library_items WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// List persisted library items.
    pub async fn list_library_items(
        &self,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<LibraryItemRecord>, Box<dyn std::error::Error>> {
        let records = query_as::<_, LibraryItemRecord>(
            "SELECT id, artist, title, kind, created_at FROM library_items ORDER BY created_at DESC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
            .await?;
        Ok(records)
    }

    /// Insert or update a destination.
    pub async fn upsert_destination(
        &self,
        record: &DestinationRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query(
            r#"
            INSERT OR REPLACE INTO destinations (id, name, path, is_default, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&record.id)
        .bind(&record.name)
        .bind(&record.path)
        .bind(record.is_default)
        .bind(record.created_at)
        .bind(record.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Delete a destination.
    pub async fn delete_destination(&self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        query("DELETE FROM destinations WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// List persisted destinations.
    pub async fn list_destinations(
        &self,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<DestinationRecord>, Box<dyn std::error::Error>> {
        let records = query_as::<_, DestinationRecord>(
            "SELECT id, name, path, is_default, created_at, updated_at FROM destinations ORDER BY is_default DESC, name, id LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
            .await?;
        Ok(records)
    }

    /// Insert or update a now-playing projection.
    pub async fn upsert_now_playing(
        &self,
        record: &NowPlayingRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query(
            r#"
            INSERT OR REPLACE INTO now_playing (username, artist, title, updated_at)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(&record.username)
        .bind(&record.artist)
        .bind(&record.title)
        .bind(record.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Clear all persisted now-playing projections.
    pub async fn clear_now_playing(&self) -> Result<(), Box<dyn std::error::Error>> {
        query("DELETE FROM now_playing").execute(&self.pool).await?;
        Ok(())
    }

    /// List persisted now-playing projections.
    pub async fn list_now_playing(
        &self,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<NowPlayingRecord>, Box<dyn std::error::Error>> {
        let records = query_as::<_, NowPlayingRecord>(
            "SELECT username, artist, title, updated_at FROM now_playing ORDER BY updated_at DESC, username LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
            .await?;
        Ok(records)
    }

    /// Insert or update a browse cache projection.
    pub async fn upsert_browse_record(
        &self,
        record: &BrowseRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query(
            r#"
            INSERT OR REPLACE INTO browse_records
            (username, status, entries_json, reason, folder, indirect_token, requested_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&record.username)
        .bind(&record.status)
        .bind(&record.entries_json)
        .bind(&record.reason)
        .bind(&record.folder)
        .bind(record.indirect_token)
        .bind(record.requested_at)
        .bind(record.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Delete a browse cache projection.
    pub async fn delete_browse_record(
        &self,
        username: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query("DELETE FROM browse_records WHERE username = ?")
            .bind(username)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// List persisted browse cache projections.
    pub async fn list_browse_records(
        &self,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<BrowseRecord>, Box<dyn std::error::Error>> {
        let records = query_as::<_, BrowseRecord>(
            "SELECT username, status, entries_json, reason, folder, indirect_token, requested_at, updated_at FROM browse_records ORDER BY updated_at DESC, username LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    /// Upsert runtime compatibility singleton state.
    pub async fn upsert_runtime_compat_state(
        &self,
        record: &RuntimeCompatRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query(
            r#"
            INSERT OR REPLACE INTO runtime_compat_state
            (id, application_restart_requested, gc_runs, autoreplace_enabled, relay_enabled,
             relay_agent_enabled, bridge_running, bridge_config_updates, profile_invites_created,
             options_updates, options_yaml_uploads, options_yaml_validations, cache_warm_runs,
             backfill_runs, songid_runs, songid_run_records_json, lidarr_sync_runs,
             lidarr_manual_imports, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&record.id)
        .bind(record.application_restart_requested)
        .bind(record.gc_runs)
        .bind(record.autoreplace_enabled)
        .bind(record.relay_enabled)
        .bind(record.relay_agent_enabled)
        .bind(record.bridge_running)
        .bind(record.bridge_config_updates)
        .bind(record.profile_invites_created)
        .bind(record.options_updates)
        .bind(record.options_yaml_uploads)
        .bind(record.options_yaml_validations)
        .bind(record.cache_warm_runs)
        .bind(record.backfill_runs)
        .bind(record.songid_runs)
        .bind(&record.songid_run_records_json)
        .bind(record.lidarr_sync_runs)
        .bind(record.lidarr_manual_imports)
        .bind(record.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Get persisted runtime compatibility singleton state.
    pub async fn get_runtime_compat_state(
        &self,
    ) -> Result<Option<RuntimeCompatRecord>, Box<dyn std::error::Error>> {
        let record =
            query_as::<_, RuntimeCompatRecord>("SELECT * FROM runtime_compat_state WHERE id = ?")
                .bind("runtime")
                .fetch_optional(&self.pool)
                .await?;
        Ok(record)
    }

    // ========================================================================
    // Database Maintenance
    // ========================================================================

    /// Get database statistics
    pub async fn get_stats(&self) -> Result<DatabaseStats, Box<dyn std::error::Error>> {
        // This endpoint is served from a single-connection pool. Keep all
        // counts in one SQLite statement so the result has one read snapshot
        // and does not acquire/release the connection once per table.
        let rows: Vec<(String, i64)> = query_as(
            r#"
            WITH counts(name, value) AS (
                SELECT 'search_count', COUNT(*) FROM searches
                UNION ALL SELECT 'search_result_count', COUNT(*) FROM search_results
                UNION ALL SELECT 'transfer_count', COUNT(*) FROM transfers
                UNION ALL SELECT 'transfer_event_count', COUNT(*) FROM transfer_events
                UNION ALL SELECT 'share_file_count', COUNT(*) FROM share_files
                UNION ALL SELECT 'event_count', COUNT(*) FROM events
                UNION ALL SELECT 'message_count', COUNT(*) FROM messages
                UNION ALL SELECT 'user_count', COUNT(*) FROM user_stats
                UNION ALL SELECT 'user_projection_count', COUNT(*) FROM user_records
                UNION ALL SELECT 'room_count', COUNT(*) FROM rooms WHERE subscribed = 1
                UNION ALL SELECT 'user_note_count', COUNT(*) FROM user_notes
                UNION ALL SELECT 'interest_count', COUNT(*) FROM interests
                UNION ALL SELECT 'security_ban_count', COUNT(*) FROM security_bans
                UNION ALL SELECT 'wishlist_count', COUNT(*) FROM wishlist_items
                UNION ALL SELECT 'contact_count', COUNT(*) FROM contacts
                UNION ALL SELECT 'share_grant_count', COUNT(*) FROM share_grants
                UNION ALL SELECT 'share_access_token_count', COUNT(*) FROM share_access_tokens
                UNION ALL SELECT 'share_group_count', COUNT(*) FROM share_groups
                UNION ALL SELECT 'share_group_member_count', COUNT(*) FROM share_group_members
                UNION ALL SELECT 'collection_count', COUNT(*) FROM collections
                UNION ALL SELECT 'collection_item_count', COUNT(*) FROM collection_items
                UNION ALL SELECT 'library_item_count', COUNT(*) FROM library_items
                UNION ALL SELECT 'destination_count', COUNT(*) FROM destinations
                UNION ALL SELECT 'now_playing_count', COUNT(*) FROM now_playing
                UNION ALL SELECT 'browse_count', COUNT(*) FROM browse_records
                UNION ALL SELECT 'runtime_state_count', COUNT(*) FROM runtime_compat_state
                UNION ALL SELECT 'oauth_state_count', COUNT(*) FROM oauth_states
                UNION ALL SELECT 'webhook_count', COUNT(*) FROM webhooks
                UNION ALL SELECT 'webhook_log_count', COUNT(*) FROM webhook_logs
            )
            SELECT name, value FROM counts
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        let counts = rows
            .into_iter()
            .collect::<std::collections::BTreeMap<_, _>>();
        let count = |name: &str| -> Result<u64, Box<dyn std::error::Error>> {
            let value = counts.get(name).copied().ok_or_else(|| {
                Box::<dyn std::error::Error>::from(format!(
                    "database statistics query omitted {name}"
                ))
            })?;
            Ok(nonnegative_database_count(value)?)
        };

        Ok(DatabaseStats {
            search_count: count("search_count")?,
            search_result_count: count("search_result_count")?,
            transfer_count: count("transfer_count")?,
            transfer_event_count: count("transfer_event_count")?,
            share_file_count: count("share_file_count")?,
            event_count: count("event_count")?,
            message_count: count("message_count")?,
            user_count: count("user_count")?,
            user_projection_count: count("user_projection_count")?,
            room_count: count("room_count")?,
            user_note_count: count("user_note_count")?,
            interest_count: count("interest_count")?,
            security_ban_count: count("security_ban_count")?,
            wishlist_count: count("wishlist_count")?,
            contact_count: count("contact_count")?,
            share_grant_count: count("share_grant_count")?,
            share_access_token_count: count("share_access_token_count")?,
            share_group_count: count("share_group_count")?,
            share_group_member_count: count("share_group_member_count")?,
            collection_count: count("collection_count")?,
            collection_item_count: count("collection_item_count")?,
            library_item_count: count("library_item_count")?,
            destination_count: count("destination_count")?,
            now_playing_count: count("now_playing_count")?,
            browse_count: count("browse_count")?,
            runtime_state_count: count("runtime_state_count")?,
            oauth_state_count: count("oauth_state_count")?,
            webhook_count: count("webhook_count")?,
            webhook_log_count: count("webhook_log_count")?,
        })
    }

    /// Cleanup old records (older than specified days)
    pub async fn cleanup_old_records(&self, days: i32) -> Result<u64, Box<dyn std::error::Error>> {
        let cutoff =
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64 - (days as i64 * 86400);

        let result = query("DELETE FROM messages WHERE created_at < ?")
            .bind(cutoff)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    /// Vacuum database (optimize storage)
    pub async fn vacuum(&self) -> Result<(), Box<dyn std::error::Error>> {
        query("VACUUM").execute(&self.pool).await?;
        Ok(())
    }

    // ========================================================================
    // Webhook Operations
    // ========================================================================

    /// Insert or update webhook record
    pub async fn insert_webhook(
        &self,
        record: &WebhookRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query(
            r#"
            INSERT OR REPLACE INTO webhooks (id, url, events, secret, active, created_at, last_triggered, retry_count, max_retries, timeout_seconds)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&record.id)
        .bind(&record.url)
        .bind(&record.events)
        .bind(&record.secret)
        .bind(record.active as i32)
        .bind(record.created_at)
        .bind(record.last_triggered)
        .bind(record.retry_count)
        .bind(record.max_retries)
        .bind(record.timeout_seconds)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Get webhook record by ID
    pub async fn get_webhook(
        &self,
        id: &str,
    ) -> Result<Option<WebhookRecord>, Box<dyn std::error::Error>> {
        let record = query_as::<_, WebhookRecord>(
            r#"SELECT id, url, events, secret, active, created_at, last_triggered, retry_count, max_retries, timeout_seconds FROM webhooks WHERE id = ?"#
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(record)
    }

    /// List all webhooks
    pub async fn list_webhooks(&self) -> Result<Vec<WebhookRecord>, Box<dyn std::error::Error>> {
        let records = query_as::<_, WebhookRecord>(
            r#"SELECT id, url, events, secret, active, created_at, last_triggered, retry_count, max_retries, timeout_seconds FROM webhooks ORDER BY created_at DESC"#
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    /// List active webhooks
    pub async fn list_active_webhooks(
        &self,
    ) -> Result<Vec<WebhookRecord>, Box<dyn std::error::Error>> {
        let records = query_as::<_, WebhookRecord>(
            r#"SELECT id, url, events, secret, active, created_at, last_triggered, retry_count, max_retries, timeout_seconds FROM webhooks WHERE active = 1 ORDER BY created_at DESC"#
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    /// Delete webhook
    pub async fn delete_webhook(&self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut transaction = self.pool.begin().await?;
        let log_delete = query("DELETE FROM webhook_logs WHERE webhook_id = ?")
            .bind(id)
            .execute(&mut *transaction)
            .await;
        if let Err(delete_error) = log_delete {
            transaction.rollback().await.map_err(|rollback_error| {
                format!(
                    "webhook log deletion failed ({delete_error}); transaction rollback failed: {rollback_error}"
                )
            })?;
            return Err(delete_error.into());
        }
        let webhook_delete = query("DELETE FROM webhooks WHERE id = ?")
            .bind(id)
            .execute(&mut *transaction)
            .await;
        if let Err(delete_error) = webhook_delete {
            transaction.rollback().await.map_err(|rollback_error| {
                format!(
                    "webhook deletion failed ({delete_error}); transaction rollback failed: {rollback_error}"
                )
            })?;
            return Err(delete_error.into());
        }
        transaction.commit().await?;
        Ok(())
    }

    /// Update webhook active status
    pub async fn update_webhook_active(
        &self,
        id: &str,
        active: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query("UPDATE webhooks SET active = ? WHERE id = ?")
            .bind(active as i32)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Record the most recent delivery outcome for a webhook.
    pub async fn update_webhook_delivery_stats(
        &self,
        id: &str,
        last_triggered: i64,
        retry_count: u32,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        let retry_count = i32::try_from(retry_count).unwrap_or(i32::MAX);
        let result = query("UPDATE webhooks SET last_triggered = ?, retry_count = ? WHERE id = ?")
            .bind(last_triggered)
            .bind(retry_count)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// Insert webhook log record
    pub async fn insert_webhook_log(
        &self,
        record: &WebhookLogRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query(
            r#"
            INSERT INTO webhook_logs (id, webhook_id, event, correlation_id, status, request_body, response_status, response_body, error_message, attempt, timestamp)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&record.id)
        .bind(&record.webhook_id)
        .bind(&record.event)
        .bind(&record.correlation_id)
        .bind(&record.status)
        .bind(&record.request_body)
        .bind(record.response_status)
        .bind(&record.response_body)
        .bind(&record.error_message)
        .bind(record.attempt)
        .bind(record.timestamp)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Mark every queued log for one webhook dispatch with its terminal outcome.
    pub async fn complete_webhook_logs(
        &self,
        webhook_id: &str,
        correlation_id: &str,
        status: &str,
        error_message: Option<&str>,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        self.complete_webhook_logs_with_attempt(
            webhook_id,
            correlation_id,
            status,
            error_message,
            None,
        )
        .await
    }

    /// Mark queued logs with their terminal outcome and actual attempt count.
    pub async fn complete_webhook_logs_with_attempt(
        &self,
        webhook_id: &str,
        correlation_id: &str,
        status: &str,
        error_message: Option<&str>,
        attempt: Option<i32>,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        let result = query(
            r#"
            UPDATE webhook_logs
            SET status = ?, error_message = ?, attempt = COALESCE(?, attempt)
            WHERE webhook_id = ? AND correlation_id = ? AND status = 'queued'
            "#,
        )
        .bind(status)
        .bind(error_message)
        .bind(attempt)
        .bind(webhook_id)
        .bind(correlation_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Get webhook logs for a specific webhook
    pub async fn get_webhook_logs(
        &self,
        webhook_id: &str,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<WebhookLogRecord>, Box<dyn std::error::Error>> {
        let records = query_as::<_, WebhookLogRecord>(
            r#"SELECT id, webhook_id, event, correlation_id, status, request_body, response_status, response_body, error_message, attempt, timestamp FROM webhook_logs WHERE webhook_id = ? ORDER BY timestamp DESC LIMIT ? OFFSET ?"#
        )
        .bind(webhook_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    /// Get recent webhook logs by event
    pub async fn get_logs_by_event(
        &self,
        event: &str,
        limit: i32,
    ) -> Result<Vec<WebhookLogRecord>, Box<dyn std::error::Error>> {
        let records = query_as::<_, WebhookLogRecord>(
            r#"SELECT id, webhook_id, event, correlation_id, status, request_body, response_status, response_body, error_message, attempt, timestamp FROM webhook_logs WHERE event = ? ORDER BY timestamp DESC LIMIT ?"#
        )
        .bind(event)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    /// Get failed webhook logs for retry
    pub async fn get_failed_webhook_logs(
        &self,
        limit: i32,
    ) -> Result<Vec<WebhookLogRecord>, Box<dyn std::error::Error>> {
        let records = query_as::<_, WebhookLogRecord>(
            r#"SELECT id, webhook_id, event, correlation_id, status, request_body, response_status, response_body, error_message, attempt, timestamp FROM webhook_logs WHERE status IN ('failed', 'timeout') AND attempt <= (SELECT max_retries FROM webhooks WHERE webhooks.id = webhook_logs.webhook_id) ORDER BY timestamp ASC LIMIT ?"#
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    /// Delete old webhook logs
    pub async fn delete_old_webhook_logs(
        &self,
        days: i32,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        let cutoff =
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64 - (days as i64 * 86400);

        let result = query("DELETE FROM webhook_logs WHERE timestamp < ?")
            .bind(cutoff)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    /// Load wishlist scheduler state
    pub async fn load_wishlist_scheduler_state(
        &self,
    ) -> Result<Option<(usize, Option<u64>)>, Box<dyn std::error::Error>> {
        let result = query_as::<_, (i64, Option<i64>)>(
            "SELECT next_index, server_interval_seconds FROM wishlist_scheduler_state WHERE id = 1",
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|(next_index, server_interval)| {
            (next_index as usize, server_interval.map(|secs| secs as u64))
        }))
    }

    /// Save wishlist scheduler state
    pub async fn save_wishlist_scheduler_state(
        &self,
        next_index: usize,
        server_interval_seconds: Option<u64>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;

        query(
            r#"
            INSERT OR REPLACE INTO wishlist_scheduler_state (id, next_index, server_interval_seconds, updated_at)
            VALUES (1, ?, ?, ?)
            "#,
        )
        .bind(next_index as i64)
        .bind(server_interval_seconds.map(|secs| secs as i64))
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Load distributed tree state
    pub async fn load_distributed_tree_state(
        &self,
    ) -> Result<Option<(u32, String, Option<String>)>, Box<dyn std::error::Error>> {
        let result = query_as::<_, (i64, String, Option<String>)>(
            "SELECT branch_level, branch_root, parent_username FROM distributed_tree_state WHERE id = 1",
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|(branch_level, branch_root, parent_username)| {
            (branch_level as u32, branch_root, parent_username)
        }))
    }

    /// Save distributed tree state
    pub async fn save_distributed_tree_state(
        &self,
        branch_level: u32,
        branch_root: &str,
        parent_username: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;

        query(
            r#"
            INSERT OR REPLACE INTO distributed_tree_state (id, branch_level, branch_root, parent_username, updated_at)
            VALUES (1, ?, ?, ?, ?)
            "#,
        )
        .bind(branch_level as i64)
        .bind(branch_root)
        .bind(parent_username)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Load distributed children
    pub async fn load_distributed_children(
        &self,
    ) -> Result<Vec<(String, u32)>, Box<dyn std::error::Error>> {
        let results = query_as::<_, (String, i64)>(
            "SELECT username, depth FROM distributed_children ORDER BY username",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(results
            .into_iter()
            .map(|(username, depth)| (username, depth as u32))
            .collect())
    }

    /// Save distributed children
    pub async fn save_distributed_children(
        &self,
        children: &[(String, u32)],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;

        // Clear existing children
        query("DELETE FROM distributed_children")
            .execute(&self.pool)
            .await?;

        // Insert new children
        for (username, depth) in children {
            query(
                r#"
                INSERT INTO distributed_children (username, depth, updated_at)
                VALUES (?, ?, ?)
                "#,
            )
            .bind(username)
            .bind(*depth as i64)
            .bind(now)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }
}

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
mod tests {
    use super::*;

    #[test]
    fn database_counts_reject_negative_values_without_wrapping() {
        assert_eq!(nonnegative_database_count(0), Ok(0));
        assert_eq!(nonnegative_database_count(i64::MAX), Ok(i64::MAX as u64));
        assert!(nonnegative_database_count(-1).is_err());
    }

    #[tokio::test]
    async fn test_database_creation() {
        let db = DatabaseManager::in_memory().await.unwrap();
        let stats = db.get_stats().await.unwrap();
        assert_eq!(stats.search_count, 0);
        assert_eq!(stats.search_result_count, 0);
        assert_eq!(stats.transfer_count, 0);
        assert_eq!(stats.message_count, 0);
        assert_eq!(stats.user_count, 0);
        assert_eq!(stats.room_count, 0);
    }

    #[tokio::test]
    async fn common_paged_reads_use_ordered_indexes() {
        let db = DatabaseManager::in_memory().await.unwrap();

        let message_plan = query(
            "EXPLAIN QUERY PLAN SELECT id FROM messages WHERE username = 'user' ORDER BY created_at DESC LIMIT 100 OFFSET 0",
        )
        .fetch_all(&db.pool)
        .await
        .unwrap();
        let message_details = message_plan
            .iter()
            .map(|row| row.try_get::<String, _>("detail").unwrap())
            .collect::<Vec<_>>();
        assert!(message_details
            .iter()
            .any(|detail| detail.contains("idx_messages_username_created")));
        assert!(!message_details
            .iter()
            .any(|detail| detail.contains("TEMP B-TREE")));

        let library_plan = query(
            "EXPLAIN QUERY PLAN SELECT id FROM library_items ORDER BY created_at DESC LIMIT 100 OFFSET 0",
        )
        .fetch_all(&db.pool)
        .await
        .unwrap();
        let library_details = library_plan
            .iter()
            .map(|row| row.try_get::<String, _>("detail").unwrap())
            .collect::<Vec<_>>();
        assert!(library_details
            .iter()
            .any(|detail| detail.contains("idx_library_items_created")));
        assert!(!library_details
            .iter()
            .any(|detail| detail.contains("TEMP B-TREE")));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn database_file_is_private_and_rejects_symlinks() {
        use std::os::unix::fs::{symlink, PermissionsExt};

        let root = std::env::temp_dir().join(format!(
            "slskr-private-db-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let db_path = root.join("slskr.db");
        std::fs::write(&db_path, []).unwrap();
        std::fs::set_permissions(&db_path, std::fs::Permissions::from_mode(0o666)).unwrap();

        let db = DatabaseManager::new(db_path.to_str().unwrap())
            .await
            .unwrap();
        db.close_for_test().await;
        assert_eq!(
            std::fs::metadata(&db_path).unwrap().permissions().mode() & 0o777,
            0o600
        );

        let linked_path = root.join("linked.db");
        symlink(&db_path, &linked_path).unwrap();
        assert!(DatabaseManager::new(linked_path.to_str().unwrap())
            .await
            .is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn test_search_operations() {
        let db = DatabaseManager::in_memory().await.unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let record = SearchRecord {
            id: "search_1".to_string(),
            query: "test query".to_string(),
            status: "completed".to_string(),
            result_count: 42,
            created_at: now,
            completed_at: Some(now + 100),
            room: None,
            target: None,
            fallback_attempts: 0,
        };

        db.insert_search(&record).await.unwrap();
        db.upsert_search_identity("search_1", "external-search-1")
            .await
            .unwrap();
        let retrieved = db.get_search("search_1").await.unwrap().unwrap();
        assert_eq!(retrieved.query, "test query");
        assert_eq!(retrieved.result_count, 42);
        assert_eq!(
            db.list_search_identities().await.unwrap().get("search_1"),
            Some(&"external-search-1".to_owned())
        );

        db.update_search_status("search_1", "archived")
            .await
            .unwrap();
        let updated = db.get_search("search_1").await.unwrap().unwrap();
        assert_eq!(updated.status, "archived");

        db.delete_search("search_1").await.unwrap();
        assert!(db.list_search_identities().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn search_result_replacement_rolls_back_on_insert_failure() {
        let db = DatabaseManager::in_memory().await.unwrap();
        let result = |filename: &str| SearchResultRecord {
            id: 0,
            search_id: "search_atomic".to_owned(),
            peer_username: Some("peer".to_owned()),
            filename: filename.to_owned(),
            size: 10,
            extension: "flac".to_owned(),
            bit_rate: None,
            sample_rate: None,
            bit_depth: None,
            length_seconds: None,
            locked: false,
            slot_free: Some(true),
            average_speed: Some(1),
            queue_length: Some(0),
            created_at: 1,
        };
        db.replace_search_results("search_atomic", &[result("original.flac")])
            .await
            .unwrap();
        query(
            r#"
            CREATE TRIGGER reject_bad_search_result
            BEFORE INSERT ON search_results
            WHEN NEW.filename = 'rejected.flac'
            BEGIN
                SELECT RAISE(ABORT, 'forced search result failure');
            END
            "#,
        )
        .execute(&db.pool)
        .await
        .unwrap();

        assert!(db
            .replace_search_results(
                "search_atomic",
                &[result("new.flac"), result("rejected.flac")],
            )
            .await
            .is_err());
        let persisted = db
            .list_search_results(Some("search_atomic"), 10, 0)
            .await
            .unwrap();
        assert_eq!(persisted.len(), 1);
        assert_eq!(persisted[0].filename, "original.flac");
    }

    #[tokio::test]
    async fn test_transfer_operations() {
        let db = DatabaseManager::in_memory().await.unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let record = TransferRecord {
            id: "transfer_1".to_string(),
            direction: "download".to_string(),
            filename: "test.mp3".to_string(),
            peer_username: "user1".to_string(),
            filesize: 1000000,
            progress: 500000,
            status: "active".to_string(),
            started_at: now,
            completed_at: None,
            request_id: Some("request_1".to_owned()),
            wishlist_item_id: Some("wish-1".to_owned()),
            request_name: Some("Test".to_owned()),
            destination_directory: Some("Artist/Album".to_owned()),
            local_path: Some("downloads/Artist/Album/test.mp3".to_owned()),
            batch_id: None,
            reason: None,
            bit_rate: Some(320),
            sample_rate: Some(44_100),
            bit_depth: Some(16),
            length_seconds: Some(180),
            artist: Some("Artist".to_owned()),
            album: Some("Album".to_owned()),
            title: Some("Test".to_owned()),
            track_number: Some(1),
            year: Some(2026),
            attempts: 1,
            auto_replace_attempts: 0,
            next_attempt_at: None,
        };

        db.insert_transfer(&record).await.unwrap();
        let retrieved = db.get_transfer("transfer_1").await.unwrap().unwrap();
        assert_eq!(retrieved.filename, "test.mp3");
        assert_eq!(retrieved.progress, 500000);
        assert_eq!(retrieved.bit_rate, Some(320));
        assert_eq!(retrieved.request_id.as_deref(), Some("request_1"));
        assert_eq!(retrieved.wishlist_item_id.as_deref(), Some("wish-1"));

        db.update_transfer_progress("transfer_1", 750000)
            .await
            .unwrap();
        let updated = db.get_transfer("transfer_1").await.unwrap().unwrap();
        assert_eq!(updated.progress, 750000);

        db.update_transfer_progress("transfer_1", u64::MAX)
            .await
            .unwrap();
        let saturated = db.get_transfer("transfer_1").await.unwrap().unwrap();
        assert_eq!(saturated.progress, i64::MAX);

        db.insert_transfer_event(&TransferEventRecord {
            id: 0,
            transfer_id: "transfer_1".to_owned(),
            direction: "download".to_owned(),
            token: 1,
            filename: "test.mp3".to_owned(),
            peer_username: Some("user1".to_owned()),
            filesize: 1_000_000,
            progress: 750_000,
            status: "peer_lookup".to_owned(),
            reason: None,
            created_at: now,
        })
        .await
        .unwrap();
        db.rollback_staged_transfers(&["transfer_1".to_owned()])
            .await
            .unwrap();
        assert!(db.get_transfer("transfer_1").await.unwrap().is_none());
        assert!(db
            .list_transfer_events(Some("transfer_1"), 10, 0)
            .await
            .unwrap()
            .is_empty());

        let mut ids = Vec::new();
        for suffix in ["a", "b", "c"] {
            let mut additional = record.clone();
            additional.id = format!("transfer_{suffix}");
            db.insert_transfer(&additional).await.unwrap();
            ids.push(additional.id);
        }
        db.delete_transfers(&ids).await.unwrap();
        assert!(db.get_transfer("transfer_a").await.unwrap().is_none());
        assert!(db.get_transfer("transfer_b").await.unwrap().is_none());
        assert!(db.get_transfer("transfer_c").await.unwrap().is_none());
        db.delete_transfers(&[]).await.unwrap();
    }

    #[tokio::test]
    async fn transfer_record_and_event_batches_roll_back_atomically() {
        let db = DatabaseManager::in_memory().await.unwrap();
        query(
            r#"
            CREATE TRIGGER reject_bad_transfer_event
            BEFORE INSERT ON transfer_events
            WHEN NEW.transfer_id = 'transfer_bad'
            BEGIN
                SELECT RAISE(ABORT, 'forced transfer event failure');
            END
            "#,
        )
        .execute(&db.pool)
        .await
        .unwrap();

        let transfer = |id: &str| TransferRecord {
            id: id.to_owned(),
            direction: "download".to_owned(),
            filename: format!("{id}.flac"),
            peer_username: "peer".to_owned(),
            filesize: 10,
            progress: 0,
            status: "queued".to_owned(),
            started_at: 1,
            completed_at: None,
            request_id: None,
            wishlist_item_id: None,
            request_name: None,
            destination_directory: None,
            local_path: None,
            batch_id: None,
            reason: None,
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
        };
        let event = |id: &str| TransferEventRecord {
            id: 0,
            transfer_id: id.to_owned(),
            direction: "download".to_owned(),
            token: 1,
            filename: format!("{id}.flac"),
            peer_username: Some("peer".to_owned()),
            filesize: 10,
            progress: 0,
            status: "queued".to_owned(),
            reason: None,
            created_at: 1,
        };
        let records = [
            (transfer("transfer_good"), event("transfer_good")),
            (transfer("transfer_bad"), event("transfer_bad")),
        ];

        assert!(db
            .insert_transfer_records_with_events(&records)
            .await
            .is_err());
        for id in ["transfer_good", "transfer_bad"] {
            assert!(db.get_transfer(id).await.unwrap().is_none(), "{id}");
            assert!(
                db.list_transfer_events(Some(id), 10, 0)
                    .await
                    .unwrap()
                    .is_empty(),
                "{id}"
            );
        }
    }

    #[tokio::test]
    async fn webhook_delete_rolls_back_log_deletion_on_failure() {
        let db = DatabaseManager::in_memory().await.unwrap();
        let webhook = WebhookRecord {
            id: "hook_atomic".to_owned(),
            url: "https://example.com/hook".to_owned(),
            events: "search.created".to_owned(),
            secret: crate::webhooks::Webhook::generate_secret().expect("test randomness"),
            active: true,
            created_at: 1,
            last_triggered: None,
            retry_count: 0,
            max_retries: 3,
            timeout_seconds: 30,
        };
        db.insert_webhook(&webhook).await.unwrap();
        db.insert_webhook_log(&WebhookLogRecord {
            id: "log_atomic".to_owned(),
            webhook_id: webhook.id.clone(),
            event: "search.created".to_owned(),
            correlation_id: "correlation".to_owned(),
            status: "success".to_owned(),
            request_body: "{}".to_owned(),
            response_status: Some(200),
            response_body: None,
            error_message: None,
            attempt: 1,
            timestamp: 1,
        })
        .await
        .unwrap();
        query(
            r#"
            CREATE TRIGGER reject_webhook_delete
            BEFORE DELETE ON webhooks
            WHEN OLD.id = 'hook_atomic'
            BEGIN
                SELECT RAISE(ABORT, 'forced webhook delete failure');
            END
            "#,
        )
        .execute(&db.pool)
        .await
        .unwrap();

        assert!(db.delete_webhook(&webhook.id).await.is_err());
        assert!(db.get_webhook(&webhook.id).await.unwrap().is_some());
        let logs = db.get_webhook_logs(&webhook.id, 10, 0).await.unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].id, "log_atomic");
    }

    #[tokio::test]
    async fn test_message_operations() {
        let db = DatabaseManager::in_memory().await.unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let record = MessageRecord {
            id: "msg_1".to_string(),
            username: "user1".to_string(),
            content: "Hello!".to_string(),
            direction: "incoming".to_string(),
            read: false,
            created_at: now,
            source_id: None,
            source_timestamp: None,
            was_replayed: false,
        };

        db.insert_message(&record).await.unwrap();
        let messages = db.list_messages_from_user("user1", 10, 0).await.unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "Hello!");

        db.mark_message_read("msg_1").await.unwrap();
        let messages = db.list_messages_from_user("user1", 10, 0).await.unwrap();
        assert!(messages[0].read);

        let newer = MessageRecord {
            id: "msg_2".to_string(),
            username: "user1".to_string(),
            content: "Later".to_string(),
            direction: "outgoing".to_string(),
            read: false,
            created_at: now + 1,
            source_id: None,
            source_timestamp: None,
            was_replayed: false,
        };
        db.insert_message(&newer).await.unwrap();

        let first_page = db.list_messages_from_user("user1", 1, 0).await.unwrap();
        assert_eq!(first_page.len(), 1);
        assert_eq!(first_page[0].id, "msg_2");

        let second_page = db.list_messages_from_user("user1", 1, 1).await.unwrap();
        assert_eq!(second_page.len(), 1);
        assert_eq!(second_page[0].id, "msg_1");
    }

    #[tokio::test]
    async fn bulk_message_operations_cross_sqlite_bind_chunk_boundary() {
        let db = DatabaseManager::in_memory().await.unwrap();
        let records = (0..101)
            .map(|index| MessageRecord {
                id: format!("bulk_msg_{index}"),
                username: "bulk-user".to_owned(),
                content: format!("message {index}"),
                direction: "incoming".to_owned(),
                read: false,
                created_at: index,
                source_id: None,
                source_timestamp: None,
                was_replayed: false,
            })
            .collect::<Vec<_>>();

        db.insert_messages(&records).await.unwrap();
        let ids = records
            .iter()
            .map(|record| record.id.clone())
            .collect::<Vec<_>>();
        db.mark_messages_read(&ids).await.unwrap();

        let persisted = db.list_messages(200, 0).await.unwrap();
        assert_eq!(persisted.len(), records.len());
        assert!(persisted.iter().all(|record| record.read));
    }

    #[tokio::test]
    async fn test_room_subscription_operations() {
        let db = DatabaseManager::in_memory().await.unwrap();

        db.subscribe_room("music", Some("owner")).await.unwrap();
        db.subscribe_room("chat", None).await.unwrap();

        let rooms = db.list_subscribed_rooms().await.unwrap();
        assert_eq!(rooms.len(), 2);
        assert_eq!(rooms[0].name, "chat");
        assert_eq!(rooms[0].owner, None);
        assert!(rooms[0].subscribed);
        assert_eq!(rooms[1].name, "music");
        assert_eq!(rooms[1].owner.as_deref(), Some("owner"));
        assert!(rooms[1].joined_at > 0);
        assert!(rooms[1].last_activity >= rooms[1].joined_at);

        db.unsubscribe_room("chat").await.unwrap();
        let rooms = db.list_subscribed_rooms().await.unwrap();
        assert_eq!(rooms.len(), 1);
        assert_eq!(rooms[0].name, "music");
    }

    #[tokio::test]
    async fn test_user_stats_operations() {
        let db = DatabaseManager::in_memory().await.unwrap();

        db.update_user_stats("testuser", 10, 5, 1000000, 500000)
            .await
            .unwrap();
        let stats = db.get_user_stats("testuser").await.unwrap();
        assert!(stats.is_some());
        let s = stats.unwrap();
        assert_eq!(s.uploads, 10);
        assert_eq!(s.downloads, 5);
    }

    #[tokio::test]
    async fn test_user_projection_operations() {
        let db = DatabaseManager::in_memory().await.unwrap();
        let record = UserProjectionRecord {
            username: "friend".to_owned(),
            watched: true,
            status: Some("Online".to_owned()),
            average_speed: Some(2048),
            upload_count: Some(7),
            file_count: Some(123),
            directory_count: Some(4),
            updated_at: 42,
        };

        db.upsert_user_projection(&record).await.unwrap();
        let records = db.list_user_projections(10, 0).await.unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].username, "friend");
        assert!(records[0].watched);
        assert_eq!(records[0].status.as_deref(), Some("Online"));
        assert_eq!(records[0].file_count, Some(123));

        let stats = db.get_stats().await.unwrap();
        assert_eq!(stats.user_projection_count, 1);
    }

    #[tokio::test]
    async fn test_oauth_state_operations() {
        let db = DatabaseManager::in_memory().await.unwrap();
        let record = OAuthStateRecord {
            state: "state-token".to_owned(),
            provider: "spotify".to_owned(),
            redirect_uri: "http://127.0.0.1/callback".to_owned(),
            created_at: 10,
            expires_at: 20,
        };

        db.upsert_oauth_state(&record).await.unwrap();
        let records = db.list_oauth_states(11, 10, 0).await.unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].state, "state-token");
        assert_eq!(records[0].provider, "spotify");

        assert!(db.list_oauth_states(20, 10, 0).await.unwrap().is_empty());
        assert_eq!(db.delete_expired_oauth_states(20).await.unwrap(), 1);

        db.upsert_oauth_state(&record).await.unwrap();
        db.delete_oauth_state("state-token").await.unwrap();
        assert!(db.list_oauth_states(11, 10, 0).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_webhook_operations_persist_config_and_logs() {
        let db = DatabaseManager::in_memory().await.unwrap();
        let webhook = WebhookRecord {
            id: "hook_1".to_owned(),
            url: "https://example.com/hook".to_owned(),
            events: "search.created,message.sent".to_owned(),
            secret: crate::webhooks::Webhook::generate_secret().expect("test randomness"),
            active: true,
            created_at: 10,
            last_triggered: None,
            retry_count: 0,
            max_retries: 3,
            timeout_seconds: 30,
        };

        db.insert_webhook(&webhook).await.unwrap();
        let webhooks = db.list_webhooks().await.unwrap();
        assert_eq!(webhooks.len(), 1);
        assert_eq!(webhooks[0].id, "hook_1");

        db.update_webhook_active("hook_1", false).await.unwrap();
        let inactive = db.get_webhook("hook_1").await.unwrap().unwrap();
        assert!(!inactive.active);

        let log = WebhookLogRecord {
            id: "log_1".to_owned(),
            webhook_id: "hook_1".to_owned(),
            event: "search.created".to_owned(),
            correlation_id: "search_1".to_owned(),
            status: "queued".to_owned(),
            request_body: "{}".to_owned(),
            response_status: None,
            response_body: None,
            error_message: None,
            attempt: 1,
            timestamp: 11,
        };
        db.insert_webhook_log(&log).await.unwrap();
        let logs = db.get_webhook_logs("hook_1", 10, 0).await.unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].event, "search.created");

        assert_eq!(
            db.complete_webhook_logs("hook_1", "search_1", "failed", Some("delivery rejected"),)
                .await
                .unwrap(),
            1
        );
        let logs = db.get_webhook_logs("hook_1", 10, 0).await.unwrap();
        assert_eq!(logs[0].status, "failed");
        assert_eq!(logs[0].error_message.as_deref(), Some("delivery rejected"));
        assert_eq!(logs[0].attempt, 1);
        assert_eq!(db.get_failed_webhook_logs(10).await.unwrap().len(), 1);

        let mut successful_log = log.clone();
        successful_log.id = "log_2".to_owned();
        successful_log.correlation_id = "search_2".to_owned();
        db.insert_webhook_log(&successful_log).await.unwrap();
        assert_eq!(
            db.complete_webhook_logs("hook_1", "search_2", "success", None)
                .await
                .unwrap(),
            1
        );
        let logs = db.get_webhook_logs("hook_1", 10, 0).await.unwrap();
        assert!(logs.iter().any(|record| {
            record.correlation_id == "search_2"
                && record.status == "success"
                && record.error_message.is_none()
        }));

        let stats = db.get_stats().await.unwrap();
        assert_eq!(stats.webhook_count, 1);
        assert_eq!(stats.webhook_log_count, 2);

        db.delete_webhook("hook_1").await.unwrap();
        assert!(db.list_webhooks().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_distributed_tree_state_operations() {
        let db = DatabaseManager::in_memory().await.unwrap();

        // Initially no state
        let state = db.load_distributed_tree_state().await.unwrap();
        assert!(state.is_none());

        // Initially no children
        let children = db.load_distributed_children().await.unwrap();
        assert!(children.is_empty());

        // Save tree state
        db.save_distributed_tree_state(5, "root-user", Some("parent-user"))
            .await
            .unwrap();

        // Load tree state
        let state = db.load_distributed_tree_state().await.unwrap();
        assert!(state.is_some());
        let (branch_level, branch_root, parent_username) = state.unwrap();
        assert_eq!(branch_level, 5);
        assert_eq!(branch_root, "root-user");
        assert_eq!(parent_username, Some("parent-user".to_owned()));

        // Save children
        let children = vec![
            ("child1".to_owned(), 2),
            ("child2".to_owned(), 3),
            ("child3".to_owned(), 1),
        ];
        db.save_distributed_children(&children).await.unwrap();

        // Load children
        let loaded_children = db.load_distributed_children().await.unwrap();
        assert_eq!(loaded_children.len(), 3);
        assert!(loaded_children.contains(&("child1".to_owned(), 2)));
        assert!(loaded_children.contains(&("child2".to_owned(), 3)));
        assert!(loaded_children.contains(&("child3".to_owned(), 1)));

        // Update tree state
        db.save_distributed_tree_state(10, "new-root", None)
            .await
            .unwrap();
        let state = db.load_distributed_tree_state().await.unwrap();
        assert!(state.is_some());
        let (branch_level, branch_root, parent_username) = state.unwrap();
        assert_eq!(branch_level, 10);
        assert_eq!(branch_root, "new-root");
        assert_eq!(parent_username, None);

        // Update children (replaces all)
        let new_children = vec![("child4".to_owned(), 5)];
        db.save_distributed_children(&new_children).await.unwrap();
        let loaded_children = db.load_distributed_children().await.unwrap();
        assert_eq!(loaded_children.len(), 1);
        assert_eq!(loaded_children[0], ("child4".to_owned(), 5));
    }

    #[tokio::test]
    async fn test_wishlist_scheduler_state_operations() {
        let db = DatabaseManager::in_memory().await.unwrap();

        // Initially no state
        let state = db.load_wishlist_scheduler_state().await.unwrap();
        assert!(state.is_none());

        // Save state
        db.save_wishlist_scheduler_state(5, Some(300))
            .await
            .unwrap();

        // Load state
        let state = db.load_wishlist_scheduler_state().await.unwrap();
        assert!(state.is_some());
        let (next_index, server_interval) = state.unwrap();
        assert_eq!(next_index, 5);
        assert_eq!(server_interval, Some(300));

        // Update state
        db.save_wishlist_scheduler_state(10, None).await.unwrap();
        let state = db.load_wishlist_scheduler_state().await.unwrap();
        assert!(state.is_some());
        let (next_index, server_interval) = state.unwrap();
        assert_eq!(next_index, 10);
        assert_eq!(server_interval, None);
    }
}
