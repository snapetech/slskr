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
use tokio::sync::{RwLock, Semaphore};
use tokio_rustls::rustls;
use uuid::Uuid;

use super::{logging, record_daemon_log, scripts, unix_timestamp, update_session, AppState};
use crate::persistence::DatabaseManager;
use crate::utils::{is_blocked_outbound_ipv4, is_blocked_outbound_ipv6};

const WEBHOOK_MIN_TIMEOUT_SECONDS: u32 = 1;
const WEBHOOK_MAX_TIMEOUT_SECONDS: u32 = 30;
const WEBHOOK_MAX_RETRIES: u32 = 8;
pub const MAX_WEBHOOKS: usize = 64;
pub const MIN_WEBHOOK_SECRET_BYTES: usize = 32;
pub const MAX_WEBHOOK_SECRET_BYTES: usize = 4 * 1024;
const MAX_WEBHOOK_URL_BYTES: usize = 2_048;
const MAX_WEBHOOK_ID_BYTES: usize = 128;
pub const MAX_WEBHOOK_EVENTS: usize = 14;
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

fn bounded_webhook_attempts(attempts: u32) -> u32 {
    attempts.clamp(1, WEBHOOK_MAX_RETRIES.saturating_add(1))
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
        for webhook in webhooks.into_iter().filter(webhook_definition_is_valid) {
            let id = webhook.id.clone();
            if manager.webhooks.len() >= MAX_WEBHOOKS && !manager.webhooks.contains_key(&id) {
                continue;
            }
            manager.webhooks.insert(id, webhook);
        }
        manager
    }

    /// Register webhook
    pub fn register(&mut self, webhook: Webhook) -> Result<String, ()> {
        if !webhook_definition_is_valid(&webhook) {
            return Err(());
        }
        let id = webhook.id.clone();
        if self.webhooks.len() >= MAX_WEBHOOKS && !self.webhooks.contains_key(&id) {
            return Err(());
        }
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

#[derive(Clone)]
pub(crate) struct WebhookDispatchContext {
    pub(crate) manager: Arc<RwLock<WebhookManager>>,
    pub(crate) deliveries: Arc<Semaphore>,
    pub(crate) persistence_turn: Arc<tokio::sync::Mutex<()>>,
    pub(crate) database: Option<DatabaseManager>,
}

#[derive(Debug)]
struct SelfIssuedWebhookVerifier {
    standard: Arc<rustls::client::WebPkiServerVerifier>,
    provider: Arc<rustls::crypto::CryptoProvider>,
}

fn der_tlv<'a>(input: &'a [u8], offset: &mut usize) -> Option<(u8, &'a [u8], &'a [u8])> {
    let start = *offset;
    let tag = *input.get(*offset)?;
    *offset += 1;
    let first = *input.get(*offset)?;
    *offset += 1;
    let length = if first & 0x80 == 0 {
        usize::from(first)
    } else {
        let count = usize::from(first & 0x7f);
        if count == 0 || count > std::mem::size_of::<usize>() {
            return None;
        }
        let mut length = 0_usize;
        for _ in 0..count {
            length = length
                .checked_mul(256)?
                .checked_add(usize::from(*input.get(*offset)?))?;
            *offset += 1;
        }
        length
    };
    let content_start = *offset;
    let end = content_start.checked_add(length)?;
    let content = input.get(content_start..end)?;
    *offset = end;
    Some((tag, input.get(start..end)?, content))
}

fn certificate_is_self_issued(der: &[u8]) -> bool {
    let mut offset = 0;
    let Some((0x30, _, certificate)) = der_tlv(der, &mut offset) else {
        return false;
    };
    let mut certificate_offset = 0;
    let Some((0x30, _, tbs)) = der_tlv(certificate, &mut certificate_offset) else {
        return false;
    };
    let mut tbs_offset = 0;
    if tbs.first() == Some(&0xa0) && der_tlv(tbs, &mut tbs_offset).is_none() {
        return false;
    }
    if der_tlv(tbs, &mut tbs_offset).is_none() || der_tlv(tbs, &mut tbs_offset).is_none() {
        return false;
    }
    let Some((_, issuer, _)) = der_tlv(tbs, &mut tbs_offset) else {
        return false;
    };
    if der_tlv(tbs, &mut tbs_offset).is_none() {
        return false;
    }
    let Some((_, subject, _)) = der_tlv(tbs, &mut tbs_offset) else {
        return false;
    };
    issuer == subject
}

