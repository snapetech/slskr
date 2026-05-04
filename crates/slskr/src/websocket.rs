//! WebSocket support for real-time event streaming (RFC 6455)

use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

/// WebSocket frame type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameType {
    Continuation,
    Text,
    Binary,
    Close,
    Ping,
    Pong,
}

/// WebSocket frame structure
#[derive(Debug, Clone)]
pub struct Frame {
    pub fin: bool,
    pub opcode: FrameType,
    pub mask: Option<[u8; 4]>,
    pub payload: Vec<u8>,
}

/// WebSocket connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Connecting,
    Open,
    Closing,
    Closed,
}

/// WebSocket client connection
pub struct WebSocketConnection {
    pub id: String,
    pub state: ConnectionState,
    pub message_queue: Arc<RwLock<VecDeque<String>>>,
    pub subscribed_topics: Vec<String>,
}

/// WebSocket server for managing multiple connections
pub struct WebSocketServer {
    connections: Arc<RwLock<Vec<WebSocketConnection>>>,
    event_history: Arc<RwLock<VecDeque<String>>>,
    max_history_size: usize,
}

impl Frame {
    /// Create a text frame
    pub fn text(content: String) -> Self {
        Self {
            fin: true,
            opcode: FrameType::Text,
            mask: None,
            payload: content.into_bytes(),
        }
    }

    /// Create a ping frame
    pub fn ping() -> Self {
        Self {
            fin: true,
            opcode: FrameType::Ping,
            mask: None,
            payload: vec![],
        }
    }

    /// Create a pong frame
    pub fn pong() -> Self {
        Self {
            fin: true,
            opcode: FrameType::Pong,
            mask: None,
            payload: vec![],
        }
    }

    /// Create a close frame
    pub fn close(code: u16, reason: &str) -> Self {
        let mut payload = code.to_be_bytes().to_vec();
        payload.extend_from_slice(reason.as_bytes());
        Self {
            fin: true,
            opcode: FrameType::Close,
            mask: None,
            payload,
        }
    }

    /// Encode frame to bytes (simplified, no masking)
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = vec![];

        // First byte: FIN (1 bit) + RSV (3 bits) + Opcode (4 bits)
        let opcode = match self.opcode {
            FrameType::Continuation => 0x0,
            FrameType::Text => 0x1,
            FrameType::Binary => 0x2,
            FrameType::Close => 0x8,
            FrameType::Ping => 0x9,
            FrameType::Pong => 0xA,
        };
        let first_byte = if self.fin { 0x80 } else { 0x00 } | opcode;
        bytes.push(first_byte);

        // Second byte: MASK (1 bit) + Payload length (7 bits)
        let payload_len = self.payload.len();
        let mask_bit = if self.mask.is_some() { 0x80 } else { 0x00 };

        if payload_len < 126 {
            bytes.push(mask_bit | (payload_len as u8));
        } else if payload_len < 65536 {
            bytes.push(mask_bit | 126);
            bytes.extend_from_slice(&(payload_len as u16).to_be_bytes());
        } else {
            bytes.push(mask_bit | 127);
            bytes.extend_from_slice(&(payload_len as u64).to_be_bytes());
        }

        // Add masking key if present
        if let Some(mask_key) = self.mask {
            bytes.extend_from_slice(&mask_key);
            // Mask payload (XOR with key)
            for (i, byte) in self.payload.iter().enumerate() {
                bytes.push(byte ^ mask_key[i % 4]);
            }
        } else {
            bytes.extend_from_slice(&self.payload);
        }

        bytes
    }

    /// Decode frame from bytes (simplified)
    pub fn decode(data: &[u8]) -> Option<(Self, usize)> {
        if data.len() < 2 {
            return None;
        }

        let fin = (data[0] & 0x80) != 0;
        let opcode = match data[0] & 0x0F {
            0x0 => FrameType::Continuation,
            0x1 => FrameType::Text,
            0x2 => FrameType::Binary,
            0x8 => FrameType::Close,
            0x9 => FrameType::Ping,
            0xA => FrameType::Pong,
            _ => return None,
        };

        let masked = (data[1] & 0x80) != 0;
        let mut payload_len = (data[1] & 0x7F) as usize;
        let mut bytes_read = 2;

        // Extended payload length
        if payload_len == 126 && data.len() >= 4 {
            payload_len = u16::from_be_bytes([data[2], data[3]]) as usize;
            bytes_read = 4;
        } else if payload_len == 127 && data.len() >= 10 {
            payload_len = u64::from_be_bytes([
                data[2], data[3], data[4], data[5], data[6], data[7], data[8], data[9],
            ]) as usize;
            bytes_read = 10;
        }

        // Masking key
        let mask = if masked && data.len() >= bytes_read + 4 {
            let mask_key = [
                data[bytes_read],
                data[bytes_read + 1],
                data[bytes_read + 2],
                data[bytes_read + 3],
            ];
            bytes_read += 4;
            Some(mask_key)
        } else {
            None
        };

        // Payload
        if data.len() < bytes_read + payload_len {
            return None;
        }

        let mut payload = data[bytes_read..bytes_read + payload_len].to_vec();

        // Unmask payload if needed
        if let Some(mask_key) = mask {
            for (i, byte) in payload.iter_mut().enumerate() {
                *byte ^= mask_key[i % 4];
            }
        }

        let frame = Frame {
            fin,
            opcode,
            mask,
            payload,
        };

        Some((frame, bytes_read + payload_len))
    }
}

impl WebSocketConnection {
    /// Create new WebSocket connection
    pub fn new(id: String) -> Self {
        Self {
            id,
            state: ConnectionState::Connecting,
            message_queue: Arc::new(RwLock::new(VecDeque::new())),
            subscribed_topics: vec![],
        }
    }

