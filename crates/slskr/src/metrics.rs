//! Prometheus metrics export for HTTP API

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Prometheus metrics collector
pub struct MetricsCollector {
    // Request metrics
    requests_total: Arc<AtomicU64>,
    requests_success: Arc<AtomicU64>,
    requests_error: Arc<AtomicU64>,
    request_duration_total_ms: Arc<AtomicU64>,

    // Transfer metrics
    transfers_started: Arc<AtomicU64>,
    transfers_completed: Arc<AtomicU64>,
    transfers_failed: Arc<AtomicU64>,
    bytes_transferred: Arc<AtomicU64>,

    // Search metrics
    searches_started: Arc<AtomicU64>,
    search_results_total: Arc<AtomicU64>,

    // Message metrics
    messages_sent: Arc<AtomicU64>,
    messages_received: Arc<AtomicU64>,

    // Cache metrics
    cache_hits: Arc<AtomicU64>,
    cache_misses: Arc<AtomicU64>,

    // Connection metrics
    connections_active: Arc<AtomicU64>,
    connections_total: Arc<AtomicU64>,
}

impl MetricsCollector {
    /// Create new metrics collector
    pub fn new() -> Self {
        Self {
            requests_total: Arc::new(AtomicU64::new(0)),
            requests_success: Arc::new(AtomicU64::new(0)),
            requests_error: Arc::new(AtomicU64::new(0)),
            request_duration_total_ms: Arc::new(AtomicU64::new(0)),

            transfers_started: Arc::new(AtomicU64::new(0)),
            transfers_completed: Arc::new(AtomicU64::new(0)),
            transfers_failed: Arc::new(AtomicU64::new(0)),
            bytes_transferred: Arc::new(AtomicU64::new(0)),

            searches_started: Arc::new(AtomicU64::new(0)),
            search_results_total: Arc::new(AtomicU64::new(0)),

            messages_sent: Arc::new(AtomicU64::new(0)),
            messages_received: Arc::new(AtomicU64::new(0)),

            cache_hits: Arc::new(AtomicU64::new(0)),
            cache_misses: Arc::new(AtomicU64::new(0)),

            connections_active: Arc::new(AtomicU64::new(0)),
            connections_total: Arc::new(AtomicU64::new(0)),
        }
    }

    // =========================================================================
    // Request Metrics
    // =========================================================================