impl rustls::client::danger::ServerCertVerifier for SelfIssuedWebhookVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &rustls::pki_types::CertificateDer<'_>,
        intermediates: &[rustls::pki_types::CertificateDer<'_>],
        server_name: &rustls::pki_types::ServerName<'_>,
        ocsp_response: &[u8],
        now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        match self.standard.verify_server_cert(
            end_entity,
            intermediates,
            server_name,
            ocsp_response,
            now,
        ) {
            Ok(valid) => Ok(valid),
            Err(original)
                if intermediates.is_empty() && certificate_is_self_issued(end_entity.as_ref()) =>
            {
                let mut roots = rustls::RootCertStore::empty();
                if roots.add(end_entity.clone()).is_err() {
                    return Err(original);
                }
                let verifier = match rustls::client::WebPkiServerVerifier::builder_with_provider(
                    Arc::new(roots),
                    Arc::clone(&self.provider),
                )
                .build()
                {
                    Ok(verifier) => verifier,
                    Err(_) => return Err(original),
                };
                verifier.verify_server_cert(
                    end_entity,
                    intermediates,
                    server_name,
                    ocsp_response,
                    now,
                )
            }
            Err(error) => Err(error),
        }
    }
    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        self.standard.verify_tls12_signature(message, cert, dss)
    }
    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        self.standard.verify_tls13_signature(message, cert, dss)
    }
    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.standard.supported_verify_schemes()
    }
}

pub(crate) fn self_issued_tls_config() -> Result<rustls::ClientConfig, Box<dyn std::error::Error>> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let roots = rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let standard = rustls::client::WebPkiServerVerifier::builder_with_provider(
        Arc::new(roots),
        Arc::clone(&provider),
    )
    .build()?;
    Ok(
        rustls::ClientConfig::builder_with_provider(Arc::clone(&provider))
            .with_safe_default_protocol_versions()?
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(SelfIssuedWebhookVerifier {
                standard,
                provider,
            }))
            .with_no_client_auth(),
    )
}

impl WebhookDispatcher {
    pub async fn send_frozen_compat_webhook(
        url: &str,
        headers: &[(String, String)],
        payload: &str,
        timeout_millis: u64,
        attempts: u32,
        ignore_certificate_errors: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let timeout = Duration::from_millis(timeout_millis.max(500));
        let resolved = validate_and_resolve_webhook_url(url, timeout).await?;
        Self::send_frozen_compat_webhook_resolved(
            url,
            headers,
            payload,
            timeout,
            attempts,
            ignore_certificate_errors,
            &resolved,
        )
        .await
    }

    async fn send_frozen_compat_webhook_resolved(
        url: &str,
        headers: &[(String, String)],
        payload: &str,
        timeout: Duration,
        attempts: u32,
        ignore_certificate_errors: bool,
        resolved: &ResolvedWebhookTarget,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut client_builder = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .resolve_to_addrs(&resolved.host, &resolved.addrs);
        if ignore_certificate_errors {
            client_builder = client_builder.use_preconfigured_tls(self_issued_tls_config()?);
        }
        let client = client_builder.build()?;
        let attempts = bounded_webhook_attempts(attempts);
        let mut last_error = "webhook delivery failed".to_owned();
        for attempt in 1..=attempts {
            if attempt > 1 {
                let completed_failures = attempt - 1;
                let delay = (((1_u64 << completed_failures.min(16)) - 1) / 2)
                    .saturating_mul(1_000)
                    .min(30_000);
                if delay > 0 {
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                }
            }
            let mut request = client.post(url).header("Content-Type", "application/json");
            for (name, value) in headers {
                request = request.header(name, value);
            }
            match request
                .body(payload.to_owned())
                .timeout(timeout)
                .send()
                .await
            {
                Ok(response) if response.status().is_success() => return Ok(()),
                Ok(response) => {
                    last_error =
                        format!("webhook delivery failed with status {}", response.status())
                }
                Err(_) => last_error = "webhook delivery request failed".to_owned(),
            }
        }
        Err(last_error.into())
    }

