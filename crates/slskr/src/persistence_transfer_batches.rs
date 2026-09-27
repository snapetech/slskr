use super::*;

impl DatabaseManager {
    // ========================================================================
    // Transfer Operations
    // ========================================================================

    /// Insert a durable transfer batch.  The primary key deliberately remains
    /// database-enforced so concurrent callers cannot create the same batch
    /// twice.
    pub async fn insert_transfer_batch(
        &self,
        record: &TransferBatchRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query(
            r#"
            INSERT INTO Batches (Id, SearchId, Username, Direction, CreatedAt, Options)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&record.id)
        .bind(&record.search_id)
        .bind(&record.username)
        .bind(record.direction)
        .bind(&record.created_at)
        .bind(&record.options_json)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Read one durable transfer batch without hydrating its associated
    /// transfer rows.
    pub async fn get_transfer_batch(
        &self,
        id: &str,
    ) -> Result<Option<TransferBatchRecord>, Box<dyn std::error::Error>> {
        let record = query_as::<_, TransferBatchRecord>(
            "SELECT Id, SearchId, Username, Direction, CreatedAt, Options FROM Batches WHERE Id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(record)
    }

    /// Update durable transfer-batch metadata for lifecycle migrations and
    /// administrative maintenance.
    pub async fn update_transfer_batch(
        &self,
        record: &TransferBatchRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query(
            r#"
            UPDATE Batches
            SET SearchId = ?, Username = ?, Direction = ?, CreatedAt = ?, Options = ?
            WHERE Id = ?
            "#,
        )
        .bind(&record.search_id)
        .bind(&record.username)
        .bind(record.direction)
        .bind(&record.created_at)
        .bind(&record.options_json)
        .bind(&record.id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Delete one durable transfer batch and return whether a row existed.
    pub async fn delete_transfer_batch(
        &self,
        id: &str,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let result = query("DELETE FROM Batches WHERE Id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}
