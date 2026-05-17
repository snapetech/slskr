#![allow(clippy::inherent_to_string_shadow_display)]
/// Request tracing and correlation ID support
///
/// Implements distributed request tracing with correlation IDs,
/// allowing tracking of requests across service boundaries.
use std::cell::RefCell;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

thread_local! {
    static CORRELATION_ID: RefCell<Option<String>> = const { RefCell::new(None) };
    static REQUEST_SPAN: RefCell<Option<RequestSpan>> = const { RefCell::new(None) };
}

/// Unique correlation ID for request tracing
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CorrelationId(String);

static COUNTER: AtomicU64 = AtomicU64::new(0);

impl CorrelationId {
    /// Generate a new correlation ID
    pub fn new() -> Self {
        let num = COUNTER.fetch_add(1, Ordering::Relaxed);
        CorrelationId(format!("corr-{}", num))
    }

    /// Create from existing string
    pub fn from_string(id: String) -> Self {
        CorrelationId(id)
    }

    /// Get as string reference
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get as string
    pub fn to_string(&self) -> String {
        self.0.clone()
    }
}

impl fmt::Display for CorrelationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for CorrelationId {
    fn default() -> Self {
        Self::new()
    }
}

/// Request timing information
#[derive(Clone, Debug)]
pub struct RequestTiming {
    pub start_time: std::time::Instant,
    pub method: String,
    pub path: String,
    pub status: u16,
    pub duration_ms: u64,
}

impl RequestTiming {
    /// Create new timing record
    pub fn new(method: String, path: String) -> Self {
        RequestTiming {
            start_time: std::time::Instant::now(),
            method,
            path,
            status: 0,
            duration_ms: 0,
        }
    }

    /// Record response status and calculate duration
    pub fn complete(&mut self, status: u16) {
        self.status = status;
        self.duration_ms = self.start_time.elapsed().as_millis() as u64;
    }

    /// Check if request is slow (>1s)
    pub fn is_slow(&self) -> bool {
        self.duration_ms > 1000
    }

    /// Check if request is very slow (>5s)
    pub fn is_very_slow(&self) -> bool {
        self.duration_ms > 5000
    }
}

/// Complete request span with timing and correlation
#[derive(Clone, Debug)]
pub struct RequestSpan {
    pub correlation_id: CorrelationId,
    pub timing: RequestTiming,
    pub user_agent: Option<String>,
    pub client_ip: Option<String>,
}

impl RequestSpan {
    /// Create new span
    pub fn new(
        method: String,
        path: String,
        user_agent: Option<String>,
        client_ip: Option<String>,
    ) -> Self {
        RequestSpan {
            correlation_id: CorrelationId::new(),
            timing: RequestTiming::new(method, path),
            user_agent,
            client_ip,
        }
    }

    /// Create span with existing correlation ID
    pub fn with_correlation(
        correlation_id: CorrelationId,
        method: String,
        path: String,
        user_agent: Option<String>,
        client_ip: Option<String>,
    ) -> Self {
        RequestSpan {
            correlation_id,
            timing: RequestTiming::new(method, path),
            user_agent,
            client_ip,
        }
    }

    /// Log formatted trace
    pub fn log(&self) {
        if !trace_logging_enabled() {
            return;
        }
        if self.timing.is_very_slow() {
            eprintln!(
                "[TRACE] {} {} {} {} - VERY SLOW {}ms ({})",
                self.correlation_id,
                self.timing.method,
                self.timing.path,
                self.timing.status,
                self.timing.duration_ms,
                self.client_ip.as_deref().unwrap_or("unknown")
            );
        } else if self.timing.is_slow() {
            eprintln!(
                "[TRACE] {} {} {} {} - SLOW {}ms ({})",
                self.correlation_id,
                self.timing.method,
                self.timing.path,
                self.timing.status,
                self.timing.duration_ms,
                self.client_ip.as_deref().unwrap_or("unknown")
            );
        } else {
            eprintln!(
                "[TRACE] {} {} {} {} - {}ms ({})",
                self.correlation_id,
                self.timing.method,
                self.timing.path,
                self.timing.status,
                self.timing.duration_ms,
                self.client_ip.as_deref().unwrap_or("unknown")
            );
        }
    }
}

