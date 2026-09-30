use super::fixtures::*;
use std::sync::atomic::{AtomicBool, Ordering};

#[tokio::test]
async fn daemon_shutdown_reconciles_queued_delivery_after_worker_cancellation() {
    let database = crate::persistence::DatabaseManager::in_memory()
        .await
        .unwrap();
    database
        .insert_webhook(&crate::persistence::WebhookRecord {
            id: "shutdown-hook".to_owned(),
            url: "https://example.invalid/hook".to_owned(),
            events: "search.created".to_owned(),
            secret: "fixture-secret".to_owned(),
            active: true,
            created_at: 1,
            last_triggered: None,
            retry_count: 0,
            max_retries: 3,
            timeout_seconds: 30,
        })
        .await
        .unwrap();
    database
        .insert_webhook_log(&crate::persistence::WebhookLogRecord {
            id: "unconfirmed".to_owned(),
            webhook_id: "shutdown-hook".to_owned(),
            event: "search.created".to_owned(),
            correlation_id: "cancelled".to_owned(),
            status: "queued".to_owned(),
            request_body: "{}".to_owned(),
            response_status: None,
            response_body: None,
            error_message: None,
            attempt: 1,
            timestamp: 11,
        })
        .await
        .unwrap();
    let (state, _receiver) = test_state_with_db(MapEnv::default(), database.clone());
    let cancelled = Arc::new(AtomicBool::new(false));
    let marker = Arc::clone(&cancelled);
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
    struct Cancelled(Arc<AtomicBool>);
    impl Drop for Cancelled {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }
    assert!(state.managed_background_tasks.try_spawn(async move {
        let _cancelled = Cancelled(marker);
        let _ = ready_tx.send(());
        std::future::pending::<()>().await;
    }));
    tokio::time::timeout(Duration::from_secs(2), ready_rx)
        .await
        .unwrap()
        .unwrap();
    state.shutdown_managed_tasks().await;
    assert!(cancelled.load(Ordering::SeqCst));
    let logs = database
        .get_webhook_logs("shutdown-hook", 10, 0)
        .await
        .unwrap();
    assert_eq!(logs[0].status, "failed");
    assert_eq!(
        logs[0].error_message.as_deref(),
        Some("delivery outcome unknown: daemon shut down before confirmation")
    );
    assert_eq!(logs[0].attempt, 1);
    fs::remove_dir_all(&state.config.state_dir).unwrap();
}
