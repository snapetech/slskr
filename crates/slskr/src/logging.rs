//! Structured logging for HTTP API requests and responses.

use std::time::Instant;

/// HTTP request log entry
#[derive(Debug, Clone)]
pub struct HttpRequestLog {
    pub method: String,
    pub path: String,
    pub query: Option<String>,
    pub remote_addr: Option<String>,
    pub timestamp: String,
}

/// HTTP response log entry
#[derive(Debug, Clone)]
pub struct HttpResponseLog {
    pub status_code: u16,
    pub content_length: usize,
    pub duration_ms: u128,
    pub error: Option<String>,
}

/// Combined HTTP transaction log
#[derive(Debug, Clone)]
pub struct HttpTransactionLog {
    pub request: HttpRequestLog,
    pub response: HttpResponseLog,
}

/// Log level for filtering
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// Global logging configuration
pub struct LogConfig {
    pub level: LogLevel,
    pub log_requests: bool,
    pub log_responses: bool,
    pub log_errors_only: bool,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            log_requests: true,
            log_responses: true,
            log_errors_only: false,
        }
    }
}

impl LogConfig {
    pub fn should_log(&self, level: LogLevel) -> bool {
        self.level <= level
    }

    pub fn level_name(level: LogLevel) -> &'static str {
        match level {
            LogLevel::Trace => "Trace",
            LogLevel::Debug => "Debug",
            LogLevel::Info => "Information",
            LogLevel::Warn => "Warning",
            LogLevel::Error => "Error",
        }
    }

    pub fn parse_level(value: &str) -> Option<LogLevel> {
        match value.trim().to_ascii_lowercase().as_str() {
            "trace" => Some(LogLevel::Trace),
            "debug" => Some(LogLevel::Debug),
            "info" | "information" => Some(LogLevel::Info),
            "warn" | "warning" => Some(LogLevel::Warn),
            "error" => Some(LogLevel::Error),
            _ => None,
        }
    }

    /// Parse log level from environment variable
    pub fn from_env() -> Self {
        let level = std::env::var("SLSKR_LOG_LEVEL")
            .ok()
            .or_else(|| std::env::var("RUST_LOG").ok())
            .and_then(|value| Self::parse_level(&value))
            .unwrap_or(LogLevel::Info);

        Self {
            level,
            log_requests: level <= LogLevel::Debug,
            log_responses: level <= LogLevel::Info,
            log_errors_only: false,
        }
    }
}

/// Format timestamp in ISO8601 with milliseconds
pub fn format_timestamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let millis = now.subsec_millis();
    let datetime = std::time::UNIX_EPOCH + std::time::Duration::from_secs(secs);

    // Simple ISO8601 formatting
    format!("{:?}Z+{:03}", datetime, millis)
}

/// Extract status code from HTTP response status string
pub fn status_code_from_string(status: &str) -> u16 {
    status
        .split_whitespace()
        .next()
        .and_then(|code| code.parse().ok())
        .unwrap_or(500)
}

/// Log HTTP request
pub fn log_request(config: &LogConfig, log: &HttpRequestLog) {
    if !config.log_requests || !config.should_log(LogLevel::Debug) {
        return;
    }

    let query_str = redacted_query_suffix(log.query.as_deref());
    eprintln!(
        "[{}] {} {} {} (from {})",
        log.timestamp,
        log.method,
        log.path,
        query_str,
        log.remote_addr.as_deref().unwrap_or("unknown")
    );
}

/// Log HTTP response
pub fn log_response(config: &LogConfig, log: &HttpResponseLog) {
    if !config.log_responses {
        return;
    }

    // Only log errors at appropriate levels
    if config.log_errors_only && log.status_code < 400 {
        return;
    }

    let status_level = match log.status_code {
        200..=299 => LogLevel::Info,
        300..=399 => LogLevel::Info,
        400..=499 => LogLevel::Warn,
        500..=599 => LogLevel::Error,
        _ => LogLevel::Info,
    };

    if !config.should_log(status_level) {
        return;
    }

    let error_str = log
        .error
        .as_ref()
        .map(|e| format!(" - {}", e))
        .unwrap_or_default();

    let level_str = LogConfig::level_name(status_level);

    eprintln!(
        "[{}] {} {} bytes in {}ms{}",
        level_str, log.status_code, log.content_length, log.duration_ms, error_str
    );
}