    pub fn record_request(&self) {
        self.requests_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_request_success(&self) {
        self.requests_success.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_request_error(&self) {
        self.requests_error.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_request_duration(&self, duration_ms: u128) {
        self.request_duration_total_ms
            .fetch_add(duration_ms as u64, Ordering::Relaxed);
    }

    // =========================================================================
    // Transfer Metrics
    // =========================================================================

    pub fn record_transfer_started(&self) {
        self.transfers_started.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_transfer_completed(&self, bytes: u64) {
        self.transfers_completed.fetch_add(1, Ordering::Relaxed);
        self.bytes_transferred.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn record_transfer_failed(&self) {
        self.transfers_failed.fetch_add(1, Ordering::Relaxed);
    }

    // =========================================================================
    // Search Metrics
    // =========================================================================

    pub fn record_search_started(&self) {
        self.searches_started.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_search_results(&self, count: u64) {
        self.search_results_total.fetch_add(count, Ordering::Relaxed);
    }

    // =========================================================================
    // Message Metrics
    // =========================================================================

    pub fn record_message_sent(&self) {
        self.messages_sent.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_message_received(&self) {
        self.messages_received.fetch_add(1, Ordering::Relaxed);
    }

    // =========================================================================
    // Cache Metrics
    // =========================================================================

    pub fn record_cache_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_cache_miss(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
    }

    // =========================================================================
    // Connection Metrics
    // =========================================================================

    pub fn record_connection_opened(&self) {
        self.connections_total.fetch_add(1, Ordering::Relaxed);
        self.connections_active.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_connection_closed(&self) {
        self.connections_active.fetch_sub(1, Ordering::Relaxed);
    }

    // =========================================================================
    // Export to Prometheus Format
    // =========================================================================

    /// Export metrics in Prometheus text format
    pub fn export_prometheus(&self) -> String {
        let mut output = String::new();

        // Request metrics
        output.push_str("# HELP http_requests_total Total HTTP requests\n");
        output.push_str("# TYPE http_requests_total counter\n");
        output.push_str(&format!(
            "http_requests_total {}\n",
            self.requests_total.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP http_requests_success Successful HTTP requests\n");
        output.push_str("# TYPE http_requests_success counter\n");
        output.push_str(&format!(
            "http_requests_success {}\n",
            self.requests_success.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP http_requests_error Failed HTTP requests\n");
        output.push_str("# TYPE http_requests_error counter\n");
        output.push_str(&format!(
            "http_requests_error {}\n",
            self.requests_error.load(Ordering::Relaxed)
        ));

        // Average request duration
        let total_requests = self.requests_total.load(Ordering::Relaxed);
        let total_duration = self.request_duration_total_ms.load(Ordering::Relaxed);
        let avg_duration = if total_requests > 0 {
            total_duration / total_requests
        } else {
            0
        };

        output.push_str("# HELP http_request_duration_ms Average HTTP request duration\n");
        output.push_str("# TYPE http_request_duration_ms gauge\n");
        output.push_str(&format!("http_request_duration_ms {}\n", avg_duration));

        // Transfer metrics
        output.push_str("# HELP transfers_started Total transfers started\n");
        output.push_str("# TYPE transfers_started counter\n");
        output.push_str(&format!(
            "transfers_started {}\n",
            self.transfers_started.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP transfers_completed Completed transfers\n");
        output.push_str("# TYPE transfers_completed counter\n");
        output.push_str(&format!(
            "transfers_completed {}\n",
            self.transfers_completed.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP transfers_failed Failed transfers\n");
        output.push_str("# TYPE transfers_failed counter\n");
        output.push_str(&format!(
            "transfers_failed {}\n",
            self.transfers_failed.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP bytes_transferred Total bytes transferred\n");
        output.push_str("# TYPE bytes_transferred counter\n");
        output.push_str(&format!(
            "bytes_transferred {}\n",
            self.bytes_transferred.load(Ordering::Relaxed)
        ));

        // Search metrics
        output.push_str("# HELP searches_started Total searches started\n");
        output.push_str("# TYPE searches_started counter\n");
        output.push_str(&format!(
            "searches_started {}\n",
            self.searches_started.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP search_results_total Total search results\n");
        output.push_str("# TYPE search_results_total counter\n");
        output.push_str(&format!(
            "search_results_total {}\n",
            self.search_results_total.load(Ordering::Relaxed)
        ));

        // Message metrics
        output.push_str("# HELP messages_sent Total messages sent\n");
        output.push_str("# TYPE messages_sent counter\n");
        output.push_str(&format!(
            "messages_sent {}\n",
            self.messages_sent.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP messages_received Total messages received\n");
        output.push_str("# TYPE messages_received counter\n");
        output.push_str(&format!(
            "messages_received {}\n",
            self.messages_received.load(Ordering::Relaxed)
        ));

        // Cache metrics
        let cache_hits = self.cache_hits.load(Ordering::Relaxed);
        let cache_misses = self.cache_misses.load(Ordering::Relaxed);
        let cache_total = cache_hits + cache_misses;
        let hit_rate = if cache_total > 0 {
            ((cache_hits as f64 / cache_total as f64) * 100.0) as u64
        } else {
            0
        };

        output.push_str("# HELP cache_hits Cache hits\n");
        output.push_str("# TYPE cache_hits counter\n");
        output.push_str(&format!("cache_hits {}\n", cache_hits));

        output.push_str("# HELP cache_misses Cache misses\n");
        output.push_str("# TYPE cache_misses counter\n");
        output.push_str(&format!("cache_misses {}\n", cache_misses));

        output.push_str("# HELP cache_hit_rate Cache hit rate percentage\n");
        output.push_str("# TYPE cache_hit_rate gauge\n");
        output.push_str(&format!("cache_hit_rate {}\n", hit_rate));

        // Connection metrics
        output.push_str("# HELP connections_active Active connections\n");
        output.push_str("# TYPE connections_active gauge\n");
        output.push_str(&format!(
            "connections_active {}\n",
            self.connections_active.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP connections_total Total connections\n");
        output.push_str("# TYPE connections_total counter\n");
        output.push_str(&format!(
            "connections_total {}\n",
            self.connections_total.load(Ordering::Relaxed)
        ));

        output
    }

    /// Export metrics as JSON
    pub fn export_json(&self) -> String {
        let total_requests = self.requests_total.load(Ordering::Relaxed);
        let total_duration = self.request_duration_total_ms.load(Ordering::Relaxed);
        let avg_duration = if total_requests > 0 {
            total_duration / total_requests
        } else {
            0
        };

        format!(
            r#"{{
  "http": {{
    "requests_total": {},
    "requests_success": {},
    "requests_error": {},
    "request_duration_ms": {}
  }},
  "transfers": {{
    "started": {},
    "completed": {},
    "failed": {},
    "bytes_transferred": {}
  }},
  "searches": {{
    "started": {},
    "results_total": {}
  }},
  "messages": {{
    "sent": {},
    "received": {}
  }},
  "cache": {{
    "hits": {},
    "misses": {},
    "hit_rate": {}
  }},
  "connections": {{
    "active": {},
    "total": {}
  }}
}}"#,
            total_requests,
            self.requests_success.load(Ordering::Relaxed),
            self.requests_error.load(Ordering::Relaxed),
            avg_duration,
            self.transfers_started.load(Ordering::Relaxed),
            self.transfers_completed.load(Ordering::Relaxed),
            self.transfers_failed.load(Ordering::Relaxed),
            self.bytes_transferred.load(Ordering::Relaxed),
            self.searches_started.load(Ordering::Relaxed),
            self.search_results_total.load(Ordering::Relaxed),
            self.messages_sent.load(Ordering::Relaxed),
            self.messages_received.load(Ordering::Relaxed),
            self.cache_hits.load(Ordering::Relaxed),
            self.cache_misses.load(Ordering::Relaxed),
            if (self.cache_hits.load(Ordering::Relaxed) + self.cache_misses.load(Ordering::Relaxed))
                > 0
            {
                ((self.cache_hits.load(Ordering::Relaxed) as f64
                    / (self.cache_hits.load(Ordering::Relaxed)
                        + self.cache_misses.load(Ordering::Relaxed)) as f64)
                    * 100.0) as u64
            } else {
                0
            },
            self.connections_active.load(Ordering::Relaxed),
            self.connections_total.load(Ordering::Relaxed),
        )
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_collector_creation() {
        let collector = MetricsCollector::new();
        assert_eq!(collector.requests_total.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_request_metrics() {
        let collector = MetricsCollector::new();

        collector.record_request();
        assert_eq!(collector.requests_total.load(Ordering::Relaxed), 1);

        collector.record_request_success();
        assert_eq!(collector.requests_success.load(Ordering::Relaxed), 1);

        collector.record_request_error();
        assert_eq!(collector.requests_error.load(Ordering::Relaxed), 1);

        collector.record_request_duration(100);
        assert_eq!(
            collector.request_duration_total_ms.load(Ordering::Relaxed),
            100
        );
    }

    #[test]
    fn test_transfer_metrics() {
        let collector = MetricsCollector::new();

        collector.record_transfer_started();
        assert_eq!(collector.transfers_started.load(Ordering::Relaxed), 1);

        collector.record_transfer_completed(1000);
        assert_eq!(collector.transfers_completed.load(Ordering::Relaxed), 1);
        assert_eq!(collector.bytes_transferred.load(Ordering::Relaxed), 1000);

        collector.record_transfer_failed();
        assert_eq!(collector.transfers_failed.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_cache_metrics() {
        let collector = MetricsCollector::new();

        for _ in 0..8 {
            collector.record_cache_hit();
        }
        for _ in 0..2 {
            collector.record_cache_miss();
        }

        assert_eq!(collector.cache_hits.load(Ordering::Relaxed), 8);
        assert_eq!(collector.cache_misses.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn test_prometheus_export() {
        let collector = MetricsCollector::new();

        collector.record_request();
        collector.record_request_success();
        collector.record_request_duration(100);

        let prometheus = collector.export_prometheus();

        assert!(prometheus.contains("http_requests_total"));
        assert!(prometheus.contains("http_requests_success"));
        assert!(prometheus.contains("# TYPE http_requests_total counter"));
    }

    #[test]
    fn test_json_export() {
        let collector = MetricsCollector::new();

        collector.record_request();
        collector.record_transfer_started();

        let json = collector.export_json();

        assert!(json.contains("\"requests_total\""));
        assert!(json.contains("\"transfers\""));
        assert!(json.contains("\"started\""));
    }
}
