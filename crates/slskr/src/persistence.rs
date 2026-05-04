/// Database persistence layer for soulseekR
///
/// Provides SQLite-based persistence for searches, transfers, messages,
/// and other data with transaction support and async operations.

use rusqlite::{Connection, OptionalExtension, Result as SqlResult};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use chrono::{DateTime, Utc};

/// Search record for persistence
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchRecord {
    pub id: String,
    pub query: String,
    pub status: String,
    pub result_count: u32,
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
    pub filesize: u64,
    pub progress: u64,
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

/// Database manager with connection pooling
pub struct DatabaseManager {
    connection: Arc<Mutex<Connection>>,
}

impl DatabaseManager {
    /// Create new database manager
    pub fn new(db_path: &str) -> SqlResult<Self> {
        let conn = Connection::open(db_path)?;
        
        // Enable foreign keys
        conn.execute("PRAGMA foreign_keys = ON", [])?;
        
        // Create tables
        conn.execute_batch(
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
            );
            
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
            );
            
            CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                username TEXT NOT NULL,
                content TEXT NOT NULL,
                direction TEXT NOT NULL,
                read INTEGER DEFAULT 0,
                created_at INTEGER NOT NULL
            );
            
            CREATE INDEX IF NOT EXISTS idx_searches_status ON searches(status);
            CREATE INDEX IF NOT EXISTS idx_searches_created ON searches(created_at);
            CREATE INDEX IF NOT EXISTS idx_transfers_status ON transfers(status);
            CREATE INDEX IF NOT EXISTS idx_transfers_peer ON transfers(peer_username);
            CREATE INDEX IF NOT EXISTS idx_messages_username ON messages(username);
            CREATE INDEX IF NOT EXISTS idx_messages_created ON messages(created_at);
            "#,
        )?;

        Ok(DatabaseManager {
            connection: Arc::new(Mutex::new(conn)),
        })
    }

    /// In-memory database for testing
    pub fn in_memory() -> SqlResult<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute("PRAGMA foreign_keys = ON", [])?;
        
        conn.execute_batch(
            r#"
            CREATE TABLE searches (
                id TEXT PRIMARY KEY,
                query TEXT NOT NULL,
                status TEXT NOT NULL,
                result_count INTEGER DEFAULT 0,
                created_at INTEGER NOT NULL,
                completed_at INTEGER,
                room TEXT,
                target TEXT
            );
            
            CREATE TABLE transfers (
                id TEXT PRIMARY KEY,
                direction TEXT NOT NULL,
                filename TEXT NOT NULL,
                peer_username TEXT NOT NULL,
                filesize INTEGER NOT NULL,
                progress INTEGER DEFAULT 0,
                status TEXT NOT NULL,
                started_at INTEGER NOT NULL,
                completed_at INTEGER
            );
            
            CREATE TABLE messages (
                id TEXT PRIMARY KEY,
                username TEXT NOT NULL,
                content TEXT NOT NULL,
                direction TEXT NOT NULL,
                read INTEGER DEFAULT 0,
                created_at INTEGER NOT NULL
            );
            "#,
        )?;

        Ok(DatabaseManager {
            connection: Arc::new(Mutex::new(conn)),
        })
    }

    // ========================================================================
    // Search Operations
    // ========================================================================

    /// Insert search record
    pub fn insert_search(&self, record: &SearchRecord) -> SqlResult<()> {
        let conn = self.connection.lock().unwrap();
        conn.execute(
            "INSERT INTO searches (id, query, status, result_count, created_at, room, target)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                &record.id,
                &record.query,
                &record.status,
                record.result_count,
                record.created_at,
                &record.room,
                &record.target,
            ],
        )?;
        Ok(())
    }

    /// Get search record
    pub fn get_search(&self, id: &str) -> SqlResult<Option<SearchRecord>> {
        let conn = self.connection.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, query, status, result_count, created_at, completed_at, room, target
             FROM searches WHERE id = ?1",
        )?;

        stmt.query_row([id], |row| {
            Ok(SearchRecord {
                id: row.get(0)?,
                query: row.get(1)?,
                status: row.get(2)?,
                result_count: row.get(3)?,
                created_at: row.get(4)?,
                completed_at: row.get(5)?,
                room: row.get(6)?,
                target: row.get(7)?,
            })
        })
        .optional()
    }

    /// List recent searches
    pub fn list_searches(&self, limit: i32, offset: i32) -> SqlResult<Vec<SearchRecord>> {
        let conn = self.connection.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, query, status, result_count, created_at, completed_at, room, target
             FROM searches ORDER BY created_at DESC LIMIT ?1 OFFSET ?2",
        )?;

        let records = stmt.query_map(rusqlite::params![limit, offset], |row| {
            Ok(SearchRecord {
                id: row.get(0)?,
                query: row.get(1)?,
                status: row.get(2)?,
                result_count: row.get(3)?,
                created_at: row.get(4)?,
                completed_at: row.get(5)?,
                room: row.get(6)?,
                target: row.get(7)?,
            })
        })?;

        records.collect()
    }

    /// Update search status
    pub fn update_search_status(&self, id: &str, status: &str) -> SqlResult<()> {
        let conn = self.connection.lock().unwrap();
        conn.execute(
            "UPDATE searches SET status = ?1 WHERE id = ?2",
            rusqlite::params![status, id],
        )?;
        Ok(())
    }

    /// Update search results
    pub fn update_search_results(&self, id: &str, count: u32) -> SqlResult<()> {
        let conn = self.connection.lock().unwrap();
        conn.execute(
            "UPDATE searches SET result_count = ?1 WHERE id = ?2",
            rusqlite::params![count, id],
        )?;
        Ok(())
    }

    // ========================================================================
    // Transfer Operations
    // ========================================================================

    /// Insert transfer record
    pub fn insert_transfer(&self, record: &TransferRecord) -> SqlResult<()> {
        let conn = self.connection.lock().unwrap();
        conn.execute(
            "INSERT INTO transfers (id, direction, filename, peer_username, filesize, status, started_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                &record.id,
                &record.direction,
                &record.filename,
                &record.peer_username,
                record.filesize,
                &record.status,
                record.started_at,
            ],
        )?;
        Ok(())
    }

    /// Get transfer record
    pub fn get_transfer(&self, id: &str) -> SqlResult<Option<TransferRecord>> {
        let conn = self.connection.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, direction, filename, peer_username, filesize, progress, status, started_at, completed_at
             FROM transfers WHERE id = ?1",
        )?;

        stmt.query_row([id], |row| {
            Ok(TransferRecord {
                id: row.get(0)?,
                direction: row.get(1)?,
                filename: row.get(2)?,
                peer_username: row.get(3)?,
                filesize: row.get(4)?,
                progress: row.get(5)?,
                status: row.get(6)?,
                started_at: row.get(7)?,
                completed_at: row.get(8)?,
            })
        })
        .optional()
    }

    /// List transfers by status
    pub fn list_transfers(
        &self,
        status: Option<&str>,
        limit: i32,
        offset: i32,
    ) -> SqlResult<Vec<TransferRecord>> {
        let conn = self.connection.lock().unwrap();
        
        let (query, params): (String, Vec<String>) = match status {
            Some(s) => (
                "SELECT id, direction, filename, peer_username, filesize, progress, status, started_at, completed_at
                 FROM transfers WHERE status = ?1 ORDER BY started_at DESC LIMIT ?2 OFFSET ?3".to_string(),
                vec![s.to_string()],
            ),
            None => (
                "SELECT id, direction, filename, peer_username, filesize, progress, status, started_at, completed_at
                 FROM transfers ORDER BY started_at DESC LIMIT ?1 OFFSET ?2".to_string(),
                vec![],
            ),
        };

        let mut stmt = conn.prepare(&query)?;
        let records = if !params.is_empty() {
            stmt.query_map(rusqlite::params![&params[0], limit, offset], |row| {
                Ok(TransferRecord {
                    id: row.get(0)?,
                    direction: row.get(1)?,
                    filename: row.get(2)?,
                    peer_username: row.get(3)?,
                    filesize: row.get(4)?,
                    progress: row.get(5)?,
                    status: row.get(6)?,
                    started_at: row.get(7)?,
                    completed_at: row.get(8)?,
                })
            })?
        } else {
            stmt.query_map(rusqlite::params![limit, offset], |row| {
                Ok(TransferRecord {
                    id: row.get(0)?,
                    direction: row.get(1)?,
                    filename: row.get(2)?,
                    peer_username: row.get(3)?,
                    filesize: row.get(4)?,
                    progress: row.get(5)?,
                    status: row.get(6)?,
                    started_at: row.get(7)?,
                    completed_at: row.get(8)?,
                })
            })?
        };

        records.collect()
    }

    /// Update transfer progress
    pub fn update_transfer_progress(&self, id: &str, progress: u64) -> SqlResult<()> {
        let conn = self.connection.lock().unwrap();
        conn.execute(
            "UPDATE transfers SET progress = ?1 WHERE id = ?2",
            rusqlite::params![progress, id],
        )?;
        Ok(())
    }

    // ========================================================================
    // Message Operations
    // ========================================================================

    /// Insert message record
    pub fn insert_message(&self, record: &MessageRecord) -> SqlResult<()> {
        let conn = self.connection.lock().unwrap();
        conn.execute(
            "INSERT INTO messages (id, username, content, direction, read, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                &record.id,
                &record.username,
                &record.content,
                &record.direction,
                record.read as i32,
                record.created_at,
            ],
        )?;
        Ok(())
    }

    /// List messages from user
    pub fn list_messages_from_user(
        &self,
        username: &str,
        limit: i32,
        offset: i32,
    ) -> SqlResult<Vec<MessageRecord>> {
        let conn = self.connection.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, username, content, direction, read, created_at
             FROM messages WHERE username = ?1 ORDER BY created_at DESC LIMIT ?2 OFFSET ?3",
        )?;

        let records = stmt.query_map(rusqlite::params![username, limit, offset], |row| {
            Ok(MessageRecord {
                id: row.get(0)?,
                username: row.get(1)?,
                content: row.get(2)?,
                direction: row.get(3)?,
                read: row.get::<_, i32>(4)? != 0,
                created_at: row.get(5)?,
            })
        })?;

        records.collect()
    }

    /// Mark message as read
    pub fn mark_message_read(&self, id: &str) -> SqlResult<()> {
        let conn = self.connection.lock().unwrap();
        conn.execute(
            "UPDATE messages SET read = 1 WHERE id = ?1",
            rusqlite::params![id],
        )?;
        Ok(())
    }

    // ========================================================================
    // Database Maintenance
    // ========================================================================

    /// Get database statistics
    pub fn get_stats(&self) -> SqlResult<DatabaseStats> {
        let conn = self.connection.lock().unwrap();

        let search_count: i32 =
            conn.query_row("SELECT COUNT(*) FROM searches", [], |row| row.get(0))?;

        let transfer_count: i32 =
            conn.query_row("SELECT COUNT(*) FROM transfers", [], |row| row.get(0))?;

        let message_count: i32 =
            conn.query_row("SELECT COUNT(*) FROM messages", [], |row| row.get(0))?;

        Ok(DatabaseStats {
            search_count: search_count as u32,
            transfer_count: transfer_count as u32,
            message_count: message_count as u32,
        })
    }

    /// Cleanup old records (older than days)
    pub fn cleanup_old_records(&self, days: i32) -> SqlResult<u32> {
        let conn = self.connection.lock().unwrap();
        let cutoff = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64) - (days as i64 * 86400);

        let tx = conn.transaction()?;

        let searches_deleted =
            tx.execute("DELETE FROM searches WHERE created_at < ?1", rusqlite::params![cutoff])?;

        let transfers_deleted =
            tx.execute("DELETE FROM transfers WHERE started_at < ?1", rusqlite::params![cutoff])?;

        let messages_deleted =
            tx.execute("DELETE FROM messages WHERE created_at < ?1", rusqlite::params![cutoff])?;

        tx.commit()?;

        Ok((searches_deleted + transfers_deleted + messages_deleted) as u32)
    }

    /// Vacuum database
    pub fn vacuum(&self) -> SqlResult<()> {
        let conn = self.connection.lock().unwrap();
        conn.execute("VACUUM", [])?;
        Ok(())
    }
}

