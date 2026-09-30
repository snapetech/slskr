use super::*;

impl DatabaseManager {
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
             backfill_runs, songid_runs, songid_run_records_json, lidarr_sync_runs,
             lidarr_manual_imports, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
        .bind(&runtime.songid_run_records_json)
        .bind(runtime.lidarr_sync_runs)
        .bind(runtime.lidarr_manual_imports)
        .bind(runtime.updated_at)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(())
    }

    /// Persist a local manual import without replacing unrelated runtime
    /// compatibility fields from a potentially stale snapshot.
    pub async fn create_library_item_and_record_manual_import(
        &self,
        library: &LibraryItemRecord,
        runtime: &RuntimeCompatRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut transaction = self.pool.begin().await?;
        query(
            r#"
            INSERT INTO library_items (id, artist, title, kind, created_at)
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
            INSERT INTO runtime_compat_state
            (id, application_restart_requested, gc_runs, autoreplace_enabled, relay_enabled,
             relay_agent_enabled, bridge_running, bridge_config_updates, profile_invites_created,
             options_updates, options_yaml_uploads, options_yaml_validations, cache_warm_runs,
             backfill_runs, songid_runs, songid_run_records_json, lidarr_sync_runs,
             lidarr_manual_imports, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                lidarr_manual_imports = excluded.lidarr_manual_imports,
                updated_at = MAX(runtime_compat_state.updated_at, excluded.updated_at)
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
        .bind(&runtime.songid_run_records_json)
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
        for batch in records.chunks(SQLITE_PARAMETER_CHUNK / 5) {
            let statement = format!(
                r#"
                INSERT OR REPLACE INTO library_items (id, artist, title, kind, created_at)
                VALUES {}
                "#,
                sql_value_rows(batch.len(), 5)
            );
            let mut insert = query(AssertSqlSafe(statement));
            for record in batch {
                insert = insert
                    .bind(&record.id)
                    .bind(&record.artist)
                    .bind(&record.title)
                    .bind(&record.kind)
                    .bind(record.created_at);
            }
            insert.execute(&mut *transaction).await?;
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
             backfill_runs, songid_runs, songid_run_records_json, lidarr_sync_runs,
             lidarr_manual_imports, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
        .bind(&record.songid_run_records_json)
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
        // This endpoint is served from a single-connection pool. Keep all
        // counts in one SQLite statement so the result has one read snapshot
        // and does not acquire/release the connection once per table.
        let rows: Vec<(String, i64)> = query_as(
            r#"
            WITH counts(name, value) AS (
                SELECT 'search_count', COUNT(*) FROM searches
                UNION ALL SELECT 'search_result_count', COUNT(*) FROM search_results
                UNION ALL SELECT 'transfer_count', COUNT(*) FROM transfers
                UNION ALL SELECT 'transfer_event_count', COUNT(*) FROM transfer_events
                UNION ALL SELECT 'share_file_count', COUNT(*) FROM share_files
                UNION ALL SELECT 'event_count', COUNT(*) FROM events
                UNION ALL SELECT 'message_count', COUNT(*) FROM messages
                UNION ALL SELECT 'user_count', COUNT(*) FROM user_stats
                UNION ALL SELECT 'user_projection_count', COUNT(*) FROM user_records
                UNION ALL SELECT 'room_count', COUNT(*) FROM rooms WHERE subscribed = 1
                UNION ALL SELECT 'user_note_count', COUNT(*) FROM user_notes
                UNION ALL SELECT 'interest_count', COUNT(*) FROM interests
                UNION ALL SELECT 'security_ban_count', COUNT(*) FROM security_bans
                UNION ALL SELECT 'wishlist_count', COUNT(*) FROM wishlist_items
                UNION ALL SELECT 'contact_count', COUNT(*) FROM contacts
                UNION ALL SELECT 'share_grant_count', COUNT(*) FROM share_grants
                UNION ALL SELECT 'share_access_token_count', COUNT(*) FROM share_access_tokens
                UNION ALL SELECT 'share_group_count', COUNT(*) FROM share_groups
                UNION ALL SELECT 'share_group_member_count', COUNT(*) FROM share_group_members
                UNION ALL SELECT 'collection_count', COUNT(*) FROM collections
                UNION ALL SELECT 'collection_item_count', COUNT(*) FROM collection_items
                UNION ALL SELECT 'library_item_count', COUNT(*) FROM library_items
                UNION ALL SELECT 'destination_count', COUNT(*) FROM destinations
                UNION ALL SELECT 'now_playing_count', COUNT(*) FROM now_playing
                UNION ALL SELECT 'browse_count', COUNT(*) FROM browse_records
                UNION ALL SELECT 'runtime_state_count', COUNT(*) FROM runtime_compat_state
                UNION ALL SELECT 'oauth_state_count', COUNT(*) FROM oauth_states
                UNION ALL SELECT 'webhook_count', COUNT(*) FROM webhooks
                UNION ALL SELECT 'webhook_log_count', COUNT(*) FROM webhook_logs
            )
            SELECT name, value FROM counts
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        let counts = rows
            .into_iter()
            .collect::<std::collections::BTreeMap<_, _>>();
        let count = |name: &str| -> Result<u64, Box<dyn std::error::Error>> {
            let value = counts.get(name).copied().ok_or_else(|| {
                Box::<dyn std::error::Error>::from(format!(
                    "database statistics query omitted {name}"
                ))
            })?;
            Ok(nonnegative_database_count(value)?)
        };

        Ok(DatabaseStats {
            search_count: count("search_count")?,
            search_result_count: count("search_result_count")?,
            transfer_count: count("transfer_count")?,
            transfer_event_count: count("transfer_event_count")?,
            share_file_count: count("share_file_count")?,
            event_count: count("event_count")?,
            message_count: count("message_count")?,
            user_count: count("user_count")?,
            user_projection_count: count("user_projection_count")?,
            room_count: count("room_count")?,
            user_note_count: count("user_note_count")?,
            interest_count: count("interest_count")?,
            security_ban_count: count("security_ban_count")?,
            wishlist_count: count("wishlist_count")?,
            contact_count: count("contact_count")?,
            share_grant_count: count("share_grant_count")?,
            share_access_token_count: count("share_access_token_count")?,
            share_group_count: count("share_group_count")?,
            share_group_member_count: count("share_group_member_count")?,
            collection_count: count("collection_count")?,
            collection_item_count: count("collection_item_count")?,
            library_item_count: count("library_item_count")?,
            destination_count: count("destination_count")?,
            now_playing_count: count("now_playing_count")?,
            browse_count: count("browse_count")?,
            runtime_state_count: count("runtime_state_count")?,
            oauth_state_count: count("oauth_state_count")?,
            webhook_count: count("webhook_count")?,
            webhook_log_count: count("webhook_log_count")?,
        })
    }

    /// Remove persisted messages older than a caller-selected cutoff.
    pub async fn cleanup_old_messages_before(
        &self,
        cutoff: i64,
    ) -> Result<u64, Box<dyn std::error::Error>> {
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
}
