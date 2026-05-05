#![allow(dead_code)]
/// Database persistence layer for soulseekR
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

/// Room subscription record
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoomRecord {
    pub name: String,
    pub owner: Option<String>,
    pub subscribed: bool,
    pub joined_at: i64,
    pub last_activity: i64,
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

        query("CREATE INDEX IF NOT EXISTS idx_transfers_started ON transfers(started_at DESC)")
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

    /// Mark message as read
    pub async fn mark_message_read(&self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        query("UPDATE messages SET read = 1 WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
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
    // Database Maintenance
    // ========================================================================

    /// Get database statistics
    pub async fn get_stats(&self) -> Result<DatabaseStats, Box<dyn std::error::Error>> {
        let search_count: (i64,) = query_as("SELECT COUNT(*) FROM searches")
            .fetch_one(&self.pool)
            .await?;

        let transfer_count: (i64,) = query_as("SELECT COUNT(*) FROM transfers")
            .fetch_one(&self.pool)
            .await?;

        let message_count: (i64,) = query_as("SELECT COUNT(*) FROM messages")
            .fetch_one(&self.pool)
            .await?;

        let user_count: (i64,) = query_as("SELECT COUNT(*) FROM user_stats")
            .fetch_one(&self.pool)
            .await?;

        let room_count: (i64,) = query_as("SELECT COUNT(*) FROM rooms WHERE subscribed = 1")
            .fetch_one(&self.pool)
            .await?;

        Ok(DatabaseStats {
            search_count: search_count.0 as u32,
            transfer_count: transfer_count.0 as u32,
            message_count: message_count.0 as u32,
            user_count: user_count.0 as u32,
            room_count: room_count.0 as u32,
        })
    }

    /// Cleanup old records (older than specified days)
    pub async fn cleanup_old_records(&self, days: i32) -> Result<u32, Box<dyn std::error::Error>> {
        let cutoff =
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64 - (days as i64 * 86400);

        let result = query("DELETE FROM messages WHERE created_at < ?")
            .bind(cutoff)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() as u32)
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
    ) -> Result<u32, Box<dyn std::error::Error>> {
        let cutoff =
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64 - (days as i64 * 86400);

        let result = query("DELETE FROM webhook_logs WHERE timestamp < ?")
            .bind(cutoff)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() as u32)
    }
}

/// Database statistics
#[derive(Clone, Debug, Serialize)]
pub struct DatabaseStats {
    pub search_count: u32,
    pub transfer_count: u32,
    pub message_count: u32,
    pub user_count: u32,
    pub room_count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_database_creation() {
        let db = DatabaseManager::in_memory().await.unwrap();
        let stats = db.get_stats().await.unwrap();
        assert_eq!(stats.search_count, 0);
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
}
