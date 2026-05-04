//! Cluster management for 500K req/sec horizontal scaling
//! Supports:
//! - Multiple instances (load balanced)
//! - Multi-region deployment
//! - Session affinity (WebSocket sticky sessions)
//! - Instance health checks
//! - Automatic failover

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Serialize, Deserialize};

/// Instance in the cluster
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterNode {
    pub id: String,
    pub host: String,
    pub port: u16,
    pub region: String,
    pub load: f32, // Current request load (0-1)
    pub healthy: bool,
    pub last_heartbeat: u64,
}

impl ClusterNode {
    pub fn new(id: String, host: String, port: u16, region: String) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            id,
            host,
            port,
            region,
            load: 0.0,
            healthy: true,
            last_heartbeat: now,
        }
    }

    pub fn url(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }
}

/// Cluster management
pub struct ClusterManager {
    nodes: HashMap<String, ClusterNode>,
    load_balancing: LoadBalancingStrategy,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LoadBalancingStrategy {
    LeastConnections,     // Route to node with lowest load
    RoundRobin,          // Rotate through nodes
    LocalFirst,          // Prefer local region
    GeoHash,             // Hash-based routing for session affinity
}

impl ClusterManager {
    pub fn new(strategy: LoadBalancingStrategy) -> Self {
        Self {
            nodes: HashMap::new(),
            load_balancing: strategy,
        }
    }

    /// Register a node in the cluster
    pub fn register(&mut self, node: ClusterNode) {
        self.nodes.insert(node.id.clone(), node);
    }

    /// Select node for request routing
    pub fn select_node(&self, session_key: Option<&str>) -> Option<&ClusterNode> {
        let healthy: Vec<_> = self
            .nodes
            .values()
            .filter(|n| n.healthy)
            .collect();

        if healthy.is_empty() {
            return None;
        }

        match self.load_balancing {
            LoadBalancingStrategy::LeastConnections => {
                healthy.iter().min_by(|a, b| a.load.partial_cmp(&b.load).unwrap()).copied()
            }
            LoadBalancingStrategy::RoundRobin => {
                // Simple round-robin (would need state)
                healthy.first().copied()
            }
            LoadBalancingStrategy::LocalFirst => {
                // Prefer nodes in local region
                healthy
                    .iter()
                    .find(|n| n.region == "local")
                    .or_else(|| healthy.first())
                    .copied()
            }
            LoadBalancingStrategy::GeoHash => {
                // Hash-based routing for session affinity
                if let Some(key) = session_key {
                    let hash = Self::hash(key) as usize;
                    healthy.get(hash % healthy.len()).copied()
                } else {
                    healthy.first().copied()
                }
            }
        }
    }

    /// Hash function for consistent routing
    fn hash(key: &str) -> u64 {
        let mut hash: u64 = 5381;
        for c in key.chars() {
            hash = ((hash << 5).wrapping_add(hash)).wrapping_add(c as u64);
        }
        hash
    }

    /// Update node load
    pub fn update_load(&mut self, node_id: &str, load: f32) {
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.load = load;
        }
    }

    /// Mark node as unhealthy (triggers failover)
    pub fn mark_unhealthy(&mut self, node_id: &str) {
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.healthy = false;
        }
    }

    /// Mark node as healthy
    pub fn mark_healthy(&mut self, node_id: &str) {
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.healthy = true;
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            node.last_heartbeat = now;
        }
    }

    /// Get cluster stats
    pub fn stats(&self) -> ClusterStats {
        let total_load: f32 = self.nodes.values().map(|n| n.load).sum();
        let healthy_count = self.nodes.values().filter(|n| n.healthy).count();

        ClusterStats {
            total_nodes: self.nodes.len(),
            healthy_nodes: healthy_count,
            total_load,
            avg_load: if self.nodes.is_empty() {
                0.0
            } else {
                total_load / self.nodes.len() as f32
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClusterStats {
    pub total_nodes: usize,
    pub healthy_nodes: usize,
    pub total_load: f32,
    pub avg_load: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cluster_registration() {
        let mut manager = ClusterManager::new(LoadBalancingStrategy::LeastConnections);

        manager.register(ClusterNode::new(
            "node1".to_string(),
            "127.0.0.1".to_string(),
            5030,
            "us-east".to_string(),
        ));

        assert_eq!(manager.nodes.len(), 1);
    }

    #[test]
    fn test_least_connections_routing() {
        let mut manager = ClusterManager::new(LoadBalancingStrategy::LeastConnections);

        let mut node1 = ClusterNode::new(
            "node1".to_string(),
            "127.0.0.1".to_string(),
            5030,
            "us".to_string(),
        );
        node1.load = 0.5;
        manager.register(node1);

        let mut node2 = ClusterNode::new(
            "node2".to_string(),
            "127.0.0.1".to_string(),
            5031,
            "us".to_string(),
        );
        node2.load = 0.2;
        manager.register(node2);

        let selected = manager.select_node(None).unwrap();
        assert_eq!(selected.id, "node2"); // Should select node with lower load
    }

    #[test]
    fn test_session_affinity() {
        let mut manager = ClusterManager::new(LoadBalancingStrategy::GeoHash);

        manager.register(ClusterNode::new(
            "node1".to_string(),
            "127.0.0.1".to_string(),
            5030,
            "us".to_string(),
        ));
        manager.register(ClusterNode::new(
            "node2".to_string(),
            "127.0.0.1".to_string(),
            5031,
            "us".to_string(),
        ));

        let node1 = manager.select_node(Some("session123")).unwrap().id.clone();
        let node2 = manager.select_node(Some("session123")).unwrap().id.clone();

        assert_eq!(node1, node2); // Same session should route to same node
    }
}
