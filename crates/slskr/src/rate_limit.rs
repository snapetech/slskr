//! Rate limiting middleware for HTTP API

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Rate limiter configuration
#[derive(Debug, Clone, Copy)]
pub struct RateLimitConfig {
    /// Maximum requests per window
    pub max_requests: u32,
    /// Time window in seconds
    pub window_seconds: u64,
    /// Whether to enable rate limiting
    pub enabled: bool,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 1000,
            window_seconds: 60,
            enabled: true,
        }
    }
}

/// Request tracking for rate limiting
#[derive(Debug, Clone)]
struct RequestWindow {
    count: u32,
    reset_at: Instant,
}

/// Rate limiter for tracking requests per IP
pub struct RateLimiter {
    config: RateLimitConfig,
    windows: Arc<RwLock<HashMap<String, RequestWindow>>>,
}

impl RateLimiter {
    /// Create new rate limiter
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            windows: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if request should be allowed
    pub async fn check_rate_limit(&self, remote_addr: Option<SocketAddr>) -> bool {
        if !self.config.enabled {
            return true;
        }

        let key = remote_addr
            .map(|addr| addr.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let now = Instant::now();
        let mut windows = self.windows.write().await;

        let window = windows.entry(key).or_insert_with(|| RequestWindow {
            count: 0,
            reset_at: now + Duration::from_secs(self.config.window_seconds),
        });

        // Reset window if expired
        if now >= window.reset_at {
            window.count = 0;
            window.reset_at = now + Duration::from_secs(self.config.window_seconds);
        }

        // Check limit
        if window.count >= self.config.max_requests {
            return false;
        }

        window.count += 1;
        true
    }

    /// Get remaining requests for IP
    pub async fn get_remaining(&self, remote_addr: Option<SocketAddr>) -> u32 {
        if !self.config.enabled {
            return self.config.max_requests;
        }

        let key = remote_addr
            .map(|addr| addr.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let now = Instant::now();
        let windows = self.windows.read().await;

        if let Some(window) = windows.get(&key) {
            if now < window.reset_at {
                return self.config.max_requests.saturating_sub(window.count);
            }
        }

        self.config.max_requests
    }

    /// Get time until reset for IP
    pub async fn get_reset_time(&self, remote_addr: Option<SocketAddr>) -> u64 {
        if !self.config.enabled {
            return 0;
        }

        let key = remote_addr
            .map(|addr| addr.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let now = Instant::now();
        let windows = self.windows.read().await;

        if let Some(window) = windows.get(&key) {
            if now < window.reset_at {
                return window.reset_at.duration_since(now).as_secs();
            }
        }

        0
    }

    /// Clear all rate limit windows
    pub async fn reset(&self) {
        let mut windows = self.windows.write().await;
        windows.clear();
    }

    /// Get statistics
    pub async fn stats(&self) -> RateLimitStats {
        let windows = self.windows.read().await;

        let total_entries = windows.len();
        let total_requests: u32 = windows.iter().map(|(_, w)| w.count).sum();
        let max_requests_seen = windows.iter().map(|(_, w)| w.count).max().unwrap_or(0);

        RateLimitStats {
            total_entries,
            total_requests,
            max_requests_seen,
            window_size_seconds: self.config.window_seconds,
            max_requests: self.config.max_requests,
        }
    }

    /// Cleanup expired windows
    pub async fn cleanup(&self) {
        let now = Instant::now();
        let mut windows = self.windows.write().await;
        windows.retain(|_, window| now < window.reset_at);
    }
}

/// Rate limit statistics
#[derive(Debug, Clone)]
pub struct RateLimitStats {
    pub total_entries: usize,
    pub total_requests: u32,
    pub max_requests_seen: u32,
    pub window_size_seconds: u64,
    pub max_requests: u32,
}

/// Rate limit response headers
pub struct RateLimitHeaders {
    pub limit: String,
    pub remaining: String,
    pub reset: String,
}

impl RateLimitHeaders {
    /// Create headers for rate limiting
    pub fn new(max_requests: u32, remaining: u32, reset_secs: u64) -> Self {
        Self {
            limit: format!("{}", max_requests),
            remaining: format!("{}", remaining),
            reset: format!("{}", reset_secs),
        }
    }

    /// Format for HTTP response
    pub fn to_response_headers(&self) -> String {
        format!(
            "RateLimit-Limit: {}\r\nRateLimit-Remaining: {}\r\nRateLimit-Reset: {}\r\n",
            self.limit, self.remaining, self.reset
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[tokio::test]
    async fn test_rate_limiter_creation() {
        let config = RateLimitConfig::default();
        let limiter = RateLimiter::new(config);
        assert!(limiter.config.enabled);
    }

    #[tokio::test]
    async fn test_rate_limit_disabled() {
        let config = RateLimitConfig {
            enabled: false,
            ..Default::default()
        };
        let limiter = RateLimiter::new(config);

        let addr = Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080));

        for _ in 0..2000 {
            assert!(limiter.check_rate_limit(addr).await);
        }
    }

    #[tokio::test]
    async fn test_rate_limit_enforcement() {
        let config = RateLimitConfig {
            max_requests: 5,
            window_seconds: 60,
            enabled: true,
        };
        let limiter = RateLimiter::new(config);

        let addr = Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080));

        for i in 0..5 {
            assert!(limiter.check_rate_limit(addr).await, "Request {} should pass", i + 1);
        }

        assert!(!limiter.check_rate_limit(addr).await, "6th request should fail");
    }

    #[tokio::test]
    async fn test_rate_limit_different_ips() {
        let config = RateLimitConfig {
            max_requests: 2,
            window_seconds: 60,
            enabled: true,
        };
        let limiter = RateLimiter::new(config);

        let addr1 = Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080));
        let addr2 = Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2)), 8080));

        assert!(limiter.check_rate_limit(addr1).await);
        assert!(limiter.check_rate_limit(addr2).await);
        assert!(limiter.check_rate_limit(addr1).await);
        assert!(limiter.check_rate_limit(addr2).await);

        assert!(!limiter.check_rate_limit(addr1).await);
        assert!(!limiter.check_rate_limit(addr2).await);
    }

    #[tokio::test]
    async fn test_rate_limit_remaining() {
        let config = RateLimitConfig {
            max_requests: 10,
            window_seconds: 60,
            enabled: true,
        };
        let limiter = RateLimiter::new(config);

        let addr = Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080));

        assert_eq!(limiter.get_remaining(addr).await, 10);
        limiter.check_rate_limit(addr).await;
        assert_eq!(limiter.get_remaining(addr).await, 9);
    }

    #[tokio::test]
    async fn test_rate_limit_headers() {
        let headers = RateLimitHeaders::new(100, 50, 30);
        let response = headers.to_response_headers();

        assert!(response.contains("RateLimit-Limit: 100"));
        assert!(response.contains("RateLimit-Remaining: 50"));
        assert!(response.contains("RateLimit-Reset: 30"));
    }

    #[tokio::test]
    async fn test_rate_limit_stats() {
        let config = RateLimitConfig {
            max_requests: 10,
            window_seconds: 60,
            enabled: true,
        };
        let limiter = RateLimiter::new(config);

        let addr = Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080));

        for _ in 0..5 {
            limiter.check_rate_limit(addr).await;
        }

        let stats = limiter.stats().await;
        assert_eq!(stats.total_entries, 1);
        assert_eq!(stats.total_requests, 5);
        assert_eq!(stats.max_requests_seen, 5);
    }
}
