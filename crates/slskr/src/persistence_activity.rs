use super::*;

impl DatabaseManager {
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
}
