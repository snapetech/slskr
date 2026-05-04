/// Database persistence layer for soulseekR
///
/// Placeholder for database persistence system.
/// Production implementation would use sqlx with SQLite backend.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

/// In-memory database manager
/// 
/// This is a reference implementation using in-memory storage.
/// In production, replace with sqlx + SQLite backend.
pub struct DatabaseManager {
    searches: HashMap<String, SearchRecord>,
    transfers: HashMap<String, TransferRecord>,
    messages: HashMap<String, MessageRecord>,
}

impl DatabaseManager {
    /// Create new database manager
    pub fn new(_db_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(DatabaseManager {
            searches: HashMap::new(),
            transfers: HashMap::new(),
            messages: HashMap::new(),
        })
    }

    /// In-memory database for testing
    pub fn in_memory() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(DatabaseManager {
            searches: HashMap::new(),
            transfers: HashMap::new(),
            messages: HashMap::new(),
        })
    }

    // ========================================================================
    // Search Operations
    // ========================================================================

    /// Insert search record
    pub fn insert_search(&mut self, record: &SearchRecord) -> Result<(), Box<dyn std::error::Error>> {
        self.searches.insert(record.id.clone(), record.clone());
        Ok(())
    }

    /// Get search record
    pub fn get_search(&self, id: &str) -> Result<Option<SearchRecord>, Box<dyn std::error::Error>> {
        Ok(self.searches.get(id).cloned())
    }

    /// List recent searches
    pub fn list_searches(&self, limit: i32, _offset: i32) -> Result<Vec<SearchRecord>, Box<dyn std::error::Error>> {
        let mut searches: Vec<_> = self.searches.values().cloned().collect();
        searches.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(searches.into_iter().take(limit as usize).collect())
    }

    /// Update search status
    pub fn update_search_status(&mut self, id: &str, status: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(record) = self.searches.get_mut(id) {
            record.status = status.to_string();
        }
        Ok(())
    }

    /// Update search results
    pub fn update_search_results(&mut self, id: &str, count: u32) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(record) = self.searches.get_mut(id) {
            record.result_count = count;
        }
        Ok(())
    }

    // ========================================================================
    // Transfer Operations
    // ========================================================================

    /// Insert transfer record
    pub fn insert_transfer(&mut self, record: &TransferRecord) -> Result<(), Box<dyn std::error::Error>> {
        self.transfers.insert(record.id.clone(), record.clone());
        Ok(())
    }

    /// Get transfer record
    pub fn get_transfer(&self, id: &str) -> Result<Option<TransferRecord>, Box<dyn std::error::Error>> {
        Ok(self.transfers.get(id).cloned())
    }

    /// List transfers
    pub fn list_transfers(
        &self,
        _status: Option<&str>,
        limit: i32,
        _offset: i32,
    ) -> Result<Vec<TransferRecord>, Box<dyn std::error::Error>> {
        let mut transfers: Vec<_> = self.transfers.values().cloned().collect();
        transfers.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        Ok(transfers.into_iter().take(limit as usize).collect())
    }

    /// Update transfer progress
    pub fn update_transfer_progress(&mut self, id: &str, progress: u64) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(record) = self.transfers.get_mut(id) {
            record.progress = progress;
        }
        Ok(())
    }

    // ========================================================================
    // Message Operations
    // ========================================================================

    /// Insert message record
    pub fn insert_message(&mut self, record: &MessageRecord) -> Result<(), Box<dyn std::error::Error>> {
        self.messages.insert(record.id.clone(), record.clone());
        Ok(())
    }

    /// List messages from user
    pub fn list_messages_from_user(
        &self,
        username: &str,
        limit: i32,
        _offset: i32,
    ) -> Result<Vec<MessageRecord>, Box<dyn std::error::Error>> {
        let mut messages: Vec<_> = self.messages
            .values()
            .filter(|m| m.username == username)
            .cloned()
            .collect();
        messages.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(messages.into_iter().take(limit as usize).collect())
    }

    /// Mark message as read
    pub fn mark_message_read(&mut self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(record) = self.messages.get_mut(id) {
            record.read = true;
        }
        Ok(())
    }

    // ========================================================================
    // Database Maintenance
    // ========================================================================

    /// Get database statistics
    pub fn get_stats(&self) -> Result<DatabaseStats, Box<dyn std::error::Error>> {
        Ok(DatabaseStats {
            search_count: self.searches.len() as u32,
            transfer_count: self.transfers.len() as u32,
            message_count: self.messages.len() as u32,
        })
    }

    /// Cleanup old records
    pub fn cleanup_old_records(&mut self, _days: i32) -> Result<u32, Box<dyn std::error::Error>> {
        // Placeholder implementation
        Ok(0)
    }

    /// Vacuum database
    pub fn vacuum(&self) -> Result<(), Box<dyn std::error::Error>> {
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
        let mut db = DatabaseManager::in_memory().unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
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

        db.insert_search(&record).unwrap();
        let retrieved = db.get_search("search_1").unwrap().unwrap();
        assert_eq!(retrieved.query, "test query");
        assert_eq!(retrieved.result_count, 42);
    }

    #[test]
    fn test_transfer_operations() {
        let mut db = DatabaseManager::in_memory().unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
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

        db.insert_transfer(&record).unwrap();
        let retrieved = db.get_transfer("transfer_1").unwrap().unwrap();
        assert_eq!(retrieved.filename, "test.mp3");
        assert_eq!(retrieved.progress, 500000);
    }

    #[test]
    fn test_message_operations() {
        let mut db = DatabaseManager::in_memory().unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
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

        db.insert_message(&record).unwrap();
        let messages = db.list_messages_from_user("user1", 10, 0).unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "Hello!");
    }
}
