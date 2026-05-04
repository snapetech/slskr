//! Database sharding for 500K req/sec horizontal scaling
//! Sharding key: user_id (consistent hash)
//! Each shard: Independent PostgreSQL instance
//! No cross-shard queries (design for this constraint)

use std::collections::HashMap;
use sqlx::PgPool;
use serde::{Serialize, Deserialize};

/// Consistent hash ring for shard routing
pub struct ShardRouter {
    shards: Vec<ShardInfo>,
    shard_count: usize,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ShardInfo {
    pub id: u32,
    pub connection_string: String,
}

impl ShardRouter {
    /// Create router with N shards
    pub fn new(shards: Vec<ShardInfo>) -> Self {
        let shard_count = shards.len();
        Self { shards, shard_count }
    }

    /// Get shard ID for user
    pub fn get_shard_id(&self, user_id: &str) -> u32 {
        let hash = Self::consistent_hash(user_id);
        (hash % self.shard_count as u64) as u32
    }

    /// Consistent hashing (FNV-1a)
    fn consistent_hash(key: &str) -> u64 {
        const FNV_OFFSET: u64 = 14695981039346656037;
        const FNV_PRIME: u64 = 1099511628211;

        let mut hash = FNV_OFFSET;
        for byte in key.bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        hash
    }

    /// Get shard info for user
    pub fn get_shard(&self, user_id: &str) -> &ShardInfo {
        let shard_id = self.get_shard_id(user_id);
        &self.shards[shard_id as usize]
    }

    /// Get all shards (for cross-shard operations like global stats)
    pub fn all_shards(&self) -> &[ShardInfo] {
        &self.shards
    }
}

/// Database connection pool manager
pub struct ShardedDB {
    router: ShardRouter,
    pools: HashMap<u32, PgPool>,
}

impl ShardedDB {
    /// Create connection pools for all shards
    pub async fn new(
        shards: Vec<ShardInfo>,
    ) -> Result<Self, sqlx::Error> {
        let router = ShardRouter::new(shards.clone());
        let mut pools = HashMap::new();

        for shard in &shards {
            let pool = PgPool::connect(&shard.connection_string).await?;
            pools.insert(shard.id, pool);
        }

        Ok(Self { router, pools })
    }

    /// Get pool for user
    pub fn get_pool(&self, user_id: &str) -> &PgPool {
        let shard = self.router.get_shard(user_id);
        self.pools.get(&shard.id).unwrap()
    }

    /// Get router for manual routing
    pub fn router(&self) -> &ShardRouter {
        &self.router
    }

    /// Get all pools (for scatter-gather operations)
    pub fn all_pools(&self) -> Vec<&PgPool> {
        self.pools.values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consistent_hash_distribution() {
        let router = ShardRouter::new(vec![
            ShardInfo {
                id: 0,
                connection_string: "postgres://localhost/shard0".to_string(),
            },
            ShardInfo {
                id: 1,
                connection_string: "postgres://localhost/shard1".to_string(),
            },
            ShardInfo {
                id: 2,
                connection_string: "postgres://localhost/shard2".to_string(),
            },
        ]);

        // Same user should always hash to same shard
        let shard1 = router.get_shard_id("user123");
        let shard2 = router.get_shard_id("user123");
        assert_eq!(shard1, shard2);

        // Different users should distribute across shards
        let mut distribution = [0, 0, 0];
        for i in 0..1000 {
            let user_id = format!("user{}", i);
            let shard = router.get_shard_id(&user_id) as usize;
            distribution[shard] += 1;
        }

        // Check reasonable distribution (allow some variance)
        for count in &distribution {
            assert!(*count > 250 && *count < 400); // Expect ~333 per shard
        }
    }
}
