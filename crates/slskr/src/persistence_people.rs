use super::*;

impl DatabaseManager {
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
}