fn trace_logging_enabled() -> bool {
    matches!(
        std::env::var("SLSKR_TRACE_REQUESTS")
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "yes" | "on"
    ) || std::env::var("SLSKR_LOG_LEVEL")
        .or_else(|_| std::env::var("RUST_LOG"))
        .map(|value| value.eq_ignore_ascii_case("trace"))
        .unwrap_or(false)
}

/// Set correlation ID for current request
pub fn set_correlation_id(id: CorrelationId) {
    CORRELATION_ID.with(|cid| {
        *cid.borrow_mut() = Some(id.to_string());
    });
}

/// Get current correlation ID
pub fn get_correlation_id() -> CorrelationId {
    CORRELATION_ID.with(|cid| {
        cid.borrow()
            .as_ref()
            .map(|id| CorrelationId::from_string(id.clone()))
            .unwrap_or_else(CorrelationId::new)
    })
}

/// Set request span
pub fn set_request_span(span: RequestSpan) {
    REQUEST_SPAN.with(|rs| {
        *rs.borrow_mut() = Some(span);
    });
}

/// Get current request span
pub fn get_request_span() -> Option<RequestSpan> {
    REQUEST_SPAN.with(|rs| rs.borrow().clone())
}

/// Complete current span and log it
pub fn complete_request_span(status: u16) {
    REQUEST_SPAN.with(|rs| {
        if let Some(mut span) = rs.borrow_mut().take() {
            span.timing.complete(status);
            span.log();
        }
    });
}

/// Clear all tracing context
pub fn clear_context() {
    CORRELATION_ID.with(|cid| *cid.borrow_mut() = None);
    REQUEST_SPAN.with(|rs| *rs.borrow_mut() = None);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_correlation_id_generation() {
        let id1 = CorrelationId::new();
        let id2 = CorrelationId::new();
        assert_ne!(id1, id2);
        assert!(id1.as_str().starts_with("corr-"));
    }

    #[test]
    fn test_correlation_id_from_string() {
        let id = CorrelationId::from_string("custom-id".to_string());
        assert_eq!(id.as_str(), "custom-id");
    }

    #[test]
    fn test_request_timing() {
        let mut timing = RequestTiming::new("GET".to_string(), "/api/test".to_string());
        assert_eq!(timing.status, 0);
        timing.complete(200);
        assert_eq!(timing.status, 200);
        assert_eq!(timing.method, "GET");
        assert_eq!(timing.path, "/api/test");
    }

    #[test]
    fn test_slow_request_detection() {
        let mut timing = RequestTiming::new("GET".to_string(), "/api/test".to_string());
        timing.duration_ms = 500;
        assert!(!timing.is_slow());

        timing.duration_ms = 1500;
        assert!(timing.is_slow());
        assert!(!timing.is_very_slow());

        timing.duration_ms = 6000;
        assert!(timing.is_very_slow());
    }

    #[test]
    fn test_request_span_creation() {
        let span = RequestSpan::new(
            "GET".to_string(),
            "/api/test".to_string(),
            Some("Mozilla/5.0".to_string()),
            Some("127.0.0.1".to_string()),
        );

        assert!(span.correlation_id.as_str().starts_with("corr-"));
        assert_eq!(span.user_agent, Some("Mozilla/5.0".to_string()));
        assert_eq!(span.client_ip, Some("127.0.0.1".to_string()));
    }

    #[test]
    fn test_correlation_context_storage() {
        clear_context();

        let id = CorrelationId::new();
        set_correlation_id(id.clone());

        let retrieved = get_correlation_id();
        assert_eq!(retrieved, id);

        clear_context();
        let new_id = get_correlation_id();
        assert_ne!(new_id, id); // Should generate new ID
    }

    #[test]
    fn test_request_span_context() {
        clear_context();

        let span = RequestSpan::new("POST".to_string(), "/api/search".to_string(), None, None);

        set_request_span(span.clone());

        let retrieved = get_request_span();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().correlation_id, span.correlation_id);
    }
}
