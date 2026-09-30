use super::*;

impl DatabaseManager {
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
}
