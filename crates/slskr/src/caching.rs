//! Response caching for HTTP API with TTL support

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Cached response entry
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub data: String,
    pub created_at: Instant,
    pub ttl: Duration,
    pub hits: u64,
}

impl CacheEntry {
    /// Create new cache entry
    pub fn new(data: String, ttl: Duration) -> Self {
        Self {
            data,
            created_at: Instant::now(),
            ttl,
            hits: 0,
        }
    }

    /// Check if entry is expired
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.ttl
    }

    /// Get time remaining until expiration
    pub fn ttl_remaining(&self) -> Duration {
        let elapsed = self.created_at.elapsed();
        if elapsed < self.ttl {
            self.ttl - elapsed
        } else {
            Duration::from_secs(0)
        }
    }

    /// Increment hit counter
    pub fn record_hit(&mut self) {
        self.hits += 1;
    }
}

/// Cache statistics
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub total_requests: u64,
}

impl CacheStats {
    /// Get hit rate percentage
    pub fn hit_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            (self.hits as f64 / self.total_requests as f64) * 100.0
        }
    }
}

/// Response cache with TTL support
pub struct ResponseCache {
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    stats: Arc<RwLock<CacheStats>>,
    max_size: usize,
}

impl ResponseCache {
    /// Create new response cache
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(CacheStats::default())),
            max_size,
        }
    }

    /// Get cached response
    pub async fn get(&self, key: &str) -> Option<String> {
        let mut cache = self.cache.write().await;
        let mut stats = self.stats.write().await;
        stats.total_requests += 1;

        if let Some(entry) = cache.get_mut(key) {
            if entry.is_expired() {
                cache.remove(key);
                stats.misses += 1;
                return None;
            }
            entry.record_hit();
            stats.hits += 1;
            return Some(entry.data.clone());
        }

        stats.misses += 1;
        None
    }

    /// Set cached response
    pub async fn set(&self, key: String, data: String, ttl: Duration) {
        let mut cache = self.cache.write().await;

        // Evict oldest entry if cache is full
        if cache.len() >= self.max_size {
            if let Some(oldest_key) = cache
                .iter()
                .min_by_key(|(_, entry)| entry.created_at)
                .map(|(k, _)| k.clone())
            {
                cache.remove(&oldest_key);
                let mut stats = self.stats.write().await;
                stats.evictions += 1;
            }
        }

        cache.insert(key, CacheEntry::new(data, ttl));
    }

    /// Clear entire cache
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// Invalidate specific key
    pub async fn invalidate(&self, key: &str) {
        let mut cache = self.cache.write().await;
        cache.remove(key);
    }

    /// Get cache size
    pub async fn size(&self) -> usize {
        let cache = self.cache.read().await;
        cache.len()
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        self.stats.read().await.clone()
    }

    /// Cleanup expired entries
    pub async fn cleanup_expired(&self) {
        let mut cache = self.cache.write().await;
        cache.retain(|_, entry| !entry.is_expired());
    }

    /// Get entry details (for monitoring)
    pub async fn get_entry_info(&self, key: &str) -> Option<(u64, Duration, u64)> {
        let cache = self.cache.read().await;
        cache.get(key).map(|entry| {
            (
                entry.hits,
                entry.ttl_remaining(),
                entry.data.len() as u64,
            )
        })
    }
}

/// Cache invalidation trigger
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheInvalidationTrigger {
    Search,
    Transfer,
    Message,
    Room,
    Browse,
    Config,
}

/// Cache configuration
pub struct CacheConfig {
    pub version_ttl: Duration,
    pub capabilities_ttl: Duration,
    pub config_ttl: Duration,
    pub stats_ttl: Duration,
    pub events_ttl: Duration,
    pub max_cache_size: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            version_ttl: Duration::from_secs(3600),      // 1 hour
            capabilities_ttl: Duration::from_secs(3600), // 1 hour
            config_ttl: Duration::from_secs(300),        // 5 minutes
            stats_ttl: Duration::from_secs(60),          // 1 minute
            events_ttl: Duration::from_secs(30),         // 30 seconds
            max_cache_size: 1000,
        }
    }
}

/// Cache key builder
pub struct CacheKeyBuilder;

