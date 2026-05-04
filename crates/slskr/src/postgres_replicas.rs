//! PostgreSQL primary-replica setup for 500K req/sec
//! Architecture:
//! - Primary (writes): Single master database
//! - Replicas (reads): N read-only followers
//! - Replication: Streaming WAL (< 1 second lag)
//! - Failover: Automatic with pgbouncer

use sqlx::{postgres::PgPoolOptions, PgPool};
use std::collections::HashMap;

/// PostgreSQL replica configuration
#[derive(Debug, Clone)]
pub struct ReplicaConfig {
    pub primary_url: String,
    pub replicas: Vec<String>,
    pub pool_size: u32,
    pub max_connections: u32,
}

/// Multi-replica database manager
pub struct PostgresCluster {
    primary: PgPool,
    replicas: Vec<PgPool>,
    read_index: std::sync::atomic::AtomicUsize,
}

impl PostgresCluster {
    /// Create PostgreSQL cluster with primary + replicas
    pub async fn new(config: ReplicaConfig) -> Result<Self, sqlx::Error> {
        // Connect to primary (write)
        let primary = PgPoolOptions::new()
            .max_connections(config.pool_size)
            .connect(&config.primary_url)
            .await?;

        // Connect to all replicas (read-only)
        let mut replicas = Vec::new();
        for replica_url in &config.replicas {
            let pool = PgPoolOptions::new()
                .max_connections(config.pool_size)
                .connect(replica_url)
                .await?;
            replicas.push(pool);
        }

        Ok(Self {
            primary,
            replicas,
            read_index: std::sync::atomic::AtomicUsize::new(0),
        })
    }

    /// Get pool for write operations (always primary)
    pub fn write_pool(&self) -> &PgPool {
        &self.primary
    }

    /// Get pool for read operations (round-robin across replicas)
    pub fn read_pool(&self) -> &PgPool {
        if self.replicas.is_empty() {
            return &self.primary;
        }

        let idx = self
            .read_index
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let replica_idx = idx % self.replicas.len();
        &self.replicas[replica_idx]
    }

    /// Get primary pool (for explicit writes)
    pub fn primary(&self) -> &PgPool {
        &self.primary
    }

    /// Get specific replica by index
    pub fn replica(&self, idx: usize) -> Option<&PgPool> {
        self.replicas.get(idx)
    }

    /// Get all replicas
    pub fn all_replicas(&self) -> &[PgPool] {
        &self.replicas
    }

    /// Get replica count
    pub fn replica_count(&self) -> usize {
        self.replicas.len()
    }

    /// Check replication lag (in milliseconds)
    pub async fn check_replication_lag(&self, replica_idx: usize) -> Result<u64, sqlx::Error> {
        if let Some(replica) = self.replica(replica_idx) {
            // Query: SELECT EXTRACT(EPOCH FROM (NOW() - pg_last_xact_replay_timestamp())) * 1000 as lag_ms;
            let row = sqlx::query_scalar::<_, f64>(
                "SELECT EXTRACT(EPOCH FROM (NOW() - pg_last_xact_replay_timestamp())) * 1000"
            )
            .fetch_one(replica)
            .await?;

            Ok(row as u64)
        } else {
            Err(sqlx::Error::RowNotFound)
        }
    }

    /// Execute read query (automatically uses replica)
    pub async fn read_query<T: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow>>(
        &self,
        sql: &str,
    ) -> Result<Vec<T>, sqlx::Error> {
        let pool = self.read_pool();
        sqlx::query_as::<_, T>(sql)
            .fetch_all(pool)
            .await
    }

    /// Execute write query (always uses primary)
    pub async fn write_query(&self, sql: &str) -> Result<sqlx::postgres::PgQueryResult, sqlx::Error> {
        sqlx::query(sql)
            .execute(&self.primary)
            .await
    }
}

/// Connection pooling strategy
pub struct PoolStrategy {
    pub pool_size: u32,
    pub max_overflow: u32,
    pub idle_timeout: u64,
    pub max_lifetime: u64,
}

impl Default for PoolStrategy {
    fn default() -> Self {
        Self {
            pool_size: 20,           // 20 connections per pool
            max_overflow: 10,        // Allow 10 overflow connections
            idle_timeout: 600,       // 10 minute idle timeout
            max_lifetime: 1800,      // 30 minute max connection lifetime
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replica_config() {
        let config = ReplicaConfig {
            primary_url: "postgres://localhost/slskr".to_string(),
            replicas: vec![
                "postgres://replica1/slskr".to_string(),
                "postgres://replica2/slskr".to_string(),
                "postgres://replica3/slskr".to_string(),
            ],
            pool_size: 20,
            max_connections: 100,
        };

        assert_eq!(config.replicas.len(), 3);
    }

    #[test]
    fn test_pool_strategy_defaults() {
        let strategy = PoolStrategy::default();
        assert_eq!(strategy.pool_size, 20);
        assert_eq!(strategy.max_overflow, 10);
    }
}
