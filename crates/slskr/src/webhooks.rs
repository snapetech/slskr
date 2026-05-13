use hmac::{Hmac, Mac};
use rand::{rngs::OsRng, RngCore};
/// Webhook support with HMAC-SHA256 request signing
///
/// Allows configuring webhooks that are triggered on API events,
/// with cryptographic signing for security and verification.
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fmt;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::sync::Arc;
use tokio::sync::Semaphore;
use uuid::Uuid;

const WEBHOOK_MIN_TIMEOUT_SECONDS: u32 = 1;
const WEBHOOK_MAX_TIMEOUT_SECONDS: u32 = 30;
pub const MAX_WEBHOOKS: usize = 64;
pub const MIN_WEBHOOK_SECRET_BYTES: usize = 32;
const WEBHOOK_ALLOW_CIDRS_ENV: &str = "SLSKR_WEBHOOK_ALLOW_CIDRS";
const WEBHOOK_DENY_CIDRS_ENV: &str = "SLSKR_WEBHOOK_DENY_CIDRS";

/// Webhook event types
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum WebhookEvent {
    SearchCreated,
    SearchCompleted,
    TransferStarted,
    TransferCompleted,
    TransferFailed,
    MessageReceived,
    MessageSent,
    UserConnected,
    UserDisconnected,
    RoomJoined,
    RoomLeft,
    ApiKeyCreated,
    ApiKeyRevoked,
    ConfigChanged,
}

impl WebhookEvent {
    pub fn from_wire(value: &str) -> Option<Self> {
        match value.trim() {
            "search.created" => Some(WebhookEvent::SearchCreated),
            "search.completed" => Some(WebhookEvent::SearchCompleted),
            "transfer.started" => Some(WebhookEvent::TransferStarted),
            "transfer.completed" => Some(WebhookEvent::TransferCompleted),
            "transfer.failed" => Some(WebhookEvent::TransferFailed),
            "message.received" => Some(WebhookEvent::MessageReceived),
            "message.sent" => Some(WebhookEvent::MessageSent),
            "user.connected" => Some(WebhookEvent::UserConnected),
            "user.disconnected" => Some(WebhookEvent::UserDisconnected),
            "room.joined" => Some(WebhookEvent::RoomJoined),
            "room.left" => Some(WebhookEvent::RoomLeft),
            "apikey.created" => Some(WebhookEvent::ApiKeyCreated),
            "apikey.revoked" => Some(WebhookEvent::ApiKeyRevoked),
            "config.changed" => Some(WebhookEvent::ConfigChanged),
            _ => None,
        }
    }
}

impl fmt::Display for WebhookEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WebhookEvent::SearchCreated => write!(f, "search.created"),
            WebhookEvent::SearchCompleted => write!(f, "search.completed"),
            WebhookEvent::TransferStarted => write!(f, "transfer.started"),
            WebhookEvent::TransferCompleted => write!(f, "transfer.completed"),
            WebhookEvent::TransferFailed => write!(f, "transfer.failed"),
            WebhookEvent::MessageReceived => write!(f, "message.received"),
            WebhookEvent::MessageSent => write!(f, "message.sent"),
            WebhookEvent::UserConnected => write!(f, "user.connected"),
            WebhookEvent::UserDisconnected => write!(f, "user.disconnected"),
            WebhookEvent::RoomJoined => write!(f, "room.joined"),
            WebhookEvent::RoomLeft => write!(f, "room.left"),
            WebhookEvent::ApiKeyCreated => write!(f, "apikey.created"),
            WebhookEvent::ApiKeyRevoked => write!(f, "apikey.revoked"),
            WebhookEvent::ConfigChanged => write!(f, "config.changed"),
        }
    }
}

/// Webhook configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Webhook {
    pub id: String,
    pub url: String,
    pub events: Vec<WebhookEvent>,
    pub secret: String,
    pub active: bool,
    pub created_at: i64,
    pub last_triggered: Option<i64>,
    pub retry_count: u32,
    pub max_retries: u32,
    pub timeout_seconds: u32,
}

