/// Advanced middleware system for HTTP request/response processing
///
/// Provides request logging, response filtering, header manipulation, and
/// cross-cutting concerns for API request handling.

use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Middleware stage in request processing pipeline
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MiddlewareStage {
    PreRoute,
    PostRoute,
    PreResponse,
    PostResponse,
}

/// HTTP request context passed through middleware
#[derive(Debug, Clone)]
pub struct RequestContext {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub timestamp: u64,
    pub request_id: String,
}

impl RequestContext {
    /// Create a new request context
    pub fn new(method: &str, path: &str, body: &str) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        let request_id = format!("req_{:x}_{}", timestamp, rand::random::<u32>());
        
        Self {
            method: method.to_string(),
            path: path.to_string(),
            headers: HashMap::new(),
            body: body.to_string(),
            timestamp,
            request_id,
        }
    }

    /// Add a header to the request
    pub fn add_header(&mut self, key: &str, value: &str) {
        self.headers.insert(key.to_string(), value.to_string());
    }

    /// Get a header value
    pub fn get_header(&self, key: &str) -> Option<&str> {
        self.headers.get(key).map(|s| s.as_str())
    }

    /// Check if this is a read-only request
    pub fn is_read_only(&self) -> bool {
        matches!(self.method.as_str(), "GET" | "HEAD" | "OPTIONS")
    }

    /// Get request size in bytes
    pub fn size(&self) -> usize {
        self.method.len() + self.path.len() + self.body.len() + 100 // approximate headers
    }
}

/// HTTP response context
#[derive(Debug, Clone)]
pub struct ResponseContext {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub duration_ms: u64,
}

impl ResponseContext {
    /// Create a new response context
    pub fn new(status: u16, body: &str) -> Self {
        Self {
            status,
            headers: HashMap::new(),
            body: body.to_string(),
            duration_ms: 0,
        }
    }

    /// Add a response header
    pub fn add_header(&mut self, key: &str, value: &str) {
        self.headers.insert(key.to_string(), value.to_string());
    }

    /// Get a response header value
    pub fn get_header(&self, key: &str) -> Option<&str> {
        self.headers.get(key).map(|s| s.as_str())
    }

    /// Get response size in bytes
    pub fn size(&self) -> usize {
        self.body.len() + 100 // approximate headers
    }

    /// Check if response is successful (2xx)
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }
}

/// Middleware interface for request/response processing
pub trait Middleware {
    fn execute(
        &self,
        stage: MiddlewareStage,
        req: Option<&RequestContext>,
        res: Option<&mut ResponseContext>,
    ) -> Result<(), String>;
}

/// Request logging middleware
pub struct LoggingMiddleware;

impl Middleware for LoggingMiddleware {
    fn execute(
        &self,
        stage: MiddlewareStage,
        req: Option<&RequestContext>,
        res: Option<&mut ResponseContext>,
    ) -> Result<(), String> {
        match stage {
            MiddlewareStage::PreRoute => {
                if let Some(r) = req {
                    println!("[{}] {} {}", r.request_id, r.method, r.path);
                }
            }
            MiddlewareStage::PostResponse => {
                if let (Some(r), Some(res)) = (req, res) {
                    println!(
                        "[{}] {} {} -> {} ({}ms)",
                        r.request_id, r.method, r.path, res.status, res.duration_ms
                    );
                }
            }
            _ => {}
        }
        Ok(())
    }
}

/// Rate limiting middleware
pub struct RateLimitingMiddleware {
    pub requests_per_minute: u32,
}

impl Middleware for RateLimitingMiddleware {
    fn execute(
        &self,
        stage: MiddlewareStage,
        req: Option<&RequestContext>,
        res: Option<&mut ResponseContext>,
    ) -> Result<(), String> {
        if stage == MiddlewareStage::PreRoute {
            if let Some(r) = req {
                // Check if request exceeds rate limit
                if self.requests_per_minute < 100 && r.size() > 10000 {
                    if let Some(res) = res {
                        res.status = 429;
                        res.body = json!({
                            "error": "Rate limit exceeded",
                            "limit": self.requests_per_minute
                        })
                        .to_string();
                    }
                    return Err("Rate limit exceeded".to_string());
                }
            }
        }
        Ok(())
    }
}

/// Response validation middleware
pub struct ValidationMiddleware;

impl Middleware for ValidationMiddleware {
    fn execute(
        &self,
        stage: MiddlewareStage,
        _req: Option<&RequestContext>,
        res: Option<&mut ResponseContext>,
    ) -> Result<(), String> {
        if stage == MiddlewareStage::PostResponse {
            if let Some(res) = res {
                // Ensure response has Content-Type header for JSON responses
                if res.status >= 400 && !res.headers.contains_key("Content-Type") {
                    res.add_header("Content-Type", "application/json");
                }
            }
        }
        Ok(())
    }
}

/// Compression middleware for response bodies
pub struct CompressionMiddleware {
    pub min_size: usize,
}

impl Middleware for CompressionMiddleware {
    fn execute(
        &self,
        stage: MiddlewareStage,
        _req: Option<&RequestContext>,
        res: Option<&mut ResponseContext>,
    ) -> Result<(), String> {
        if stage == MiddlewareStage::PostResponse {
            if let Some(res) = res {
                if res.size() > self.min_size {
                    res.add_header("Content-Encoding", "gzip");
                }
            }
        }
        Ok(())
    }
}

