use super::*;

impl DatabaseManager {
    // ========================================================================
    // Webhook Operations
    // ========================================================================

    /// Insert or update webhook record
    pub async fn insert_webhook(
        &self,
        record: &WebhookRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query(
            r#"
            INSERT OR REPLACE INTO webhooks (id, url, events, secret, active, created_at, last_triggered, retry_count, max_retries, timeout_seconds)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&record.id)
        .bind(&record.url)
        .bind(&record.events)
        .bind(&record.secret)
        .bind(record.active as i32)
        .bind(record.created_at)
        .bind(record.last_triggered)
        .bind(record.retry_count)
        .bind(record.max_retries)
        .bind(record.timeout_seconds)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Get webhook record by ID
    pub async fn get_webhook(
        &self,
        id: &str,
    ) -> Result<Option<WebhookRecord>, Box<dyn std::error::Error>> {
        let record = query_as::<_, WebhookRecord>(
            r#"SELECT id, url, events, secret, active, created_at, last_triggered, retry_count, max_retries, timeout_seconds FROM webhooks WHERE id = ?"#
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(record)
    }

    /// List all webhooks
    pub async fn list_webhooks(&self) -> Result<Vec<WebhookRecord>, Box<dyn std::error::Error>> {
        let records = query_as::<_, WebhookRecord>(
            r#"SELECT id, url, events, secret, active, created_at, last_triggered, retry_count, max_retries, timeout_seconds FROM webhooks ORDER BY created_at DESC"#
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    /// List active webhooks
    pub async fn list_active_webhooks(
        &self,
    ) -> Result<Vec<WebhookRecord>, Box<dyn std::error::Error>> {
        let records = query_as::<_, WebhookRecord>(
            r#"SELECT id, url, events, secret, active, created_at, last_triggered, retry_count, max_retries, timeout_seconds FROM webhooks WHERE active = 1 ORDER BY created_at DESC"#
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    /// Delete webhook
    pub async fn delete_webhook(&self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut transaction = self.pool.begin().await?;
        let log_delete = query("DELETE FROM webhook_logs WHERE webhook_id = ?")
            .bind(id)
            .execute(&mut *transaction)
            .await;
        if let Err(delete_error) = log_delete {
            transaction.rollback().await.map_err(|rollback_error| {
                format!(
                    "webhook log deletion failed ({delete_error}); transaction rollback failed: {rollback_error}"
                )
            })?;
            return Err(delete_error.into());
        }
        let webhook_delete = query("DELETE FROM webhooks WHERE id = ?")
            .bind(id)
            .execute(&mut *transaction)
            .await;
        if let Err(delete_error) = webhook_delete {
            transaction.rollback().await.map_err(|rollback_error| {
                format!(
                    "webhook deletion failed ({delete_error}); transaction rollback failed: {rollback_error}"
                )
            })?;
            return Err(delete_error.into());
        }
        transaction.commit().await?;
        Ok(())
    }

    /// Update webhook active status
    pub async fn update_webhook_active(
        &self,
        id: &str,
        active: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query("UPDATE webhooks SET active = ? WHERE id = ?")
            .bind(active as i32)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Record the most recent delivery outcome for a webhook.
    pub async fn update_webhook_delivery_stats(
        &self,
        id: &str,
        last_triggered: i64,
        retry_count: u32,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        let retry_count = i32::try_from(retry_count).unwrap_or(i32::MAX);
        let result = query("UPDATE webhooks SET last_triggered = ?, retry_count = ? WHERE id = ?")
            .bind(last_triggered)
            .bind(retry_count)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// Insert webhook log record
    pub async fn insert_webhook_log(
        &self,
        record: &WebhookLogRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        query(
            r#"
            INSERT INTO webhook_logs (id, webhook_id, event, correlation_id, status, request_body, response_status, response_body, error_message, attempt, timestamp)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&record.id)
        .bind(&record.webhook_id)
        .bind(&record.event)
        .bind(&record.correlation_id)
        .bind(&record.status)
        .bind(&record.request_body)
        .bind(record.response_status)
        .bind(&record.response_body)
        .bind(&record.error_message)
        .bind(record.attempt)
        .bind(record.timestamp)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Mark every queued log for one webhook dispatch with its terminal outcome.
    pub async fn complete_webhook_logs(
        &self,
        webhook_id: &str,
        correlation_id: &str,
        status: &str,
        error_message: Option<&str>,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        self.complete_webhook_logs_with_attempt(
            webhook_id,
            correlation_id,
            status,
            error_message,
            None,
        )
        .await
    }

    /// Mark queued logs with their terminal outcome and actual attempt count.
    pub async fn complete_webhook_logs_with_attempt(
        &self,
        webhook_id: &str,
        correlation_id: &str,
        status: &str,
        error_message: Option<&str>,
        attempt: Option<i32>,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        let result = query(
            r#"
            UPDATE webhook_logs
            SET status = ?, error_message = ?, attempt = COALESCE(?, attempt)
            WHERE webhook_id = ? AND correlation_id = ? AND status = 'queued'
            "#,
        )
        .bind(status)
        .bind(error_message)
        .bind(attempt)
        .bind(webhook_id)
        .bind(correlation_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Reconcile queued deliveries after all their workers have stopped.
    /// Remote delivery may have happened; only its local outcome is unknown.
    pub async fn fail_unconfirmed_webhook_logs(
        &self,
        reason: &str,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        let result = query(
            "UPDATE webhook_logs SET status = 'failed', error_message = ? WHERE status = 'queued'",
        )
        .bind(reason)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Get webhook logs for a specific webhook
    pub async fn get_webhook_logs(
        &self,
        webhook_id: &str,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<WebhookLogRecord>, Box<dyn std::error::Error>> {
        let records = query_as::<_, WebhookLogRecord>(
            r#"SELECT id, webhook_id, event, correlation_id, status, request_body, response_status, response_body, error_message, attempt, timestamp FROM webhook_logs WHERE webhook_id = ? ORDER BY timestamp DESC LIMIT ? OFFSET ?"#
        )
        .bind(webhook_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    /// Get recent webhook logs by event
    pub async fn get_logs_by_event(
        &self,
        event: &str,
        limit: i32,
    ) -> Result<Vec<WebhookLogRecord>, Box<dyn std::error::Error>> {
        let records = query_as::<_, WebhookLogRecord>(
            r#"SELECT id, webhook_id, event, correlation_id, status, request_body, response_status, response_body, error_message, attempt, timestamp FROM webhook_logs WHERE event = ? ORDER BY timestamp DESC LIMIT ?"#
        )
        .bind(event)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    /// Get failed webhook logs for retry
    pub async fn get_failed_webhook_logs(
        &self,
        limit: i32,
    ) -> Result<Vec<WebhookLogRecord>, Box<dyn std::error::Error>> {
        let records = query_as::<_, WebhookLogRecord>(
            r#"SELECT id, webhook_id, event, correlation_id, status, request_body, response_status, response_body, error_message, attempt, timestamp FROM webhook_logs WHERE status IN ('failed', 'timeout') AND attempt <= (SELECT max_retries FROM webhooks WHERE webhooks.id = webhook_logs.webhook_id) ORDER BY timestamp ASC LIMIT ?"#
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    /// Delete old webhook logs
    pub async fn delete_old_webhook_logs(
        &self,
        days: i32,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        let cutoff =
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64 - (days as i64 * 86400);

        let result = query("DELETE FROM webhook_logs WHERE timestamp < ?")
            .bind(cutoff)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }
}
