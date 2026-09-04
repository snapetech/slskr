//! Mesh transport security primitives matching the frozen native profile contracts.
//!
//! The HTTP rate limiter and the overlay certificate verifier cover separate
//! concerns. This module keeps the mesh transport controls together so that
//! DHT quotas, replay protection, certificate rotation, and per-peer
//! transport policy can be exercised independently and then composed by mesh
//! callers.

use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    fs,
    io::Read,
    net::{IpAddr, SocketAddr},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub const MAX_REMOTE_PAYLOAD_SIZE: usize = 1024 * 1024;
pub const MAX_PARSE_DEPTH: usize = 32;
const MAX_RATE_LIMIT_BUCKETS: usize = 4096;
const PREVIOUS_PIN_GRACE: u64 = 30 * 24 * 60 * 60;
const MAX_CERTIFICATE_PIN_PEERS: usize = 1_024;
const MAX_CERTIFICATE_PINS_PER_PEER: usize = 8;
const MAX_CERTIFICATE_PEER_ID_BYTES: usize = 512;
const MAX_CERTIFICATE_PIN_BYTES: usize = 128;
const MAX_CERTIFICATE_PIN_STATE_BYTES: usize = 2 * 1024 * 1024;

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RateLimiterStatistics {
    pub active_buckets: usize,
    pub total_tokens_consumed: u64,
    pub total_requests_blocked: u64,
}

#[derive(Debug)]
struct TokenBucket {
    capacity: u64,
    refill_rate: f64,
    tokens: f64,
    last_refill: Instant,
    last_access: Instant,
    tokens_consumed: u64,
    requests_blocked: u64,
}

impl TokenBucket {
    fn new(capacity: u64, refill_rate: f64) -> Self {
        let now = Instant::now();
        Self {
            capacity,
            refill_rate,
            tokens: capacity as f64,
            last_refill: now,
            last_access: now,
            tokens_consumed: 0,
            requests_blocked: 0,
        }
    }

    fn refill(&mut self, now: Instant) {
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        if elapsed > 0.0 {
            self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.capacity as f64);
            self.last_refill = now;
        }
    }

    fn try_consume(&mut self, tokens: u64) -> bool {
        let now = Instant::now();
        self.last_access = now;
        self.refill(now);
        if self.tokens >= tokens as f64 {
            self.tokens -= tokens as f64;
            self.tokens_consumed = self.tokens_consumed.saturating_add(tokens);
            true
        } else {
            self.requests_blocked = self.requests_blocked.saturating_add(1);
            false
        }
    }

    fn current_tokens(&mut self) -> u64 {
        self.refill(Instant::now());
        self.tokens.floor() as u64
    }

    fn reset(&mut self) {
        let now = Instant::now();
        self.tokens = self.capacity as f64;
        self.last_refill = now;
        self.last_access = now;
        self.tokens_consumed = 0;
        self.requests_blocked = 0;
    }
}

/// Thread-safe token bucket collection used by mesh and DHT controls.
#[derive(Debug, Default)]
pub struct RateLimiter {
    buckets: Mutex<HashMap<String, TokenBucket>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn try_consume(
        &self,
        bucket_key: &str,
        tokens: u64,
        capacity: u64,
        refill_rate: f64,
    ) -> bool {
        if capacity == 0 || !refill_rate.is_finite() || refill_rate < 0.0 {
            return false;
        }
        let mut buckets = self.buckets.lock().expect("mesh rate limiter lock");
        if !buckets.contains_key(bucket_key) && buckets.len() >= MAX_RATE_LIMIT_BUCKETS {
            drop(buckets);
            self.cleanup_expired_buckets(Duration::from_secs(10 * 60));
            buckets = self.buckets.lock().expect("mesh rate limiter lock");
            if buckets.len() >= MAX_RATE_LIMIT_BUCKETS {
                return false;
            }
        }
        let bucket = buckets
            .entry(bucket_key.to_owned())
            .or_insert_with(|| TokenBucket::new(capacity, refill_rate));
        bucket.try_consume(tokens)
    }

    pub fn current_tokens(&self, bucket_key: &str) -> u64 {
        self.buckets
            .lock()
            .expect("mesh rate limiter lock")
            .get_mut(bucket_key)
            .map(TokenBucket::current_tokens)
            .unwrap_or(0)
    }

    pub fn reset_bucket(&self, bucket_key: &str) {
        if let Some(bucket) = self
            .buckets
            .lock()
            .expect("mesh rate limiter lock")
            .get_mut(bucket_key)
        {
            bucket.reset();
        }
    }

    pub fn cleanup_expired_buckets(&self, max_age: Duration) {
        let cutoff = Instant::now()
            .checked_sub(max_age)
            .unwrap_or_else(Instant::now);
        self.buckets
            .lock()
            .expect("mesh rate limiter lock")
            .retain(|_, bucket| bucket.last_access >= cutoff);
    }

