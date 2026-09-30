use super::*;

impl DatabaseManager {
    /// Insert transfer record
    pub async fn insert_transfer(
        &self,
        record: &TransferRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut transaction = self.pool.begin().await?;
        let deleted = query("SELECT id FROM transfer_tombstones WHERE id = ?")
            .bind(&record.id)
            .fetch_optional(&mut *transaction)
            .await?
            .is_some();
        if deleted {
            transaction.commit().await?;
            return Ok(());
        }
        query(
            r#"
            INSERT INTO transfers (id, direction, filename, peer_username, filesize, progress, status, started_at, completed_at, request_id, wishlist_item_id, request_name, destination_directory, local_path, batch_id, reason, bit_rate, sample_rate, bit_depth, length_seconds, artist, album, title, track_number, year, attempts, auto_replace_attempts, next_attempt_at, updated_at_ms)
            VALUES (
                ?, ?, ?, ?, ?, ?, ?, ?,
                ?, ?, ?, ?, ?, ?, ?, ?,
                ?, ?, ?, ?, ?, ?, ?, ?,
                ?, ?, ?, ?, ?
            )
            ON CONFLICT(id) DO UPDATE SET
                direction = excluded.direction,
                filename = excluded.filename,
                peer_username = excluded.peer_username,
                filesize = excluded.filesize,
                progress = excluded.progress,
                status = excluded.status,
                started_at = excluded.started_at,
                completed_at = excluded.completed_at,
                request_id = excluded.request_id,
                wishlist_item_id = excluded.wishlist_item_id,
                request_name = excluded.request_name,
                destination_directory = excluded.destination_directory,
                local_path = excluded.local_path,
                batch_id = excluded.batch_id,
                reason = excluded.reason,
                bit_rate = excluded.bit_rate,
                sample_rate = excluded.sample_rate,
                bit_depth = excluded.bit_depth,
                length_seconds = excluded.length_seconds,
                artist = excluded.artist,
                album = excluded.album,
                title = excluded.title,
                track_number = excluded.track_number,
                year = excluded.year,
                attempts = excluded.attempts,
                auto_replace_attempts = excluded.auto_replace_attempts,
                next_attempt_at = excluded.next_attempt_at,
                updated_at_ms = excluded.updated_at_ms
            WHERE transfers.updated_at_ms <= excluded.updated_at_ms
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
        .bind(record.updated_at_ms)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(())
    }

    /// Get transfer record
    pub async fn get_transfer(
        &self,
        id: &str,
    ) -> Result<Option<TransferRecord>, Box<dyn std::error::Error>> {
        let record = query_as::<_, TransferRecord>(
            "SELECT id, direction, filename, peer_username, filesize, progress, status, started_at, completed_at, request_id, wishlist_item_id, request_name, destination_directory, local_path, batch_id, reason, bit_rate, sample_rate, bit_depth, length_seconds, artist, album, title, track_number, year, attempts, auto_replace_attempts, next_attempt_at, updated_at_ms FROM transfers WHERE id = ?"
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
                "SELECT id, direction, filename, peer_username, filesize, progress, status, started_at, completed_at, request_id, wishlist_item_id, request_name, destination_directory, local_path, batch_id, reason, bit_rate, sample_rate, bit_depth, length_seconds, artist, album, title, track_number, year, attempts, auto_replace_attempts, next_attempt_at, updated_at_ms FROM transfers WHERE status = ? ORDER BY started_at DESC LIMIT ? OFFSET ?"
            )
            .bind(status)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        } else {
            query_as::<_, TransferRecord>(
                "SELECT id, direction, filename, peer_username, filesize, progress, status, started_at, completed_at, request_id, wishlist_item_id, request_name, destination_directory, local_path, batch_id, reason, bit_rate, sample_rate, bit_depth, length_seconds, artist, album, title, track_number, year, attempts, auto_replace_attempts, next_attempt_at, updated_at_ms FROM transfers ORDER BY started_at DESC LIMIT ? OFFSET ?"
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
        updated_at_ms: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query("UPDATE transfers SET progress = ?, updated_at_ms = ? WHERE id = ? AND updated_at_ms < ?")
            .bind(i64::try_from(progress).unwrap_or(i64::MAX))
            .bind(i64::try_from(updated_at_ms).unwrap_or(i64::MAX))
            .bind(id)
            .bind(i64::try_from(updated_at_ms).unwrap_or(i64::MAX))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Delete transfer record
    pub async fn delete_transfer(&self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.delete_transfers(&[id.to_owned()]).await
    }

    /// Delete a set of transfer records atomically.
    pub async fn delete_transfers(&self, ids: &[String]) -> Result<(), Box<dyn std::error::Error>> {
        if ids.is_empty() {
            return Ok(());
        }
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| i64::try_from(duration.as_millis()).unwrap_or(i64::MAX))
            .unwrap_or(0);
        let mut records = Vec::with_capacity(ids.len());
        for id in ids {
            let current = self.get_transfer(id).await?;
            let deleted_at_ms = current
                .map(|record| record.updated_at_ms.saturating_add(1).max(now_ms))
                .unwrap_or(now_ms);
            records.push((id.clone(), deleted_at_ms));
        }
        self.delete_transfer_records(&records).await
    }

    /// Delete transfer snapshots without letting an older queued write recreate
    /// them. The transfer id may be reused only by a later revision.
    pub async fn delete_transfer_records(
        &self,
        records: &[(String, i64)],
    ) -> Result<(), Box<dyn std::error::Error>> {
        if records.is_empty() {
            return Ok(());
        }
        let mut transaction = self.pool.begin().await?;
        for (id, deleted_at_ms) in records {
            let current_revision = query("SELECT updated_at_ms FROM transfers WHERE id = ?")
                .bind(id)
                .fetch_optional(&mut *transaction)
                .await?
                .map(|row| row.try_get::<i64, _>("updated_at_ms"))
                .transpose()?;
            if current_revision.is_some_and(|revision| revision > *deleted_at_ms) {
                continue;
            }
            query(
                r#"
                INSERT INTO transfer_tombstones (id, deleted_at_ms)
                VALUES (?, ?)
                ON CONFLICT(id) DO UPDATE SET
                    deleted_at_ms = MAX(transfer_tombstones.deleted_at_ms, excluded.deleted_at_ms)
                "#,
            )
            .bind(id)
            .bind(deleted_at_ms)
            .execute(&mut *transaction)
            .await?;
            query("DELETE FROM transfers WHERE id = ? AND updated_at_ms <= ?")
                .bind(id)
                .bind(deleted_at_ms)
                .execute(&mut *transaction)
                .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    /// Return the largest numeric transfer id retained in either live rows or
    /// tombstones, so restart never reuses an id whose old write may still be
    /// queued elsewhere.
    pub async fn max_transfer_id(&self) -> Result<u64, Box<dyn std::error::Error>> {
        let row = query(
            "SELECT MAX(CAST(id AS INTEGER)) AS max_id FROM (SELECT id FROM transfers UNION ALL SELECT id FROM transfer_tombstones)",
        )
        .fetch_one(&self.pool)
        .await?;
        let value = row.try_get::<Option<i64>, _>("max_id")?.unwrap_or(0);
        Ok(u64::try_from(value).unwrap_or(0))
    }

    /// Roll back transfers that were staged but never dispatched, including their event trail.
    pub async fn rollback_staged_transfers(
        &self,
        records: &[(String, i64)],
    ) -> Result<(), Box<dyn std::error::Error>> {
        if records.is_empty() {
            return Ok(());
        }
        let mut transaction = self.pool.begin().await?;
        for (id, deleted_at_ms) in records {
            let current_revision = query("SELECT updated_at_ms FROM transfers WHERE id = ?")
                .bind(id)
                .fetch_optional(&mut *transaction)
                .await?
                .map(|row| row.try_get::<i64, _>("updated_at_ms"))
                .transpose()?;
            if current_revision.is_some_and(|revision| revision > *deleted_at_ms) {
                continue;
            }
            query(
                r#"
                INSERT INTO transfer_tombstones (id, deleted_at_ms)
                VALUES (?, ?)
                ON CONFLICT(id) DO UPDATE SET
                    deleted_at_ms = MAX(transfer_tombstones.deleted_at_ms, excluded.deleted_at_ms)
                "#,
            )
            .bind(id)
            .bind(deleted_at_ms)
            .execute(&mut *transaction)
            .await?;
            query("DELETE FROM transfer_events WHERE transfer_id = ? AND updated_at_ms <= ?")
                .bind(id)
                .bind(deleted_at_ms)
                .execute(&mut *transaction)
                .await?;
            query("DELETE FROM transfers WHERE id = ? AND updated_at_ms <= ?")
                .bind(id)
                .bind(deleted_at_ms)
                .execute(&mut *transaction)
                .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    /// Append a transfer transition/progress event.
    pub async fn insert_transfer_event(
        &self,
        record: &TransferEventRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut transaction = self.pool.begin().await?;
        let deleted = query("SELECT id FROM transfer_tombstones WHERE id = ?")
            .bind(&record.transfer_id)
            .fetch_optional(&mut *transaction)
            .await?
            .is_some();
        if deleted {
            transaction.commit().await?;
            return Ok(());
        }
        query(
            r#"
            INSERT INTO transfer_events
                (transfer_id, direction, token, filename, peer_username, filesize, progress, status, reason, created_at, updated_at_ms)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
        .bind(record.updated_at_ms)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(())
    }

    /// Atomically persist transfer projections and their corresponding events.
    pub async fn insert_transfer_records_with_events(
        &self,
        records: &[(TransferRecord, TransferEventRecord)],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut transaction = self.pool.begin().await?;
        for (transfer, event) in records {
            let tombstone = query("SELECT id FROM transfer_tombstones WHERE id = ?")
                .bind(&transfer.id)
                .fetch_optional(&mut *transaction)
                .await?
                .is_some();
            if !tombstone {
                let transfer_insert = query(
                    r#"
                    INSERT INTO transfers (id, direction, filename, peer_username, filesize, progress, status, started_at, completed_at, request_id, wishlist_item_id, request_name, destination_directory, local_path, batch_id, reason, bit_rate, sample_rate, bit_depth, length_seconds, artist, album, title, track_number, year, attempts, auto_replace_attempts, next_attempt_at, updated_at_ms)
                    VALUES (
                        ?, ?, ?, ?, ?, ?, ?, ?,
                        ?, ?, ?, ?, ?, ?, ?, ?,
                        ?, ?, ?, ?, ?, ?, ?, ?,
                        ?, ?, ?, ?, ?
                    )
                    ON CONFLICT(id) DO UPDATE SET
                        direction = excluded.direction,
                        filename = excluded.filename,
                        peer_username = excluded.peer_username,
                        filesize = excluded.filesize,
                        progress = excluded.progress,
                        status = excluded.status,
                        started_at = excluded.started_at,
                        completed_at = excluded.completed_at,
                        request_id = excluded.request_id,
                        wishlist_item_id = excluded.wishlist_item_id,
                        request_name = excluded.request_name,
                        destination_directory = excluded.destination_directory,
                        local_path = excluded.local_path,
                        batch_id = excluded.batch_id,
                        reason = excluded.reason,
                        bit_rate = excluded.bit_rate,
                        sample_rate = excluded.sample_rate,
                        bit_depth = excluded.bit_depth,
                        length_seconds = excluded.length_seconds,
                        artist = excluded.artist,
                        album = excluded.album,
                        title = excluded.title,
                        track_number = excluded.track_number,
                        year = excluded.year,
                        attempts = excluded.attempts,
                        auto_replace_attempts = excluded.auto_replace_attempts,
                        next_attempt_at = excluded.next_attempt_at,
                        updated_at_ms = excluded.updated_at_ms
                    WHERE transfers.updated_at_ms <= excluded.updated_at_ms
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
                .bind(transfer.updated_at_ms)
                .execute(&mut *transaction)
                .await;
                if let Err(error) = transfer_insert {
                    let _ = transaction.rollback().await;
                    return Err(error.into());
                }
            } else {
                continue;
            }
            let event_insert = query(
                r#"
                INSERT INTO transfer_events
                    (transfer_id, direction, token, filename, peer_username, filesize, progress, status, reason, created_at, updated_at_ms)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
            .bind(event.updated_at_ms)
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
                SELECT id, transfer_id, direction, token, filename, peer_username, filesize, progress, status, reason, created_at, updated_at_ms
                FROM transfer_events
                WHERE transfer_id = ?
                ORDER BY updated_at_ms DESC, id DESC
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
                SELECT id, transfer_id, direction, token, filename, peer_username, filesize, progress, status, reason, created_at, updated_at_ms
                FROM transfer_events
                ORDER BY updated_at_ms DESC, id DESC
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
}