impl Webhook {
    /// Create new webhook
    pub fn new(url: String, events: Vec<WebhookEvent>, secret: String) -> Self {
        Webhook {
            id: format!("hook_{}", Uuid::new_v4()),
            url,
            events,
            secret,
            active: true,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
            last_triggered: None,
            retry_count: 0,
            max_retries: 3,
            timeout_seconds: 30,
        }
    }

    /// Generate signing secret
    pub fn generate_secret() -> String {
        let mut secret = [0_u8; 32];
        OsRng.fill_bytes(&mut secret);
        format!("secret_{}", hex::encode(secret))
    }

    /// Check if webhook should handle event
    pub fn handles_event(&self, event: WebhookEvent) -> bool {
        self.active && self.events.contains(&event)
    }

    /// Check if webhook is ready to retry
    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }
}

pub fn validate_webhook_secret(secret: &str) -> Result<(), &'static str> {
    if secret.len() < MIN_WEBHOOK_SECRET_BYTES {
        return Err("webhook secret must be at least 32 bytes");
    }
    if secret
        .bytes()
        .any(|byte| matches!(byte, 0x00..=0x1f | 0x7f))
    {
        return Err("webhook secret must not contain control characters");
    }
    let unique = secret.chars().collect::<HashSet<_>>().len();
    if unique < 8 {
        return Err("webhook secret has too little character variety");
    }
    Ok(())
}

/// Webhook payload
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WebhookPayload {
    pub id: String,
    pub event: String,
    pub timestamp: i64,
    pub correlation_id: String,
    pub data: serde_json::Value,
}

impl WebhookPayload {
    /// Create new webhook payload
    pub fn new(event: WebhookEvent, correlation_id: String, data: serde_json::Value) -> Self {
        WebhookPayload {
            id: format!("evt_{}", Uuid::new_v4()),
            event: event.to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
            correlation_id,
            data,
        }
    }

    /// Serialize to JSON bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    /// Serialize to JSON string
    pub fn to_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

/// HMAC-SHA256 signature for webhook payload
#[derive(Clone, Debug)]
pub struct WebhookSignature {
    pub signature: String,
    pub timestamp: i64,
    pub algorithm: String,
}

impl WebhookSignature {
    const MAX_TIMESTAMP_AGE_SECONDS: i64 = 5 * 60;

    /// Create signature for payload using secret
    pub fn create(payload: &[u8], secret: &str) -> Result<Self, Box<dyn std::error::Error>> {
        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())?;
        mac.update(payload);
        let signature = hex::encode(mac.finalize().into_bytes());

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        Ok(WebhookSignature {
            signature,
            timestamp,
            algorithm: "hmac-sha256".to_string(),
        })
    }

    /// Verify signature
    pub fn verify(&self, payload: &[u8], secret: &str) -> Result<bool, Box<dyn std::error::Error>> {
        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())?;
        mac.update(payload);
        let expected = hex::encode(mac.finalize().into_bytes());

        Ok(constant_time_compare(
            self.signature.as_bytes(),
            expected.as_bytes(),
        ))
    }

    /// Get as header value
    pub fn as_header(&self) -> String {
        format!("t={}, {}", self.timestamp, self.signature)
    }

    /// Parse from header
    pub fn from_header(header: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let parts: Vec<&str> = header.split(", ").collect();
        if parts.len() < 2 {
            return Err("Invalid signature header format".into());
        }

        let timestamp_part = parts[0];
        let sig_part = parts[1];

        let timestamp = timestamp_part
            .strip_prefix("t=")
            .ok_or("Missing timestamp")?
            .parse::<i64>()?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;
        if (now - timestamp).abs() > Self::MAX_TIMESTAMP_AGE_SECONDS {
            return Err("signature timestamp is stale".into());
        }

        Ok(WebhookSignature {
            signature: sig_part.to_string(),
            timestamp,
            algorithm: "hmac-sha256".to_string(),
        })
    }
}

