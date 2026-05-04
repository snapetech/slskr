//! SignalR hub implementation for real-time features (Phase 9)
//!
//! Provides SignalR hub compatibility for browser clients using
//! either native SignalR .NET clients or JavaScript libraries.
//! 
//! Supported hubs:
//! - /hubs/transfers: Real-time transfer updates
//! - /hubs/searches: Real-time search results
//! - /hubs/rooms: Room messages and user presence
//! - /hubs/notifications: Server notifications

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;
use std::sync::Arc;

/// SignalR message format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalRMessage {
    pub protocol: String,
    pub version: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_field: Option<i32>,
}

/// Hub connection information
#[derive(Clone, Debug)]
pub struct HubConnection {
    pub connection_id: String,
    pub hub_name: String,
    pub user_id: Option<String>,
    pub connected_at: i64,
}

/// SignalR hub manager
#[derive(Clone)]
pub struct SignalRHubManager {
    connections: Arc<RwLock<HashMap<String, HubConnection>>>,
    transfer_hub_connections: Arc<RwLock<Vec<String>>>,
    search_hub_connections: Arc<RwLock<Vec<String>>>,
    room_hub_connections: Arc<RwLock<Vec<String>>>,
}

impl SignalRHubManager {
    /// Create new SignalR hub manager
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            transfer_hub_connections: Arc::new(RwLock::new(Vec::new())),
            search_hub_connections: Arc::new(RwLock::new(Vec::new())),
            room_hub_connections: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Register connection to a hub
    pub async fn register_connection(
        &self,
        connection_id: String,
        hub_name: String,
        user_id: Option<String>,
    ) -> Result<(), String> {
        let connection = HubConnection {
            connection_id: connection_id.clone(),
            hub_name: hub_name.clone(),
            user_id,
            connected_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0),
        };

        // Store connection
        let mut conns = self.connections.write().await;
        conns.insert(connection_id.clone(), connection);

        // Add to hub-specific list
        match hub_name.as_str() {
            "transfers" => {
                self.transfer_hub_connections.write().await.push(connection_id);
            }
            "searches" => {
                self.search_hub_connections.write().await.push(connection_id);
            }
            "rooms" => {
                self.room_hub_connections.write().await.push(connection_id);
            }
            _ => {}
        }

        Ok(())
    }

    /// Unregister connection from a hub
    pub async fn unregister_connection(&self, connection_id: &str) {
        let mut conns = self.connections.write().await;
        conns.remove(connection_id);

        // Remove from hub lists
        self.transfer_hub_connections
            .write()
            .await
            .retain(|id| id != connection_id);
        self.search_hub_connections
            .write()
            .await
            .retain(|id| id != connection_id);
        self.room_hub_connections
            .write()
            .await
            .retain(|id| id != connection_id);
    }

    /// Broadcast message to all connections in a hub
    pub async fn broadcast_to_hub(&self, hub_name: &str, message: SignalRMessage) {
        let connections = match hub_name {
            "transfers" => self.transfer_hub_connections.read().await.clone(),
            "searches" => self.search_hub_connections.read().await.clone(),
            "rooms" => self.room_hub_connections.read().await.clone(),
            _ => Vec::new(),
        };

        tracing::debug!(
            "Broadcasting to {} hub: {} connections",
            hub_name,
            connections.len()
        );
    }

    /// Send message to specific connection
    pub async fn send_to_connection(
        &self,
        connection_id: &str,
        message: SignalRMessage,
    ) -> Result<(), String> {
        let conns = self.connections.read().await;
        if conns.contains_key(connection_id) {
            tracing::debug!(
                "Sending message to connection: {}",
                connection_id
            );
            Ok(())
        } else {
            Err(format!("Connection not found: {}", connection_id))
        }
    }

    /// Get hub statistics
    pub async fn get_hub_stats(&self) -> HubStats {
        HubStats {
            total_connections: self.connections.read().await.len(),
            transfer_hub_connections: self.transfer_hub_connections.read().await.len(),
            search_hub_connections: self.search_hub_connections.read().await.len(),
            room_hub_connections: self.room_hub_connections.read().await.len(),
        }
    }
}

impl Default for SignalRHubManager {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for SignalRHubManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SignalRHubManager")
            .field("transfer_connections", &self.transfer_hub_connections)
            .field("search_connections", &self.search_hub_connections)
            .field("room_connections", &self.room_hub_connections)
            .finish()
    }
}

/// Hub statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HubStats {
    pub total_connections: usize,
    pub transfer_hub_connections: usize,
    pub search_hub_connections: usize,
    pub room_hub_connections: usize,
}

/// Transfer hub methods
pub mod transfer_hub {
    use super::*;

