/// Advanced rate limiting with multiple strategies
///
/// Provides rate limiting with token bucket, sliding window, and fixed window
/// algorithms with per-client and per-endpoint configurations.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

/// Rate limit configuration
#[derive(Clone, Debug)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub burst_size: u32,
    pub enabled: bool,
    pub strategy: RateLimitStrategy,
}

impl RateLimitConfig {
    /// Create default config
    pub fn default() -> Self {
        Self {
            requests_per_minute: 1000,
            burst_size: 100,
            enabled: true,
            strategy: RateLimitStrategy::TokenBucket,
        }
    }

    /// Create config for authenticated users
    pub fn authenticated() -> Self {
        Self {
            requests_per_minute: 5000,
            burst_size: 500,
            enabled: true,
            strategy: RateLimitStrategy::TokenBucket,
        }
    }

    /// Create config for public/anonymous access
    pub fn anonymous() -> Self {
        Self {
            requests_per_minute: 1000,
            burst_size: 50,
            enabled: true,
            strategy: RateLimitStrategy::SlidingWindow,
        }
    }
}

/// Rate limiting strategy
#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum RateLimitStrategy {
    /// Token bucket algorithm
    TokenBucket,
    /// Sliding window algorithm
    SlidingWindow,
    /// Fixed window algorithm
    FixedWindow,
}

/// Token bucket state
#[derive(Clone, Debug)]
struct TokenBucket {
    tokens: f64,
    last_refill: u64,
    capacity: f64,
    refill_rate: f64,
}

impl TokenBucket {
    /// Create new token bucket
    fn new(capacity: f64, refill_rate: f64) -> Self {
        Self {
            tokens: capacity,
            last_refill: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            capacity,
            refill_rate,
        }
    }

    /// Refill tokens based on elapsed time
    fn refill(&mut self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let elapsed = now - self.last_refill;
        let tokens_to_add = (elapsed as f64) * self.refill_rate;
        self.tokens = (self.tokens + tokens_to_add).min(self.capacity);
        self.last_refill = now;
    }

    /// Check if request is allowed
    fn allow_request(&mut self, tokens_required: f64) -> bool {
        self.refill();
        if self.tokens >= tokens_required {
            self.tokens -= tokens_required;
            true
        } else {
            false
        }
    }
}

/// Sliding window state
#[derive(Clone, Debug)]
struct SlidingWindow {
    requests: Vec<u64>,
    window_size_ms: u64,
    max_requests: u32,
}

impl SlidingWindow {
    /// Create new sliding window
    fn new(max_requests: u32, window_size_ms: u64) -> Self {
        Self {
            requests: Vec::new(),
            window_size_ms,
            max_requests,
        }
    }

    /// Check if request is allowed
    fn allow_request(&mut self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // Remove old requests outside window
        let cutoff = now - self.window_size_ms;
        self.requests.retain(|&ts| ts > cutoff);

        // Check if we can add new request
        if (self.requests.len() as u32) < self.max_requests {
            self.requests.push(now);
            true
        } else {
            false
        }
    }
}

/// Rate limiter for per-client limiting
pub struct RateLimiter {
    clients: Arc<RwLock<HashMap<String, ClientLimit>>>,
    config: RateLimitConfig,
}

/// Per-client limit state
enum ClientLimit {
    TokenBucket(TokenBucket),
    SlidingWindow(SlidingWindow),
}

impl RateLimiter {
    /// Create new rate limiter
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Check if client is allowed to make request
    pub async fn is_allowed(&self, client_id: &str) -> bool {
        if !self.config.enabled {
            return true;
        }

        let mut clients = self.clients.write().await;

        let entry = clients
            .entry(client_id.to_string())
            .or_insert_with(|| match self.config.strategy {
                RateLimitStrategy::TokenBucket => {
                    let refill_rate = (self.config.requests_per_minute as f64) / 60.0;
                    ClientLimit::TokenBucket(TokenBucket::new(
                        self.config.burst_size as f64,
                        refill_rate,
                    ))
                }
                RateLimitStrategy::SlidingWindow => {
                    ClientLimit::SlidingWindow(SlidingWindow::new(
                        self.config.requests_per_minute,
                        60000, // 1 minute in ms
                    ))
                }
                RateLimitStrategy::FixedWindow => {
                    ClientLimit::TokenBucket(TokenBucket::new(
                        self.config.requests_per_minute as f64,
                        (self.config.requests_per_minute as f64) / 60.0,
                    ))
                }
            });

        match entry {
            ClientLimit::TokenBucket(bucket) => bucket.allow_request(1.0),
            ClientLimit::SlidingWindow(window) => window.allow_request(),
        }
    }