/// Webhook manager
#[derive(Debug, Clone)]
pub struct WebhookManager {
    webhooks: HashMap<String, Webhook>,
}

impl WebhookManager {
    /// Create new webhook manager
    pub fn new() -> Self {
        WebhookManager {
            webhooks: HashMap::new(),
        }
    }

    pub fn from_webhooks(webhooks: Vec<Webhook>) -> Self {
        let mut manager = Self::new();
        for webhook in webhooks.into_iter().take(MAX_WEBHOOKS) {
            manager.webhooks.insert(webhook.id.clone(), webhook);
        }
        manager
    }

    /// Register webhook
    pub fn register(&mut self, webhook: Webhook) -> Result<String, ()> {
        if self.webhooks.len() >= MAX_WEBHOOKS {
            return Err(());
        }
        let id = webhook.id.clone();
        self.webhooks.insert(id.clone(), webhook);
        Ok(id)
    }

    /// Unregister webhook
    pub fn unregister(&mut self, id: &str) -> Option<Webhook> {
        self.webhooks.remove(id)
    }

    /// Get webhook
    pub fn get(&self, id: &str) -> Option<&Webhook> {
        self.webhooks.get(id)
    }

    /// Get mutable webhook
    pub fn get_mut(&mut self, id: &str) -> Option<&mut Webhook> {
        self.webhooks.get_mut(id)
    }

    /// Get all webhooks
    pub fn get_all(&self) -> Vec<&Webhook> {
        self.webhooks.values().collect()
    }

    /// Get webhooks for event
    pub fn get_for_event(&self, event: WebhookEvent) -> Vec<&Webhook> {
        self.webhooks
            .values()
            .filter(|w| w.handles_event(event))
            .collect()
    }

    /// List all webhooks
    pub fn list(&self) -> Vec<String> {
        self.webhooks.keys().cloned().collect()
    }

    /// Clear all webhooks
    pub fn clear(&mut self) {
        self.webhooks.clear();
    }
}

impl Default for WebhookManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Constant-time comparison to prevent timing attacks
fn constant_time_compare(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }

    result == 0
}

/// Webhook dispatcher for async event publishing
pub struct WebhookDispatcher;

impl WebhookDispatcher {
    /// Dispatch event to all matching webhooks
    pub async fn dispatch(
        manager: &WebhookManager,
        deliveries: Arc<Semaphore>,
        correlation_id: String,
        event: WebhookEvent,
        data: serde_json::Value,
    ) {
        let webhooks = manager.get_for_event(event);

        if webhooks.is_empty() {
            return;
        }

        let payload = WebhookPayload::new(event, correlation_id, data);
        let payload_json = payload.to_string().unwrap_or_default();

        for webhook in webhooks {
            // Spawn async task for each webhook delivery (no blocking)
            let webhook_url = webhook.url.clone();
            let webhook_secret = webhook.secret.clone();
            let webhook_timeout = webhook.timeout_seconds;
            let payload_clone = payload_json.clone();
            let deliveries = Arc::clone(&deliveries);

            tokio::spawn(async move {
                let Ok(_delivery_permit) = deliveries.try_acquire_owned() else {
                    eprintln!(
                        "[WEBHOOK] Dropped delivery to {} because the delivery pool is full",
                        sanitized_webhook_url_for_log(&webhook_url)
                    );
                    return;
                };
                let _ = Self::send_webhook(
                    &webhook_url,
                    &webhook_secret,
                    &payload_clone,
                    webhook_timeout,
                )
                .await;
            });
        }
    }

