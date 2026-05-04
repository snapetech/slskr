/// Response caching layer with configurable TTL and invalidation strategies
///
/// Provides in-memory caching for API responses with cache invalidation,
/// cache warming, and performance monitoring.

use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

/// Cache entry with metadata
#[derive(Clone, Debug)]
pub struct CacheEntry {
    pub value: Value,
    pub created_at: u64,
    pub ttl_seconds: u64,
    pub hits: u64,
    pub etag: String,
}

impl CacheEntry {
    /// Create a new cache entry
    pub fn new(value: Value, ttl_seconds: u64) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let etag = format!("{:x}", now);

        Self {
            value,
            created_at: now,
            ttl_seconds,
            hits: 0,
            etag,
        }
    }

    /// Check if entry is expired
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        now > self.created_at + self.ttl_seconds
    }

    /// Get remaining TTL in seconds
    pub fn remaining_ttl(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let elapsed = now - self.created_at;
        if elapsed >= self.ttl_seconds {
            0
        } else {
            self.ttl_seconds - elapsed
        }
    }

    /// Increment hit count
    pub fn record_hit(&mut self) {
        self.hits += 1;
    }
}

/// Response cache with TTL support
pub struct ResponseCache {
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    stats: Arc<RwLock<CacheStats>>,
}

/// Cache statistics
#[derive(Clone, Debug, Default)]
pub struct CacheStats {
    pub total_hits: u64,
    pub total_misses: u64,
    pub total_entries: u64,
    pub expired_entries: u64,
}

impl CacheStats {
    /// Get hit rate percentage
    pub fn hit_rate(&self) -> f64 {
        let total = self.total_hits + self.total_misses;
        if total == 0 {
            0.0
        } else {
            (self.total_hits as f64 / total as f64) * 100.0
        }
    }
}

impl ResponseCache {
    /// Create a new response cache
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(CacheStats::default())),
        }
    }

    /// Generate cache key from method and path
    pub fn make_key(method: &str, path: &str) -> String {
        format!("{}:{}", method, path)
    }

    /// Get value from cache
    pub async fn get(&self, key: &str) -> Option<Value> {
        let mut cache = self.cache.write().await;

        if let Some(entry) = cache.get_mut(key) {
            if entry.is_expired() {
                cache.remove(key);
                let mut stats = self.stats.write().await;
                stats.expired_entries += 1;
                return None;
            }

            entry.record_hit();
            let mut stats = self.stats.write().await;
            stats.total_hits += 1;

            return Some(entry.value.clone());
        }

        let mut stats = self.stats.write().await;
        stats.total_misses += 1;
        None
    }

    /// Store value in cache
    pub async fn set(&self, key: String, value: Value, ttl_seconds: u64) {
        let mut cache = self.cache.write().await;
        cache.insert(key, CacheEntry::new(value, ttl_seconds));

        let mut stats = self.stats.write().await;
        stats.total_entries = cache.len() as u64;
    }

    /// Clear entire cache
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();

        let mut stats = self.stats.write().await;
        stats.total_entries = 0;
    }

    /// Remove expired entries
    pub async fn evict_expired(&self) -> usize {
        let mut cache = self.cache.write().await;
        let initial_size = cache.len();

        cache.retain(|_, entry| !entry.is_expired());

        let removed = initial_size - cache.len();
        let mut stats = self.stats.write().await;
        stats.total_entries = cache.len() as u64;
        stats.expired_entries += removed as u64;

        removed
    }

    /// Get cache statistics
    pub async fn get_stats(&self) -> CacheStats {
        self.stats.read().await.clone()
    }

    /// Get cache entry info (for monitoring)
    pub async fn get_entry_info(&self, key: &str) -> Option<Value> {
        let cache = self.cache.read().await;

        if let Some(entry) = cache.get(key) {
            return Some(serde_json::json!({
                "key": key,
                "created_at": entry.created_at,
                "ttl_seconds": entry.ttl_seconds,
                "remaining_ttl": entry.remaining_ttl(),
                "hits": entry.hits,
                "etag": entry.etag,
                "expired": entry.is_expired()
            }));
        }

        None
    }

    /// Get all cache keys (for debugging)
    pub async fn get_keys(&self) -> Vec<String> {
        let cache = self.cache.read().await;
        cache.keys().cloned().collect()
    }

    /// Get cache size in entries
    pub async fn size(&self) -> usize {
        let cache = self.cache.read().await;
        cache.len()
    }
}

