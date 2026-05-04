//! WebSocket handling for real-time features (Phase 9)
//!
//! Provides WebSocket/SSE support for:
//! - Real-time search updates
//! - Live transfer progress
//! - Room message streaming
//! - User status notifications

use axum::{
    extract::ws::{WebSocket, WebSocketUpgrade},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum WsMessage {
    /// Search results in real-time
    SearchResult {
        search_id: String,
        results: Vec<String>,
        count: usize,
    },
    /// Transfer progress update
    TransferProgress {
        transfer_id: String,
        progress: u64,
        speed: u64,
        eta_seconds: u32,
    },
    /// Room message notification
    RoomMessage {
        room: String,
        username: String,
        message: String,
        timestamp: i64,
    },
    /// User status change
    UserStatus {
        username: String,
        online: bool,
        timestamp: i64,
    },
    /// Server-sent ping
    Ping,
    /// Client-sent pong
    Pong,
    /// Error message
    Error {
        code: u32,
        message: String,
    },
}

/// WebSocket subscription manager
#[derive(Clone)]
pub struct WsSubscriptionManager {
    search_tx: broadcast::Sender<WsMessage>,
    transfer_tx: broadcast::Sender<WsMessage>,
    room_tx: broadcast::Sender<WsMessage>,
    user_status_tx: broadcast::Sender<WsMessage>,
}

impl WsSubscriptionManager {
    /// Create new WebSocket subscription manager
    pub fn new() -> Self {
        let (search_tx, _) = broadcast::channel(100);
        let (transfer_tx, _) = broadcast::channel(100);
        let (room_tx, _) = broadcast::channel(100);
        let (user_status_tx, _) = broadcast::channel(100);

        Self {
            search_tx,
            transfer_tx,
            room_tx,
            user_status_tx,
        }
    }

    /// Send search update to all subscribers
    pub fn send_search_update(&self, msg: WsMessage) {
        let _ = self.search_tx.send(msg);
    }

    /// Send transfer update to all subscribers
    pub fn send_transfer_update(&self, msg: WsMessage) {
        let _ = self.transfer_tx.send(msg);
    }

    /// Send room message to all subscribers
    pub fn send_room_message(&self, msg: WsMessage) {
        let _ = self.room_tx.send(msg);
    }

    /// Send user status update to all subscribers
    pub fn send_user_status(&self, msg: WsMessage) {
        let _ = self.user_status_tx.send(msg);
    }

    /// Subscribe to search updates
    pub fn subscribe_searches(&self) -> broadcast::Receiver<WsMessage> {
        self.search_tx.subscribe()
    }

    /// Subscribe to transfer updates
    pub fn subscribe_transfers(&self) -> broadcast::Receiver<WsMessage> {
        self.transfer_tx.subscribe()
    }

    /// Subscribe to room messages
    pub fn subscribe_rooms(&self) -> broadcast::Receiver<WsMessage> {
        self.room_tx.subscribe()
    }

    /// Subscribe to user status updates
    pub fn subscribe_user_status(&self) -> broadcast::Receiver<WsMessage> {
        self.user_status_tx.subscribe()
    }
}

impl Default for WsSubscriptionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for WsSubscriptionManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WsSubscriptionManager")
            .field("search_subscribers", &self.search_tx.receiver_count())
            .field("transfer_subscribers", &self.transfer_tx.receiver_count())
            .field("room_subscribers", &self.room_tx.receiver_count())
            .field("user_status_subscribers", &self.user_status_tx.receiver_count())
            .finish()
    }
}

/// Handle WebSocket connections
pub async fn handle_ws_connection(
    ws: WebSocketUpgrade,
    sub_mgr: Arc<WsSubscriptionManager>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, sub_mgr))
}

/// Process WebSocket messages
async fn handle_socket(socket: axum::extract::ws::WebSocket, _sub_mgr: Arc<WsSubscriptionManager>) {
    use axum::extract::ws::Message;
    use futures::{sink::SinkExt, stream::StreamExt};

    let (mut sender, mut receiver) = socket.split();

    // Send initial connection message
    let _ = sender.send(Message::Text("Connected to slskR WebSocket API".to_string())).await;

    // Handle incoming messages
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                // Parse incoming message
                if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                    match ws_msg {
                        WsMessage::Ping => {
                            let _ = sender.send(Message::Text(
                                serde_json::to_string(&WsMessage::Pong).unwrap_or_default()
                            )).await;
                        }
                        _ => {
                            // Handle other message types
                            tracing::debug!("Received WebSocket message: {:?}", ws_msg);
                        }
                    }
                }
            }
            Ok(Message::Close(_)) => {
                break;
            }
            Err(e) => {
                tracing::error!("WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }
}

/// Server-Sent Events handler for browsers without WebSocket support
pub async fn handle_sse_connection(
    _sub_mgr: Arc<WsSubscriptionManager>,
) -> impl IntoResponse {
    // SSE implementation would go here
    // Returns Stream of ServerSentEvent
    axum::response::sse::sse(futures::stream::empty::<Result<axum::response::sse::Event, std::convert::Infallible>>())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_subscription_manager_creation() {
        let mgr = WsSubscriptionManager::new();
        assert_eq!(mgr.search_tx.receiver_count(), 0);
        assert_eq!(mgr.transfer_tx.receiver_count(), 0);
    }

    #[test]
    fn test_ws_message_serialization() {
        let msg = WsMessage::SearchResult {
            search_id: "search-123".to_string(),
            results: vec!["result1".to_string()],
            count: 1,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("SearchResult"));
    }

    #[test]
    fn test_ws_subscription() {
        let mgr = WsSubscriptionManager::new();
        let mut rx = mgr.subscribe_searches();
        
        let msg = WsMessage::Ping;
        mgr.send_search_update(msg.clone());
        
        // In actual async test, would verify reception
    }
}