    /// Send webhook to URL with HMAC-SHA256 signature
    pub async fn send_webhook(
        url: &str,
        secret: &str,
        payload: &str,
        timeout_secs: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let payload_bytes = payload.as_bytes();

        // Create HMAC signature
        let sig = WebhookSignature::create(payload_bytes, secret)?;

        let resolved = validate_and_resolve_webhook_url(url)?;

        // Disable redirects so validation cannot be bypassed after the initial URL check.
        let mut client_builder =
            reqwest::Client::builder().redirect(reqwest::redirect::Policy::none());
        for addr in &resolved.addrs {
            client_builder = client_builder.resolve(&resolved.host, *addr);
        }
        let client = client_builder.build()?;

        let timeout = timeout_secs.clamp(WEBHOOK_MIN_TIMEOUT_SECONDS, WEBHOOK_MAX_TIMEOUT_SECONDS);
        let response = client
            .post(url)
            .header("X-Webhook-Signature", sig.as_header())
            .header("X-Webhook-Event", "webhook")
            .header("Content-Type", "application/json")
            .body(payload.to_string())
            .timeout(std::time::Duration::from_secs(timeout as u64))
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!("webhook delivery failed with status {status}").into());
        }

        // Log successful delivery
        eprintln!(
            "[WEBHOOK] Delivered to: {} (status: {}, payload: {} bytes)",
            sanitized_webhook_url_for_log(url),
            status,
            payload.len()
        );

        Ok(())
    }

    /// Create test dispatch payload
    pub fn test_payload(event: WebhookEvent, description: &str) -> serde_json::Value {
        serde_json::json!({
            "event": event.to_string(),
            "description": description,
            "test": true,
        })
    }
}

struct ResolvedWebhookTarget {
    host: String,
    addrs: Vec<SocketAddr>,
}

pub(crate) fn validate_webhook_url_for_registration(
    url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let parsed = reqwest::Url::parse(url)?;
    match parsed.scheme() {
        "http" | "https" => {}
        _ => return Err("webhook URL scheme must be http or https".into()),
    }
    let Some(host) = parsed.host_str() else {
        return Err("webhook URL must include a host".into());
    };
    if host.eq_ignore_ascii_case("localhost") {
        return Err("webhook URL host is not allowed".into());
    }
    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_blocked_webhook_ip(ip) {
            return Err("webhook URL IP is not allowed".into());
        }
    }
    parsed
        .port_or_known_default()
        .ok_or("webhook URL port is unknown")?;
    Ok(())
}

fn validate_and_resolve_webhook_url(
    url: &str,
) -> Result<ResolvedWebhookTarget, Box<dyn std::error::Error>> {
    validate_webhook_url_for_registration(url)?;
    let parsed = reqwest::Url::parse(url)?;
    let host = parsed.host_str().ok_or("webhook URL must include a host")?;

    let port = parsed
        .port_or_known_default()
        .ok_or("webhook URL port is unknown")?;
    let addrs = (host, port).to_socket_addrs()?.collect::<Vec<_>>();
    if addrs.is_empty() {
        return Err("webhook URL did not resolve".into());
    }
    for addr in &addrs {
        if is_blocked_webhook_ip(addr.ip()) {
            return Err("webhook URL resolves to a blocked address".into());
        }
    }
    Ok(ResolvedWebhookTarget {
        host: host.to_string(),
        addrs,
    })
}

fn is_blocked_webhook_ip(ip: IpAddr) -> bool {
    WebhookOutboundPolicy::from_env()
        .map(|policy| policy.blocks(ip))
        .unwrap_or(true)
}

#[derive(Clone, Debug, Default)]
struct WebhookOutboundPolicy {
    allow_cidrs: Vec<IpCidr>,
    deny_cidrs: Vec<IpCidr>,
}

impl WebhookOutboundPolicy {
    fn from_env() -> Result<Self, String> {
        Ok(Self {
            allow_cidrs: parse_cidr_env(WEBHOOK_ALLOW_CIDRS_ENV)?,
            deny_cidrs: parse_cidr_env(WEBHOOK_DENY_CIDRS_ENV)?,
        })
    }