    /// Dispatch event to all matching webhooks
    pub async fn dispatch(
        tasks: &crate::managed_tasks::ManagedTaskRegistry,
        context: WebhookDispatchContext,
        eligible_webhooks: Vec<Webhook>,
        correlation_id: String,
        event: WebhookEvent,
        data: serde_json::Value,
    ) {
        let WebhookDispatchContext {
            manager,
            deliveries,
            persistence_turn,
            database,
        } = context;
        let webhooks = eligible_webhooks;

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
                Self::record_delivery_stats(
                    &manager,
                    database.as_ref(),
                    webhook.clone(),
                    0,
                    Arc::clone(&persistence_turn),
                )
                .await;
                continue;
            };
            // Spawn async task for each webhook delivery (no blocking)
            let webhook_url = webhook.url.clone();
            let webhook_secret = webhook.secret.clone();
            let webhook_timeout = webhook.timeout_seconds;
            let webhook_max_retries = webhook.max_retries;
            let webhook_id = webhook.id.clone();
            let stats_webhook = webhook.clone();
            let payload_clone = payload_json.clone();
            let job_database = database.clone();
            let job_correlation_id = correlation_id.clone();
            let job_manager = Arc::clone(&manager);
            let job_persistence_turn = Arc::clone(&persistence_turn);

            if !tasks.try_spawn(async move {
                let _delivery_permit = delivery_permit;
                let (status, retry_count, error_message) = match Self::send_webhook_with_retries(
                    &webhook_url,
                    &webhook_secret,
                    &payload_clone,
                    webhook_timeout,
                    webhook_max_retries,
                )
                .await
                {
                    Ok(retry_count) => ("success", retry_count, None),
                    Err(error) => {
                        let error = sanitized_webhook_delivery_error(&error.to_string());
                        eprintln!(
                            "[WEBHOOK] Delivery to {} failed: {error}",
                            sanitized_webhook_url_for_log(&webhook_url)
                        );
                        (
                            "failed",
                            webhook_max_retries.min(WEBHOOK_MAX_RETRIES),
                            Some(error),
                        )
                    }
                };
                Self::record_delivery_stats(
                    &job_manager,
                    job_database.as_ref(),
                    stats_webhook,
                    retry_count,
                    job_persistence_turn,
                )
                .await;
                if let Some(database) = job_database {
                    if let Err(error) = database
                        .complete_webhook_logs_with_attempt(
                            &webhook_id,
                            &job_correlation_id,
                            status,
                            error_message.as_deref(),
                            Some(i32::try_from(retry_count.saturating_add(1)).unwrap_or(i32::MAX)),
                        )
                        .await
                    {
                        eprintln!("[WEBHOOK] Failed to persist delivery outcome: {error}");
                    }
                }
            }) {
                if let Some(database) = database.as_ref() {
                    if let Err(error) = database
                        .complete_webhook_logs(
                            &webhook.id,
                            &correlation_id,
                            "failed",
                            Some("daemon task admission is closed"),
                        )
                        .await
                    {
                        eprintln!("[WEBHOOK] Failed to persist rejected delivery outcome: {error}");
                    }
                }
                Self::record_delivery_stats(
                    &manager,
                    database.as_ref(),
                    webhook,
                    0,
                    Arc::clone(&persistence_turn),
                )
                .await;
            }
        }
    }

    async fn record_delivery_stats(
        manager: &Arc<RwLock<WebhookManager>>,
        database: Option<&DatabaseManager>,
        expected_webhook: Webhook,
        retry_count: u32,
        persistence_turn: Arc<tokio::sync::Mutex<()>>,
    ) {
        let _persistence_turn = persistence_turn.lock().await;
        let webhook_id = expected_webhook.id.clone();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let previous_stats = {
            let mut manager = manager.write().await;
            let Some(webhook) = manager.get_mut(&webhook_id) else {
                return;
            };
            if !same_webhook_definition(webhook, &expected_webhook) {
                return;
            }
            let previous_stats = (webhook.last_triggered, webhook.retry_count);
            webhook.last_triggered = Some(timestamp);
            webhook.retry_count = retry_count;
            previous_stats
        };
        if let Some(database) = database {
            let persistence_error = match database
                .update_webhook_delivery_stats(&webhook_id, timestamp, retry_count)
                .await
            {
                Ok(rows) if rows > 0 => None,
                Ok(_) => Some("persisted webhook row is missing".to_owned()),
                Err(error) => Some(error.to_string()),
            };
            if let Some(error) = persistence_error {
                let mut manager = manager.write().await;
                if let Some(webhook) = manager.get_mut(&expected_webhook.id) {
                    if same_webhook_definition(webhook, &expected_webhook)
                        && webhook.last_triggered == Some(timestamp)
                        && webhook.retry_count == retry_count
                    {
                        webhook.last_triggered = previous_stats.0;
                        webhook.retry_count = previous_stats.1;
                    }
                }
                eprintln!("[WEBHOOK] Failed to persist delivery statistics: {error}");
            }
        }
    }

    /// Real retry-with-backoff wrapper around `send_webhook` -- the
    /// dynamic webhook API previously delivered exactly once and never
    /// retried a failed delivery at all, despite `Webhook` exposing
    /// `max_retries`/`can_retry()` as if retries were real.
    async fn send_webhook_with_retries(
        url: &str,
        secret: &str,
        payload: &str,
        timeout_secs: u32,
        max_retries: u32,
    ) -> Result<u32, Box<dyn std::error::Error>> {
        let max_retries = max_retries.min(WEBHOOK_MAX_RETRIES);
        let attempts = max_retries.saturating_add(1);
        let mut last_error = "webhook delivery failed".to_owned();
        for attempt in 1..=attempts {
            if attempt > 1 {
                let delay = Self::calculate_backoff(attempt - 2);
                if !delay.is_zero() {
                    tokio::time::sleep(delay).await;
                }
            }
            match Self::send_webhook(url, secret, payload, timeout_secs).await {
                Ok(()) => return Ok(attempt - 1),
                Err(error) => last_error = error.to_string(),
            }
        }
        Err(last_error.into())
    }

    /// Exponential backoff delay between retry attempts: 30s, 60s, 120s,
    /// 240s, 480s (max). `attempt` is 0-indexed by completed failures.
    fn calculate_backoff(attempt: u32) -> Duration {
        let seconds = 30_u64.saturating_mul(2_u64.saturating_pow(attempt));
        Duration::from_secs(seconds.min(480))
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

fn same_webhook_definition(current: &Webhook, expected: &Webhook) -> bool {
    current.id == expected.id
        && current.url == expected.url
        && current.events == expected.events
        && current.secret == expected.secret
        && current.active == expected.active
        && current.created_at == expected.created_at
        && current.max_retries == expected.max_retries
        && current.timeout_seconds == expected.timeout_seconds
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
        IpAddr::V6(ip) => is_blocked_outbound_ipv6(ip),
    }
}

