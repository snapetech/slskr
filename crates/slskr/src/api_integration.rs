/// API Integration Module - Shows how to wire all components into the routing
/// This is a guide for integrating webhooks, persistence, tracing, and other features

use crate::webhooks::{WebhookManager, WebhookEvent, WebhookPayload};
use crate::persistence::DatabaseManager;
use crate::tracing::{RequestSpan, set_request_span, complete_request_span, CorrelationId};

/// Example routing additions for the HTTP server

pub fn example_routing_additions() {
    // These routes should be added to the main routing match statement:
    
    // WEBHOOK ROUTES
    // ("POST", "/api/admin/webhooks") => create_webhook_handler()
    // ("GET", "/api/admin/webhooks") => list_webhooks_handler()
    // ("DELETE", path) if path.starts_with("/api/admin/webhooks/") => delete_webhook_handler(path)
    // ("POST", path) if path.ends_with("/test") => test_webhook_handler(path)
    
    // DATABASE ROUTES
    // ("GET", "/api/admin/database/stats") => database_stats_handler()
    // ("POST", "/api/admin/database/cleanup") => database_cleanup_handler()
    // ("POST", "/api/admin/database/vacuum") => database_vacuum_handler()
    
    // GRAPHQL ROUTE
    // ("POST", "/api/graphql") => graphql_handler()
    // ("GET", "/api/graphql/schema") => graphql_schema_handler()
}

/// Example: How to trigger webhooks after creating a search

pub fn example_trigger_webhook_on_search_creation(
    search_id: &str,
    query: &str,
    webhook_mgr: &WebhookManager,
    correlation_id: &CorrelationId,
) {
    // After search is created:
    let payload = WebhookPayload::new(
        WebhookEvent::SearchCreated,
        correlation_id.to_string(),
        serde_json::json!({
            "id": search_id,
            "query": query,
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        }),
    );

    // Get all webhooks subscribed to this event
    for webhook in webhook_mgr.get_for_event(WebhookEvent::SearchCreated) {
        // Serialize payload
        if let Ok(payload_bytes) = payload.to_bytes() {
            // Create signature
            let _signature = crate::webhooks::WebhookSignature::create(
                &payload_bytes,
                &webhook.secret,
            );
            
            // Send HTTP POST to webhook.url with:
            // - Body: payload_bytes
            // - Header: X-Webhook-Signature: <signature>
            // - Header: X-Correlation-ID: <correlation_id>
            // Example using reqwest:
            // tokio::spawn(async move {
            //     let client = reqwest::Client::new();
            //     client.post(&webhook.url)
            //         .json(&payload)
            //         .send()
            //         .await
            //         .ok();
            // });
        }
    }
}

/// Example: How to add request tracing to middleware

pub fn example_add_request_tracing(
    method: &str,
    path: &str,
    user_agent: Option<String>,
    client_ip: Option<String>,
) -> CorrelationId {
    let span = RequestSpan::new(
        method.to_string(),
        path.to_string(),
        user_agent,
        client_ip,
    );

    let correlation_id = span.correlation_id.clone();
    set_request_span(span);
    correlation_id
}

/// Example: How to complete request span

pub fn example_complete_request_span(status_code: u16) {
    complete_request_span(status_code);
    // This automatically logs request timing with correlation ID
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webhook_payload_creation() {
        let payload = WebhookPayload::new(
            WebhookEvent::SearchCreated,
            "test-correlation-id".to_string(),
            serde_json::json!({"test": "data"}),
        );

        assert!(payload.to_bytes().is_ok());
    }

    #[test]
    fn test_request_tracing() {
        let corr_id = example_add_request_tracing(
            "GET",
            "/api/searches",
            Some("Mozilla/5.0".to_string()),
            Some("127.0.0.1".to_string()),
        );

        assert!(corr_id.as_str().starts_with("corr-"));
    }

    #[tokio::test]
    async fn test_database_persistence() {
        let db = DatabaseManager::in_memory().await.unwrap();
        
        let search = db.get_search("test-1").await.unwrap();
        assert!(search.is_none());
    }
}
