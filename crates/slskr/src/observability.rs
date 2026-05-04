/// Advanced observability and monitoring system
///
/// Provides metrics collection, request tracing, performance monitoring,
/// and operational intelligence for production environments.

use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

/// Request metrics
#[derive(Clone, Debug)]
pub struct RequestMetrics {
    pub method: String,
    pub path: String,
    pub status: u16,
    pub duration_ms: u64,
    pub timestamp: u64,
    pub request_id: String,
    pub error: Option<String>,
}

impl RequestMetrics {
    /// Create new request metrics
    pub fn new(method: &str, path: &str, status: u16, duration_ms: u64, request_id: &str) -> Self {
        Self {
            method: method.to_string(),
            path: path.to_string(),
            status,
            duration_ms,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            request_id: request_id.to_string(),
            error: None,
        }
    }

    /// Mark as error
    pub fn with_error(mut self, error: &str) -> Self {
        self.error = Some(error.to_string());
        self
    }

    /// Check if request was successful
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// Convert to JSON
    pub fn to_json(&self) -> Value {
        json!({
            "method": self.method,
            "path": self.path,
            "status": self.status,
            "duration_ms": self.duration_ms,
            "timestamp": self.timestamp,
            "request_id": self.request_id,
            "error": self.error,
            "success": self.is_success()
        })
    }
}

/// Performance metrics aggregator
#[derive(Clone, Debug, Default)]
pub struct PerformanceMetrics {
    pub total_requests: u64,
    pub total_errors: u64,
    pub total_duration_ms: u64,
    pub slowest_request_ms: u64,
    pub fastest_request_ms: u64,
    pub p50_latency_ms: u64,
    pub p95_latency_ms: u64,
    pub p99_latency_ms: u64,
}

impl PerformanceMetrics {
    /// Calculate average latency
    pub fn average_latency_ms(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.total_duration_ms as f64 / self.total_requests as f64
        }
    }

    /// Calculate error rate
    pub fn error_rate_percent(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            (self.total_errors as f64 / self.total_requests as f64) * 100.0
        }
    }

    /// Convert to JSON
    pub fn to_json(&self) -> Value {
        json!({
            "total_requests": self.total_requests,
            "total_errors": self.total_errors,
            "average_latency_ms": self.average_latency_ms(),
            "slowest_request_ms": self.slowest_request_ms,
            "fastest_request_ms": self.fastest_request_ms,
            "p50_latency_ms": self.p50_latency_ms,
            "p95_latency_ms": self.p95_latency_ms,
            "p99_latency_ms": self.p99_latency_ms,
            "error_rate_percent": self.error_rate_percent()
        })
    }
}

/// Metrics collector
pub struct MetricsCollector {
    requests: Arc<RwLock<Vec<RequestMetrics>>>,
    performance: Arc<RwLock<PerformanceMetrics>>,
}

impl MetricsCollector {
    /// Create new metrics collector
    pub fn new() -> Self {
        Self {
            requests: Arc::new(RwLock::new(Vec::new())),
            performance: Arc::new(RwLock::new(PerformanceMetrics::default())),
        }
    }

    /// Record a request
    pub async fn record(&self, metrics: RequestMetrics) {
        // Update performance metrics
        {
            let mut perf = self.performance.write().await;
            perf.total_requests += 1;

            if metrics.is_success() {
                if metrics.duration_ms > perf.slowest_request_ms {
                    perf.slowest_request_ms = metrics.duration_ms;
                }
                if perf.fastest_request_ms == 0 || metrics.duration_ms < perf.fastest_request_ms {
                    perf.fastest_request_ms = metrics.duration_ms;
                }
            } else {
                perf.total_errors += 1;
            }

            perf.total_duration_ms += metrics.duration_ms;
        }

        // Store request
        {
            let mut requests = self.requests.write().await;
            requests.push(metrics);

            // Keep only last 1000 requests for memory efficiency
            if requests.len() > 1000 {
                requests.remove(0);
            }
        }
    }

    /// Get performance metrics
    pub async fn get_performance_metrics(&self) -> PerformanceMetrics {
        self.performance.read().await.clone()
    }

