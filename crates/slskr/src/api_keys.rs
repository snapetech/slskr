//! API key management system for authentication

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// API key with metadata
#[derive(Debug, Clone)]
pub struct ApiKey {
    pub id: String,
    pub key: String,
    pub name: String,
    pub created_at: Instant,
    pub last_used: Option<Instant>,
    pub expires_at: Option<Instant>,
    pub scopes: Vec<String>,
    pub rate_limit: Option<u32>,
    pub active: bool,
}

impl ApiKey {
    /// Create new API key
    pub fn new(id: String, key: String, name: String, scopes: Vec<String>) -> Self {
        Self {
            id,
            key,
            name,
            created_at: Instant::now(),
            last_used: None,
            expires_at: None,
            scopes,
            rate_limit: None,
            active: true,
        }
    }

    /// Check if key is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Instant::now() >= expires_at
        } else {
            false
        }
    }

    /// Check if key is valid
    pub fn is_valid(&self) -> bool {
        self.active && !self.is_expired()
    }

    /// Check if key has specific scope
    pub fn has_scope(&self, scope: &str) -> bool {
        self.scopes.contains(&scope.to_string())
    }

    /// Set expiration time
    pub fn set_expiration(&mut self, duration: Duration) {
        self.expires_at = Some(Instant::now() + duration);
    }

    /// Update last used timestamp
    pub fn update_last_used(&mut self) {
        self.last_used = Some(Instant::now());
    }

    /// Revoke key
    pub fn revoke(&mut self) {
        self.active = false;
    }

    /// Get age in seconds
    pub fn age_seconds(&self) -> u64 {
        self.created_at.elapsed().as_secs()
    }

    /// Get time until expiration in seconds
    pub fn expires_in_seconds(&self) -> Option<u64> {
        self.expires_at.map(|exp| exp.duration_since(Instant::now()).as_secs())
    }
}

/// API key manager
pub struct ApiKeyManager {
    keys: Arc<RwLock<HashMap<String, ApiKey>>>,
    key_by_hash: Arc<RwLock<HashMap<String, String>>>,
}

