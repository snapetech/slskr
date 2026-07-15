/// Database persistence layer for slskr
///
/// SQLite-backed durable storage using sqlx for async operations.
/// Provides full persistence for searches, transfers, messages, and user stats.
use serde::{Deserialize, Serialize};
use sqlx_core::{from_row::FromRow, query::query, query_as::query_as, row::Row, Error};
use sqlx_sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions, SqliteRow};
use std::time::{SystemTime, UNIX_EPOCH};

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
    pub locked: bool,
    pub slot_free: Option<bool>,
    pub average_speed: Option<i64>,
    pub queue_length: Option<i64>,
    pub created_at: i64,
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
}

/// Wishlist item record for persistence
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WishlistItemRecord {
    pub id: String,
    pub artist: String,
    pub title: String,
    pub kind: String,
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
    pub name: String,
    pub description: String,
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
            name: row.try_get("name")?,
            description: row.try_get("description")?,
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
    #[cfg(test)]
    pub async fn close_for_test(&self) {
        self.pool.close().await;
    }

    #[cfg(test)]
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
        let connect_options = SqliteConnectOptions::new()
            .filename(db_path)
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(connect_options)
            .await?;

        let manager = DatabaseManager { pool };
        manager.initialize().await?;
        Ok(manager)
    }

    /// In-memory database for testing
    pub async fn in_memory() -> Result<Self, Box<dyn std::error::Error>> {
        let pool = SqlitePool::connect("sqlite::memory:").await?;
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
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

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
                created_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

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
                name TEXT NOT NULL,
                description TEXT NOT NULL,
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
                lidarr_sync_runs INTEGER NOT NULL,
                lidarr_manual_imports INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        self.ensure_runtime_compat_columns().await?;

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

        query("CREATE INDEX IF NOT EXISTS idx_share_group_members_username ON share_group_members(username)")
            .execute(&self.pool)
            .await?;

        query("CREATE INDEX IF NOT EXISTS idx_collection_items_collection ON collection_items(collection_id, position)")
            .execute(&self.pool)
            .await?;

        query("CREATE INDEX IF NOT EXISTS idx_library_items_artist ON library_items(artist)")
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
            INSERT OR REPLACE INTO searches (id, query, status, result_count, created_at, completed_at, room, target)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
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
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Get search record
    pub async fn get_search(
        &self,
        id: &str,
    ) -> Result<Option<SearchRecord>, Box<dyn std::error::Error>> {
        let record = query_as::<_, SearchRecord>(
            "SELECT id, query, status, result_count, created_at, completed_at, room, target FROM searches WHERE id = ?"
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
            "SELECT id, query, status, result_count, created_at, completed_at, room, target FROM searches ORDER BY created_at DESC LIMIT ? OFFSET ?"
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
        query("DELETE FROM search_results WHERE search_id = ?")
            .bind(search_id)
            .execute(&self.pool)
            .await?;
        for record in records {
            query(
                r#"
                INSERT INTO search_results
                (search_id, peer_username, filename, size, extension, locked, slot_free, average_speed, queue_length, created_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(search_id)
            .bind(&record.peer_username)
            .bind(&record.filename)
            .bind(record.size)
            .bind(&record.extension)
            .bind(record.locked)
            .bind(record.slot_free)
            .bind(record.average_speed)
            .bind(record.queue_length)
            .bind(record.created_at)
            .execute(&self.pool)
            .await?;
        }
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
                SELECT id, search_id, peer_username, filename, size, extension, locked, slot_free, average_speed, queue_length, created_at
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
                SELECT id, search_id, peer_username, filename, size, extension, locked, slot_free, average_speed, queue_length, created_at
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
        query("DELETE FROM search_results WHERE search_id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        query("DELETE FROM searches WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Delete all search records
    pub async fn delete_all_searches(&self) -> Result<(), Box<dyn std::error::Error>> {
        query("DELETE FROM search_results")
            .execute(&self.pool)
            .await?;
        query("DELETE FROM searches").execute(&self.pool).await?;
        Ok(())
    }

    // ========================================================================
    // Transfer Operations
    // ========================================================================

    /// Insert transfer record
    pub async fn insert_transfer(
        &self,
        record: &TransferRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query(
            r#"
            INSERT OR REPLACE INTO transfers (id, direction, filename, peer_username, filesize, progress, status, started_at, completed_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
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
            "SELECT id, direction, filename, peer_username, filesize, progress, status, started_at, completed_at FROM transfers WHERE id = ?"
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
                "SELECT id, direction, filename, peer_username, filesize, progress, status, started_at, completed_at FROM transfers WHERE status = ? ORDER BY started_at DESC LIMIT ? OFFSET ?"
            )
            .bind(status)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        } else {
            query_as::<_, TransferRecord>(
                "SELECT id, direction, filename, peer_username, filesize, progress, status, started_at, completed_at FROM transfers ORDER BY started_at DESC LIMIT ? OFFSET ?"
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
            .bind(progress as i64)
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
        for record in records {
            query(
                r#"
                INSERT OR REPLACE INTO share_files
                (filename, size, extension, root_label, local_path, updated_at)
                VALUES (?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&record.filename)
            .bind(record.size)
            .bind(&record.extension)
            .bind(&record.root_label)
            .bind(&record.local_path)
            .bind(record.updated_at)
            .execute(&mut *tx)
            .await?;
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
            SELECT filename, size, extension, root_label, local_path, updated_at
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
            INSERT INTO messages (id, username, content, direction, read, created_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&record.id)
        .bind(&record.username)
        .bind(&record.content)
        .bind(&record.direction)
        .bind(record.read as i32)
        .bind(record.created_at)
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
            for record in records {
                query(
                    r#"
                    INSERT INTO messages (id, username, content, direction, read, created_at)
                    VALUES (?, ?, ?, ?, ?, ?)
                    "#,
                )
                .bind(&record.id)
                .bind(&record.username)
                .bind(&record.content)
                .bind(&record.direction)
                .bind(record.read as i32)
                .bind(record.created_at)
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
        let mut transaction = self.pool.begin().await?;
        for id in ids {
            query("UPDATE messages SET read = 1 WHERE id = ?")
                .bind(id)
                .execute(&mut *transaction)
                .await?;
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
            INSERT OR REPLACE INTO user_notes (id, username, note, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&record.id)
        .bind(&record.username)
        .bind(&record.note)
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
            "SELECT id, username, note, created_at, updated_at FROM user_notes ORDER BY updated_at DESC LIMIT ? OFFSET ?",
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
            INSERT OR REPLACE INTO security_bans (kind, value, created_at)
            VALUES (?, ?, ?)
            "#,
        )
        .bind(&record.kind)
        .bind(&record.value)
        .bind(record.created_at)
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
            "SELECT kind, value, created_at FROM security_bans ORDER BY created_at DESC",
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
            INSERT OR REPLACE INTO wishlist_items (id, artist, title, kind, added_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&record.id)
        .bind(&record.artist)
        .bind(&record.title)
        .bind(&record.kind)
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
        for record in records {
            query(
                r#"
                INSERT OR REPLACE INTO wishlist_items (id, artist, title, kind, added_at)
                VALUES (?, ?, ?, ?, ?)
                "#,
            )
            .bind(&record.id)
            .bind(&record.artist)
            .bind(&record.title)
            .bind(&record.kind)
            .bind(record.added_at)
            .execute(&mut *transaction)
            .await?;
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
            "SELECT id, artist, title, kind, added_at FROM wishlist_items ORDER BY added_at DESC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    /// Insert a durable ignored wishlist result rule.
    pub async fn upsert_wishlist_ignored_result(
        &self,
        record: &WishlistIgnoredResultRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query(
            r#"
            INSERT OR REPLACE INTO wishlist_ignored_results
                (id, wishlist_item_id, username, directory, created_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&record.id)
        .bind(&record.wishlist_item_id)
        .bind(&record.username)
        .bind(&record.directory)
        .bind(record.created_at)
        .execute(&self.pool)
        .await?;
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
            INSERT OR REPLACE INTO share_grants (id, collection_id, username, shared_at, permissions)
            VALUES (?, ?, ?, ?, ?)
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
        query("DELETE FROM share_grants WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
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
            INSERT OR REPLACE INTO collections (id, name, description, created_at, updated_at)
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
                INSERT INTO collections (id, name, description, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?)
                ON CONFLICT(id) DO UPDATE SET
                    name = excluded.name,
                    description = excluded.description,
                    created_at = excluded.created_at,
                    updated_at = excluded.updated_at
                "#,
            )
            .bind(&record.id)
            .bind(&record.name)
            .bind(&record.description)
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
                        (id, collection_id, content_id, artist, title, kind, added_at, position)
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                )
                .bind(&item.id)
                .bind(&item.collection_id)
                .bind(&item.content_id)
                .bind(&item.artist)
                .bind(&item.title)
                .bind(&item.kind)
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
            "SELECT id, name, description, created_at, updated_at FROM collections ORDER BY updated_at DESC LIMIT ? OFFSET ?",
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
            INSERT OR REPLACE INTO collection_items (id, collection_id, content_id, artist, title, kind, added_at, position)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&record.id)
        .bind(&record.collection_id)
        .bind(&record.content_id)
        .bind(&record.artist)
        .bind(&record.title)
        .bind(&record.kind)
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
            "SELECT id, collection_id, content_id, artist, title, kind, added_at, position FROM collection_items ORDER BY collection_id, position, added_at LIMIT ? OFFSET ?",
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
             backfill_runs, songid_runs, lidarr_sync_runs, lidarr_manual_imports, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
        for record in records {
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
            .execute(&mut *transaction)
            .await?;
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
             backfill_runs, songid_runs, lidarr_sync_runs, lidarr_manual_imports, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
        let search_count: (i64,) = query_as("SELECT COUNT(*) FROM searches")
            .fetch_one(&self.pool)
            .await?;
        let search_result_count: (i64,) = query_as("SELECT COUNT(*) FROM search_results")
            .fetch_one(&self.pool)
            .await?;

        let transfer_count: (i64,) = query_as("SELECT COUNT(*) FROM transfers")
            .fetch_one(&self.pool)
            .await?;

        let transfer_event_count: (i64,) = query_as("SELECT COUNT(*) FROM transfer_events")
            .fetch_one(&self.pool)
            .await?;

        let share_file_count: (i64,) = query_as("SELECT COUNT(*) FROM share_files")
            .fetch_one(&self.pool)
            .await?;

        let event_count: (i64,) = query_as("SELECT COUNT(*) FROM events")
            .fetch_one(&self.pool)
            .await?;

        let message_count: (i64,) = query_as("SELECT COUNT(*) FROM messages")
            .fetch_one(&self.pool)
            .await?;

        let user_count: (i64,) = query_as("SELECT COUNT(*) FROM user_stats")
            .fetch_one(&self.pool)
            .await?;

        let user_projection_count: (i64,) = query_as("SELECT COUNT(*) FROM user_records")
            .fetch_one(&self.pool)
            .await?;

        let room_count: (i64,) = query_as("SELECT COUNT(*) FROM rooms WHERE subscribed = 1")
            .fetch_one(&self.pool)
            .await?;

        let user_note_count: (i64,) = query_as("SELECT COUNT(*) FROM user_notes")
            .fetch_one(&self.pool)
            .await?;

        let interest_count: (i64,) = query_as("SELECT COUNT(*) FROM interests")
            .fetch_one(&self.pool)
            .await?;

        let security_ban_count: (i64,) = query_as("SELECT COUNT(*) FROM security_bans")
            .fetch_one(&self.pool)
            .await?;

        let wishlist_count: (i64,) = query_as("SELECT COUNT(*) FROM wishlist_items")
            .fetch_one(&self.pool)
            .await?;

        let contact_count: (i64,) = query_as("SELECT COUNT(*) FROM contacts")
            .fetch_one(&self.pool)
            .await?;

        let share_grant_count: (i64,) = query_as("SELECT COUNT(*) FROM share_grants")
            .fetch_one(&self.pool)
            .await?;

        let share_group_count: (i64,) = query_as("SELECT COUNT(*) FROM share_groups")
            .fetch_one(&self.pool)
            .await?;

        let share_group_member_count: (i64,) = query_as("SELECT COUNT(*) FROM share_group_members")
            .fetch_one(&self.pool)
            .await?;

        let collection_count: (i64,) = query_as("SELECT COUNT(*) FROM collections")
            .fetch_one(&self.pool)
            .await?;

        let collection_item_count: (i64,) = query_as("SELECT COUNT(*) FROM collection_items")
            .fetch_one(&self.pool)
            .await?;

        let library_item_count: (i64,) = query_as("SELECT COUNT(*) FROM library_items")
            .fetch_one(&self.pool)
            .await?;

        let destination_count: (i64,) = query_as("SELECT COUNT(*) FROM destinations")
            .fetch_one(&self.pool)
            .await?;

        let now_playing_count: (i64,) = query_as("SELECT COUNT(*) FROM now_playing")
            .fetch_one(&self.pool)
            .await?;

        let browse_count: (i64,) = query_as("SELECT COUNT(*) FROM browse_records")
            .fetch_one(&self.pool)
            .await?;

        let runtime_state_count: (i64,) = query_as("SELECT COUNT(*) FROM runtime_compat_state")
            .fetch_one(&self.pool)
            .await?;

        let oauth_state_count: (i64,) = query_as("SELECT COUNT(*) FROM oauth_states")
            .fetch_one(&self.pool)
            .await?;

        let webhook_count: (i64,) = query_as("SELECT COUNT(*) FROM webhooks")
            .fetch_one(&self.pool)
            .await?;

        let webhook_log_count: (i64,) = query_as("SELECT COUNT(*) FROM webhook_logs")
            .fetch_one(&self.pool)
            .await?;

        Ok(DatabaseStats {
            search_count: nonnegative_database_count(search_count.0)?,
            search_result_count: nonnegative_database_count(search_result_count.0)?,
            transfer_count: nonnegative_database_count(transfer_count.0)?,
            transfer_event_count: nonnegative_database_count(transfer_event_count.0)?,
            share_file_count: nonnegative_database_count(share_file_count.0)?,
            event_count: nonnegative_database_count(event_count.0)?,
            message_count: nonnegative_database_count(message_count.0)?,
            user_count: nonnegative_database_count(user_count.0)?,
            user_projection_count: nonnegative_database_count(user_projection_count.0)?,
            room_count: nonnegative_database_count(room_count.0)?,
            user_note_count: nonnegative_database_count(user_note_count.0)?,
            interest_count: nonnegative_database_count(interest_count.0)?,
            security_ban_count: nonnegative_database_count(security_ban_count.0)?,
            wishlist_count: nonnegative_database_count(wishlist_count.0)?,
            contact_count: nonnegative_database_count(contact_count.0)?,
            share_grant_count: nonnegative_database_count(share_grant_count.0)?,
            share_group_count: nonnegative_database_count(share_group_count.0)?,
            share_group_member_count: nonnegative_database_count(share_group_member_count.0)?,
            collection_count: nonnegative_database_count(collection_count.0)?,
            collection_item_count: nonnegative_database_count(collection_item_count.0)?,
            library_item_count: nonnegative_database_count(library_item_count.0)?,
            destination_count: nonnegative_database_count(destination_count.0)?,
            now_playing_count: nonnegative_database_count(now_playing_count.0)?,
            browse_count: nonnegative_database_count(browse_count.0)?,
            runtime_state_count: nonnegative_database_count(runtime_state_count.0)?,
            oauth_state_count: nonnegative_database_count(oauth_state_count.0)?,
            webhook_count: nonnegative_database_count(webhook_count.0)?,
            webhook_log_count: nonnegative_database_count(webhook_log_count.0)?,
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
        query("DELETE FROM webhook_logs WHERE webhook_id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        query("DELETE FROM webhooks WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
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
        let result = query(
            r#"
            UPDATE webhook_logs
            SET status = ?, error_message = ?
            WHERE webhook_id = ? AND correlation_id = ? AND status = 'queued'
            "#,
        )
        .bind(status)
        .bind(error_message)
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
            r#"SELECT id, webhook_id, event, correlation_id, status, request_body, response_status, response_body, error_message, attempt, timestamp FROM webhook_logs WHERE status IN ('failed', 'timeout') AND attempt < (SELECT max_retries FROM webhooks WHERE webhooks.id = webhook_logs.webhook_id) ORDER BY timestamp ASC LIMIT ?"#
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
        };

        db.insert_search(&record).await.unwrap();
        let retrieved = db.get_search("search_1").await.unwrap().unwrap();
        assert_eq!(retrieved.query, "test query");
        assert_eq!(retrieved.result_count, 42);

        db.update_search_status("search_1", "archived")
            .await
            .unwrap();
        let updated = db.get_search("search_1").await.unwrap().unwrap();
        assert_eq!(updated.status, "archived");
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
        };

        db.insert_transfer(&record).await.unwrap();
        let retrieved = db.get_transfer("transfer_1").await.unwrap().unwrap();
        assert_eq!(retrieved.filename, "test.mp3");
        assert_eq!(retrieved.progress, 500000);

        db.update_transfer_progress("transfer_1", 750000)
            .await
            .unwrap();
        let updated = db.get_transfer("transfer_1").await.unwrap().unwrap();
        assert_eq!(updated.progress, 750000);
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
}
