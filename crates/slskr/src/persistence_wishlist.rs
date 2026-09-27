use super::*;

impl DatabaseManager {
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
        searches: &[(SearchRecord, String, Vec<SearchResultRecord>)],
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
        for batch in searches.chunks(SQLITE_PARAMETER_CHUNK / 9) {
            let statement = format!(
                r#"
                INSERT OR REPLACE INTO searches
                    (id, query, status, result_count, created_at, completed_at, room, target, fallback_attempts)
                VALUES {}
                "#,
                sql_value_rows(batch.len(), 9)
            );
            let mut insert = query(AssertSqlSafe(statement));
            for (search, _, _) in batch {
                insert = insert
                    .bind(&search.id)
                    .bind(&search.query)
                    .bind(&search.status)
                    .bind(search.result_count)
                    .bind(search.created_at)
                    .bind(search.completed_at)
                    .bind(&search.room)
                    .bind(&search.target)
                    .bind(search.fallback_attempts);
            }
            insert.execute(&mut *transaction).await?;
        }
        for batch in searches.chunks(SQLITE_PARAMETER_CHUNK / 2) {
            let statement = format!(
                "DELETE FROM search_identities WHERE search_id IN ({})",
                sql_placeholders(batch.len())
            );
            let mut delete = query(AssertSqlSafe(statement));
            for (search, _, _) in batch {
                delete = delete.bind(&search.id);
            }
            delete.execute(&mut *transaction).await?;

            let statement = format!(
                "INSERT OR REPLACE INTO search_identities (search_id, external_id) VALUES {}",
                sql_value_rows(batch.len(), 2)
            );
            let mut insert = query(AssertSqlSafe(statement));
            for (search, external_id, _) in batch {
                insert = insert.bind(&search.id).bind(external_id);
            }
            insert.execute(&mut *transaction).await?;
        }
        for batch in searches.chunks(SQLITE_PARAMETER_CHUNK) {
            let statement = format!(
                "DELETE FROM search_results WHERE search_id IN ({})",
                sql_placeholders(batch.len())
            );
            let mut delete = query(AssertSqlSafe(statement));
            for (search, _, _) in batch {
                delete = delete.bind(&search.id);
            }
            delete.execute(&mut *transaction).await?;
        }
        for (search, _, results) in searches {
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
                        .bind(result.created_at);
                }
                insert.execute(&mut *transaction).await?;
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
}