impl ApiKeyManager {
    /// Create new API key manager
    pub fn new() -> Self {
        Self {
            keys: Arc::new(RwLock::new(HashMap::new())),
            key_by_hash: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create and store new API key
    pub async fn create_key(
        &self,
        id: String,
        key: String,
        name: String,
        scopes: Vec<String>,
    ) -> ApiKey {
        let api_key = ApiKey::new(id.clone(), key.clone(), name, scopes);
        let key_hash = hash_key(&key);

        let mut keys = self.keys.write().await;
        let mut key_by_hash = self.key_by_hash.write().await;

        keys.insert(id, api_key.clone());
        key_by_hash.insert(key_hash, api_key.id.clone());

        api_key
    }

    /// Get key by ID
    pub async fn get_key(&self, id: &str) -> Option<ApiKey> {
        let keys = self.keys.read().await;
        keys.get(id).cloned()
    }

    /// Validate key
    pub async fn validate_key(&self, key: &str) -> Option<ApiKey> {
        let key_hash = hash_key(key);
        let key_by_hash = self.key_by_hash.read().await;

        if let Some(id) = key_by_hash.get(&key_hash) {
            let keys = self.keys.read().await;
            if let Some(api_key) = keys.get(id) {
                if api_key.is_valid() {
                    return Some(api_key.clone());
                }
            }
        }

        None
    }

    /// List all keys
    pub async fn list_keys(&self) -> Vec<ApiKey> {
        let keys = self.keys.read().await;
        keys.values().cloned().collect()
    }

    /// List active keys
    pub async fn list_active_keys(&self) -> Vec<ApiKey> {
        let keys = self.keys.read().await;
        keys.values()
            .filter(|k| k.is_valid())
            .cloned()
            .collect()
    }

    /// Revoke key
    pub async fn revoke_key(&self, id: &str) -> bool {
        let mut keys = self.keys.write().await;
        if let Some(key) = keys.get_mut(id) {
            key.revoke();
            return true;
        }
        false
    }

    /// Delete key
    pub async fn delete_key(&self, id: &str) -> bool {
        let mut keys = self.keys.write().await;
        if let Some(key) = keys.remove(id) {
            let mut key_by_hash = self.key_by_hash.write().await;
            let key_hash = hash_key(&key.key);
            key_by_hash.remove(&key_hash);
            return true;
        }
        false
    }

    /// Update key last used
    pub async fn record_usage(&self, id: &str) {
        let mut keys = self.keys.write().await;
        if let Some(key) = keys.get_mut(id) {
            key.update_last_used();
        }
    }

    /// Check if key has scope
    pub async fn has_scope(&self, key: &str, scope: &str) -> bool {
        if let Some(api_key) = self.validate_key(key).await {
            api_key.has_scope(scope)
        } else {
            false
        }
    }

    /// Cleanup expired keys
    pub async fn cleanup_expired(&self) {
        let mut keys = self.keys.write().await;
        let mut key_by_hash = self.key_by_hash.write().await;

        let expired_ids: Vec<String> = keys
            .iter()
            .filter(|(_, k)| k.is_expired())
            .map(|(id, _)| id.clone())
            .collect();

        for id in expired_ids {
            if let Some(key) = keys.remove(&id) {
                let key_hash = hash_key(&key.key);
                key_by_hash.remove(&key_hash);
            }
        }
    }

    /// Get statistics
    pub async fn stats(&self) -> KeyStats {
        let keys = self.keys.read().await;

        let total = keys.len();
        let active = keys.values().filter(|k| k.is_valid()).count();
        let expired = keys.values().filter(|k| k.is_expired()).count();
        let revoked = keys.values().filter(|k| !k.active).count();

        KeyStats {
            total,
            active,
            expired,
            revoked,
        }
    }
}

impl Default for ApiKeyManager {
    fn default() -> Self {
        Self::new()
    }
}

/// API key statistics
#[derive(Debug, Clone)]
pub struct KeyStats {
    pub total: usize,
    pub active: usize,
    pub expired: usize,
    pub revoked: usize,
}

/// Hash API key for storage
fn hash_key(key: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// Generate random API key
pub fn generate_api_key() -> String {
    use std::time::SystemTime;

    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();

    format!("sk_{:x}{:x}", nanos, nanos << 16)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_key_creation() {
        let key = ApiKey::new(
            "key-1".to_string(),
            "secret".to_string(),
            "Test Key".to_string(),
            vec!["read".to_string(), "write".to_string()],
        );

        assert_eq!(key.id, "key-1");
        assert_eq!(key.name, "Test Key");
        assert!(key.is_valid());
    }

    #[test]
    fn test_api_key_scope() {
        let key = ApiKey::new(
            "key-1".to_string(),
            "secret".to_string(),
            "Test Key".to_string(),
            vec!["read".to_string()],
        );

        assert!(key.has_scope("read"));
        assert!(!key.has_scope("write"));
    }

    #[test]
    fn test_api_key_expiration() {
        let mut key = ApiKey::new(
            "key-1".to_string(),
            "secret".to_string(),
            "Test Key".to_string(),
            vec![],
        );

        assert!(!key.is_expired());

        key.set_expiration(Duration::from_millis(1));
        std::thread::sleep(Duration::from_millis(2));

        assert!(key.is_expired());
    }

    #[test]
    fn test_api_key_revoke() {
        let mut key = ApiKey::new(
            "key-1".to_string(),
            "secret".to_string(),
            "Test Key".to_string(),
            vec![],
        );

        assert!(key.is_valid());
        key.revoke();
        assert!(!key.is_valid());
    }

    #[tokio::test]
    async fn test_api_key_manager() {
        let manager = ApiKeyManager::new();

        let key = manager
            .create_key(
                "key-1".to_string(),
                "secret-key".to_string(),
                "Test Key".to_string(),
                vec!["read".to_string()],
            )
            .await;

        assert_eq!(key.id, "key-1");

        let retrieved = manager.get_key("key-1").await;
        assert!(retrieved.is_some());
    }

    #[tokio::test]
    async fn test_api_key_validation() {
        let manager = ApiKeyManager::new();

        manager
            .create_key(
                "key-1".to_string(),
                "secret-key".to_string(),
                "Test Key".to_string(),
                vec!["read".to_string()],
            )
            .await;

        let validated = manager.validate_key("secret-key").await;
        assert!(validated.is_some());

        let invalid = manager.validate_key("wrong-key").await;
        assert!(invalid.is_none());
    }

    #[tokio::test]
    async fn test_api_key_manager_list() {
        let manager = ApiKeyManager::new();

        manager
            .create_key(
                "key-1".to_string(),
                "secret-1".to_string(),
                "Key 1".to_string(),
                vec![],
            )
            .await;

        manager
            .create_key(
                "key-2".to_string(),
                "secret-2".to_string(),
                "Key 2".to_string(),
                vec![],
            )
            .await;

        let keys = manager.list_keys().await;
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn test_generate_api_key() {
        let key1 = generate_api_key();
        let key2 = generate_api_key();

        assert!(key1.starts_with("sk_"));
        assert!(key2.starts_with("sk_"));
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_hash_key() {
        let hash1 = hash_key("secret");
        let hash2 = hash_key("secret");
        let hash3 = hash_key("different");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }
}
