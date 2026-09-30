use super::*;

impl DatabaseManager {
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

    /// Atomically update a search projection and append only newly accepted
    /// result rows. Search response bursts can therefore update metadata
    /// without deleting and reinserting the complete historical projection.
    pub async fn append_search_results(
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
        for batch in ids.chunks(SQLITE_PARAMETER_CHUNK) {
            let placeholders = sql_placeholders(batch.len());
            let mut delete_results = query(AssertSqlSafe(format!(
                "DELETE FROM search_results WHERE search_id IN ({placeholders})"
            )));
            for id in batch {
                delete_results = delete_results.bind(id);
            }
            delete_results.execute(&mut *transaction).await?;

            let mut delete_searches = query(AssertSqlSafe(format!(
                "DELETE FROM searches WHERE id IN ({placeholders})"
            )));
            for id in batch {
                delete_searches = delete_searches.bind(id);
            }
            delete_searches.execute(&mut *transaction).await?;

            let mut delete_identities = query(AssertSqlSafe(format!(
                "DELETE FROM search_identities WHERE search_id IN ({placeholders})"
            )));
            for id in batch {
                delete_identities = delete_identities.bind(id);
            }
            delete_identities.execute(&mut *transaction).await?;
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
}