fn sanitized_webhook_url_for_log(url: &str) -> String {
    match reqwest::Url::parse(url) {
        Ok(parsed) => parsed.origin().ascii_serialization(),
        Err(_) => "<invalid webhook url>".to_string(),
    }
}

pub(super) fn webhook_from_persisted(record: crate::persistence::WebhookRecord) -> Option<Webhook> {
    let events = record
        .events
        .split(',')
        .filter_map(WebhookEvent::from_wire)
        .collect::<Vec<_>>();
    if events.is_empty() {
        return None;
    }
    Some(Webhook {
        id: record.id,
        url: record.url,
        events,
        secret: record.secret,
        active: record.active,
        created_at: record.created_at,
        last_triggered: record.last_triggered,
        retry_count: u32::try_from(record.retry_count).unwrap_or(0),
        max_retries: u32::try_from(record.max_retries).unwrap_or(3),
        timeout_seconds: u32::try_from(record.timeout_seconds).unwrap_or(30),
    })
}

fn webhook_persistence_record(webhook: &Webhook) -> crate::persistence::WebhookRecord {
    crate::persistence::WebhookRecord {
        id: webhook.id.clone(),
        url: webhook.url.clone(),
        events: webhook
            .events
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(","),
        secret: webhook.secret.clone(),
        active: webhook.active,
        created_at: webhook.created_at,
        last_triggered: webhook.last_triggered,
        retry_count: i32::try_from(webhook.retry_count).unwrap_or(i32::MAX),
        max_retries: i32::try_from(webhook.max_retries).unwrap_or(i32::MAX),
        timeout_seconds: i32::try_from(webhook.timeout_seconds).unwrap_or(i32::MAX),
    }
}