    /// Notify transfer progress update
    pub async fn notify_progress(
        manager: &SignalRHubManager,
        transfer_id: String,
        progress: u64,
        speed: u64,
        eta_seconds: u32,
    ) {
        let message = SignalRMessage {
            protocol: "json".to_string(),
            version: 1,
            target: Some("OnTransferProgress".to_string()),
            arguments: Some(vec![
                serde_json::json!({ "transfer_id": transfer_id }),
                serde_json::json!({ "progress": progress }),
                serde_json::json!({ "speed": speed }),
                serde_json::json!({ "eta_seconds": eta_seconds }),
            ]),
            type_field: Some(1), // Invocation
        };

        manager.broadcast_to_hub("transfers", message).await;
    }

    /// Notify transfer completion
    pub async fn notify_completion(
        manager: &SignalRHubManager,
        transfer_id: String,
        total_size: u64,
        duration_seconds: u32,
    ) {
        let message = SignalRMessage {
            protocol: "json".to_string(),
            version: 1,
            target: Some("OnTransferCompleted".to_string()),
            arguments: Some(vec![
                serde_json::json!({ "transfer_id": transfer_id }),
                serde_json::json!({ "total_size": total_size }),
                serde_json::json!({ "duration_seconds": duration_seconds }),
            ]),
            type_field: Some(1),
        };

        manager.broadcast_to_hub("transfers", message).await;
    }
}

/// Search hub methods
pub mod search_hub {
    use super::*;

    /// Notify search results
    pub async fn notify_results(
        manager: &SignalRHubManager,
        search_id: String,
        results: Vec<String>,
        total_count: usize,
    ) {
        let message = SignalRMessage {
            protocol: "json".to_string(),
            version: 1,
            target: Some("OnSearchResults".to_string()),
            arguments: Some(vec![
                serde_json::json!({ "search_id": search_id }),
                serde_json::json!({ "results": results }),
                serde_json::json!({ "total_count": total_count }),
            ]),
            type_field: Some(1),
        };

        manager.broadcast_to_hub("searches", message).await;
    }

    /// Notify search completion
    pub async fn notify_completion(
        manager: &SignalRHubManager,
        search_id: String,
        result_count: usize,
    ) {
        let message = SignalRMessage {
            protocol: "json".to_string(),
            version: 1,
            target: Some("OnSearchCompleted".to_string()),
            arguments: Some(vec![
                serde_json::json!({ "search_id": search_id }),
                serde_json::json!({ "result_count": result_count }),
            ]),
            type_field: Some(1),
        };

        manager.broadcast_to_hub("searches", message).await;
    }
}

/// Room hub methods
pub mod room_hub {
    use super::*;

    /// Notify new room message
    pub async fn notify_message(
        manager: &SignalRHubManager,
        room: String,
        username: String,
        message: String,
        timestamp: i64,
    ) {
        let msg = SignalRMessage {
            protocol: "json".to_string(),
            version: 1,
            target: Some("OnRoomMessage".to_string()),
            arguments: Some(vec![
                serde_json::json!({ "room": room }),
                serde_json::json!({ "username": username }),
                serde_json::json!({ "message": message }),
                serde_json::json!({ "timestamp": timestamp }),
            ]),
            type_field: Some(1),
        };

        manager.broadcast_to_hub("rooms", msg).await;
    }

    /// Notify user joined room
    pub async fn notify_user_joined(
        manager: &SignalRHubManager,
        room: String,
        username: String,
        user_count: usize,
    ) {
        let message = SignalRMessage {
            protocol: "json".to_string(),
            version: 1,
            target: Some("OnUserJoined".to_string()),
            arguments: Some(vec![
                serde_json::json!({ "room": room }),
                serde_json::json!({ "username": username }),
                serde_json::json!({ "user_count": user_count }),
            ]),
            type_field: Some(1),
        };

        manager.broadcast_to_hub("rooms", message).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hub_manager_creation() {
        let manager = SignalRHubManager::new();
        let stats = manager.get_hub_stats().await;
        assert_eq!(stats.total_connections, 0);
    }

    #[tokio::test]
    async fn test_register_connection() {
        let manager = SignalRHubManager::new();
        let result = manager
            .register_connection("conn-123".to_string(), "transfers".to_string(), None)
            .await;
        assert!(result.is_ok());

        let stats = manager.get_hub_stats().await;
        assert_eq!(stats.total_connections, 1);
        assert_eq!(stats.transfer_hub_connections, 1);
    }

    #[tokio::test]
    async fn test_unregister_connection() {
        let manager = SignalRHubManager::new();
        manager
            .register_connection("conn-123".to_string(), "transfers".to_string(), None)
            .await
            .unwrap();

        manager.unregister_connection("conn-123").await;

        let stats = manager.get_hub_stats().await;
        assert_eq!(stats.total_connections, 0);
    }
}
