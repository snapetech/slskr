/// Server-Sent Events (SSE) streaming support for Phase 12
///
/// Provides real-time event streaming for subscriptions to searches, transfers,
/// messages, and other server state changes.

use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

/// SSE Event type
#[derive(Clone, Debug)]
pub struct SseEvent {
    pub id: String,
    pub event_type: String,
    pub data: Value,
    pub timestamp: u64,
}

impl SseEvent {
    /// Create a new SSE event
    pub fn new(event_type: &str, data: Value) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Self {
            id: format!("event_{}", timestamp),
            event_type: event_type.to_string(),
            data,
            timestamp,
        }
    }

    /// Format event as SSE stream format
    pub fn to_sse_format(&self) -> String {
        format!(
            "id: {}\nevent: {}\ndata: {}\n\n",
            self.id,
            self.event_type,
            self.data.to_string()
        )
    }
}

/// SSE subscription manager
pub struct SseSubscriptionManager {
    subscriptions: Arc<RwLock<HashMap<String, Vec<SseEvent>>>>,
}

impl SseSubscriptionManager {
    /// Create a new subscription manager
    pub fn new() -> Self {
        Self {
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Subscribe to an event stream (searches, transfers, messages, stats)
    pub async fn subscribe(&self, stream_type: &str) -> String {
        let mut subs = self.subscriptions.write().await;
        let key = format!("stream_{}", stream_type);
        
        // Initialize event queue for this stream
        if !subs.contains_key(&key) {
            subs.insert(key.clone(), Vec::new());
        }
        
        key
    }

    /// Emit an event to a stream
    pub async fn emit(&self, stream_type: &str, event: SseEvent) {
        let mut subs = self.subscriptions.write().await;
        let key = format!("stream_{}", stream_type);
        
        subs.entry(key)
            .or_insert_with(Vec::new)
            .push(event);
    }

    /// Get all events for a stream
    pub async fn get_events(&self, stream_type: &str) -> Vec<SseEvent> {
        let subs = self.subscriptions.read().await;
        let key = format!("stream_{}", stream_type);
        
        subs.get(&key)
            .map(|events| events.clone())
            .unwrap_or_default()
    }

    /// Clear events for a stream
    pub async fn clear_events(&self, stream_type: &str) {
        let mut subs = self.subscriptions.write().await;
        let key = format!("stream_{}", stream_type);
        
        if let Some(events) = subs.get_mut(&key) {
            events.clear();
        }
    }

    /// Format events as SSE stream
    pub async fn to_sse_stream(&self, stream_type: &str) -> String {
        let events = self.get_events(stream_type).await;
        
        events
            .iter()
            .map(|e| e.to_sse_format())
            .collect::<Vec<_>>()
            .join("")
    }
}

/// Generate SSE endpoint response headers
pub fn sse_headers() -> (String, String) {
    (
        "text/event-stream".to_string(),
        "Cache-Control: no-cache\nConnection: keep-alive".to_string(),
    )
}

/// Handle SSE subscription requests
pub fn handle_sse_subscribe(stream_type: &str) -> String {
    // Initial SSE event stream header comment
    format!(
        ": SSE stream for {}\n\n",
        stream_type
    )
}

/// Create SSE event for search activity
pub fn create_search_event(search_id: &str, status: &str, result_count: i32) -> SseEvent {
    SseEvent::new("search:update", json!({
        "searchId": search_id,
        "status": status,
        "resultCount": result_count,
        "timestamp": SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }))
}

/// Create SSE event for transfer activity
pub fn create_transfer_event(transfer_id: &str, status: &str, progress: f64) -> SseEvent {
    SseEvent::new("transfer:update", json!({
        "transferId": transfer_id,
        "status": status,
        "progress": progress,
        "timestamp": SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }))
}

/// Create SSE event for message activity
pub fn create_message_event(message_id: &str, username: &str, body: &str) -> SseEvent {
    SseEvent::new("message:new", json!({
        "messageId": message_id,
        "username": username,
        "body": body,
        "timestamp": SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }))
}

/// Create SSE event for connection status changes
pub fn create_status_event(status: &str, details: &str) -> SseEvent {
    SseEvent::new("connection:status", json!({
        "status": status,
        "details": details,
        "timestamp": SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }))
}

/// Create SSE event for user status changes
pub fn create_user_status_event(username: &str, status: &str) -> SseEvent {
    SseEvent::new("user:status", json!({
        "username": username,
        "status": status,
        "timestamp": SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }))
}

/// Create SSE event for room activity
pub fn create_room_event(room_name: &str, event_type: &str, user: Option<&str>) -> SseEvent {
    SseEvent::new("room:update", json!({
        "roomName": room_name,
        "eventType": event_type,
        "user": user,
        "timestamp": SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }))
}

/// Validate SSE stream type
pub fn is_valid_sse_stream(stream_type: &str) -> bool {
    matches!(
        stream_type,
        "searches" | "transfers" | "messages" | "users" | "status" | "rooms"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sse_event_creation() {
        let event = SseEvent::new("test:event", json!({"key": "value"}));
        assert_eq!(event.event_type, "test:event");
        assert!(event.data.get("key").is_some());
    }

    #[test]
    fn test_sse_event_format() {
        let event = SseEvent::new("test", json!({"data": "test"}));
        let formatted = event.to_sse_format();
        assert!(formatted.contains("event: test"));
        assert!(formatted.contains("data: "));
    }

    #[test]
    fn test_search_event_creation() {
        let event = create_search_event("search_123", "completed", 42);
        assert_eq!(event.event_type, "search:update");
        assert_eq!(event.data["resultCount"], 42);
    }

    #[test]
    fn test_transfer_event_creation() {
        let event = create_transfer_event("transfer_456", "in_progress", 0.75);
        assert_eq!(event.event_type, "transfer:update");
        assert_eq!(event.data["progress"], 0.75);
    }

    #[test]
    fn test_message_event_creation() {
        let event = create_message_event("msg_789", "testuser", "hello");
        assert_eq!(event.event_type, "message:new");
        assert_eq!(event.data["username"], "testuser");
    }

    #[test]
    fn test_valid_sse_stream_types() {
        assert!(is_valid_sse_stream("searches"));
        assert!(is_valid_sse_stream("transfers"));
        assert!(is_valid_sse_stream("messages"));
        assert!(is_valid_sse_stream("rooms"));
        assert!(!is_valid_sse_stream("invalid"));
    }

    #[tokio::test]
    async fn test_subscription_manager() {
        let manager = SseSubscriptionManager::new();
        
        // Subscribe to searches
        let sub_key = manager.subscribe("searches").await;
        assert!(sub_key.contains("stream_searches"));
        
        // Emit an event
        let event = create_search_event("s1", "completed", 10);
        manager.emit("searches", event.clone()).await;
        
        // Get events
        let events = manager.get_events("searches").await;
        assert_eq!(events.len(), 1);
    }

    #[tokio::test]
    async fn test_clear_events() {
        let manager = SseSubscriptionManager::new();
        
        manager.emit("searches", create_search_event("s1", "completed", 5)).await;
        manager.emit("searches", create_search_event("s2", "completed", 10)).await;
        
        let events_before = manager.get_events("searches").await;
        assert_eq!(events_before.len(), 2);
        
        manager.clear_events("searches").await;
        
        let events_after = manager.get_events("searches").await;
        assert_eq!(events_after.len(), 0);
    }

    #[tokio::test]
    async fn test_sse_stream_format() {
        let manager = SseSubscriptionManager::new();
        
        manager.emit("searches", create_search_event("s1", "pending", 0)).await;
        
        let stream = manager.to_sse_stream("searches").await;
        assert!(stream.contains("event: search:update"));
        assert!(stream.contains("data: "));
    }
}
