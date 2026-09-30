use super::*;

impl DatabaseManager {
    // ========================================================================
    // OAuth State Operations
    // ========================================================================

    /// Insert or update a pending OAuth state.
    pub async fn upsert_oauth_state(
        &self,
        record: &OAuthStateRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query(
            r#"
            INSERT INTO oauth_states (state, provider, redirect_uri, created_at, expires_at)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT(state) DO UPDATE SET
                provider = excluded.provider,
                redirect_uri = excluded.redirect_uri,
                created_at = excluded.created_at,
                expires_at = excluded.expires_at
            "#,
        )
        .bind(&record.state)
        .bind(&record.provider)
        .bind(&record.redirect_uri)
        .bind(record.created_at)
        .bind(record.expires_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Delete a pending OAuth state after consumption.
    pub async fn delete_oauth_state(&self, state: &str) -> Result<(), Box<dyn std::error::Error>> {
        query("DELETE FROM oauth_states WHERE state = ?")
            .bind(state)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Delete expired pending OAuth states.
    pub async fn delete_expired_oauth_states(
        &self,
        now: i64,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        let result = query("DELETE FROM oauth_states WHERE expires_at <= ?")
            .bind(now)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// List non-expired pending OAuth states.
    pub async fn list_oauth_states(
        &self,
        now: i64,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<OAuthStateRecord>, Box<dyn std::error::Error>> {
        let records = query_as::<_, OAuthStateRecord>(
            r#"
            SELECT state, provider, redirect_uri, created_at, expires_at
            FROM oauth_states
            WHERE expires_at > ?
            ORDER BY created_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(now)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }
}