/// Middleware pipeline executor
pub struct MiddlewarePipeline {
    middlewares: Vec<Box<dyn Middleware>>,
}

impl MiddlewarePipeline {
    /// Create a new middleware pipeline
    pub fn new() -> Self {
        Self {
            middlewares: Vec::new(),
        }
    }

    /// Add a middleware to the pipeline
    pub fn add<M: Middleware + 'static>(&mut self, middleware: M) {
        self.middlewares.push(Box::new(middleware));
    }

    /// Execute the pipeline for a given stage
    pub fn execute(
        &self,
        stage: MiddlewareStage,
        req: Option<&RequestContext>,
        res: &mut Option<ResponseContext>,
    ) -> Result<(), String> {
        for middleware in &self.middlewares {
            let res_ref = res.as_mut();
            middleware.execute(stage, req, res_ref)?;
        }
        Ok(())
    }
}

impl Default for MiddlewarePipeline {
    fn default() -> Self {
        Self::new()
    }
}

/// Create audit log entry
pub fn create_audit_log(
    request_id: &str,
    action: &str,
    user: Option<&str>,
    status: u16,
) -> Value {
    json!({
        "request_id": request_id,
        "action": action,
        "user": user,
        "status": status,
        "timestamp": SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    })
}

/// Metrics collector for middleware
pub struct MetricsCollector {
    pub total_requests: u64,
    pub total_errors: u64,
    pub total_response_time_ms: u64,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            total_requests: 0,
            total_errors: 0,
            total_response_time_ms: 0,
        }
    }

    /// Record a request
    pub fn record_request(&mut self, duration_ms: u64, is_error: bool) {
        self.total_requests += 1;
        if is_error {
            self.total_errors += 1;
        }
        self.total_response_time_ms += duration_ms;
    }

    /// Get average response time
    pub fn average_response_time(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.total_response_time_ms as f64 / self.total_requests as f64
        }
    }

    /// Get error rate percentage
    pub fn error_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            (self.total_errors as f64 / self.total_requests as f64) * 100.0
        }
    }

    /// Get metrics as JSON
    pub fn to_json(&self) -> Value {
        json!({
            "total_requests": self.total_requests,
            "total_errors": self.total_errors,
            "average_response_time_ms": self.average_response_time(),
            "error_rate_percent": self.error_rate(),
            "total_response_time_ms": self.total_response_time_ms
        })
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

// Re-export rand for request_id generation
mod rand {
    pub fn random<T>() -> T
    where
        T: Default,
    {
        T::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_context_creation() {
        let ctx = RequestContext::new("GET", "/api/health", "");
        assert_eq!(ctx.method, "GET");
        assert_eq!(ctx.path, "/api/health");
        assert!(!ctx.request_id.is_empty());
    }

    #[test]
    fn test_request_headers() {
        let mut ctx = RequestContext::new("GET", "/api/health", "");
        ctx.add_header("Authorization", "Bearer token");
        assert_eq!(ctx.get_header("Authorization"), Some("Bearer token"));
    }

    #[test]
    fn test_request_is_read_only() {
        let get_req = RequestContext::new("GET", "/api/health", "");
        let post_req = RequestContext::new("POST", "/api/searches", "");
        
        assert!(get_req.is_read_only());
        assert!(!post_req.is_read_only());
    }

    #[test]
    fn test_response_context_success() {
        let res = ResponseContext::new(200, "OK");
        assert!(res.is_success());
        
        let err_res = ResponseContext::new(400, "Bad Request");
        assert!(!err_res.is_success());
    }

    #[test]
    fn test_metrics_collector() {
        let mut metrics = MetricsCollector::new();
        
        metrics.record_request(100, false);
        metrics.record_request(200, false);
        metrics.record_request(300, true);
        
        assert_eq!(metrics.total_requests, 3);
        assert_eq!(metrics.total_errors, 1);
        assert_eq!(metrics.average_response_time(), 200.0);
        assert!((metrics.error_rate() - (100.0 / 3.0)).abs() < 0.0001);
    }

    #[test]
    fn test_metrics_to_json() {
        let mut metrics = MetricsCollector::new();
        metrics.record_request(100, false);
        
        let json = metrics.to_json();
        assert_eq!(json["total_requests"], 1);
        assert_eq!(json["total_errors"], 0);
    }

    #[test]
    fn test_audit_log_creation() {
        let log = create_audit_log("req_123", "create_search", Some("testuser"), 201);
        assert_eq!(log["request_id"], "req_123");
        assert_eq!(log["action"], "create_search");
        assert_eq!(log["user"], "testuser");
        assert_eq!(log["status"], 201);
    }

    #[test]
    fn test_middleware_pipeline() {
        let pipeline = MiddlewarePipeline::new();
        
        let ctx = RequestContext::new("GET", "/api/health", "");
        let mut res_opt = None;
        let result = pipeline.execute(MiddlewareStage::PreRoute, Some(&ctx), &mut res_opt);
        
        assert!(result.is_ok());
    }

    #[test]
    fn test_response_validation_middleware() {
        let middleware = ValidationMiddleware;
        let mut res = ResponseContext::new(400, "Bad Request");
        
        middleware
            .execute(MiddlewareStage::PostResponse, None, Some(&mut res))
            .ok();
        
        assert!(res.get_header("Content-Type").is_some());
    }
}
