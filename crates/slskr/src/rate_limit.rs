//! Rate limiting middleware for HTTP API

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
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

const MAX_USER_WINDOWS: usize = 16_384;
const MAX_IP_WINDOWS: usize = 16_384;

impl RateLimiter {
    /// Create new rate limiter
    pub fn new(mut config: RateLimitConfig) -> Self {
        config.window_seconds = config.window_seconds.max(1);
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
        let key = user_key(username);
        let now = Instant::now();
        let mut windows = self.user_windows.write().await;
        if !windows.contains_key(&key) && windows.len() >= MAX_USER_WINDOWS {
            windows.retain(|_, window| now < window.reset_at);
            if windows.len() >= MAX_USER_WINDOWS {
                return false;
            }
        }

        let window = windows.entry(key).or_insert_with(|| RequestWindow {
            count: 0,
            reset_at: reset_deadline(now, self.config.window_seconds),
        });

        // Reset window if expired
        if now >= window.reset_at {
            window.count = 0;
            window.reset_at = reset_deadline(now, self.config.window_seconds);
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
        let key = ip_key(remote_addr).unwrap_or_else(|| "unknown".to_string());

        let now = Instant::now();
        let mut windows = self.ip_windows.write().await;
        if !windows.contains_key(&key) && windows.len() >= MAX_IP_WINDOWS {
            windows.retain(|_, window| now < window.reset_at);
            if windows.len() >= MAX_IP_WINDOWS {
                return false;
            }
        }

        let window = windows.entry(key).or_insert_with(|| RequestWindow {
            count: 0,
            reset_at: reset_deadline(now, self.config.window_seconds),
        });

        // Reset window if expired
        if now >= window.reset_at {
            window.count = 0;
            window.reset_at = reset_deadline(now, self.config.window_seconds);
        }

        // Check limit
        if window.count >= max_requests {
            return false;
        }

        window.count += 1;
        true
    }

    /// Get remaining requests for user or IP
    pub async fn get_remaining(&self, remote_addr: Option<SocketAddr>, user: Option<&str>) -> u32 {
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
        let key = user_key(username);
        let now = Instant::now();
        let windows = self.user_windows.read().await;

        if let Some(window) = windows.get(&key) {
            if now < window.reset_at {
                return max_requests.saturating_sub(window.count);
            }
        }

        max_requests
    }

    async fn get_ip_remaining(&self, remote_addr: Option<SocketAddr>) -> u32 {
        let max_requests = self.config.max_requests_anonymous;
        let key = ip_key(remote_addr).unwrap_or_else(|| "unknown".to_string());

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
    pub async fn get_reset_time(&self, remote_addr: Option<SocketAddr>, user: Option<&str>) -> u64 {
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
        let key = user_key(username);
        let now = Instant::now();
        let windows = self.user_windows.read().await;

        if let Some(window) = windows.get(&key) {
            if now < window.reset_at {
                return duration_ceiling_seconds(window.reset_at.duration_since(now));
            }
        }

        0
    }

    async fn get_ip_reset_time(&self, remote_addr: Option<SocketAddr>) -> u64 {
        let key = ip_key(remote_addr).unwrap_or_else(|| "unknown".to_string());

        let now = Instant::now();
        let windows = self.ip_windows.read().await;

        if let Some(window) = windows.get(&key) {
            if now < window.reset_at {
                return duration_ceiling_seconds(window.reset_at.duration_since(now));
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
        let ip_requests = ip_windows
            .values()
            .fold(0_u32, |total, window| total.saturating_add(window.count));
        let ip_max = ip_windows.values().map(|w| w.count).max().unwrap_or(0);

        let user_entries = user_windows.len();
        let user_requests = user_windows
            .values()
            .fold(0_u32, |total, window| total.saturating_add(window.count));
        let user_max = user_windows.values().map(|w| w.count).max().unwrap_or(0);

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

fn ip_key(remote_addr: Option<SocketAddr>) -> Option<String> {
    remote_addr.map(|addr| match addr.ip() {
        IpAddr::V4(ip) => ip.to_string(),
        IpAddr::V6(ip) => ip.to_string(),
    })
}

fn user_key(username: &str) -> String {
    username.to_ascii_lowercase()
}

fn duration_ceiling_seconds(duration: Duration) -> u64 {
    duration
        .as_secs()
        .saturating_add(u64::from(duration.subsec_nanos() != 0))
}

fn reset_deadline(now: Instant, window_seconds: u64) -> Instant {
    now.checked_add(Duration::from_secs(window_seconds))
        .unwrap_or_else(|| farthest_deadline(now))
}

fn farthest_deadline(now: Instant) -> Instant {
    let mut low = Duration::ZERO;
    let mut high = Duration::MAX;
    while low < high {
        let midpoint = low + (high - low) / 2 + Duration::from_nanos(1);
        if now.checked_add(midpoint).is_some() {
            low = midpoint;
        } else {
            high = midpoint - Duration::from_nanos(1);
        }
    }
    now.checked_add(low).unwrap_or(now)
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

        let addr = Some(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            8080,
        ));

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

        let addr = Some(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            8080,
        ));

        for i in 0..5 {
            assert!(
                limiter.check_rate_limit(addr, None).await,
                "Request {} should pass",
                i + 1
            );
        }

        assert!(
            !limiter.check_rate_limit(addr, None).await,
            "6th request should fail"
        );
    }

    #[tokio::test]
    async fn zero_window_does_not_disable_rate_limiting() {
        let limiter = RateLimiter::new(RateLimitConfig {
            max_requests_anonymous: 1,
            window_seconds: 0,
            enabled: true,
            ..Default::default()
        });

        assert_eq!(limiter.config.window_seconds, 1);
        assert!(limiter.check_rate_limit(None, None).await);
        assert!(!limiter.check_rate_limit(None, None).await);
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
            assert!(
                limiter.check_rate_limit(None, Some("testuser")).await,
                "Request {} should pass",
                i + 1
            );
        }

        assert!(
            !limiter.check_rate_limit(None, Some("testuser")).await,
            "6th request should fail"
        );
    }

    #[tokio::test]
    async fn authenticated_user_limit_is_case_insensitive() {
        let limiter = RateLimiter::new(RateLimitConfig {
            max_requests_authenticated: 2,
            window_seconds: 60,
            enabled: true,
            ..Default::default()
        });

        assert!(limiter.check_rate_limit(None, Some("Alice")).await);
        assert_eq!(limiter.get_remaining(None, Some("ALICE")).await, 1);
        assert!(limiter.check_rate_limit(None, Some("alice")).await);
        assert_eq!(limiter.get_remaining(None, Some("aLiCe")).await, 0);
        assert!(!limiter.check_rate_limit(None, Some("ALICE")).await);
        assert!(limiter.get_reset_time(None, Some("alice")).await > 0);
        assert_eq!(limiter.user_windows.read().await.len(), 1);
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

        let addr1 = Some(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            8080,
        ));
        let addr2 = Some(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2)),
            8080,
        ));

        assert!(limiter.check_rate_limit(addr1, None).await);
        assert!(limiter.check_rate_limit(addr2, None).await);
        assert!(limiter.check_rate_limit(addr1, None).await);
        assert!(limiter.check_rate_limit(addr2, None).await);

        assert!(!limiter.check_rate_limit(addr1, None).await);
        assert!(!limiter.check_rate_limit(addr2, None).await);
    }

    #[tokio::test]
    async fn test_rate_limit_ip_window_cap() {
        let config = RateLimitConfig {
            max_requests_anonymous: 10,
            window_seconds: 60,
            enabled: true,
            ..Default::default()
        };
        let limiter = RateLimiter::new(config);

        for index in 0..MAX_IP_WINDOWS {
            let addr = Some(SocketAddr::new(
                IpAddr::V4(Ipv4Addr::new(
                    10,
                    ((index >> 16) & 0xff) as u8,
                    ((index >> 8) & 0xff) as u8,
                    (index & 0xff) as u8,
                )),
                8080,
            ));
            assert!(limiter.check_rate_limit(addr, None).await);
        }

        let over_cap = Some(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(11, 0, 0, 1)),
            8080,
        ));
        assert!(!limiter.check_rate_limit(over_cap, None).await);
    }

    #[tokio::test]
    async fn full_ip_window_table_reclaims_expired_entries() {
        let limiter = RateLimiter::new(RateLimitConfig::default());
        let reset_at = Instant::now();
        let mut windows = limiter.ip_windows.write().await;
        for index in 0..MAX_IP_WINDOWS {
            windows.insert(
                format!("expired-{index}"),
                RequestWindow { count: 1, reset_at },
            );
        }
        drop(windows);

        let addr = Some(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1)),
            8080,
        ));
        assert!(limiter.check_rate_limit(addr, None).await);
        assert_eq!(limiter.ip_windows.read().await.len(), 1);
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

        let addr = Some(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            8080,
        ));

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

        let addr = Some(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            8080,
        ));

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

    #[test]
    fn reset_duration_rounds_up_while_window_is_active() {
        assert_eq!(duration_ceiling_seconds(Duration::ZERO), 0);
        assert_eq!(duration_ceiling_seconds(Duration::from_nanos(1)), 1);
        assert_eq!(duration_ceiling_seconds(Duration::from_millis(999)), 1);
        assert_eq!(duration_ceiling_seconds(Duration::from_millis(1001)), 2);
    }

    #[tokio::test]
    async fn rate_limit_stats_saturate_instead_of_overflowing() {
        let limiter = RateLimiter::new(RateLimitConfig::default());
        let reset_at = Instant::now() + Duration::from_secs(60);
        let mut windows = limiter.ip_windows.write().await;
        windows.insert(
            "first".to_owned(),
            RequestWindow {
                count: u32::MAX,
                reset_at,
            },
        );
        windows.insert("second".to_owned(), RequestWindow { count: 1, reset_at });
        drop(windows);

        assert_eq!(limiter.stats().await.ip_requests, u32::MAX);
    }

    #[tokio::test]
    async fn unrepresentable_window_does_not_panic_on_first_request() {
        let limiter = RateLimiter::new(RateLimitConfig {
            max_requests_anonymous: 1,
            window_seconds: u64::MAX,
            ..RateLimitConfig::default()
        });

        assert!(limiter.check_rate_limit(None, None).await);
        assert!(!limiter.check_rate_limit(None, None).await);
        assert!(limiter.get_reset_time(None, None).await > 0);
    }
}