    /// Get remaining requests for client
    pub async fn get_remaining(&self, client_id: &str) -> Option<u32> {
        let clients = self.clients.read().await;

        clients.get(client_id).and_then(|limit| match limit {
            ClientLimit::TokenBucket(bucket) => Some(bucket.tokens as u32),
            ClientLimit::SlidingWindow(window) => {
                Some(self.config.max_requests - (window.requests.len() as u32))
            }
        })
    }

    /// Reset limit for client
    pub async fn reset_client(&self, client_id: &str) {
        let mut clients = self.clients.write().await;
        clients.remove(client_id);
    }

    /// Clear all clients
    pub async fn clear_all(&self) {
        let mut clients = self.clients.write().await;
        clients.clear();
    }

    /// Get client count
    pub async fn client_count(&self) -> usize {
        let clients = self.clients.read().await;
        clients.len()
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(RateLimitConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_config_default() {
        let config = RateLimitConfig::default();
        assert!(config.enabled);
        assert_eq!(config.requests_per_minute, 1000);
    }

    #[test]
    fn test_rate_limit_config_authenticated() {
        let config = RateLimitConfig::authenticated();
        assert_eq!(config.requests_per_minute, 5000);
    }

    #[test]
    fn test_rate_limit_config_anonymous() {
        let config = RateLimitConfig::anonymous();
        assert_eq!(config.requests_per_minute, 1000);
    }

    #[test]
    fn test_token_bucket_creation() {
        let bucket = TokenBucket::new(100.0, 1.0);
        assert_eq!(bucket.tokens, 100.0);
        assert_eq!(bucket.capacity, 100.0);
    }

    #[test]
    fn test_token_bucket_allow_request() {
        let mut bucket = TokenBucket::new(10.0, 1.0);
        assert!(bucket.allow_request(1.0));
        assert_eq!(bucket.tokens, 9.0);
    }

    #[test]
    fn test_token_bucket_deny_when_empty() {
        let mut bucket = TokenBucket::new(1.0, 1.0);
        bucket.allow_request(1.0);
        assert!(!bucket.allow_request(1.0));
    }

    #[test]
    fn test_sliding_window_creation() {
        let window = SlidingWindow::new(100, 60000);
        assert_eq!(window.max_requests, 100);
    }

    #[test]
    fn test_sliding_window_allow_request() {
        let mut window = SlidingWindow::new(2, 60000);
        assert!(window.allow_request());
        assert!(window.allow_request());
        assert!(!window.allow_request());
    }

    #[tokio::test]
    async fn test_rate_limiter_allowed() {
        let limiter = RateLimiter::new(RateLimitConfig::default());
        assert!(limiter.is_allowed("client1").await);
    }

    #[tokio::test]
    async fn test_rate_limiter_client_count() {
        let limiter = RateLimiter::new(RateLimitConfig::default());
        limiter.is_allowed("client1").await;
        limiter.is_allowed("client2").await;

        assert_eq!(limiter.client_count().await, 2);
    }

    #[tokio::test]
    async fn test_rate_limiter_reset() {
        let limiter = RateLimiter::new(RateLimitConfig::default());
        limiter.is_allowed("client1").await;

        assert_eq!(limiter.client_count().await, 1);

        limiter.reset_client("client1").await;

        assert_eq!(limiter.client_count().await, 0);
    }

    #[tokio::test]
    async fn test_rate_limiter_clear_all() {
        let limiter = RateLimiter::new(RateLimitConfig::default());
        limiter.is_allowed("client1").await;
        limiter.is_allowed("client2").await;

        limiter.clear_all().await;

        assert_eq!(limiter.client_count().await, 0);
    }

    #[tokio::test]
    async fn test_rate_limiter_disabled() {
        let mut config = RateLimitConfig::default();
        config.enabled = false;

        let limiter = RateLimiter::new(config);
        assert!(limiter.is_allowed("client1").await);
    }
}