pub(super) fn extract_webhook_events(body: &str) -> Result<Vec<WebhookEvent>, &'static str> {
    let payload = serde_json::from_str::<serde_json::Value>(body)
        .map_err(|_| "webhook payload must be valid JSON")?;
    let Some(value) = payload.get("events") else {
        return Ok(vec![WebhookEvent::SearchCreated]);
    };

    let values = match value {
        serde_json::Value::Array(values) => values
            .iter()
            .map(|value| value.as_str().ok_or("webhook events must be strings"))
            .collect::<Result<Vec<_>, _>>()?,
        serde_json::Value::String(value) => value.split(',').collect::<Vec<_>>(),
        _ => return Err("webhook events must be an array or comma-separated string"),
    };
    if values.len() > MAX_WEBHOOK_EVENTS {
        return Err("too many webhook events");
    }

    let mut events = Vec::with_capacity(values.len());
    for value in values {
        let event = WebhookEvent::from_wire(value).ok_or("invalid webhook event")?;
        if !events.contains(&event) {
            events.push(event);
        }
    }
    if events.is_empty() {
        return Err("no valid events specified");
    }
    Ok(events)
}

pub(super) async fn persist_webhook_checked(
    state: &AppState,
    webhook: &Webhook,
) -> Result<bool, String> {
    let Some(db) = state.db.as_ref() else {
        return Ok(false);
    };
    db.insert_webhook(&webhook_persistence_record(webhook))
        .await
        .map_err(|error| format!("webhook persistence failed: {error}"))?;
    Ok(true)
}

pub(super) async fn persist_webhook_delete_checked(
    state: &AppState,
    id: &str,
) -> Result<bool, String> {
    let Some(db) = state.db.as_ref() else {
        return Ok(false);
    };
    db.delete_webhook(id)
        .await
        .map_err(|error| format!("webhook deletion persistence failed: {error}"))?;
    Ok(true)
}

pub(super) async fn rollback_webhooks_if_unchanged(
    state: &AppState,
    previous: WebhookManager,
    mutated: &WebhookManager,
) {
    let mut webhooks = state.webhooks.write().await;
    if *webhooks == *mutated {
        *webhooks = previous;
    }
}

pub(super) async fn persist_webhook_dispatch_logs(
    state: &AppState,
    manager: &WebhookManager,
    event: WebhookEvent,
    correlation_id: &str,
    request_body: &str,
    _persistence_turn: &tokio::sync::MutexGuard<'_, ()>,
) -> Option<String> {
    let db = state.db.as_ref()?;
    let mut failures = 0_usize;
    let mut first_error = None;
    for webhook in manager.get_for_event(event) {
        let record = crate::persistence::WebhookLogRecord {
            id: format!("log_{}", uuid::Uuid::new_v4()),
            webhook_id: webhook.id.clone(),
            event: event.to_string(),
            correlation_id: correlation_id.to_owned(),
            status: "queued".to_owned(),
            request_body: request_body.to_owned(),
            response_status: None,
            response_body: None,
            error_message: None,
            attempt: 1,
            timestamp: i64::try_from(unix_timestamp()).unwrap_or(i64::MAX),
        };
        if let Err(error) = db.insert_webhook_log(&record).await {
            failures = failures.saturating_add(1);
            if first_error.is_none() {
                first_error = Some(error.to_string());
            }
        }
    }
    first_error.map(|error| {
        format!("webhook audit persistence failed for {failures} delivery record(s): {error}")
    })
}