    /// Subscribe to a topic
    pub fn subscribe(&mut self, topic: String) {
        if !self.subscribed_topics.contains(&topic) {
            self.subscribed_topics.push(topic);
        }
    }

    /// Unsubscribe from a topic
    pub fn unsubscribe(&mut self, topic: &str) {
        self.subscribed_topics.retain(|t| t != topic);
    }

    /// Check if subscribed to topic
    pub fn is_subscribed(&self, topic: &str) -> bool {
        self.subscribed_topics.contains(&topic.to_string())
    }

    /// Queue message for delivery
    pub async fn queue_message(&self, message: String) {
        let mut queue = self.message_queue.write().await;
        queue.push_back(message);
    }

    /// Get next message from queue
    pub async fn dequeue_message(&self) -> Option<String> {
        let mut queue = self.message_queue.write().await;
        queue.pop_front()
    }
}

impl WebSocketServer {
    /// Create new WebSocket server
    pub fn new(max_history_size: usize) -> Self {
        Self {
            connections: Arc::new(RwLock::new(Vec::new())),
            event_history: Arc::new(RwLock::new(VecDeque::new())),
            max_history_size,
        }
    }

    /// Register new connection
    pub async fn register(&self, connection: WebSocketConnection) {
        let mut conns = self.connections.write().await;
        conns.push(connection);
    }

    /// Unregister connection
    pub async fn unregister(&self, connection_id: &str) {
        let mut conns = self.connections.write().await;
        conns.retain(|c| c.id != connection_id);
    }

    /// Broadcast message to all subscribers of topic
    pub async fn broadcast(&self, topic: String, message: String) {
        let conns = self.connections.read().await;
        for conn in conns.iter() {
            if conn.is_subscribed(&topic) {
                conn.queue_message(message.clone()).await;
            }
        }

        // Add to history
        let mut history = self.event_history.write().await;
        let event = format!("{{\"topic\":\"{}\",\"message\":{}}}", topic, message);
        history.push_back(event);

        if history.len() > self.max_history_size {
            history.pop_front();
        }
    }

    /// Get event history for topic
    pub async fn get_history(&self, topic: &str) -> Vec<String> {
        let history = self.event_history.read().await;
        history
            .iter()
            .filter(|e| e.contains(&format!("\"{}\"", topic)))
            .cloned()
            .collect()
    }

    /// Get active connection count
    pub async fn connection_count(&self) -> usize {
        let conns = self.connections.read().await;
        conns.len()
    }

    /// Get connections subscribed to topic
    pub async fn subscribers_for_topic(&self, topic: &str) -> usize {
        let conns = self.connections.read().await;
        conns
            .iter()
            .filter(|c| c.is_subscribed(topic))
            .count()
    }
}

/// WebSocket handshake helper
pub struct WebSocketHandshake;

impl WebSocketHandshake {
    /// Generate WebSocket Sec-WebSocket-Key
    pub fn generate_key() -> String {
        use std::time::SystemTime;
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .subsec_nanos();
        format!("ws-key-{}", nanos)
    }

    /// Build handshake response headers
    pub fn build_response(key: &str) -> String {
        // Simplified: In production, compute base64(SHA1(key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"))
        let response_key = format!("{}258EAFA5", key);

        format!(
            "HTTP/1.1 101 Switching Protocols\r\n\
             Upgrade: websocket\r\n\
             Connection: Upgrade\r\n\
             Sec-WebSocket-Accept: {}\r\n\
             \r\n",
            response_key
        )
    }

    /// Check if request is WebSocket upgrade request
    pub fn is_upgrade_request(request: &str) -> bool {
        request.contains("Upgrade: websocket") && request.contains("Connection:")
    }

    /// Extract Sec-WebSocket-Key from request
    pub fn extract_key(request: &str) -> Option<String> {
        for line in request.lines() {
            if line.starts_with("Sec-WebSocket-Key:") {
                return Some(line.split(':').nth(1)?.trim().to_string());
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_text_creation() {
        let frame = Frame::text("Hello".to_string());
        assert_eq!(frame.opcode, FrameType::Text);
        assert!(frame.fin);
    }

    #[test]
    fn test_frame_ping_pong() {
        let ping = Frame::ping();
        let pong = Frame::pong();
        assert_eq!(ping.opcode, FrameType::Ping);
        assert_eq!(pong.opcode, FrameType::Pong);
    }

    #[test]
    fn test_frame_encoding() {
        let frame = Frame::text("Hi".to_string());
        let encoded = frame.encode();
        assert!(!encoded.is_empty());
    }

    #[test]
    fn test_websocket_connection_creation() {
        let conn = WebSocketConnection::new("conn-1".to_string());
        assert_eq!(conn.state, ConnectionState::Connecting);
    }

    #[test]
    fn test_subscription_management() {
        let mut conn = WebSocketConnection::new("conn-1".to_string());
        conn.subscribe("events".to_string());
        assert!(conn.is_subscribed("events"));
        conn.unsubscribe("events");
        assert!(!conn.is_subscribed("events"));
    }

    #[tokio::test]
    async fn test_websocket_server_creation() {
        let server = WebSocketServer::new(100);
        assert_eq!(server.connection_count().await, 0);
    }

    #[test]
    fn test_websocket_handshake_upgrade_detection() {
        let request = "GET / HTTP/1.1\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n";
        assert!(WebSocketHandshake::is_upgrade_request(request));
    }

    #[test]
    fn test_websocket_key_extraction() {
        let request = "GET / HTTP/1.1\r\nSec-WebSocket-Key: test-key-123\r\n";
        let key = WebSocketHandshake::extract_key(request);
        assert_eq!(key, Some("test-key-123".to_string()));
    }
}