/// Log complete HTTP transaction
pub fn log_transaction(config: &LogConfig, log: &HttpTransactionLog) {
    // Skip logging successful requests if errors-only mode
    if config.log_errors_only && log.response.status_code < 400 {
        return;
    }

    let status_level = response_level(log.response.status_code);
    if !config.should_log(status_level) {
        return;
    }

    let method_color = match log.request.method.as_str() {
        "GET" => "36",    // cyan
        "POST" => "32",   // green
        "PUT" => "33",    // yellow
        "DELETE" => "31", // red
        _ => "37",        // white
    };

    let status_color = match log.response.status_code {
        200..=299 => "32", // green
        300..=399 => "36", // cyan
        400..=499 => "33", // yellow
        500..=599 => "31", // red
        _ => "37",         // white
    };

    let query_str = redacted_query_suffix(log.request.query.as_deref());

    let error_str = log
        .response
        .error
        .as_ref()
        .map(|e| format!(" - {}", e))
        .unwrap_or_default();

    eprintln!(
        "[{}] \x1b[{}m{} {}{}\x1b[0m \x1b[{}m{}\x1b[0m {} bytes in {}ms{}",
        log.request.timestamp,
        method_color,
        log.request.method,
        log.request.path,
        query_str,
        status_color,
        log.response.status_code,
        log.response.content_length,
        log.response.duration_ms,
        error_str
    );
}

pub fn response_level(status_code: u16) -> LogLevel {
    match status_code {
        400..=499 => LogLevel::Warn,
        500..=599 => LogLevel::Error,
        _ => LogLevel::Info,
    }
}

pub fn transaction_summary(log: &HttpTransactionLog) -> String {
    let query_str = redacted_query_suffix(log.request.query.as_deref());
    let error_str = log
        .response
        .error
        .as_ref()
        .map(|e| format!(" - {}", e))
        .unwrap_or_default();
    format!(
        "{} {}{} {} {} bytes in {}ms{}",
        log.request.method,
        log.request.path,
        query_str,
        log.response.status_code,
        log.response.content_length,
        log.response.duration_ms,
        error_str
    )
}

fn redacted_query_suffix(query: Option<&str>) -> &'static str {
    if query.is_some() {
        "?<redacted>"
    } else {
        ""
    }
}

/// Start timing measurement
#[inline]
pub fn start_timer() -> Instant {
    Instant::now()
}

/// Get elapsed milliseconds
#[inline]
pub fn elapsed_ms(start: Instant) -> u128 {
    start.elapsed().as_millis()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_code_parsing() {
        assert_eq!(status_code_from_string("200 OK"), 200);
        assert_eq!(status_code_from_string("404 Not Found"), 404);
        assert_eq!(status_code_from_string("500 Internal Server Error"), 500);
    }

    #[test]
    fn test_log_level_parsing() {
        let config = LogConfig {
            level: LogLevel::Debug,
            ..Default::default()
        };
        assert!(config.log_requests);
        assert!(config.log_responses);
    }

    #[test]
    fn transaction_summary_redacts_query_credentials() {
        let summary = transaction_summary(&HttpTransactionLog {
            request: HttpRequestLog {
                method: "GET".to_owned(),
                path: "/api/integrations/spotify/callback".to_owned(),
                query: Some("code=oauth-secret&state=state-secret".to_owned()),
                remote_addr: None,
                timestamp: "fixture".to_owned(),
            },
            response: HttpResponseLog {
                status_code: 200,
                content_length: 2,
                duration_ms: 1,
                error: None,
            },
        });
        assert_eq!(
            summary,
            "GET /api/integrations/spotify/callback?<redacted> 200 2 bytes in 1ms"
        );
        assert!(!summary.contains("oauth-secret"));
        assert!(!summary.contains("state-secret"));
    }

    #[test]
    fn test_timer() {
        let start = start_timer();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let elapsed = elapsed_ms(start);
        assert!(elapsed >= 9); // Allow for timer variance
    }
}
