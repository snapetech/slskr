use super::*;

impl DatabaseManager {
    /// Insert or update a contact.
    pub async fn upsert_contact(
        &self,
        record: &ContactRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query(
            r#"
            INSERT OR REPLACE INTO contacts (id, username, online, status, free_upload_slots, queue_length, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&record.id)
        .bind(&record.username)
        .bind(record.online as i32)
        .bind(&record.status)
        .bind(record.free_upload_slots)
        .bind(record.queue_length)
        .bind(record.created_at)
        .bind(record.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Delete a contact.
    pub async fn delete_contact(&self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        query("DELETE FROM contacts WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// List persisted contacts.
    pub async fn list_contacts(
        &self,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<ContactRecord>, Box<dyn std::error::Error>> {
        let records = query_as::<_, ContactRecord>(
            "SELECT id, username, online, status, free_upload_slots, queue_length, created_at, updated_at FROM contacts ORDER BY updated_at DESC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    /// Insert or update a share grant only while its collection still exists.
    pub async fn upsert_share_grant(
        &self,
        record: &ShareGrantRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let result = query(
            r#"
            INSERT INTO share_grants (id, collection_id, username, shared_at, permissions, max_concurrent_streams)
            SELECT ?, ?, ?, ?, ?, ?
            WHERE EXISTS (SELECT 1 FROM collections WHERE id = ?)
            ON CONFLICT(id) DO UPDATE SET
                collection_id = excluded.collection_id,
                username = excluded.username,
                shared_at = excluded.shared_at,
                permissions = excluded.permissions,
                max_concurrent_streams = excluded.max_concurrent_streams
            "#,
        )
        .bind(&record.id)
        .bind(&record.collection_id)
        .bind(&record.username)
        .bind(record.shared_at)
        .bind(&record.permissions)
        .bind(record.max_concurrent_streams)
        .bind(&record.collection_id)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "share grant collection does not exist",
            )
            .into());
        }
        Ok(())
    }

    /// Delete a share grant.
    pub async fn delete_share_grant(&self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut transaction = self.pool.begin().await?;
        query("DELETE FROM share_access_tokens WHERE grant_id = ?")
            .bind(id)
            .execute(&mut *transaction)
            .await?;
        query("DELETE FROM share_grants WHERE id = ?")
            .bind(id)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(())
    }

    /// List persisted share grants.
    pub async fn list_share_grants(
        &self,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<ShareGrantRecord>, Box<dyn std::error::Error>> {
        let records = query_as::<_, ShareGrantRecord>(
            "SELECT id, collection_id, username, shared_at, permissions, max_concurrent_streams FROM share_grants ORDER BY shared_at DESC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    /// Persist a delegated-share verifier. Callers must provide only a digest.
    pub async fn upsert_share_access_token(
        &self,
        record: &ShareAccessTokenRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if record.token_digest.len() != 64
            || !record
                .token_digest
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "share access token verifier must be a SHA-256 hex digest",
            )
            .into());
        }
        let result = query(
            r#"
            INSERT INTO share_access_tokens (token_digest, grant_id, expires_at)
            SELECT ?, ?, ?
            WHERE EXISTS (SELECT 1 FROM share_grants WHERE id = ?)
            ON CONFLICT(token_digest) DO UPDATE SET
                grant_id = excluded.grant_id,
                expires_at = excluded.expires_at
            "#,
        )
        .bind(&record.token_digest)
        .bind(&record.grant_id)
        .bind(record.expires_at)
        .bind(&record.grant_id)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() != 1 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "share access token grant is unavailable",
            )
            .into());
        }
        Ok(())
    }

    /// Delete expired delegated-share verifiers.
    pub async fn delete_expired_share_access_tokens(
        &self,
        now: i64,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        let result = query("DELETE FROM share_access_tokens WHERE expires_at <= ?")
            .bind(now)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// List unexpired delegated-share verifiers without exposing raw tokens.
    pub async fn list_share_access_tokens(
        &self,
        now: i64,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<ShareAccessTokenRecord>, Box<dyn std::error::Error>> {
        let records = query_as::<_, ShareAccessTokenRecord>(
            "SELECT token_digest, grant_id, expires_at FROM share_access_tokens WHERE expires_at > ? ORDER BY expires_at ASC LIMIT ? OFFSET ?",
        )
        .bind(now)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    /// Insert or update a share group.
    pub async fn upsert_share_group(
        &self,
        record: &ShareGroupRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query(
            r#"
            INSERT OR REPLACE INTO share_groups (id, name, description, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&record.id)
        .bind(&record.name)
        .bind(&record.description)
        .bind(record.created_at)
        .bind(record.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Replace a share group and its complete membership snapshot atomically.
    pub async fn replace_share_group(
        &self,
        record: &ShareGroupRecord,
        members: &[ShareGroupMemberRecord],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut transaction = self.pool.begin().await?;
        let result = async {
            query(
                r#"
                INSERT OR REPLACE INTO share_groups (id, name, description, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?)
                "#,
            )
            .bind(&record.id)
            .bind(&record.name)
            .bind(&record.description)
            .bind(record.created_at)
            .bind(record.updated_at)
            .execute(&mut *transaction)
            .await?;
            query("DELETE FROM share_group_members WHERE group_id = ?")
                .bind(&record.id)
                .execute(&mut *transaction)
                .await?;
            for member in members {
                query(
                    r#"
                    INSERT INTO share_group_members (group_id, username, added_at)
                    VALUES (?, ?, ?)
                    "#,
                )
                .bind(&member.group_id)
                .bind(&member.username)
                .bind(member.added_at)
                .execute(&mut *transaction)
                .await?;
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

    /// Delete a share group and its members.
    pub async fn delete_share_group(&self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut transaction = self.pool.begin().await?;
        query("DELETE FROM share_group_members WHERE group_id = ?")
            .bind(id)
            .execute(&mut *transaction)
            .await?;
        query("DELETE FROM share_groups WHERE id = ?")
            .bind(id)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(())
    }

    /// List persisted share groups.
    pub async fn list_share_groups(
        &self,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<ShareGroupRecord>, Box<dyn std::error::Error>> {
        let records = query_as::<_, ShareGroupRecord>(
            "SELECT id, name, description, created_at, updated_at FROM share_groups ORDER BY updated_at DESC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    /// Insert or update a share group member.
    pub async fn upsert_share_group_member(
        &self,
        record: &ShareGroupMemberRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query(
            r#"
            INSERT OR REPLACE INTO share_group_members (group_id, username, added_at)
            VALUES (?, ?, ?)
            "#,
        )
        .bind(&record.group_id)
        .bind(&record.username)
        .bind(record.added_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Delete a share group member.
    pub async fn delete_share_group_member(
        &self,
        group_id: &str,
        username: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query("DELETE FROM share_group_members WHERE group_id = ? AND username = ?")
            .bind(group_id)
            .bind(username)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// List persisted share group members.
    pub async fn list_share_group_members(
        &self,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<ShareGroupMemberRecord>, Box<dyn std::error::Error>> {
        let records = query_as::<_, ShareGroupMemberRecord>(
            "SELECT group_id, username, added_at FROM share_group_members ORDER BY added_at DESC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    /// Insert or update a collection.
    pub async fn upsert_collection(
        &self,
        record: &CollectionRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query(
            r#"
            INSERT OR REPLACE INTO collections
                (id, owner_user_id, name, description, collection_type, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&record.id)
        .bind(&record.owner_user_id)
        .bind(&record.name)
        .bind(&record.description)
        .bind(&record.collection_type)
        .bind(record.created_at)
        .bind(record.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Persist a collection and its exact ordered item snapshot atomically.
    pub async fn replace_collection(
        &self,
        record: &CollectionRecord,
        items: &[CollectionItemRecord],
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.replace_collection_snapshot(record, items, false).await
    }

    /// Replace a collection and its exact ordered item snapshot only while its
    /// parent row still exists. This prevents a delayed item/update write from
    /// recreating a collection after a concurrent delete.
    pub async fn replace_existing_collection(
        &self,
        record: &CollectionRecord,
        items: &[CollectionItemRecord],
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.replace_collection_snapshot(record, items, true).await
    }

    async fn replace_collection_snapshot(
        &self,
        record: &CollectionRecord,
        items: &[CollectionItemRecord],
        require_existing: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut transaction = self.pool.begin().await?;
        if require_existing {
            let exists = query("SELECT 1 FROM collections WHERE id = ?")
                .bind(&record.id)
                .fetch_optional(&mut *transaction)
                .await?
                .is_some();
            if !exists {
                transaction.rollback().await?;
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "collection no longer exists",
                )
                .into());
            }
        }
        let result = async {
            query(
                r#"
                INSERT INTO collections
                    (id, owner_user_id, name, description, collection_type, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(id) DO UPDATE SET
                    owner_user_id = excluded.owner_user_id,
                    name = excluded.name,
                    description = excluded.description,
                    collection_type = excluded.collection_type,
                    created_at = excluded.created_at,
                    updated_at = excluded.updated_at
                "#,
            )
            .bind(&record.id)
            .bind(&record.owner_user_id)
            .bind(&record.name)
            .bind(&record.description)
            .bind(&record.collection_type)
            .bind(record.created_at)
            .bind(record.updated_at)
            .execute(&mut *transaction)
            .await?;
            query("DELETE FROM collection_items WHERE collection_id = ?")
                .bind(&record.id)
                .execute(&mut *transaction)
                .await?;
            for item in items {
                query(
                    r#"
                    INSERT INTO collection_items
                        (id, collection_id, content_id, artist, title, kind, file_name, album, content_hash, added_at, position)
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                )
                .bind(&item.id)
                .bind(&item.collection_id)
                .bind(&item.content_id)
                .bind(&item.artist)
                .bind(&item.title)
                .bind(&item.kind)
                .bind(&item.file_name)
                .bind(&item.album)
                .bind(&item.content_hash)
                .bind(item.added_at)
                .bind(item.position)
                .execute(&mut *transaction)
                .await?;
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

    /// Delete a collection, its items, and its access grants atomically.
    pub async fn delete_collection(&self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut transaction = self.pool.begin().await?;
        query(
            "DELETE FROM share_access_tokens WHERE grant_id IN (SELECT id FROM share_grants WHERE collection_id = ?)",
        )
        .bind(id)
        .execute(&mut *transaction)
        .await?;
        query("DELETE FROM share_grants WHERE collection_id = ?")
            .bind(id)
            .execute(&mut *transaction)
            .await?;
        query("DELETE FROM collection_items WHERE collection_id = ?")
            .bind(id)
            .execute(&mut *transaction)
            .await?;
        query("DELETE FROM collections WHERE id = ?")
            .bind(id)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(())
    }

    /// List persisted collections.
    pub async fn list_collections(
        &self,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<CollectionRecord>, Box<dyn std::error::Error>> {
        let records = query_as::<_, CollectionRecord>(
            "SELECT id, owner_user_id, name, description, collection_type, created_at, updated_at FROM collections ORDER BY updated_at DESC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    /// Insert or update a collection item.
    pub async fn upsert_collection_item(
        &self,
        record: &CollectionItemRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query(
            r#"
            INSERT OR REPLACE INTO collection_items
                (id, collection_id, content_id, artist, title, kind, file_name, album, content_hash, added_at, position)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&record.id)
        .bind(&record.collection_id)
        .bind(&record.content_id)
        .bind(&record.artist)
        .bind(&record.title)
        .bind(&record.kind)
        .bind(&record.file_name)
        .bind(&record.album)
        .bind(&record.content_hash)
        .bind(record.added_at)
        .bind(record.position)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Delete a collection item.
    pub async fn delete_collection_item(&self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        query("DELETE FROM collection_items WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// List persisted collection items.
    pub async fn list_collection_items(
        &self,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<CollectionItemRecord>, Box<dyn std::error::Error>> {
        let records = query_as::<_, CollectionItemRecord>(
            "SELECT id, collection_id, content_id, artist, title, kind, file_name, album, content_hash, added_at, position FROM collection_items ORDER BY collection_id, position, added_at LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }
}