pub(super) async fn dispatch_webhook_event(
    state: &AppState,
    correlation_id: String,
    event: WebhookEvent,
    data: serde_json::Value,
) {
    let frozen_event_name = frozen_webhook_event_name(event);
    scripts::dispatch(
        &state.managed_background_tasks,
        state.integration_settings.read().await.scripts.clone(),
        state.config.state_dir.join("scripts"),
        state.config.controller_profile,
        frozen_event_name,
        &data,
    );
    dispatch_frozen_webhook_event(state, event, &data).await;
    let (eligible_webhooks, log_error) = {
        let persistence_turn = state.webhook_persistence_lock.lock().await;
        let webhooks = state.webhooks.read().await.clone();
        let request_body = WebhookPayload::new(event, correlation_id.clone(), data.clone())
            .to_string()
            .unwrap_or_default();
        let log_error = persist_webhook_dispatch_logs(
            state,
            &webhooks,
            event,
            &correlation_id,
            &request_body,
            &persistence_turn,
        )
        .await;
        let eligible_webhooks = webhooks.get_for_event(event).into_iter().cloned().collect();
        (eligible_webhooks, log_error)
    };
    if let Some(error) = log_error {
        update_session(state, |snapshot| {
            snapshot.last_error = Some(error);
        })
        .await;
    }
    WebhookDispatcher::dispatch(
        &state.managed_background_tasks,
        WebhookDispatchContext {
            manager: Arc::clone(&state.webhooks),
            deliveries: Arc::clone(&state.webhook_deliveries),
            persistence_turn: Arc::clone(&state.webhook_persistence_lock),
            database: state.db.clone(),
        },
        eligible_webhooks,
        correlation_id,
        event,
        data,
    )
    .await;
}

fn frozen_webhook_event_name(event: WebhookEvent) -> &'static str {
    match event {
        WebhookEvent::TransferCompleted => "DownloadFileComplete",
        WebhookEvent::TransferFailed => "DownloadFileFailed",
        WebhookEvent::MessageReceived => "PrivateMessageReceived",
        WebhookEvent::SearchCompleted => "SearchResponsesReceived",
        WebhookEvent::UserConnected => "SoulseekClientConnected",
        WebhookEvent::UserDisconnected => "SoulseekClientDisconnected",
        WebhookEvent::RoomJoined | WebhookEvent::RoomLeft => "RoomMessageReceived",
        WebhookEvent::TransferStarted => "PeerDownloadedFromUs",
        WebhookEvent::SearchCreated => "PeerSearchedUs",
        WebhookEvent::MessageSent
        | WebhookEvent::ApiKeyCreated
        | WebhookEvent::ApiKeyRevoked
        | WebhookEvent::ConfigChanged => "Noop",
    }
}

async fn dispatch_frozen_webhook_event(
    state: &AppState,
    event: WebhookEvent,
    data: &serde_json::Value,
) {
    let event_name = frozen_webhook_event_name(event);
    let hooks = state
        .integration_settings
        .read()
        .await
        .frozen_webhooks
        .clone();
    let payload = serde_json::json!({
        "id": uuid::Uuid::new_v4(),
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "type": event_name,
        "version": 0,
        "data": data,
    })
    .to_string();
    for (name, hook) in hooks {
        if !hook.on.iter().any(|value| {
            value.eq_ignore_ascii_case("Any") || value.eq_ignore_ascii_case(event_name)
        }) {
            continue;
        }
        let Ok(permit) = Arc::clone(&state.webhook_deliveries).try_acquire_owned() else {
            record_daemon_log(
                state,
                logging::LogLevel::Warn,
                "webhook",
                format!("Frozen webhook {name} delivery pool is full"),
            )
            .await;
            continue;
        };
        let headers = hook
            .call
            .headers
            .into_iter()
            .map(|header| (header.name, header.value))
            .collect::<Vec<_>>();
        let url = hook.call.url;
        let timeout = u64::try_from(hook.timeout).unwrap_or(500);
        let attempts = u32::try_from(hook.retry.attempts).unwrap_or(1);
        let ignore_certificate_errors = hook.call.ignore_certificate_errors;
        let payload = payload.clone();
        let rejected_name = name.clone();
        if !state.managed_background_tasks.try_spawn(async move {
            let _permit = permit;
            if let Err(error) = WebhookDispatcher::send_frozen_compat_webhook(
                &url,
                &headers,
                &payload,
                timeout,
                attempts,
                ignore_certificate_errors,
            )
            .await
            {
                eprintln!("[WEBHOOK] Frozen webhook {name} delivery failed: {error}");
            }
        }) {
            record_daemon_log(
                state,
                logging::LogLevel::Warn,
                "webhook",
                format!("Frozen webhook {rejected_name} rejected during daemon shutdown"),
            )
            .await;
        }
    }
}

#[cfg(test)]
#[path = "webhooks_tests.rs"]
mod tests;