    fn blocks(&self, ip: IpAddr) -> bool {
        if self.deny_cidrs.iter().any(|cidr| cidr.contains(ip)) {
            return true;
        }
        default_blocks_webhook_ip(ip) && !self.allow_cidrs.iter().any(|cidr| cidr.contains(ip))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct IpCidr {
    network: IpAddr,
    prefix: u8,
}

impl IpCidr {
    fn parse(value: &str) -> Result<Self, String> {
        let (addr, prefix) = value
            .split_once('/')
            .ok_or_else(|| format!("CIDR {value:?} must include a prefix length"))?;
        let network = addr
            .parse::<IpAddr>()
            .map_err(|error| format!("CIDR {value:?} has invalid address: {error}"))?;
        let prefix = prefix
            .parse::<u8>()
            .map_err(|error| format!("CIDR {value:?} has invalid prefix: {error}"))?;
        let max_prefix = match network {
            IpAddr::V4(_) => 32,
            IpAddr::V6(_) => 128,
        };
        if prefix > max_prefix {
            return Err(format!("CIDR {value:?} prefix exceeds {max_prefix}"));
        }
        Ok(Self { network, prefix })
    }

    fn contains(&self, ip: IpAddr) -> bool {
        match (self.network, ip) {
            (IpAddr::V4(network), IpAddr::V4(ip)) => {
                let network = u32::from(network);
                let ip = u32::from(ip);
                self.prefix == 0 || network >> (32 - self.prefix) == ip >> (32 - self.prefix)
            }
            (IpAddr::V6(network), IpAddr::V6(ip)) => {
                let network = u128::from_be_bytes(network.octets());
                let ip = u128::from_be_bytes(ip.octets());
                self.prefix == 0 || network >> (128 - self.prefix) == ip >> (128 - self.prefix)
            }
            _ => false,
        }
    }
}

fn parse_cidr_env(name: &str) -> Result<Vec<IpCidr>, String> {
    let Ok(value) = env::var(name) else {
        return Ok(Vec::new());
    };
    value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(IpCidr::parse)
        .collect()
}

fn default_blocks_webhook_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            ip.is_private()
                || ip.is_loopback()
                || ip.is_link_local()
                || ip.is_broadcast()
                || ip.is_documentation()
                || ip.octets()[0] == 0
                || ip.octets()[0] >= 224
        }
        IpAddr::V6(ip) => {
            if let Some(v4) = ip.to_ipv4_mapped().or_else(|| ip.to_ipv4()) {
                return is_blocked_webhook_ip(IpAddr::V4(v4));
            }
            let segments = ip.segments();
            if segments[0] == 0x2002 || (segments[0] == 0x2001 && segments[1] == 0) {
                return true;
            }
            ip.is_loopback()
                || ip.is_unspecified()
                || ip.is_multicast()
                || (segments[0] == 0x2001 && segments[1] == 0x0db8)
                || (segments[0] & 0xfe00) == 0xfc00
                || (segments[0] & 0xffc0) == 0xfe80
        }
    }
}

fn sanitized_webhook_url_for_log(url: &str) -> String {
    match reqwest::Url::parse(url) {
        Ok(parsed) => {
            let host = parsed.host_str().unwrap_or("<unknown>");
            let port = parsed
                .port()
                .map(|port| format!(":{port}"))
                .unwrap_or_default();
            format!("{}://{}{}{}", parsed.scheme(), host, port, parsed.path())
        }
        Err(_) => "<invalid webhook url>".to_string(),
    }
}

/// Webhook retry scheduler for failed deliveries
pub struct WebhookRetryScheduler;

