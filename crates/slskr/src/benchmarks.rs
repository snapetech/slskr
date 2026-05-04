//! Performance benchmarking module for slskR
//!
//! Provides utilities for measuring and optimizing:
//! - Endpoint latency
//! - Throughput capacity
//! - Database query performance
//! - Memory usage
//! - Cache effectiveness

use std::time::{Duration, Instant};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Benchmark result for a single operation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub name: String,
    pub iterations: u32,
    pub total_duration: Duration,
    pub min_duration: Duration,
    pub max_duration: Duration,
    pub avg_duration: Duration,
    pub median_duration: Duration,
    pub p95_duration: Duration,
    pub p99_duration: Duration,
    pub throughput: f64, // ops/second
}

/// Performance metrics aggregator
#[derive(Clone, Debug)]
pub struct PerformanceMetrics {
    endpoint_latencies: HashMap<String, Vec<Duration>>,
    database_query_times: HashMap<String, Vec<Duration>>,
    cache_hits: u64,
    cache_misses: u64,
    memory_peak_mb: f64,
}

impl PerformanceMetrics {
    /// Create new metrics aggregator
    pub fn new() -> Self {
        Self {
            endpoint_latencies: HashMap::new(),
            database_query_times: HashMap::new(),
            cache_hits: 0,
            cache_misses: 0,
            memory_peak_mb: 0.0,
        }
    }

    /// Record endpoint latency
    pub fn record_endpoint_latency(&mut self, endpoint: String, duration: Duration) {
        self.endpoint_latencies
            .entry(endpoint)
            .or_insert_with(Vec::new)
            .push(duration);
    }

    /// Record database query time
    pub fn record_query_time(&mut self, query: String, duration: Duration) {
        self.database_query_times
            .entry(query)
            .or_insert_with(Vec::new)
            .push(duration);
    }

    /// Record cache hit
    pub fn record_cache_hit(&mut self) {
        self.cache_hits += 1;
    }

    /// Record cache miss
    pub fn record_cache_miss(&mut self) {
        self.cache_misses += 1;
    }

    /// Get cache hit ratio
    pub fn cache_hit_ratio(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            0.0
        } else {
            self.cache_hits as f64 / total as f64
        }
    }

    /// Get endpoint statistics
    pub fn endpoint_stats(&self, endpoint: &str) -> Option<BenchmarkResult> {
        self.endpoint_latencies.get(endpoint).map(|durations| {
            calculate_stats(endpoint.to_string(), durations)
        })
    }

    /// Get database query statistics
    pub fn query_stats(&self, query: &str) -> Option<BenchmarkResult> {
        self.database_query_times.get(query).map(|durations| {
            calculate_stats(query.to_string(), durations)
        })
    }

    /// Get all endpoint statistics
    pub fn all_endpoint_stats(&self) -> Vec<BenchmarkResult> {
        self.endpoint_latencies
            .iter()
            .map(|(endpoint, durations)| calculate_stats(endpoint.clone(), durations))
            .collect()
    }

    /// Get all query statistics
    pub fn all_query_stats(&self) -> Vec<BenchmarkResult> {
        self.database_query_times
            .iter()
            .map(|(query, durations)| calculate_stats(query.clone(), durations))
            .collect()
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for PerformanceMetrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PerformanceMetrics")
            .field("endpoints", &self.endpoint_latencies.len())
            .field("queries", &self.database_query_times.len())
            .field("cache_hit_ratio", &self.cache_hit_ratio())
            .finish()
    }
}

/// Calculate statistics for benchmark results
fn calculate_stats(name: String, durations: &[Duration]) -> BenchmarkResult {
    if durations.is_empty() {
        return BenchmarkResult {
            name,
            iterations: 0,
            total_duration: Duration::ZERO,
            min_duration: Duration::ZERO,
            max_duration: Duration::ZERO,
            avg_duration: Duration::ZERO,
            median_duration: Duration::ZERO,
            p95_duration: Duration::ZERO,
            p99_duration: Duration::ZERO,
            throughput: 0.0,
        };
    }

    let mut sorted = durations.to_vec();
    sorted.sort();

    let total_duration: Duration = durations.iter().sum();
    let avg_duration = total_duration / durations.len() as u32;
    let min_duration = sorted[0];
    let max_duration = sorted[sorted.len() - 1];

    let median_idx = sorted.len() / 2;
    let median_duration = sorted[median_idx];

    let p95_idx = (sorted.len() as f64 * 0.95) as usize;
    let p95_duration = sorted[p95_idx.min(sorted.len() - 1)];

    let p99_idx = (sorted.len() as f64 * 0.99) as usize;
    let p99_duration = sorted[p99_idx.min(sorted.len() - 1)];

    let throughput = if total_duration.as_secs_f64() > 0.0 {
        durations.len() as f64 / total_duration.as_secs_f64()
    } else {
        0.0
    };

    BenchmarkResult {
        name,
        iterations: durations.len() as u32,
        total_duration,
        min_duration,
        max_duration,
        avg_duration,
        median_duration,
        p95_duration,
        p99_duration,
        throughput,
    }
}

/// Benchmark context for timing operations
pub struct BenchmarkContext {
    start: Instant,
    name: String,
}

impl BenchmarkContext {
    /// Create new benchmark context
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            start: Instant::now(),
            name: name.into(),
        }
    }

    /// Get elapsed duration
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    /// Get name
    pub fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_metrics_creation() {
        let metrics = PerformanceMetrics::new();
        assert_eq!(metrics.cache_hit_ratio(), 0.0);
    }

    #[test]
    fn test_cache_hit_ratio() {
        let mut metrics = PerformanceMetrics::new();
        metrics.record_cache_hit();
        metrics.record_cache_hit();
        metrics.record_cache_miss();
        
        let ratio = metrics.cache_hit_ratio();
        assert!((ratio - 2.0/3.0).abs() < 0.01);
    }

    #[test]
    fn test_endpoint_latency_recording() {
        let mut metrics = PerformanceMetrics::new();
        let endpoint = "/api/health".to_string();
        
        metrics.record_endpoint_latency(endpoint.clone(), Duration::from_millis(1));
        metrics.record_endpoint_latency(endpoint.clone(), Duration::from_millis(2));
        metrics.record_endpoint_latency(endpoint.clone(), Duration::from_millis(3));
        
        let stats = metrics.endpoint_stats(&endpoint).unwrap();
        assert_eq!(stats.iterations, 3);
        assert_eq!(stats.min_duration.as_millis(), 1);
        assert_eq!(stats.max_duration.as_millis(), 3);
    }

    #[test]
    fn test_benchmark_context() {
        let ctx = BenchmarkContext::new("test_operation");
        std::thread::sleep(Duration::from_millis(10));
        let elapsed = ctx.elapsed();
        assert!(elapsed.as_millis() >= 10);
    }
}