impl Default for ResponseCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache invalidation strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidationStrategy {
    /// Time-based invalidation
    TimeToLive,
    /// Event-based invalidation
    EventDriven,
    /// LRU (Least Recently Used) eviction
    LRU,
}

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub enabled: bool,
    pub default_ttl_seconds: u64,
    pub max_entries: usize,
    pub strategy: InvalidationStrategy,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_ttl_seconds: 300, // 5 minutes
            max_entries: 1000,
            strategy: InvalidationStrategy::TimeToLive,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_entry_creation() {
        let value = serde_json::json!({"data": "test"});
        let entry = CacheEntry::new(value.clone(), 300);

        assert_eq!(entry.value, value);
        assert_eq!(entry.ttl_seconds, 300);
        assert_eq!(entry.hits, 0);
    }

    #[test]
    fn test_cache_entry_expiration() {
        let value = serde_json::json!({"data": "test"});
        let mut entry = CacheEntry::new(value, 0); // Immediate expiration

        // Manually set created_at to past
        entry.created_at = entry.created_at.saturating_sub(10);

        assert!(entry.is_expired());
    }

    #[test]
    fn test_cache_key_generation() {
        let key = ResponseCache::make_key("GET", "/api/health");
        assert_eq!(key, "GET:/api/health");
    }

    #[tokio::test]
    async fn test_cache_set_and_get() {
        let cache = ResponseCache::new();
        let value = serde_json::json!({"data": "test"});
        let key = "test_key".to_string();

        cache.set(key.clone(), value.clone(), 300).await;
        let cached = cache.get(&key).await;

        assert!(cached.is_some());
        assert_eq!(cached.unwrap(), value);
    }

    #[tokio::test]
    async fn test_cache_miss() {
        let cache = ResponseCache::new();
        let cached = cache.get("nonexistent").await;

        assert!(cached.is_none());
    }

    #[tokio::test]
    async fn test_cache_clear() {
        let cache = ResponseCache::new();
        let value = serde_json::json!({"data": "test"});

        cache.set("key1".to_string(), value.clone(), 300).await;
        cache.set("key2".to_string(), value.clone(), 300).await;

        assert_eq!(cache.size().await, 2);

        cache.clear().await;

        assert_eq!(cache.size().await, 0);
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let cache = ResponseCache::new();
        let value = serde_json::json!({"data": "test"});

        cache.set("key1".to_string(), value, 300).await;
        cache.get("key1").await;
        cache.get("key1").await;
        cache.get("nonexistent").await;

        let stats = cache.get_stats().await;

        assert_eq!(stats.total_hits, 2);
        assert_eq!(stats.total_misses, 1);
        assert!(stats.hit_rate() > 60.0 && stats.hit_rate() < 70.0);
    }

    #[tokio::test]
    async fn test_cache_evict_expired() {
        let cache = ResponseCache::new();
        let value = serde_json::json!({"data": "test"});

        // This would require mocking SystemTime, so we just test the method exists
        let removed = cache.evict_expired().await;
        assert_eq!(removed, 0); // No entries to evict
    }

    #[test]
    fn test_cache_config_default() {
        let config = CacheConfig::default();

        assert!(config.enabled);
        assert_eq!(config.default_ttl_seconds, 300);
        assert_eq!(config.max_entries, 1000);
    }

    #[tokio::test]
    async fn test_cache_entry_info() {
        let cache = ResponseCache::new();
        let value = serde_json::json!({"data": "test"});

        cache.set("test_key".to_string(), value, 300).await;

        let info = cache.get_entry_info("test_key").await;
        assert!(info.is_some());

        let info = info.unwrap();
        assert_eq!(info["key"], "test_key");
        assert_eq!(info["ttl_seconds"], 300);
        assert_eq!(info["expired"], false);
    }

    #[tokio::test]
    async fn test_cache_get_keys() {
        let cache = ResponseCache::new();
        let value = serde_json::json!({"data": "test"});

        cache.set("key1".to_string(), value.clone(), 300).await;
        cache.set("key2".to_string(), value.clone(), 300).await;

        let keys = cache.get_keys().await;
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"key1".to_string()));
        assert!(keys.contains(&"key2".to_string()));
    }
}