impl WebhookRetryScheduler {
    /// Start background retry scheduler
    #[allow(dead_code)]
    pub fn start(
        _db: Option<std::sync::Arc<crate::persistence::DatabaseManager>>,
        _manager: std::sync::Arc<tokio::sync::RwLock<WebhookManager>>,
    ) {
        // Background task for retrying failed webhooks
        // In production, this would be wired to the DatabaseManager
        tokio::spawn(async {
            // Retry scheduler would run periodically (every 5 minutes)
            // and attempt to deliver failed webhook payloads with exponential backoff
        });
    }

    /// Calculate exponential backoff delay
    #[allow(dead_code)]
    fn calculate_backoff(attempt: u32) -> std::time::Duration {
        // 30s, 60s, 120s, 240s, 480s (max)
        let seconds = 30 * 2_u64.saturating_pow(attempt);
        std::time::Duration::from_secs(seconds.min(480))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webhook_event_display() {
        assert_eq!(WebhookEvent::SearchCreated.to_string(), "search.created");
        assert_eq!(
            WebhookEvent::TransferStarted.to_string(),
            "transfer.started"
        );
        assert_eq!(WebhookEvent::MessageSent.to_string(), "message.sent");
    }

    #[test]
    fn test_webhook_creation() {
        let secret = Webhook::generate_secret();
        validate_webhook_secret(&secret).expect("generated secret is strong enough");
        let webhook = Webhook::new(
            "http://example.com/hook".to_string(),
            vec![WebhookEvent::SearchCreated, WebhookEvent::TransferStarted],
            secret,
        );

        assert!(webhook.id.starts_with("hook_"));
        assert_ne!(webhook.id, "hook_0");
        assert_eq!(webhook.url, "http://example.com/hook");
        assert!(webhook.active);
        assert_eq!(webhook.max_retries, 3);
    }

    #[test]
    fn test_webhook_secret_validation() {
        assert!(validate_webhook_secret("short").is_err());
        assert!(validate_webhook_secret("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").is_err());
        assert!(validate_webhook_secret("abcdefghij0123456789ABCDEFGHIJ!!").is_ok());
        assert!(validate_webhook_secret("abcdefghij0123456789\nABCDEFGHIJ!!").is_err());
    }

    #[test]
    fn test_webhook_event_handling() {
        let secret = Webhook::generate_secret();
        let webhook = Webhook::new(
            "http://example.com/hook".to_string(),
            vec![WebhookEvent::SearchCreated],
            secret,
        );

        assert!(webhook.handles_event(WebhookEvent::SearchCreated));
        assert!(!webhook.handles_event(WebhookEvent::TransferStarted));
    }

    #[test]
    fn test_webhook_payload_creation() {
        let payload = WebhookPayload::new(
            WebhookEvent::SearchCreated,
            "corr-123".to_string(),
            serde_json::json!({"query": "test"}),
        );

        assert!(payload.id.starts_with("evt_"));
        assert_ne!(payload.id, "evt_0");
        assert_eq!(payload.event, "search.created");
        assert_eq!(payload.correlation_id, "corr-123");
    }

    #[test]
    fn test_webhook_signature_creation_and_verification() {
        let secret = "test-secret";
        let payload = b"test payload";

        let sig = WebhookSignature::create(payload, secret).unwrap();
        assert!(sig.verify(payload, secret).unwrap());
        assert!(!sig.verify(b"different", secret).unwrap());
    }

    #[test]
    fn test_webhook_signature_header_format() {
        let secret = "test-secret";
        let payload = b"test payload";

        let sig = WebhookSignature::create(payload, secret).unwrap();
        let header = sig.as_header();

        assert!(header.contains("t="));
        assert_eq!(header.split(", ").count(), 2);
    }

    #[test]
    fn test_webhook_manager() {
        let mut manager = WebhookManager::new();
        let secret = Webhook::generate_secret();

        let webhook = Webhook::new(
            "http://example.com/hook".to_string(),
            vec![WebhookEvent::SearchCreated],
            secret,
        );

        let id = webhook.id.clone();
        manager.register(webhook).expect("register webhook");

        assert!(manager.get(&id).is_some());
        assert_eq!(manager.list().len(), 1);

        let results = manager.get_for_event(WebhookEvent::SearchCreated);
        assert_eq!(results.len(), 1);

        manager.unregister(&id);
        assert!(manager.get(&id).is_none());
    }

    #[test]
    fn test_constant_time_compare() {
        let a = b"test";
        let b_same = b"test";
        let b_diff = b"different";

        assert!(constant_time_compare(a, b_same));
        assert!(!constant_time_compare(a, b_diff));
        assert!(!constant_time_compare(a, b"te")); // Different length
    }

    #[test]
    fn test_blocked_webhook_ipv6_embedded_ipv4() {
        assert!(is_blocked_webhook_ip("::ffff:127.0.0.1".parse().unwrap()));
        assert!(is_blocked_webhook_ip(
            "::ffff:192.168.1.10".parse().unwrap()
        ));
        assert!(is_blocked_webhook_ip("2002:c0a8:0101::1".parse().unwrap()));
        assert!(is_blocked_webhook_ip(
            "2001:0000:4136:e378::1".parse().unwrap()
        ));
        assert!(is_blocked_webhook_ip("2001:db8::1".parse().unwrap()));
        assert!(is_blocked_webhook_ip("ff02::1".parse().unwrap()));
    }

    #[test]
    fn test_webhook_outbound_policy_covers_operator_cidrs() {
        let policy = WebhookOutboundPolicy {
            allow_cidrs: vec![IpCidr::parse("10.42.0.0/16").unwrap()],
            deny_cidrs: vec![IpCidr::parse("93.184.216.0/24").unwrap()],
        };

        assert!(!policy.blocks("10.42.1.5".parse().unwrap()));
        assert!(policy.blocks("10.43.1.5".parse().unwrap()));
        assert!(policy.blocks("93.184.216.34".parse().unwrap()));
        assert!(!policy.blocks("93.184.217.34".parse().unwrap()));
        assert!(policy.blocks("2001:db8::42".parse().unwrap()));
        assert!(policy.blocks("ff02::1".parse().unwrap()));
    }

    #[test]
    fn test_webhook_cidr_parser_matches_ipv4_and_ipv6_prefixes() {
        let v4 = IpCidr::parse("198.51.100.0/24").unwrap();
        assert!(v4.contains("198.51.100.23".parse().unwrap()));
        assert!(!v4.contains("198.51.101.23".parse().unwrap()));

        let v6 = IpCidr::parse("2001:db8:abcd::/48").unwrap();
        assert!(v6.contains("2001:db8:abcd::1".parse().unwrap()));
        assert!(!v6.contains("2001:db8:abce::1".parse().unwrap()));

        assert!(IpCidr::parse("10.0.0.0/33").is_err());
        assert!(IpCidr::parse("2001:db8::/129").is_err());
    }

    #[test]
    fn test_webhook_registration_url_validation() {
        assert!(validate_webhook_url_for_registration("https://example.com/hook").is_ok());
        assert!(validate_webhook_url_for_registration("ftp://example.com/hook").is_err());
        assert!(validate_webhook_url_for_registration("http://localhost/hook").is_err());
        assert!(validate_webhook_url_for_registration("http://127.0.0.1/hook").is_err());
        assert!(validate_webhook_url_for_registration("http://10.0.0.5/hook").is_err());
        assert!(validate_webhook_url_for_registration("http://169.254.169.254/hook").is_err());
    }

    #[test]
    fn test_sanitized_webhook_url_omits_query() {
        assert_eq!(
            sanitized_webhook_url_for_log("https://example.com/hook/path?token=secret"),
            "https://example.com/hook/path"
        );
    }

    #[test]
    fn test_stale_webhook_signature_header_rejected() {
        let old = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
            - 600;
        let err = WebhookSignature::from_header(&format!("t={old}, abc")).unwrap_err();
        assert!(err.to_string().contains("stale"), "{err}");
    }
}
