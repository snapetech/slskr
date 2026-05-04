//! Rate limiting middleware for HTTP API

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Rate limiter configuration
#[derive(Debug, Clone, Copy)]
pub struct RateLimitConfig {
    /// Maximum requests per window (anonymous/IP-based)
    pub max_requests_anonymous: u32,
    /// Maximum requests per window (authenticated/user-based)
    pub max_requests_authenticated: u32,
    /// Time window in seconds
    pub window_seconds: u64,
    /// Whether to enable rate limiting
    pub enabled: bool,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests_anonymous: 1000,
            max_requests_authenticated: 5000,
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

/// Rate limiter for tracking requests per IP and per user
#[derive(Debug)]
pub struct RateLimiter {
    config: RateLimitConfig,
    ip_windows: Arc<RwLock<HashMap<String, RequestWindow>>>,
    user_windows: Arc<RwLock<HashMap<String, RequestWindow>>>,
}

impl RateLimiter {
    /// Create new rate limiter
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            ip_windows: Arc::new(RwLock::new(HashMap::new())),
            user_windows: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if request should be allowed (per-user for authenticated, per-IP for anonymous)
    pub async fn check_rate_limit(
        &self,
        remote_addr: Option<SocketAddr>,
        user: Option<&str>,
    ) -> bool {
        if !self.config.enabled {
            return true;
        }

        if let Some(username) = user {
            self.check_user_limit(username).await
        } else {
            self.check_ip_limit(remote_addr).await
        }
    }

    /// Check rate limit for authenticated user
    async fn check_user_limit(&self, username: &str) -> bool {
        let max_requests = self.config.max_requests_authenticated;
        let now = Instant::now();
        let mut windows = self.user_windows.write().await;

        let window = windows.entry(username.to_string()).or_insert_with(|| RequestWindow {
            count: 0,
            reset_at: now + Duration::from_secs(self.config.window_seconds),
        });

        // Reset window if expired
        if now >= window.reset_at {
            window.count = 0;
            window.reset_at = now + Duration::from_secs(self.config.window_seconds);
        }

        // Check limit
        if window.count >= max_requests {
            return false;
        }

        window.count += 1;
        true
    }

    /// Check rate limit for anonymous IP
    async fn check_ip_limit(&self, remote_addr: Option<SocketAddr>) -> bool {
        let max_requests = self.config.max_requests_anonymous;
        let key = remote_addr
            .map(|addr| addr.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let now = Instant::now();
        let mut windows = self.ip_windows.write().await;

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
        if window.count >= max_requests {
            return false;
        }

        window.count += 1;
        true
    }

    /// Get remaining requests for user or IP
    pub async fn get_remaining(
        &self,
        remote_addr: Option<SocketAddr>,
        user: Option<&str>,
    ) -> u32 {
        if !self.config.enabled {
            return if user.is_some() {
                self.config.max_requests_authenticated
            } else {
                self.config.max_requests_anonymous
            };
        }

        if let Some(username) = user {
            self.get_user_remaining(username).await
        } else {
            self.get_ip_remaining(remote_addr).await
        }
    }

    async fn get_user_remaining(&self, username: &str) -> u32 {
        let max_requests = self.config.max_requests_authenticated;
        let now = Instant::now();
        let windows = self.user_windows.read().await;

        if let Some(window) = windows.get(username) {
            if now < window.reset_at {
                return max_requests.saturating_sub(window.count);
            }
        }

        max_requests
    }

    async fn get_ip_remaining(&self, remote_addr: Option<SocketAddr>) -> u32 {
        let max_requests = self.config.max_requests_anonymous;
        let key = remote_addr
            .map(|addr| addr.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let now = Instant::now();
        let windows = self.ip_windows.read().await;

        if let Some(window) = windows.get(&key) {
            if now < window.reset_at {
                return max_requests.saturating_sub(window.count);
            }
        }

        max_requests
    }

    /// Get time until reset for user or IP
    pub async fn get_reset_time(
        &self,
        remote_addr: Option<SocketAddr>,
        user: Option<&str>,
    ) -> u64 {
        if !self.config.enabled {
            return 0;
        }

        if let Some(username) = user {
            self.get_user_reset_time(username).await
        } else {
            self.get_ip_reset_time(remote_addr).await
        }
    }

    async fn get_user_reset_time(&self, username: &str) -> u64 {
        let now = Instant::now();
        let windows = self.user_windows.read().await;

        if let Some(window) = windows.get(username) {
            if now < window.reset_at {
                return window.reset_at.duration_since(now).as_secs();
            }
        }

        0
    }

    async fn get_ip_reset_time(&self, remote_addr: Option<SocketAddr>) -> u64 {
        let key = remote_addr
            .map(|addr| addr.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let now = Instant::now();
        let windows = self.ip_windows.read().await;

        if let Some(window) = windows.get(&key) {
            if now < window.reset_at {
                return window.reset_at.duration_since(now).as_secs();
            }
        }

        0
    }

    /// Clear all rate limit windows
    pub async fn reset(&self) {
        let mut ip_windows = self.ip_windows.write().await;
        ip_windows.clear();
        let mut user_windows = self.user_windows.write().await;
        user_windows.clear();
    }

    /// Get statistics
    pub async fn stats(&self) -> RateLimitStats {
        let ip_windows = self.ip_windows.read().await;
        let user_windows = self.user_windows.read().await;

        let ip_entries = ip_windows.len();
        let ip_requests: u32 = ip_windows.iter().map(|(_, w)| w.count).sum();
        let ip_max = ip_windows.iter().map(|(_, w)| w.count).max().unwrap_or(0);

        let user_entries = user_windows.len();
        let user_requests: u32 = user_windows.iter().map(|(_, w)| w.count).sum();
        let user_max = user_windows.iter().map(|(_, w)| w.count).max().unwrap_or(0);

        RateLimitStats {
            ip_entries,
            ip_requests,
            ip_max_requests_seen: ip_max,
            user_entries,
            user_requests,
            user_max_requests_seen: user_max,
            window_size_seconds: self.config.window_seconds,
            max_requests_anonymous: self.config.max_requests_anonymous,
            max_requests_authenticated: self.config.max_requests_authenticated,
        }
    }

    /// Cleanup expired windows
    pub async fn cleanup(&self) {
        let now = Instant::now();
        let mut ip_windows = self.ip_windows.write().await;
        ip_windows.retain(|_, window| now < window.reset_at);
        let mut user_windows = self.user_windows.write().await;
        user_windows.retain(|_, window| now < window.reset_at);
    }
}

/// Rate limit statistics
#[derive(Debug, Clone)]
pub struct RateLimitStats {
    pub ip_entries: usize,
    pub ip_requests: u32,
    pub ip_max_requests_seen: u32,
    pub user_entries: usize,
    pub user_requests: u32,
    pub user_max_requests_seen: u32,
    pub window_size_seconds: u64,
    pub max_requests_anonymous: u32,
    pub max_requests_authenticated: u32,
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
            assert!(limiter.check_rate_limit(addr, None).await);
        }
    }

    #[tokio::test]
    async fn test_rate_limit_enforcement_ip() {
        let config = RateLimitConfig {
            max_requests_anonymous: 5,
            window_seconds: 60,
            enabled: true,
            ..Default::default()
        };
        let limiter = RateLimiter::new(config);

        let addr = Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080));