impl CacheKeyBuilder {
    /// Build cache key for endpoint
    pub fn endpoint(method: &str, path: &str, query: Option<&str>) -> String {
        match query {
            Some(q) => format!("{}:{}:{}", method, path, q),
            None => format!("{}:{}", method, path),
        }
    }

    /// Build cache key for version endpoint
    pub fn version() -> String {
        "GET:/api/version".to_string()
    }

    /// Build cache key for capabilities endpoint
    pub fn capabilities() -> String {
        "GET:/api/capabilities".to_string()
    }

    /// Build cache key for config endpoint
    pub fn config() -> String {
        "GET:/api/config".to_string()
    }

    /// Build cache key for stats endpoint
    pub fn stats() -> String {
        "GET:/api/stats".to_string()
    }

    /// Build cache key for events endpoint
    pub fn events(offset: Option<usize>) -> String {
        match offset {
            Some(o) => format!("GET:/api/events:{}", o),
            None => "GET:/api/events".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_entry_creation() {
        let entry = CacheEntry::new("data".to_string(), Duration::from_secs(60));
        assert!(!entry.is_expired());
    }

    #[test]
    fn test_cache_entry_expiration() {
        let mut entry = CacheEntry::new("data".to_string(), Duration::from_millis(1));
        std::thread::sleep(Duration::from_millis(2));
        assert!(entry.is_expired());
    }

    #[test]
    fn test_cache_entry_hit_counting() {
        let mut entry = CacheEntry::new("data".to_string(), Duration::from_secs(60));
        assert_eq!(entry.hits, 0);
        entry.record_hit();
        assert_eq!(entry.hits, 1);
    }

    #[test]
    fn test_cache_stats_hit_rate() {
        let mut stats = CacheStats::default();
        stats.hits = 75;
        stats.total_requests = 100;
        assert_eq!(stats.hit_rate(), 75.0);
    }

    #[tokio::test]
    async fn test_cache_set_and_get() {
        let cache = ResponseCache::new(10);
        cache
            .set(
                "key1".to_string(),
                "value1".to_string(),
                Duration::from_secs(60),
            )
            .await;

        let result = cache.get("key1").await;
        assert_eq!(result, Some("value1".to_string()));
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        let cache = ResponseCache::new(10);
        cache
            .set(
                "key1".to_string(),
                "value1".to_string(),
                Duration::from_millis(10),
            )
            .await;

        tokio::time::sleep(Duration::from_millis(20)).await;
        let result = cache.get("key1").await;
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_cache_invalidation() {
        let cache = ResponseCache::new(10);
        cache
            .set(
                "key1".to_string(),
                "value1".to_string(),
                Duration::from_secs(60),
            )
            .await;

        cache.invalidate("key1").await;
        let result = cache.get("key1").await;
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_cache_eviction() {
        let cache = ResponseCache::new(2);
        cache
            .set(
                "key1".to_string(),
                "value1".to_string(),
                Duration::from_secs(60),
            )
            .await;
        cache
            .set(
                "key2".to_string(),
                "value2".to_string(),
                Duration::from_secs(60),
            )
            .await;
        cache
            .set(
                "key3".to_string(),
                "value3".to_string(),
                Duration::from_secs(60),
            )
            .await;

        let size = cache.size().await;
        assert_eq!(size, 2);
    }

    #[test]
    fn test_cache_key_builder() {
        let key1 = CacheKeyBuilder::endpoint("GET", "/api/stats", None);
        assert_eq!(key1, "GET:/api/stats");

        let key2 = CacheKeyBuilder::endpoint("GET", "/api/stats", Some("limit=10"));
        assert_eq!(key2, "GET:/api/stats:limit=10");
    }

    #[test]
    fn test_cache_key_builder_version() {
        let key = CacheKeyBuilder::version();
        assert_eq!(key, "GET:/api/version");
    }

    #[tokio::test]
    async fn test_cache_stats_tracking() {
        let cache = ResponseCache::new(10);
        cache
            .set(
                "key1".to_string(),
                "value1".to_string(),
                Duration::from_secs(60),
            )
            .await;

        let _ = cache.get("key1").await; // hit
        let _ = cache.get("key2").await; // miss

        let stats = cache.stats().await;
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
    }
}
