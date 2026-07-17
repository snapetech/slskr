use hmac::{Hmac, KeyInit, Mac};
use rand::{rngs::SysRng, TryRng};
/// Webhook support with HMAC-SHA256 request signing
///
/// Allows configuring webhooks that are triggered on API events,
/// with cryptographic signing for security and verification.
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fmt;
use std::future::Future;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use uuid::Uuid;

use crate::persistence::DatabaseManager;
use crate::utils::{is_blocked_outbound_ipv4, is_non_global_special_use_ipv6, nat64_embedded_ipv4};

const WEBHOOK_MIN_TIMEOUT_SECONDS: u32 = 1;
const WEBHOOK_MAX_TIMEOUT_SECONDS: u32 = 30;
pub const MAX_WEBHOOKS: usize = 64;
pub const MIN_WEBHOOK_SECRET_BYTES: usize = 32;
pub const MAX_WEBHOOK_SECRET_BYTES: usize = 4 * 1024;
const MAX_WEBHOOK_URL_BYTES: usize = 2_048;
const MAX_WEBHOOK_ID_BYTES: usize = 128;
const MAX_WEBHOOK_EVENTS: usize = 14;
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
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
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
    pub fn generate_secret() -> Option<String> {
        Self::generate_secret_with(|secret| SysRng.try_fill_bytes(secret).is_ok())
    }

    fn generate_secret_with(fill: impl FnOnce(&mut [u8; 32]) -> bool) -> Option<String> {
        let mut secret = [0_u8; 32];
        fill(&mut secret).then(|| format!("secret_{}", hex::encode(secret)))
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
    if secret.len() > MAX_WEBHOOK_SECRET_BYTES {
        return Err("webhook secret is too long");
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
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;
        let signature = sign_webhook_payload(payload, secret, timestamp)?;

        Ok(WebhookSignature {
            signature,
            timestamp,
            algorithm: "hmac-sha256".to_string(),
        })
    }

    /// Verify signature
    pub fn verify(&self, payload: &[u8], secret: &str) -> Result<bool, Box<dyn std::error::Error>> {
        if !webhook_timestamp_is_fresh(self.timestamp)? {
            return Ok(false);
        }
        let expected = sign_webhook_payload(payload, secret, self.timestamp)?;

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
        if parts.len() != 2 {
            return Err("Invalid signature header format".into());
        }

        let timestamp_part = parts[0];
        let sig_part = parts[1];

        let timestamp = timestamp_part
            .strip_prefix("t=")
            .ok_or("Missing timestamp")?
            .parse::<i64>()?;
        if !webhook_timestamp_is_fresh(timestamp)? {
            return Err("signature timestamp is stale".into());
        }

        Ok(WebhookSignature {
            signature: sig_part.to_string(),
            timestamp,
            algorithm: "hmac-sha256".to_string(),
        })
    }
}

fn sign_webhook_payload(
    payload: &[u8],
    secret: &str,
    timestamp: i64,
) -> Result<String, hmac::digest::InvalidLength> {
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())?;
    mac.update(timestamp.to_string().as_bytes());
    mac.update(b".");
    mac.update(payload);
    Ok(hex::encode(mac.finalize().into_bytes()))
}

fn webhook_timestamp_is_fresh(timestamp: i64) -> Result<bool, std::time::SystemTimeError> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as i64;
    Ok(now.abs_diff(timestamp) <= WebhookSignature::MAX_TIMESTAMP_AGE_SECONDS as u64)
}