        for i in 0..5 {
            assert!(limiter.check_rate_limit(addr, None).await, "Request {} should pass", i + 1);
        }

        assert!(!limiter.check_rate_limit(addr, None).await, "6th request should fail");
    }

    #[tokio::test]
    async fn test_rate_limit_enforcement_user() {
        let config = RateLimitConfig {
            max_requests_authenticated: 5,
            window_seconds: 60,
            enabled: true,
            ..Default::default()
        };
        let limiter = RateLimiter::new(config);

        for i in 0..5 {
            assert!(limiter.check_rate_limit(None, Some("testuser")).await, "Request {} should pass", i + 1);
        }

        assert!(!limiter.check_rate_limit(None, Some("testuser")).await, "6th request should fail");
    }

    #[tokio::test]
    async fn test_rate_limit_different_ips() {
        let config = RateLimitConfig {
            max_requests_anonymous: 2,
            window_seconds: 60,
            enabled: true,
            ..Default::default()
        };
        let limiter = RateLimiter::new(config);

        let addr1 = Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080));
        let addr2 = Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2)), 8080));

        assert!(limiter.check_rate_limit(addr1, None).await);
        assert!(limiter.check_rate_limit(addr2, None).await);
        assert!(limiter.check_rate_limit(addr1, None).await);
        assert!(limiter.check_rate_limit(addr2, None).await);

        assert!(!limiter.check_rate_limit(addr1, None).await);
        assert!(!limiter.check_rate_limit(addr2, None).await);
    }

    #[tokio::test]
    async fn test_rate_limit_different_users() {
        let config = RateLimitConfig {
            max_requests_authenticated: 2,
            window_seconds: 60,
            enabled: true,
            ..Default::default()
        };
        let limiter = RateLimiter::new(config);

        assert!(limiter.check_rate_limit(None, Some("user1")).await);
        assert!(limiter.check_rate_limit(None, Some("user2")).await);
        assert!(limiter.check_rate_limit(None, Some("user1")).await);
        assert!(limiter.check_rate_limit(None, Some("user2")).await);

        assert!(!limiter.check_rate_limit(None, Some("user1")).await);
        assert!(!limiter.check_rate_limit(None, Some("user2")).await);
    }

    #[tokio::test]
    async fn test_rate_limit_remaining_ip() {
        let config = RateLimitConfig {
            max_requests_anonymous: 10,
            window_seconds: 60,
            enabled: true,
            ..Default::default()
        };
        let limiter = RateLimiter::new(config);

        let addr = Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080));

        assert_eq!(limiter.get_remaining(addr, None).await, 10);
        limiter.check_rate_limit(addr, None).await;
        assert_eq!(limiter.get_remaining(addr, None).await, 9);
    }

    #[tokio::test]
    async fn test_rate_limit_remaining_user() {
        let config = RateLimitConfig {
            max_requests_authenticated: 10,
            window_seconds: 60,
            enabled: true,
            ..Default::default()
        };
        let limiter = RateLimiter::new(config);

        assert_eq!(limiter.get_remaining(None, Some("testuser")).await, 10);
        limiter.check_rate_limit(None, Some("testuser")).await;
        assert_eq!(limiter.get_remaining(None, Some("testuser")).await, 9);
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
            max_requests_anonymous: 10,
            max_requests_authenticated: 10,
            window_seconds: 60,
            enabled: true,
        };
        let limiter = RateLimiter::new(config);

        let addr = Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080));

        for _ in 0..5 {
            limiter.check_rate_limit(addr, None).await;
        }

        for _ in 0..3 {
            limiter.check_rate_limit(None, Some("user1")).await;
        }

        let stats = limiter.stats().await;
        assert_eq!(stats.ip_entries, 1);
        assert_eq!(stats.ip_requests, 5);
        assert_eq!(stats.ip_max_requests_seen, 5);
        assert_eq!(stats.user_entries, 1);
        assert_eq!(stats.user_requests, 3);
        assert_eq!(stats.user_max_requests_seen, 3);
    }
}