    pub fn statistics(&self) -> RateLimiterStatistics {
        let buckets = self.buckets.lock().expect("mesh rate limiter lock");
        RateLimiterStatistics {
            active_buckets: buckets.len(),
            total_tokens_consumed: buckets.values().map(|bucket| bucket.tokens_consumed).sum(),
            total_requests_blocked: buckets.values().map(|bucket| bucket.requests_blocked).sum(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DhtRateLimiterStatistics {
    pub active_buckets: usize,
    pub total_requests_blocked: u64,
    pub descriptor_fetch_tokens: u64,
}

/// DHT-specific quota buckets from Mesh/Dht/DhtRateLimiter.
#[derive(Clone, Debug)]
pub struct DhtRateLimiter {
    rate_limiter: std::sync::Arc<RateLimiter>,
}

impl DhtRateLimiter {
    pub fn new(rate_limiter: std::sync::Arc<RateLimiter>) -> Self {
        Self { rate_limiter }
    }

    pub fn should_allow_descriptor_fetch(&self, peer_id: &str) -> bool {
        self.rate_limiter
            .try_consume(&format!("dht-descriptor-fetch-{peer_id}"), 1, 100, 1.67)
    }

    pub fn should_allow_descriptor_publish(&self, peer_id: &str) -> bool {
        self.rate_limiter
            .try_consume(&format!("dht-descriptor-publish-{peer_id}"), 1, 20, 0.333)
    }

    pub fn should_allow_query(&self, query_type: &str, requester_id: &str) -> bool {
        self.rate_limiter.try_consume(
            &format!("dht-query-{query_type}-{requester_id}"),
            1,
            200,
            3.33,
        )
    }

    pub fn report_failed_operation(&self, operation_type: &str, peer_id: &str) -> bool {
        self.rate_limiter.try_consume(
            &format!("dht-failure-{operation_type}-{peer_id}"),
            1,
            10,
            0.167,
        )
    }

    pub fn statistics(&self) -> DhtRateLimiterStatistics {
        let statistics = self.rate_limiter.statistics();
        DhtRateLimiterStatistics {
            active_buckets: statistics.active_buckets,
            total_requests_blocked: statistics.total_requests_blocked,
            // Matches the frozen implementation's global statistics key.
            descriptor_fetch_tokens: self
                .rate_limiter
                .current_tokens("dht-descriptor-fetch-global"),
        }
    }

    pub fn cleanup_expired_buckets(&self) {
        self.rate_limiter
            .cleanup_expired_buckets(Duration::from_secs(60 * 60));
    }
}

/// Frozen DhtRendezvous overlay admission limits.  The Soulseek listener and
/// mesh gateway use this control for connection, message, delta, and mesh
/// search budgets; the result keeps the reason available to the caller while
/// preserving the frozen bool-like decision surface.
pub const OVERLAY_MAX_CONNECTIONS_PER_IP: usize = 3;
pub const OVERLAY_MAX_CONNECTIONS_PER_MINUTE: usize = 30;
pub const OVERLAY_MAX_TOTAL_CONNECTIONS: usize = 100;
pub const OVERLAY_MAX_MESSAGES_PER_SECOND: usize = 10;
pub const OVERLAY_MAX_DELTA_REQUESTS_PER_HOUR: usize = 60;
pub const OVERLAY_MAX_MESH_SEARCH_REQUESTS_PER_MINUTE: usize = 30;
pub const OVERLAY_VIOLATION_BACKOFF: Duration = Duration::from_secs(300);
pub const OVERLAY_MAX_VIOLATIONS_BEFORE_BAN: u32 = 3;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OverlayRateLimitResult {
    pub allowed: bool,
    pub reason: Option<String>,
}

impl OverlayRateLimitResult {
    fn allowed() -> Self {
        Self {
            allowed: true,
            reason: None,
        }
    }

    fn rejected(reason: impl Into<String>) -> Self {
        Self {
            allowed: false,
            reason: Some(reason.into()),
        }
    }
}

#[derive(Debug, Default)]
struct OverlayIpState {
    active_connections: usize,
    violations: u32,
    backoff_until: Option<Instant>,
}

#[derive(Debug, Default)]
struct OverlayPeerState {
    message_times: VecDeque<Instant>,
    delta_request_times: VecDeque<Instant>,
    mesh_search_request_times: VecDeque<Instant>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OverlayRateLimiterStats {
    pub total_connections: usize,
    pub tracked_ips: usize,
    pub tracked_peers: usize,
    pub recent_connections: usize,
    pub violations: u64,
    pub rejected: u64,
}

/// Thread-safe overlay admission control matching the frozen
/// `DhtRendezvous.Security.OverlayRateLimiter` capacities and windows.
#[derive(Debug, Default)]
pub struct OverlayRateLimiter {
    ip_states: Mutex<HashMap<IpAddr, OverlayIpState>>,
    peer_states: Mutex<HashMap<String, OverlayPeerState>>,
    global: Mutex<OverlayGlobalState>,
}

#[derive(Debug, Default)]
struct OverlayGlobalState {
    total_connections: usize,
    recent_connections: VecDeque<Instant>,
    violations: u64,
    rejected: u64,
}

impl OverlayRateLimiter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn check_connection(&self, ip: IpAddr) -> OverlayRateLimitResult {
        let now = Instant::now();
        let mut ip_states = self.ip_states.lock().expect("overlay IP limiter lock");
        let state = ip_states.entry(ip).or_default();
        if state.backoff_until.is_some_and(|until| until > now) {
            let mut global = self.global.lock().expect("overlay global limiter lock");
            global.rejected = global.rejected.saturating_add(1);
            return OverlayRateLimitResult::rejected("IP is in violation backoff");
        }
        if state.active_connections >= OVERLAY_MAX_CONNECTIONS_PER_IP {
            state.violations = state.violations.saturating_add(1);
            if state.violations >= OVERLAY_MAX_VIOLATIONS_BEFORE_BAN {
                state.backoff_until = Some(now + OVERLAY_VIOLATION_BACKOFF);
            }
            let mut global = self.global.lock().expect("overlay global limiter lock");
            global.violations = global.violations.saturating_add(1);
            global.rejected = global.rejected.saturating_add(1);
            return OverlayRateLimitResult::rejected("too many connections from this IP");
        }

        let mut global = self.global.lock().expect("overlay global limiter lock");
        prune_instants(&mut global.recent_connections, now, Duration::from_secs(60));
        if global.total_connections >= OVERLAY_MAX_TOTAL_CONNECTIONS {
            global.rejected = global.rejected.saturating_add(1);
            return OverlayRateLimitResult::rejected("overlay connection capacity reached");
        }
        if global.recent_connections.len() >= OVERLAY_MAX_CONNECTIONS_PER_MINUTE {
            global.rejected = global.rejected.saturating_add(1);
            return OverlayRateLimitResult::rejected("overlay connection rate exceeded");
        }
        global.total_connections = global.total_connections.saturating_add(1);
        global.recent_connections.push_back(now);
        state.active_connections = state.active_connections.saturating_add(1);
        OverlayRateLimitResult::allowed()
    }

    pub fn record_disconnection(&self, ip: IpAddr) {
        if let Some(state) = self
            .ip_states
            .lock()
            .expect("overlay IP limiter lock")
            .get_mut(&ip)
        {
            state.active_connections = state.active_connections.saturating_sub(1);
        }
        let mut global = self.global.lock().expect("overlay global limiter lock");
        global.total_connections = global.total_connections.saturating_sub(1);
    }

    pub fn check_message(&self, connection_id: &str) -> OverlayRateLimitResult {
        let now = Instant::now();
        let mut peers = self.peer_states.lock().expect("overlay peer limiter lock");
        let state = peers.entry(connection_id.to_owned()).or_default();
        prune_instants(&mut state.message_times, now, Duration::from_secs(1));
        if state.message_times.len() >= OVERLAY_MAX_MESSAGES_PER_SECOND {
            self.record_rejection();
            return OverlayRateLimitResult::rejected("overlay message rate exceeded");
        }
        state.message_times.push_back(now);
        OverlayRateLimitResult::allowed()
    }

    pub fn check_delta_request(&self, peer_id: &str) -> OverlayRateLimitResult {
        self.check_peer_window(
            peer_id,
            Duration::from_secs(60 * 60),
            OVERLAY_MAX_DELTA_REQUESTS_PER_HOUR,
            |state| &mut state.delta_request_times,
            "overlay delta request rate exceeded",
        )
    }

    pub fn check_mesh_search_request(&self, peer_id: &str) -> OverlayRateLimitResult {
        self.check_peer_window(
            peer_id,
            Duration::from_secs(60),
            OVERLAY_MAX_MESH_SEARCH_REQUESTS_PER_MINUTE,
            |state| &mut state.mesh_search_request_times,
            "overlay mesh-search rate exceeded",
        )
    }

    fn check_peer_window(
        &self,
        peer_id: &str,
        window: Duration,
        maximum: usize,
        queue: impl Fn(&mut OverlayPeerState) -> &mut VecDeque<Instant>,
        reason: &str,
    ) -> OverlayRateLimitResult {
        let now = Instant::now();
        let mut peers = self.peer_states.lock().expect("overlay peer limiter lock");
        let state = peers.entry(peer_id.to_owned()).or_default();
        let entries = queue(state);
        prune_instants(entries, now, window);
        if entries.len() >= maximum {
            self.record_rejection();
            return OverlayRateLimitResult::rejected(reason);
        }
        entries.push_back(now);
        OverlayRateLimitResult::allowed()
    }

    pub fn record_violation(&self, ip: IpAddr) {
        let now = Instant::now();
        let mut state = self.ip_states.lock().expect("overlay IP limiter lock");
        let state = state.entry(ip).or_default();
        state.violations = state.violations.saturating_add(1);
        if state.violations >= OVERLAY_MAX_VIOLATIONS_BEFORE_BAN {
            state.backoff_until = Some(now + OVERLAY_VIOLATION_BACKOFF);
        }
        let mut global = self.global.lock().expect("overlay global limiter lock");
        global.violations = global.violations.saturating_add(1);
    }

    pub fn remove_connection(&self, connection_id: &str) {
        self.peer_states
            .lock()
            .expect("overlay peer limiter lock")
            .remove(connection_id);
    }

    pub fn stats(&self) -> OverlayRateLimiterStats {
        let now = Instant::now();
        let mut global = self.global.lock().expect("overlay global limiter lock");
        prune_instants(&mut global.recent_connections, now, Duration::from_secs(60));
        let ip_states = self.ip_states.lock().expect("overlay IP limiter lock");
        let peer_states = self.peer_states.lock().expect("overlay peer limiter lock");
        OverlayRateLimiterStats {
            total_connections: global.total_connections,
            tracked_ips: ip_states.len(),
            tracked_peers: peer_states.len(),
            recent_connections: global.recent_connections.len(),
            violations: global.violations,
            rejected: global.rejected,
        }
    }

    fn record_rejection(&self) {
        let mut global = self.global.lock().expect("overlay global limiter lock");
        global.rejected = global.rejected.saturating_add(1);
    }
}

fn prune_instants(queue: &mut VecDeque<Instant>, now: Instant, window: Duration) {
    while queue
        .front()
        .is_some_and(|timestamp| now.duration_since(*timestamp) >= window)
    {
        queue.pop_front();
    }
}

#[derive(Clone, Debug)]
pub struct OverlayBlocklistEntry {
    pub reason: String,
    pub blocked_at: SystemTime,
    pub expires_at: SystemTime,
    pub permanent: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OverlayBlocklistStats {
    pub blocked_ips: usize,
    pub blocked_usernames: usize,
    pub permanent_ip_bans: usize,
    pub permanent_username_bans: usize,
}

/// In-memory overlay IP/username blocklist. Durable controller bans remain
/// owned by `SecurityState`; this type supplies the frozen overlay admission
/// semantics for a live gateway.
#[derive(Debug, Default)]
pub struct OverlayBlocklist {
    ips: Mutex<HashMap<IpAddr, OverlayBlocklistEntry>>,
    usernames: Mutex<HashMap<String, OverlayBlocklistEntry>>,
}

impl OverlayBlocklist {
    pub const DEFAULT_BAN_DURATION: Duration = Duration::from_secs(24 * 60 * 60);
    pub const PERMANENT_BAN_DURATION: Duration = Duration::from_secs(3_650 * 24 * 60 * 60);

    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_blocked_ip(&self, ip: IpAddr) -> bool {
        is_active_entry(
            &mut self.ips.lock().expect("overlay IP blocklist lock"),
            &ip,
        )
    }

    pub fn is_blocked_username(&self, username: &str) -> bool {
        if username.trim().is_empty() {
            return false;
        }
        let key = username.to_ascii_lowercase();
        is_active_entry(
            &mut self
                .usernames
                .lock()
                .expect("overlay username blocklist lock"),
            &key,
        )
    }

    pub fn block_ip(
        &self,
        ip: IpAddr,
        reason: impl Into<String>,
        duration: Option<Duration>,
        permanent: bool,
    ) {
        self.ips
            .lock()
            .expect("overlay IP blocklist lock")
            .insert(ip, new_blocklist_entry(reason, duration, permanent));
    }

    pub fn block_username(
        &self,
        username: &str,
        reason: impl Into<String>,
        duration: Option<Duration>,
        permanent: bool,
    ) {
        if username.trim().is_empty() {
            return;
        }
        self.usernames
            .lock()
            .expect("overlay username blocklist lock")
            .insert(
                username.to_ascii_lowercase(),
                new_blocklist_entry(reason, duration, permanent),
            );
    }

    pub fn unblock_ip(&self, ip: IpAddr) -> bool {
        self.ips
            .lock()
            .expect("overlay IP blocklist lock")
            .remove(&ip)
            .is_some()
    }

    pub fn unblock_username(&self, username: &str) -> bool {
        self.usernames
            .lock()
            .expect("overlay username blocklist lock")
            .remove(&username.to_ascii_lowercase())
            .is_some()
    }

    pub fn stats(&self) -> OverlayBlocklistStats {
        let now = SystemTime::now();
        let mut ips = self.ips.lock().expect("overlay IP blocklist lock");
        let mut usernames = self
            .usernames
            .lock()
            .expect("overlay username blocklist lock");
        ips.retain(|_, entry| entry.expires_at > now);
        usernames.retain(|_, entry| entry.expires_at > now);
        OverlayBlocklistStats {
            blocked_ips: ips.len(),
            blocked_usernames: usernames.len(),
            permanent_ip_bans: ips.values().filter(|entry| entry.permanent).count(),
            permanent_username_bans: usernames.values().filter(|entry| entry.permanent).count(),
        }
    }
}

fn new_blocklist_entry(
    reason: impl Into<String>,
    duration: Option<Duration>,
    permanent: bool,
) -> OverlayBlocklistEntry {
    let blocked_at = SystemTime::now();
    let duration = if permanent {
        OverlayBlocklist::PERMANENT_BAN_DURATION
    } else {
        duration.unwrap_or(OverlayBlocklist::DEFAULT_BAN_DURATION)
    };
    OverlayBlocklistEntry {
        reason: reason.into(),
        blocked_at,
        expires_at: blocked_at + duration,
        permanent,
    }
}

fn is_active_entry<K: std::cmp::Eq + std::hash::Hash>(
    entries: &mut HashMap<K, OverlayBlocklistEntry>,
    key: &K,
) -> bool {
    let Some(entry) = entries.get(key) else {
        return false;
    };
    if entry.expires_at > SystemTime::now() {
        true
    } else {
        entries.remove(key);
        false
    }
}

/// Replay cache with an atomic check-and-record operation.
#[derive(Debug)]
pub struct ReplayCache {
    entries: Mutex<HashMap<String, Instant>>,
    cache_duration: Duration,
    max_cache_size: usize,
}

impl ReplayCache {
    pub fn new(cache_duration: Duration, max_cache_size: usize) -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            cache_duration,
            max_cache_size: max_cache_size.max(1),
        }
    }

    /// Returns true when the message was already recorded in the active window.
    pub fn check_and_record(&self, message_id: &str) -> bool {
        if message_id.trim().is_empty() {
            return false;
        }
        let now = Instant::now();
        let mut entries = self.entries.lock().expect("mesh replay cache lock");
        let prune_threshold = self.max_cache_size.saturating_mul(9) / 10;
        if entries.len() >= prune_threshold {
            entries.retain(|_, seen_at| now.duration_since(*seen_at) < self.cache_duration);
        }
        if let Some(seen_at) = entries.get(message_id).copied() {
            if now.duration_since(seen_at) < self.cache_duration {
                return true;
            }
            entries.remove(message_id);
        }
        if entries.len() >= self.max_cache_size {
            let oldest = entries
                .iter()
                .min_by_key(|(_, seen_at)| **seen_at)
                .map(|(message_id, _)| message_id.clone());
            if let Some(oldest) = oldest {
                entries.remove(&oldest);
            }
        }
        entries.insert(message_id.to_owned(), now);
        false
    }

    pub fn cache_size(&self) -> usize {
        self.entries.lock().expect("mesh replay cache lock").len()
    }
}

#[derive(Clone, Debug, Default)]
pub struct ConnectionAttemptInfo {
    pub attempt_count: u32,
    pub backoff_until: Option<Instant>,
    pub last_success: Option<Instant>,
    pub last_failure: Option<Instant>,
}

impl ConnectionAttemptInfo {
    pub fn is_in_backoff(&self) -> bool {
        self.backoff_until
            .is_some_and(|until| until > Instant::now())
    }
}

/// Per-peer exponential backoff for repeated connection failures.
#[derive(Debug)]
pub struct ConnectionRateLimiter {
    attempts: Mutex<HashMap<String, ConnectionAttemptInfo>>,
    backoff_base: Duration,
    max_attempts: u32,
}

impl ConnectionRateLimiter {
    pub fn new(backoff_base: Duration, max_attempts: u32) -> Self {
        Self {
            attempts: Mutex::new(HashMap::new()),
            backoff_base,
            max_attempts: max_attempts.max(1),
        }
    }

    pub fn is_connection_allowed(&self, peer_id: &str) -> bool {
        let mut attempts = self.attempts.lock().expect("mesh connection limiter lock");
        let info = attempts.entry(peer_id.to_owned()).or_default();
        let now = Instant::now();
        if info.backoff_until.is_some_and(|until| until > now) {
            return false;
        }
        if info.attempt_count >= self.max_attempts {
            let multiplier = (info.attempt_count - self.max_attempts + 1).min(10);
            let seconds =
                self.backoff_base.as_secs_f64() * 2_f64.powi(multiplier.saturating_sub(1) as i32);
            info.backoff_until = Some(now + Duration::from_secs_f64(seconds));
            return false;
        }
        true
    }

    pub fn record_success(&self, peer_id: &str) {
        let mut attempts = self.attempts.lock().expect("mesh connection limiter lock");
        let info = attempts.entry(peer_id.to_owned()).or_default();
        info.attempt_count = 0;
        info.backoff_until = None;
        info.last_success = Some(Instant::now());
    }

    pub fn record_failure(&self, peer_id: &str) {
        let mut attempts = self.attempts.lock().expect("mesh connection limiter lock");
        let info = attempts.entry(peer_id.to_owned()).or_default();
        info.attempt_count = info.attempt_count.saturating_add(1);
        info.last_failure = Some(Instant::now());
    }

    pub fn attempt_info(&self, peer_id: &str) -> ConnectionAttemptInfo {
        self.attempts
            .lock()
            .expect("mesh connection limiter lock")
            .get(peer_id)
            .cloned()
            .unwrap_or_default()
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[allow(clippy::enum_variant_names)]
pub enum MeshTransportType {
    DirectQuic,
    TorOnionQuic,
    I2PQuic,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MeshTransportOptions {
    pub enable_direct: bool,
    pub tor_enabled: bool,
    pub i2p_enabled: bool,
}

impl Default for MeshTransportOptions {
    fn default() -> Self {
        Self {
            enable_direct: true,
            tor_enabled: false,
            i2p_enabled: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransportPolicy {
    pub peer_id: Option<String>,
    pub pod_id: Option<String>,
    pub prefer_private_transports: bool,
    pub disable_clearnet: bool,
    pub allowed_transport_types: Option<Vec<MeshTransportType>>,
    pub transport_preference_order: Option<Vec<MeshTransportType>>,
    pub is_enabled: bool,
}

impl Default for TransportPolicy {
    fn default() -> Self {
        Self {
            peer_id: None,
            pod_id: None,
            prefer_private_transports: false,
            disable_clearnet: false,
            allowed_transport_types: None,
            transport_preference_order: None,
            is_enabled: true,
        }
    }
}

impl TransportPolicy {
    pub fn enabled_for(peer_id: Option<String>, pod_id: Option<String>) -> Self {
        Self {
            peer_id,
            pod_id,
            is_enabled: true,
            ..Self::default()
        }
    }

    pub fn applies_to(&self, peer_id: &str, pod_id: Option<&str>) -> bool {
        self.is_enabled
            && self
                .peer_id
                .as_deref()
                .is_none_or(|value| value.is_empty() || value == peer_id)
            && self
                .pod_id
                .as_deref()
                .is_none_or(|value| value.is_empty() || Some(value) == pod_id)
    }

    pub fn effective_preference_order(
        &self,
        global_order: &[MeshTransportType],
    ) -> Vec<MeshTransportType> {
        self.transport_preference_order
            .clone()
            .unwrap_or_else(|| global_order.to_vec())
    }

    pub fn is_transport_allowed(
        &self,
        transport_type: MeshTransportType,
        global_options: MeshTransportOptions,
    ) -> bool {
        if self
            .allowed_transport_types
            .as_ref()
            .is_some_and(|allowed| !allowed.is_empty() && !allowed.contains(&transport_type))
        {
            return false;
        }
        if self.disable_clearnet && transport_type == MeshTransportType::DirectQuic {
            return false;
        }
        match transport_type {
            MeshTransportType::DirectQuic => global_options.enable_direct,
            MeshTransportType::TorOnionQuic => global_options.tor_enabled,
            MeshTransportType::I2PQuic => global_options.i2p_enabled,
        }
    }
}

#[derive(Debug, Default)]
pub struct TransportPolicyManager {
    policies: Mutex<Vec<TransportPolicy>>,
}

impl TransportPolicyManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_or_update_policy(&self, policy: TransportPolicy) {
        let mut policies = self.policies.lock().expect("mesh policy lock");
        policies.retain(|existing| {
            existing.peer_id != policy.peer_id || existing.pod_id != policy.pod_id
        });
        policies.push(policy);
    }

    pub fn remove_policy(&self, peer_id: &str, pod_id: Option<&str>) {
        self.policies
            .lock()
            .expect("mesh policy lock")
            .retain(|policy| {
                policy.peer_id.as_deref() != Some(peer_id) || policy.pod_id.as_deref() != pod_id
            });
    }

    pub fn applicable_policy(
        &self,
        peer_id: &str,
        pod_id: Option<&str>,
    ) -> Option<TransportPolicy> {
        self.policies
            .lock()
            .expect("mesh policy lock")
            .iter()
            .filter(|policy| policy.applies_to(peer_id, pod_id))
            .max_by_key(|policy| {
                u8::from(
                    policy
                        .peer_id
                        .as_deref()
                        .is_some_and(|value| !value.is_empty()),
                ) * 2
                    + u8::from(
                        policy
                            .pod_id
                            .as_deref()
                            .is_some_and(|value| !value.is_empty()),
                    )
            })
            .cloned()
    }

    pub fn all_policies(&self) -> Vec<TransportPolicy> {
        self.policies.lock().expect("mesh policy lock").clone()
    }
}

pub struct SecurityUtils;

impl SecurityUtils {
    pub fn certificate_public_key_pin(certificate_der: &[u8]) -> Result<[u8; 32], String> {
        slskr_client::quic_control::certificate_public_key_pin(certificate_der)
            .map_err(|error| error.to_string())
    }

    pub fn certificate_pin_base64(certificate_der: &[u8]) -> Result<String, String> {
        Ok(STANDARD.encode(Self::certificate_public_key_pin(certificate_der)?))
    }

    pub fn validate_certificate_pin(certificate_der: &[u8], expected_pins: &[String]) -> bool {
        Self::certificate_pin_base64(certificate_der)
            .ok()
            .is_some_and(|actual| expected_pins.iter().any(|expected| expected == &actual))
    }

    pub fn validate_certificate_pin_and_time(
        certificate_der: &[u8],
        expected_pins: &[String],
    ) -> bool {
        if !Self::validate_certificate_pin(certificate_der, expected_pins) {
            return false;
        }
        x509_parser::parse_x509_certificate(certificate_der)
            .is_ok_and(|(_, certificate)| certificate.tbs_certificate.validity.is_valid())
    }

    pub fn parse_json_safely<T: DeserializeOwned>(
        json: &str,
        max_size: usize,
        max_depth: usize,
    ) -> Result<T, String> {
        if json.is_empty() {
            return Err("JSON cannot be null or empty".to_owned());
        }
        if json.len() > max_size {
            return Err(format!(
                "JSON payload exceeds maximum size of {max_size} bytes"
            ));
        }
        if json_nesting_depth(json) > max_depth {
            return Err(format!("JSON payload exceeds maximum depth of {max_depth}"));
        }
        serde_json::from_str(json).map_err(|error| error.to_string())
    }

    pub fn validate_messagepack_size(data: &[u8], max_size: usize) -> Result<(), String> {
        if data.is_empty() {
            return Err("MessagePack data cannot be null or empty".to_owned());
        }
        if data.len() > max_size {
            return Err(format!(
                "MessagePack payload exceeds maximum size of {max_size} bytes"
            ));
        }
        Ok(())
    }
}

fn json_nesting_depth(json: &str) -> usize {
    let mut depth: usize = 0;
    let mut maximum: usize = 0;
    let mut in_string = false;
    let mut escaped = false;
    for byte in json.bytes() {
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            continue;
        }
        match byte {
            b'"' => in_string = true,
            b'{' | b'[' => {
                depth += 1;
                maximum = maximum.max(depth);
            }
            b'}' | b']' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    maximum
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CertificatePinType {
    Current,
    Previous,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct PeerCertificateInfo {
    pub peer_id: String,
    pub current_pins: Vec<String>,
    pub previous_pins: Vec<String>,
    pub last_rotation: u64,
    pub last_validation: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CertificatePinStatistics {
    pub total_peers: usize,
    pub peers_with_current_pins: usize,
    pub peers_with_previous_pins: usize,
    pub total_current_pins: usize,
    pub total_previous_pins: usize,
}

#[derive(Debug, Deserialize, Serialize)]
struct PersistedPinData {
    peer_certificates: Vec<PeerCertificateInfo>,
    last_updated: u64,
}

/// Durable current/previous SPKI pin manager with a 30-day rotation window.
#[derive(Debug)]
pub struct CertificatePinManager {
    storage_path: PathBuf,
    peers: Mutex<HashMap<String, PeerCertificateInfo>>,
}

impl CertificatePinManager {
    pub fn new(data_directory: impl AsRef<Path>) -> Result<Self, String> {
        let mesh_directory = data_directory.as_ref().join("mesh");
        match fs::symlink_metadata(&mesh_directory) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                return Err("mesh pin directory must be a regular directory".to_owned());
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir_all(&mesh_directory)
                    .map_err(|error| format!("mesh pin directory creation failed: {error}"))?;
            }
            Err(error) => {
                return Err(format!("mesh pin directory inspection failed: {error}"));
            }
        }
        let manager = Self {
            storage_path: mesh_directory.join("certificate-pins.json"),
            peers: Mutex::new(HashMap::new()),
        };
        manager.load_persisted_pins()?;
        Ok(manager)
    }

    pub fn validate_certificate_pin(&self, peer_id: &str, certificate_der: &[u8]) -> bool {
        let mut peers = self.peers.lock().expect("mesh certificate pin lock");
        let Some(info) = peers.get_mut(peer_id) else {
            return false;
        };
        let Ok(pin) = SecurityUtils::certificate_pin_base64(certificate_der) else {
            return false;
        };
        if info.current_pins.iter().any(|candidate| candidate == &pin) {
            info.last_validation = unix_seconds();
            return true;
        }
        if info.previous_pins.iter().any(|candidate| candidate == &pin)
            && unix_seconds().saturating_sub(info.last_rotation) < PREVIOUS_PIN_GRACE
        {
            info.last_validation = unix_seconds();
            return true;
        }
        false
    }

    pub fn add_pin(
        &self,
        peer_id: &str,
        pin: &str,
        pin_type: CertificatePinType,
    ) -> Result<(), String> {
        let peer_id = peer_id.trim();
        let pin = pin.trim();
        validate_certificate_peer_id(peer_id)?;
        validate_certificate_pin_text(pin)?;
        let mut peers = self.peers.lock().expect("mesh certificate pin lock");
        if !peers.contains_key(peer_id) && peers.len() >= MAX_CERTIFICATE_PIN_PEERS {
            return Err("certificate pin peer capacity is full".to_owned());
        }
        let previous = peers.get(peer_id).cloned();
        let info = peers
            .entry(peer_id.to_owned())
            .or_insert_with(|| PeerCertificateInfo {
                peer_id: peer_id.to_owned(),
                ..PeerCertificateInfo::default()
            });
        match pin_type {
            CertificatePinType::Current => {
                if !info.current_pins.iter().any(|candidate| candidate == pin) {
                    info.previous_pins.append(&mut info.current_pins);
                    trim_certificate_pins(&mut info.previous_pins);
                    info.current_pins.push(pin.to_owned());
                    info.last_rotation = unix_seconds();
                }
            }
            CertificatePinType::Previous => {
                if !info.previous_pins.iter().any(|candidate| candidate == pin) {
                    info.previous_pins.push(pin.to_owned());
                    trim_certificate_pins(&mut info.previous_pins);
                }
            }
        }
        if let Err(error) = self.persist_pins_locked(&peers) {
            if let Some(previous) = previous {
                peers.insert(peer_id.to_owned(), previous);
            } else {
                peers.remove(peer_id);
            }
            return Err(error);
        }
        Ok(())
    }

    pub fn rotate_pin(&self, peer_id: &str, pin: &str) -> Result<(), String> {
        self.add_pin(peer_id, pin, CertificatePinType::Current)
    }

    pub fn peer_certificate_info(&self, peer_id: &str) -> Option<PeerCertificateInfo> {
        self.peers
            .lock()
            .expect("mesh certificate pin lock")
            .get(peer_id)
            .cloned()
    }

    pub fn remove_peer_pins(&self, peer_id: &str) -> Result<(), String> {
        let mut peers = self.peers.lock().expect("mesh certificate pin lock");
        let previous = peers.remove(peer_id);
        if let Err(error) = self.persist_pins_locked(&peers) {
            if let Some(previous) = previous {
                peers.insert(peer_id.to_owned(), previous);
            }
            return Err(error);
        }
        Ok(())
    }

    pub fn cleanup_expired_pins(&self) -> Result<(), String> {
        let now = unix_seconds();
        let mut peers = self.peers.lock().expect("mesh certificate pin lock");
        let previous = peers.clone();
        peers.retain(|_, info| {
            if now.saturating_sub(info.last_rotation) >= PREVIOUS_PIN_GRACE {
                info.previous_pins.clear();
            }
            !info.current_pins.is_empty()
                || !info.previous_pins.is_empty()
                || now.saturating_sub(info.last_validation) < 90 * 24 * 60 * 60
        });
        if let Err(error) = self.persist_pins_locked(&peers) {
            *peers = previous;
            return Err(error);
        }
        Ok(())
    }

    pub fn statistics(&self) -> CertificatePinStatistics {
        let peers = self.peers.lock().expect("mesh certificate pin lock");
        CertificatePinStatistics {
            total_peers: peers.len(),
            peers_with_current_pins: peers
                .values()
                .filter(|info| !info.current_pins.is_empty())
                .count(),
            peers_with_previous_pins: peers
                .values()
                .filter(|info| !info.previous_pins.is_empty())
                .count(),
            total_current_pins: peers.values().map(|info| info.current_pins.len()).sum(),
            total_previous_pins: peers.values().map(|info| info.previous_pins.len()).sum(),
        }
    }

    fn load_persisted_pins(&self) -> Result<(), String> {
        let metadata = match fs::symlink_metadata(&self.storage_path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => {
                return Err(format!("mesh certificate pins metadata failed: {error}"));
            }
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err("mesh certificate pins must be a regular file".to_owned());
        }
        let mut options = fs::OpenOptions::new();
        options.read(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
        }
        let file = options
            .open(&self.storage_path)
            .map_err(|error| format!("mesh certificate pins open failed: {error}"))?;
        let metadata = file
            .metadata()
            .map_err(|error| format!("mesh certificate pins metadata failed: {error}"))?;
        if !metadata.is_file() {
            return Err("mesh certificate pins must be a regular file".to_owned());
        }
        if metadata.len() > MAX_CERTIFICATE_PIN_STATE_BYTES as u64 {
            return Err("mesh certificate pin state exceeds the 2 MiB limit".to_owned());
        }
        let mut bytes = Vec::new();
        file.take((MAX_CERTIFICATE_PIN_STATE_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("mesh certificate pins read failed: {error}"))?;
        if bytes.len() > MAX_CERTIFICATE_PIN_STATE_BYTES {
            return Err("mesh certificate pin state exceeds the 2 MiB limit".to_owned());
        }
        let persisted = serde_json::from_slice::<PersistedPinData>(&bytes)
            .map_err(|error| format!("mesh certificate pins are invalid: {error}"))?;
        if persisted.peer_certificates.len() > MAX_CERTIFICATE_PIN_PEERS {
            return Err("mesh certificate pin peer capacity is full".to_owned());
        }
        let mut loaded = HashMap::with_capacity(persisted.peer_certificates.len());
        for info in persisted.peer_certificates {
            let info = validate_peer_certificate_info(info)?;
            if loaded.insert(info.peer_id.clone(), info).is_some() {
                return Err("mesh certificate pin peer is duplicated".to_owned());
            }
        }
        let mut peers = self.peers.lock().expect("mesh certificate pin lock");
        peers.extend(loaded);
        Ok(())
    }

    fn persist_pins(&self) -> Result<(), String> {
        let peers = self.peers.lock().expect("mesh certificate pin lock");
        self.persist_pins_locked(&peers)
    }

    fn persist_pins_locked(
        &self,
        peers: &HashMap<String, PeerCertificateInfo>,
    ) -> Result<(), String> {
        if peers.len() > MAX_CERTIFICATE_PIN_PEERS
            || peers
                .values()
                .any(|info| validate_peer_certificate_info(info.clone()).is_err())
        {
            return Err("mesh certificate pin state exceeds its record limits".to_owned());
        }
        let body = serde_json::to_vec_pretty(&PersistedPinData {
            peer_certificates: peers.values().cloned().collect(),
            last_updated: unix_seconds(),
        })
        .map_err(|error| format!("mesh certificate pins serialization failed: {error}"))?;
        if body.len() > MAX_CERTIFICATE_PIN_STATE_BYTES {
            return Err("mesh certificate pin state exceeds the 2 MiB limit".to_owned());
        }

        crate::write_file_atomic(&self.storage_path, body)
            .map_err(|error| format!("mesh certificate pins write failed: {error}"))
    }
}

fn validate_certificate_peer_id(peer_id: &str) -> Result<(), String> {
    if peer_id.is_empty()
        || peer_id.len() > MAX_CERTIFICATE_PEER_ID_BYTES
        || peer_id.chars().any(char::is_control)
    {
        return Err("certificate pin peer ID is invalid or too long".to_owned());
    }
    Ok(())
}

fn validate_certificate_pin_text(pin: &str) -> Result<(), String> {
    if pin.is_empty() || pin.len() > MAX_CERTIFICATE_PIN_BYTES {
        return Err("certificate pin is invalid or too long".to_owned());
    }
    let decoded = STANDARD
        .decode(pin.as_bytes())
        .map_err(|_| "certificate pin is not valid base64".to_owned())?;
    if decoded.len() != 32 {
        return Err("certificate pin must encode a SHA-256 digest".to_owned());
    }
    Ok(())
}

fn trim_certificate_pins(pins: &mut Vec<String>) {
    if pins.len() > MAX_CERTIFICATE_PINS_PER_PEER {
        let excess = pins.len() - MAX_CERTIFICATE_PINS_PER_PEER;
        pins.drain(0..excess);
    }
}

fn validate_peer_certificate_info(
    mut info: PeerCertificateInfo,
) -> Result<PeerCertificateInfo, String> {
    info.peer_id = info.peer_id.trim().to_owned();
    validate_certificate_peer_id(&info.peer_id)?;
    if info.current_pins.len() > MAX_CERTIFICATE_PINS_PER_PEER
        || info.previous_pins.len() > MAX_CERTIFICATE_PINS_PER_PEER
    {
        return Err("certificate pin list capacity is full".to_owned());
    }
    for pin in info.current_pins.iter().chain(&info.previous_pins) {
        validate_certificate_pin_text(pin)?;
    }
    Ok(info)
}

/// Endpoint-scoped certificate pin validation.
pub struct EndpointCertificatePinValidator;

impl EndpointCertificatePinValidator {
    pub fn validate(
        endpoint: SocketAddr,
        certificate_der: &[u8],
        trusted_pins: &BTreeMap<String, Vec<String>>,
    ) -> bool {
        trusted_pins.get(&endpoint.to_string()).is_some_and(|pins| {
            SecurityUtils::validate_certificate_pin_and_time(certificate_der, pins)
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecurityEvent {
    pub kind: String,
    pub peer_id: String,
    pub fields: BTreeMap<String, String>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum SecuritySinkSeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Clone, Debug)]
pub struct SecuritySinkEvent {
    pub event_type: String,
    pub severity: SecuritySinkSeverity,
    pub message: String,
    pub ip_address: Option<IpAddr>,
    pub username: Option<String>,
    pub source: Option<String>,
    pub details: BTreeMap<String, String>,
    pub timestamp: SystemTime,
}

impl SecuritySinkEvent {
    pub fn new(
        event_type: impl Into<String>,
        severity: SecuritySinkSeverity,
        message: impl Into<String>,
        ip_address: Option<IpAddr>,
        username: Option<String>,
        source: Option<String>,
    ) -> Self {
        Self {
            event_type: event_type.into(),
            severity,
            message: message.into(),
            ip_address,
            username,
            source,
            details: BTreeMap::new(),
            timestamp: SystemTime::now(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SecuritySinkStats {
    pub total_events: usize,
    pub events_last_hour: usize,
    pub critical_events: usize,
    pub high_events: usize,
    pub medium_events: usize,
    pub low_events: usize,
    pub unique_ips: usize,
    pub unique_users: usize,
}

#[derive(Debug, Default)]
struct SecurityEventSinkState {
    events: VecDeque<SecuritySinkEvent>,
    event_counts: HashMap<String, u64>,
}

/// Bounded security event aggregation owned by the live security state.
/// Frozen SecurityEventSink is intentionally process-local; it does not write
/// a durable event file, so restart starts with an empty sink.
#[derive(Clone, Debug, Default)]
pub struct SecurityEventSink {
    state: Arc<Mutex<SecurityEventSinkState>>,
}

impl SecurityEventSink {
    pub const MAX_EVENTS: usize = 10_000;

    pub fn new() -> Self {
        Self::default()
    }

    pub fn report(&self, event: SecuritySinkEvent) {
        let mut state = self.state.lock().expect("security event sink lock");
        *state
            .event_counts
            .entry(event.event_type.clone())
            .or_default() += 1;
        state.events.push_back(event);
        while state.events.len() > Self::MAX_EVENTS {
            state.events.pop_front();
        }
    }

    pub fn report_async(&self, event: SecuritySinkEvent) {
        self.report(event);
    }

    pub fn recent_events(
        &self,
        count: usize,
        minimum_severity: SecuritySinkSeverity,
    ) -> Vec<SecuritySinkEvent> {
        self.state
            .lock()
            .expect("security event sink lock")
            .events
            .iter()
            .rev()
            .filter(|event| event.severity >= minimum_severity)
            .take(count)
            .cloned()
            .collect()
    }

    pub fn events_for_ip(&self, ip: IpAddr, count: usize) -> Vec<SecuritySinkEvent> {
        self.state
            .lock()
            .expect("security event sink lock")
            .events
            .iter()
            .rev()
            .filter(|event| event.ip_address == Some(ip))
            .take(count)
            .cloned()
            .collect()
    }

    pub fn events_for_user(&self, username: &str, count: usize) -> Vec<SecuritySinkEvent> {
        self.state
            .lock()
            .expect("security event sink lock")
            .events
            .iter()
            .rev()
            .filter(|event| {
                event
                    .username
                    .as_deref()
                    .is_some_and(|value| value.eq_ignore_ascii_case(username))
            })
            .take(count)
            .cloned()
            .collect()
    }

    pub fn stats(&self) -> SecuritySinkStats {
        let state = self.state.lock().expect("security event sink lock");
        let cutoff = SystemTime::now()
            .checked_sub(Duration::from_secs(60 * 60))
            .unwrap_or(UNIX_EPOCH);
        let mut ips = std::collections::HashSet::new();
        let mut users = std::collections::HashSet::new();
        let mut stats = SecuritySinkStats {
            total_events: state.events.len(),
            ..SecuritySinkStats::default()
        };
        for event in &state.events {
            if event.timestamp > cutoff {
                stats.events_last_hour += 1;
            }
            match event.severity {
                SecuritySinkSeverity::Critical => stats.critical_events += 1,
                SecuritySinkSeverity::High => stats.high_events += 1,
                SecuritySinkSeverity::Medium => stats.medium_events += 1,
                SecuritySinkSeverity::Low => stats.low_events += 1,
                SecuritySinkSeverity::Info => {}
            }
            if let Some(ip) = event.ip_address {
                ips.insert(ip);
            }
            if let Some(username) = event.username.as_deref() {
                users.insert(username.to_ascii_lowercase());
            }
        }
        stats.unique_ips = ips.len();
        stats.unique_users = users.len();
        stats
    }

    pub fn reset(&self) {
        let mut state = self.state.lock().expect("security event sink lock");
        state.events.clear();
        state.event_counts.clear();
    }
}

/// In-memory structured security event sink. Peer identifiers are redacted at
/// the boundary so diagnostics cannot accidentally expose full identities.
#[derive(Debug, Default)]
pub struct SecurityEventLogger {
    events: Mutex<Vec<SecurityEvent>>,
}

impl SecurityEventLogger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(
        &self,
        kind: &str,
        peer_id: &str,
        fields: impl IntoIterator<Item = (String, String)>,
    ) {
        self.events
            .lock()
            .expect("mesh security event lock")
            .push(SecurityEvent {
                kind: kind.to_owned(),
                peer_id: redact_peer_id(peer_id),
                fields: fields.into_iter().collect(),
            });
    }

    pub fn log_rate_limit_violation(
        &self,
        peer_id: &str,
        service: &str,
        current: u64,
        maximum: u64,
    ) {
        self.record(
            "rate-limit-violation",
            peer_id,
            [
                ("service".to_owned(), service.to_owned()),
                ("current".to_owned(), current.to_string()),
                ("max".to_owned(), maximum.to_string()),
            ],
        );
    }

    pub fn log_payload_size_violation(&self, peer_id: &str, service: &str, size: usize) {
        self.record(
            "payload-size-violation",
            peer_id,
            [
                ("service".to_owned(), service.to_owned()),
                ("size".to_owned(), size.to_string()),
            ],
        );
    }

    pub fn log_unauthorized_access(&self, peer_id: &str, service: &str, reason: &str) {
        self.record(
            "unauthorized-access",
            peer_id,
            [
                ("service".to_owned(), service.to_owned()),
                ("reason".to_owned(), reason.to_owned()),
            ],
        );
    }

    pub fn snapshot(&self) -> Vec<SecurityEvent> {
        self.events
            .lock()
            .expect("mesh security event lock")
            .clone()
    }
}

fn redact_peer_id(peer_id: &str) -> String {
    let chars = peer_id.chars().collect::<Vec<_>>();
    if chars.len() <= 8 {
        return "***".to_owned();
    }
    let prefix = chars[..4].iter().collect::<String>();
    let suffix = chars[chars.len() - 4..].iter().collect::<String>();
    format!("{prefix}…{suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_bucket_and_dht_limits_match_frozen_capacities() {
        let limiter = std::sync::Arc::new(RateLimiter::new());
        assert!(limiter.try_consume("bucket", 2, 2, 0.0));
        assert!(!limiter.try_consume("bucket", 1, 2, 0.0));
        assert_eq!(limiter.current_tokens("bucket"), 0);
        let dht = DhtRateLimiter::new(limiter);
        assert!(dht.should_allow_descriptor_publish("peer"));
        assert_eq!(dht.statistics().active_buckets, 2);
    }

    #[test]
    fn replay_and_connection_limits_are_atomic() {
        let replay = ReplayCache::new(Duration::from_secs(60), 8);
        assert!(!replay.check_and_record("message"));
        assert!(replay.check_and_record("message"));
        assert!(!replay.check_and_record(""));

        let limiter = ConnectionRateLimiter::new(Duration::from_secs(60), 2);
        assert!(limiter.is_connection_allowed("peer"));
        limiter.record_failure("peer");
        limiter.record_failure("peer");
        assert!(!limiter.is_connection_allowed("peer"));
        limiter.record_success("peer");
        assert!(limiter.is_connection_allowed("peer"));
    }

    #[test]
    fn replay_cache_enforces_configured_capacity_for_unique_messages() {
        let replay = ReplayCache::new(Duration::from_secs(60), 2);
        for index in 0..100 {
            assert!(!replay.check_and_record(&format!("message-{index}")));
        }
        assert_eq!(replay.cache_size(), 2);
    }

    #[test]
    fn transport_policy_prefers_most_specific_match() {
        let manager = TransportPolicyManager::new();
        manager.add_or_update_policy(TransportPolicy {
            peer_id: None,
            pod_id: Some("pod".to_owned()),
            disable_clearnet: true,
            is_enabled: true,
            ..TransportPolicy::default()
        });
        manager.add_or_update_policy(TransportPolicy {
            peer_id: Some("peer".to_owned()),
            pod_id: Some("pod".to_owned()),
            allowed_transport_types: Some(vec![MeshTransportType::TorOnionQuic]),
            is_enabled: true,
            ..TransportPolicy::default()
        });
        let policy = manager.applicable_policy("peer", Some("pod")).unwrap();
        assert_eq!(
            policy.allowed_transport_types,
            Some(vec![MeshTransportType::TorOnionQuic])
        );
        assert!(!policy.is_transport_allowed(
            MeshTransportType::DirectQuic,
            MeshTransportOptions {
                enable_direct: true,
                tor_enabled: true,
                i2p_enabled: false,
            }
        ));
    }

    #[test]
    fn security_event_logger_redacts_peer_identity() {
        let logger = SecurityEventLogger::new();
        logger.log_unauthorized_access("full-secret-peer-id", "mesh", "bad key");
        let events = logger.snapshot();
        assert_eq!(events.len(), 1);
        assert!(!format!("{events:?}").contains("full-secret-peer-id"));
        assert_eq!(events[0].kind, "unauthorized-access");
    }

    #[test]
    fn safe_json_enforces_size_and_depth() {
        let value: BTreeMap<String, u32> =
            SecurityUtils::parse_json_safely("{\"value\":1}", 64, 2).unwrap();
        assert_eq!(value["value"], 1);
        assert!(SecurityUtils::parse_json_safely::<serde_json::Value>("{}", 1, 32).is_err());
        assert!(SecurityUtils::parse_json_safely::<serde_json::Value>("[[[0]]]", 64, 2).is_err());
    }

    #[test]
    fn certificate_pin_mutations_roll_back_when_persistence_fails() {
        let root = std::env::temp_dir().join(format!(
            "slskr-certificate-pin-rollback-{}",
            uuid::Uuid::new_v4()
        ));
        let mut manager = CertificatePinManager::new(&root).unwrap();
        let first_pin = STANDARD.encode([1u8; 32]);
        let second_pin = STANDARD.encode([2u8; 32]);
        manager
            .add_pin("peer", &first_pin, CertificatePinType::Current)
            .unwrap();

        let failed_storage_path = root.join("mesh/certificate-pins-directory");
        std::fs::create_dir(&failed_storage_path).unwrap();
        manager.storage_path = failed_storage_path;

        assert!(manager
            .add_pin("peer", &second_pin, CertificatePinType::Current)
            .is_err());
        let info = manager.peer_certificate_info("peer").unwrap();
        assert_eq!(info.current_pins, vec![first_pin]);
        assert!(info.previous_pins.is_empty());

        assert!(manager.remove_peer_pins("peer").is_err());
        assert!(manager.peer_certificate_info("peer").is_some());

        {
            let mut peers = manager.peers.lock().unwrap();
            peers.insert(
                "expired".to_owned(),
                PeerCertificateInfo {
                    peer_id: "expired".to_owned(),
                    previous_pins: vec![second_pin],
                    last_rotation: 0,
                    last_validation: 0,
                    ..PeerCertificateInfo::default()
                },
            );
        }
        assert!(manager.cleanup_expired_pins().is_err());
        assert!(manager.peer_certificate_info("expired").is_some());

        std::fs::remove_dir_all(root).unwrap();
    }
}