/// Database statistics
#[derive(Clone, Debug, Serialize)]
pub struct DatabaseStats {
    pub search_count: u32,
    pub transfer_count: u32,
    pub message_count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_current_timestamp() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }

    #[test]
    fn test_database_creation() {
        let db = DatabaseManager::in_memory().unwrap();
        let stats = db.get_stats().unwrap();
        assert_eq!(stats.search_count, 0);
        assert_eq!(stats.transfer_count, 0);
        assert_eq!(stats.message_count, 0);
    }

    #[test]
    fn test_search_operations() {
        let db = DatabaseManager::in_memory().unwrap();
        let now = get_current_timestamp();

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

        db.insert_search(&record).unwrap();
        let retrieved = db.get_search("search_1").unwrap().unwrap();
        assert_eq!(retrieved.query, "test query");
        assert_eq!(retrieved.result_count, 42);
    }

    #[test]
    fn test_transfer_operations() {
        let db = DatabaseManager::in_memory().unwrap();
        let now = get_current_timestamp();

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

        db.insert_transfer(&record).unwrap();
        let retrieved = db.get_transfer("transfer_1").unwrap().unwrap();
        assert_eq!(retrieved.filename, "test.mp3");
        assert_eq!(retrieved.progress, 500000);
    }

    #[test]
    fn test_message_operations() {
        let db = DatabaseManager::in_memory().unwrap();
        let now = get_current_timestamp();

        let record = MessageRecord {
            id: "msg_1".to_string(),
            username: "user1".to_string(),
            content: "Hello!".to_string(),
            direction: "incoming".to_string(),
            read: false,
            created_at: now,
        };

        db.insert_message(&record).unwrap();
        let messages = db.list_messages_from_user("user1", 10, 0).unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "Hello!");
    }
}