    /// Get recent requests
    pub async fn get_recent_requests(&self, limit: usize) -> Vec<RequestMetrics> {
        let requests = self.requests.read().await;
        requests
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get requests by path
    pub async fn get_requests_by_path(&self, path: &str) -> Vec<RequestMetrics> {
        let requests = self.requests.read().await;
        requests
            .iter()
            .filter(|r| r.path == path)
            .cloned()
            .collect()
    }

    /// Get error requests
    pub async fn get_errors(&self) -> Vec<RequestMetrics> {
        let requests = self.requests.read().await;
        requests
            .iter()
            .filter(|r| !r.is_success())
            .cloned()
            .collect()
    }

    /// Get slow requests (> threshold_ms)
    pub async fn get_slow_requests(&self, threshold_ms: u64) -> Vec<RequestMetrics> {
        let requests = self.requests.read().await;
        requests
            .iter()
            .filter(|r| r.duration_ms > threshold_ms)
            .cloned()
            .collect()
    }

    /// Clear old metrics (older than days_old)
    pub async fn clear_old_metrics(&self, days_old: u64) {
        let cutoff = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - (days_old * 86400);

        let mut requests = self.requests.write().await;
        requests.retain(|r| r.timestamp > cutoff);
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Health check report
#[derive(Clone, Debug)]
pub struct HealthReport {
    pub status: String,
    pub timestamp: u64,
    pub uptime_seconds: u64,
    pub total_requests: u64,
    pub error_count: u64,
    pub cache_hit_rate: f64,
    pub average_latency_ms: f64,
}

impl HealthReport {
    /// Create health report
    pub fn new(
        total_requests: u64,
        error_count: u64,
        cache_hit_rate: f64,
        average_latency_ms: f64,
        uptime_seconds: u64,
    ) -> Self {
        let status = if error_count == 0 && average_latency_ms < 1000.0 {
            "healthy".to_string()
        } else if average_latency_ms < 2000.0 {
            "degraded".to_string()
        } else {
            "unhealthy".to_string()
        };

        Self {
            status,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            uptime_seconds,
            total_requests,
            error_count,
            cache_hit_rate,
            average_latency_ms,
        }
    }

    /// Convert to JSON
    pub fn to_json(&self) -> Value {
        json!({
            "status": self.status,
            "timestamp": self.timestamp,
            "uptime_seconds": self.uptime_seconds,
            "total_requests": self.total_requests,
            "error_count": self.error_count,
            "cache_hit_rate": self.cache_hit_rate,
            "average_latency_ms": self.average_latency_ms
        })
    }
}

/// Trace information for request
#[derive(Clone, Debug)]
pub struct Trace {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub start_time_ms: u64,
    pub end_time_ms: u64,
    pub tags: HashMap<String, String>,
}

impl Trace {
    /// Create new trace
    pub fn new(trace_id: &str, span_id: &str) -> Self {
        let start_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        Self {
            trace_id: trace_id.to_string(),
            span_id: span_id.to_string(),
            parent_span_id: None,
            start_time_ms: start_ms,
            end_time_ms: start_ms,
            tags: HashMap::new(),
        }
    }

    /// Add tag
    pub fn add_tag(&mut self, key: &str, value: &str) {
        self.tags.insert(key.to_string(), value.to_string());
    }

    /// Complete trace
    pub fn complete(&mut self) {
        self.end_time_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
    }

    /// Get duration
    pub fn duration_ms(&self) -> u64 {
        self.end_time_ms.saturating_sub(self.start_time_ms)
    }

    /// Convert to JSON
    pub fn to_json(&self) -> Value {
        json!({
            "trace_id": self.trace_id,
            "span_id": self.span_id,
            "parent_span_id": self.parent_span_id,
            "duration_ms": self.duration_ms(),
            "tags": self.tags
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_metrics_creation() {
        let metrics = RequestMetrics::new("GET", "/api/health", 200, 45, "req_123");

        assert_eq!(metrics.method, "GET");
        assert_eq!(metrics.path, "/api/health");
        assert_eq!(metrics.status, 200);
        assert!(metrics.is_success());
    }

    #[test]
    fn test_request_metrics_error() {
        let metrics = RequestMetrics::new("POST", "/api/search", 400, 120, "req_456")
            .with_error("Invalid query");

        assert!(!metrics.is_success());
        assert_eq!(metrics.error, Some("Invalid query".to_string()));
    }

    #[test]
    fn test_performance_metrics_calculation() {
        let mut perf = PerformanceMetrics::default();
        perf.total_requests = 100;
        perf.total_errors = 5;
        perf.total_duration_ms = 5000;

        assert_eq!(perf.average_latency_ms(), 50.0);
        assert!(perf.error_rate_percent() > 4.9 && perf.error_rate_percent() < 5.1);
    }

    #[tokio::test]
    async fn test_metrics_collector_record() {
        let collector = MetricsCollector::new();

        let metrics = RequestMetrics::new("GET", "/api/health", 200, 50, "req_1");
        collector.record(metrics).await;

        let perf = collector.get_performance_metrics().await;
        assert_eq!(perf.total_requests, 1);
    }

    #[tokio::test]
    async fn test_metrics_collector_recent_requests() {
        let collector = MetricsCollector::new();

        let m1 = RequestMetrics::new("GET", "/api/health", 200, 50, "req_1");
        let m2 = RequestMetrics::new("GET", "/api/version", 200, 40, "req_2");

        collector.record(m1).await;
        collector.record(m2).await;

        let recent = collector.get_recent_requests(10).await;
        assert_eq!(recent.len(), 2);
    }

    #[test]
    fn test_health_report_creation() {
        let report = HealthReport::new(1000, 5, 95.0, 250.0, 3600);

        assert_eq!(report.status, "healthy");
        assert_eq!(report.total_requests, 1000);
    }

    #[test]
    fn test_trace_creation() {
        let trace = Trace::new("trace_123", "span_456");

        assert_eq!(trace.trace_id, "trace_123");
        assert_eq!(trace.span_id, "span_456");
    }

    #[test]
    fn test_trace_add_tag() {
        let mut trace = Trace::new("trace_123", "span_456");
        trace.add_tag("endpoint", "/api/search");
        trace.add_tag("method", "GET");

        assert_eq!(trace.tags.get("endpoint"), Some(&"/api/search".to_string()));
    }
}
