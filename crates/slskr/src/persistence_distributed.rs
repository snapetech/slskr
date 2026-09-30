use super::*;

fn checked_distributed_depth(value: i64) -> Result<u32, Box<dyn std::error::Error>> {
    Ok(u32::try_from(value)?)
}

impl DatabaseManager {
    /// Load wishlist scheduler state
    pub async fn load_wishlist_scheduler_state(
        &self,
    ) -> Result<Option<(usize, Option<u64>)>, Box<dyn std::error::Error>> {
        let result = query_as::<_, (i64, Option<i64>)>(
            "SELECT next_index, server_interval_seconds FROM wishlist_scheduler_state WHERE id = 1",
        )
        .fetch_optional(&self.pool)
        .await?;

        result
            .map(
                |(next_index, server_interval)| -> Result<_, Box<dyn std::error::Error>> {
                    Ok((
                        usize::try_from(next_index)?,
                        server_interval.map(u64::try_from).transpose()?,
                    ))
                },
            )
            .transpose()
    }

    /// Save wishlist scheduler state
    pub async fn save_wishlist_scheduler_state(
        &self,
        next_index: usize,
        server_interval_seconds: Option<u64>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;

        query(
            r#"
            INSERT OR REPLACE INTO wishlist_scheduler_state (id, next_index, server_interval_seconds, updated_at)
            VALUES (1, ?, ?, ?)
            "#,
        )
        .bind(i64::try_from(next_index)?)
        .bind(server_interval_seconds.map(i64::try_from).transpose()?)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Load distributed tree state
    pub async fn load_distributed_tree_state(
        &self,
    ) -> Result<Option<(u32, String, Option<String>)>, Box<dyn std::error::Error>> {
        let result = query_as::<_, (i64, String, Option<String>)>(
            "SELECT branch_level, branch_root, parent_username FROM distributed_tree_state WHERE id = 1",
        )
        .fetch_optional(&self.pool)
        .await?;

        result
            .map(|(branch_level, branch_root, parent_username)| {
                checked_distributed_depth(branch_level)
                    .map(|level| (level, branch_root, parent_username))
            })
            .transpose()
    }

    /// Load the distributed tree and child state from one read snapshot.
    pub async fn load_distributed_state(
        &self,
    ) -> Result<
        (Option<(u32, String, Option<String>)>, Vec<(String, u32)>),
        Box<dyn std::error::Error>,
    > {
        let mut transaction = self.pool.begin().await?;
        let tree_state = query_as::<_, (i64, String, Option<String>)>(
            "SELECT branch_level, branch_root, parent_username FROM distributed_tree_state WHERE id = 1",
        )
        .fetch_optional(&mut *transaction)
        .await?
        .map(|(branch_level, branch_root, parent_username)| {
            checked_distributed_depth(branch_level)
                .map(|level| (level, branch_root, parent_username))
        })
        .transpose()?;
        let children = query_as::<_, (String, i64)>(
            "SELECT username, depth FROM distributed_children ORDER BY username",
        )
        .fetch_all(&mut *transaction)
        .await?
        .into_iter()
        .map(|(username, depth)| checked_distributed_depth(depth).map(|depth| (username, depth)))
        .collect::<Result<Vec<_>, _>>()?;
        transaction.commit().await?;
        Ok((tree_state, children))
    }

    /// Save distributed tree state
    pub async fn save_distributed_tree_state(
        &self,
        branch_level: u32,
        branch_root: &str,
        parent_username: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;

        query(
            r#"
            INSERT OR REPLACE INTO distributed_tree_state (id, branch_level, branch_root, parent_username, updated_at)
            VALUES (1, ?, ?, ?, ?)
            "#,
        )
        .bind(branch_level as i64)
        .bind(branch_root)
        .bind(parent_username)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Replace distributed tree metadata and child depths atomically.
    pub async fn save_distributed_state(
        &self,
        branch_level: u32,
        branch_root: &str,
        parent_username: Option<&str>,
        children: &[(String, u32)],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
        let mut transaction = self.pool.begin().await?;
        query(
            r#"
            INSERT OR REPLACE INTO distributed_tree_state (id, branch_level, branch_root, parent_username, updated_at)
            VALUES (1, ?, ?, ?, ?)
            "#,
        )
        .bind(branch_level as i64)
        .bind(branch_root)
        .bind(parent_username)
        .bind(now)
        .execute(&mut *transaction)
        .await?;

        query("DELETE FROM distributed_children")
            .execute(&mut *transaction)
            .await?;
        for batch in children.chunks(SQLITE_PARAMETER_CHUNK / 3) {
            let statement = format!(
                r#"
                INSERT INTO distributed_children (username, depth, updated_at)
                VALUES {}
                "#,
                sql_value_rows(batch.len(), 3)
            );
            let mut insert = query(AssertSqlSafe(statement));
            for (username, depth) in batch {
                insert = insert.bind(username).bind(*depth as i64).bind(now);
            }
            insert.execute(&mut *transaction).await?;
        }

        transaction.commit().await?;
        Ok(())
    }

    /// Load distributed children
    pub async fn load_distributed_children(
        &self,
    ) -> Result<Vec<(String, u32)>, Box<dyn std::error::Error>> {
        let results = query_as::<_, (String, i64)>(
            "SELECT username, depth FROM distributed_children ORDER BY username",
        )
        .fetch_all(&self.pool)
        .await?;

        results
            .into_iter()
            .map(|(username, depth)| {
                checked_distributed_depth(depth).map(|depth| (username, depth))
            })
            .collect()
    }

    /// Save distributed children
    pub async fn save_distributed_children(
        &self,
        children: &[(String, u32)],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;

        let mut transaction = self.pool.begin().await?;
        query("DELETE FROM distributed_children")
            .execute(&mut *transaction)
            .await?;

        for batch in children.chunks(SQLITE_PARAMETER_CHUNK / 3) {
            let statement = format!(
                r#"
                INSERT INTO distributed_children (username, depth, updated_at)
                VALUES {}
                "#,
                sql_value_rows(batch.len(), 3)
            );
            let mut insert = query(AssertSqlSafe(statement));
            for (username, depth) in batch {
                insert = insert.bind(username).bind(*depth as i64).bind(now);
            }
            insert.execute(&mut *transaction).await?;
        }

        transaction.commit().await?;
        Ok(())
    }
}