/// Webhook manager
#[derive(Debug, Clone, PartialEq, Eq)]
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
        for webhook in webhooks
            .into_iter()
            .filter(webhook_definition_is_valid)
            .take(MAX_WEBHOOKS)
        {
            manager.webhooks.insert(webhook.id.clone(), webhook);
        }
        manager
    }

    /// Register webhook
    pub fn register(&mut self, webhook: Webhook) -> Result<String, ()> {
        if self.webhooks.len() >= MAX_WEBHOOKS || !webhook_definition_is_valid(&webhook) {
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
        database: Option<DatabaseManager>,
        correlation_id: String,
        event: WebhookEvent,
        data: serde_json::Value,
    ) {
        let webhooks = manager.get_for_event(event);

        if webhooks.is_empty() {
            return;
        }

        let payload = WebhookPayload::new(event, correlation_id.clone(), data);
        let payload_json = payload.to_string().unwrap_or_default();

        for webhook in webhooks {
            let Ok(delivery_permit) = Arc::clone(&deliveries).try_acquire_owned() else {
                eprintln!(
                    "[WEBHOOK] Dropped delivery to {} because the delivery pool is full",
                    sanitized_webhook_url_for_log(&webhook.url)
                );
                if let Some(database) = database.as_ref() {
                    if let Err(error) = database
                        .complete_webhook_logs(
                            &webhook.id,
                            &correlation_id,
                            "failed",
                            Some("webhook delivery pool is full"),
                        )
                        .await
                    {
                        eprintln!("[WEBHOOK] Failed to persist dropped delivery outcome: {error}");
                    }
                }
                continue;
            };
            // Spawn async task for each webhook delivery (no blocking)
            let webhook_url = webhook.url.clone();
            let webhook_secret = webhook.secret.clone();
            let webhook_timeout = webhook.timeout_seconds;
            let webhook_id = webhook.id.clone();
            let payload_clone = payload_json.clone();
            let database = database.clone();
            let correlation_id = correlation_id.clone();

            tokio::spawn(async move {
                let _delivery_permit = delivery_permit;
                let (status, error_message) = match Self::send_webhook(
                    &webhook_url,
                    &webhook_secret,
                    &payload_clone,
                    webhook_timeout,
                )
                .await
                {
                    Ok(()) => ("success", None),
                    Err(error) => {
                        let error = sanitized_webhook_delivery_error(&error.to_string());
                        eprintln!(
                            "[WEBHOOK] Delivery to {} failed: {error}",
                            sanitized_webhook_url_for_log(&webhook_url)
                        );
                        ("failed", Some(error))
                    }
                };
                if let Some(database) = database {
                    if let Err(error) = database
                        .complete_webhook_logs(
                            &webhook_id,
                            &correlation_id,
                            status,
                            error_message.as_deref(),
                        )
                        .await
                    {
                        eprintln!("[WEBHOOK] Failed to persist delivery outcome: {error}");
                    }
                }
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

        let timeout = timeout_secs.clamp(WEBHOOK_MIN_TIMEOUT_SECONDS, WEBHOOK_MAX_TIMEOUT_SECONDS);
        let request_timeout = Duration::from_secs(timeout as u64);
        let resolved = validate_and_resolve_webhook_url(url, request_timeout).await?;

        // Disable redirects so validation cannot be bypassed after the initial URL check.
        let mut client_builder = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy();
        for addr in &resolved.addrs {
            client_builder = client_builder.resolve(&resolved.host, *addr);
        }
        let client = client_builder.build()?;

        let response = client
            .post(url)
            .header("X-Webhook-Signature", sig.as_header())
            .header("X-Webhook-Event", "webhook")
            .header("Content-Type", "application/json")
            .body(payload.to_string())
            .timeout(request_timeout)
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

fn sanitized_webhook_delivery_error(error: &str) -> String {
    if error.starts_with("webhook delivery failed with status ") {
        error.to_owned()
    } else {
        "webhook delivery request failed".to_owned()
    }
}

struct ResolvedWebhookTarget {
    host: String,
    addrs: Vec<SocketAddr>,
}

pub(crate) fn validate_webhook_url_for_registration(
    url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if url.len() > MAX_WEBHOOK_URL_BYTES {
        return Err("webhook URL is too long".into());
    }
    let parsed = reqwest::Url::parse(url)?;
    match parsed.scheme() {
        "http" | "https" => {}
        _ => return Err("webhook URL scheme must be http or https".into()),
    }
    let Some(host) = parsed.host_str() else {
        return Err("webhook URL must include a host".into());
    };
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err("webhook URL must not contain embedded credentials".into());
    }
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

fn webhook_definition_is_valid(webhook: &Webhook) -> bool {
    !webhook.id.trim().is_empty()
        && webhook.id.len() <= MAX_WEBHOOK_ID_BYTES
        && !webhook.events.is_empty()
        && webhook.events.len() <= MAX_WEBHOOK_EVENTS
        && validate_webhook_url_for_registration(&webhook.url).is_ok()
        && validate_webhook_secret(&webhook.secret).is_ok()
}

async fn validate_and_resolve_webhook_url(
    url: &str,
    timeout: Duration,
) -> Result<ResolvedWebhookTarget, Box<dyn std::error::Error>> {
    validate_webhook_url_for_registration(url)?;
    let parsed = reqwest::Url::parse(url)?;
    let host = parsed.host_str().ok_or("webhook URL must include a host")?;

    let port = parsed
        .port_or_known_default()
        .ok_or("webhook URL port is unknown")?;
    let addrs = resolve_webhook_addrs(
        async move {
            tokio::net::lookup_host((host, port))
                .await
                .map(|addrs| addrs.collect())
        },
        timeout,
    )
    .await?;
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

async fn resolve_webhook_addrs<F>(
    resolution: F,
    timeout: Duration,
) -> Result<Vec<SocketAddr>, Box<dyn std::error::Error>>
where
    F: Future<Output = std::io::Result<Vec<SocketAddr>>>,
{
    tokio::time::timeout(timeout, resolution)
        .await
        .map_err(|_| "webhook DNS resolution timed out")?
        .map_err(Into::into)
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
        IpAddr::V4(ip) => is_blocked_outbound_ipv4(ip),
        IpAddr::V6(ip) => {
            if let Some(v4) = ip.to_ipv4_mapped().or_else(|| ip.to_ipv4()) {
                return is_blocked_webhook_ip(IpAddr::V4(v4));
            }
            if let Some(v4) = nat64_embedded_ipv4(ip) {
                return is_blocked_outbound_ipv4(v4);
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
                || is_non_global_special_use_ipv6(ip)
        }
    }
}

fn sanitized_webhook_url_for_log(url: &str) -> String {
    match reqwest::Url::parse(url) {
        Ok(parsed) => parsed.origin().ascii_serialization(),
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
        let seconds = 30_u64.saturating_mul(2_u64.saturating_pow(attempt));
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
        let secret = Webhook::generate_secret().expect("test randomness");
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
    fn webhook_secret_generation_fails_closed_when_randomness_is_unavailable() {
        assert!(Webhook::generate_secret_with(|_| false).is_none());

        let secret = Webhook::generate_secret_with(|bytes| {
            bytes.fill(0xcd);
            true
        })
        .expect("deterministic randomness fixture");
        assert_eq!(secret, format!("secret_{}", "cd".repeat(32)));
    }

    #[test]
    fn test_webhook_secret_validation() {
        assert!(validate_webhook_secret("short").is_err());
        assert!(validate_webhook_secret("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").is_err());
        assert!(validate_webhook_secret("abcdefghij0123456789ABCDEFGHIJ!!").is_ok());
        assert!(validate_webhook_secret("abcdefghij0123456789\nABCDEFGHIJ!!").is_err());
        assert!(validate_webhook_secret(&"abcdefgh".repeat(MAX_WEBHOOK_SECRET_BYTES / 8)).is_ok());
        assert!(validate_webhook_secret(&format!(
            "{}x",
            "abcdefgh".repeat(MAX_WEBHOOK_SECRET_BYTES / 8)
        ))
        .is_err());
    }

    #[test]
    fn test_webhook_event_handling() {
        let secret = Webhook::generate_secret().expect("test randomness");
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
    fn test_webhook_signature_authenticates_timestamp() {
        let secret = "test-secret";
        let payload = b"test payload";
        let mut signature = WebhookSignature::create(payload, secret).unwrap();

        signature.timestamp += 1;
        assert!(!signature.verify(payload, secret).unwrap());
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
        let secret = Webhook::generate_secret().expect("test randomness");

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
    fn webhook_manager_rejects_invalid_runtime_and_persisted_definitions() {
        let valid = Webhook::new(
            "https://example.com/hook".to_owned(),
            vec![WebhookEvent::SearchCreated],
            Webhook::generate_secret().expect("test randomness"),
        );
        let mut invalid = valid.clone();
        invalid.secret = "x".repeat(MAX_WEBHOOK_SECRET_BYTES + 1);

        let mut manager = WebhookManager::new();
        assert!(manager.register(invalid.clone()).is_err());
        assert!(manager.get_all().is_empty());

        let mut persisted = vec![invalid; MAX_WEBHOOKS];
        let valid_id = valid.id.clone();
        persisted.push(valid);
        let manager = WebhookManager::from_webhooks(persisted);
        assert_eq!(manager.get_all().len(), 1);
        assert!(manager.get(&valid_id).is_some());
    }

    #[tokio::test]
    async fn dispatch_does_not_spawn_when_delivery_pool_is_full() {
        let mut manager = WebhookManager::new();
        let webhook = Webhook::new(
            "https://example.com/hook".to_owned(),
            vec![WebhookEvent::SearchCreated],
            Webhook::generate_secret().expect("test randomness"),
        );
        let webhook_id = webhook.id.clone();
        manager.register(webhook.clone()).expect("register webhook");
        let database = DatabaseManager::in_memory().await.expect("in-memory db");
        database
            .insert_webhook(&crate::persistence::WebhookRecord {
                id: webhook_id.clone(),
                url: webhook.url.clone(),
                events: WebhookEvent::SearchCreated.to_string(),
                secret: webhook.secret.clone(),
                active: true,
                created_at: 1,
                last_triggered: None,
                retry_count: 0,
                max_retries: 3,
                timeout_seconds: 30,
            })
            .await
            .expect("persist webhook");
        database
            .insert_webhook_log(&crate::persistence::WebhookLogRecord {
                id: "log_pool_full".to_owned(),
                webhook_id: webhook_id.clone(),
                event: WebhookEvent::SearchCreated.to_string(),
                correlation_id: "correlation".to_owned(),
                status: "queued".to_owned(),
                request_body: "{}".to_owned(),
                response_status: None,
                response_body: None,
                error_message: None,
                attempt: 1,
                timestamp: 1,
            })
            .await
            .expect("persist queued log");
        let deliveries = Arc::new(Semaphore::new(0));

        WebhookDispatcher::dispatch(
            &manager,
            Arc::clone(&deliveries),
            Some(database.clone()),
            "correlation".to_owned(),
            WebhookEvent::SearchCreated,
            serde_json::json!({"query": "bounded"}),
        )
        .await;

        assert_eq!(Arc::strong_count(&deliveries), 1);
        assert_eq!(deliveries.available_permits(), 0);
        let logs = database
            .get_webhook_logs(&webhook_id, 10, 0)
            .await
            .expect("read delivery log");
        assert_eq!(logs[0].status, "failed");
        assert_eq!(
            logs[0].error_message.as_deref(),
            Some("webhook delivery pool is full")
        );
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
    fn test_blocked_webhook_special_use_ip_ranges() {
        for address in ["100.64.0.1", "192.0.0.8", "192.88.99.1", "198.18.0.1"] {
            assert!(is_blocked_webhook_ip(address.parse().unwrap()));
        }
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
        for address in [
            "64:ff9b::7f00:1",
            "64:ff9b:1::1",
            "100::1",
            "2001:2::1",
            "2001:10::1",
            "2001:20::1",
        ] {
            assert!(is_blocked_webhook_ip(address.parse().unwrap()));
        }
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
        assert!(validate_webhook_url_for_registration(&format!(
            "https://example.com/{}",
            "x".repeat(MAX_WEBHOOK_URL_BYTES)
        ))
        .is_err());
        assert!(
            validate_webhook_url_for_registration("https://operator:secret@example.com/hook")
                .is_err()
        );
        assert!(validate_webhook_url_for_registration("ftp://example.com/hook").is_err());
        assert!(validate_webhook_url_for_registration("http://localhost/hook").is_err());
        assert!(validate_webhook_url_for_registration("http://127.0.0.1/hook").is_err());
        assert!(validate_webhook_url_for_registration("http://10.0.0.5/hook").is_err());
        assert!(validate_webhook_url_for_registration("http://169.254.169.254/hook").is_err());
    }

    #[tokio::test]
    async fn webhook_dns_resolution_is_bounded() {
        let error = resolve_webhook_addrs(
            std::future::pending::<std::io::Result<Vec<SocketAddr>>>(),
            Duration::ZERO,
        )
        .await
        .unwrap_err();
        assert_eq!(error.to_string(), "webhook DNS resolution timed out");
    }

    #[test]
    fn test_webhook_retry_backoff_saturates_before_overflow() {
        for (attempt, seconds) in [(0, 30), (1, 60), (4, 480), (5, 480), (u32::MAX, 480)] {
            assert_eq!(
                WebhookRetryScheduler::calculate_backoff(attempt),
                std::time::Duration::from_secs(seconds)
            );
        }
    }

    #[test]
    fn test_sanitized_webhook_url_omits_secret_path_and_query() {
        assert_eq!(
            sanitized_webhook_url_for_log(
                "https://example.com/services/secret-path?token=secret-query"
            ),
            "https://example.com"
        );
        assert_eq!(
            sanitized_webhook_delivery_error(
                "request failed for https://example.com/hook?token=secret-query"
            ),
            "webhook delivery request failed"
        );
        assert_eq!(
            sanitized_webhook_delivery_error(
                "webhook delivery failed with status 503 Service Unavailable"
            ),
            "webhook delivery failed with status 503 Service Unavailable"
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

        let err = WebhookSignature::from_header(&format!("t={}, abc", i64::MIN)).unwrap_err();
        assert!(err.to_string().contains("stale"), "{err}");
    }
}
