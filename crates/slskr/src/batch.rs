/// Batch operations API for Phase 12
///
/// Allows clients to send multiple API requests in a single HTTP call with
/// proper validation, error handling, and optional atomic execution.
use serde_json::{json, Value};
use std::collections::HashMap;

pub const MAX_BATCH_OPERATIONS: usize = 100;

/// Represents a single operation in a batch request
#[derive(Debug, Clone)]
pub struct BatchOperation {
    pub id: String,
    pub method: String,
    pub path: String,
    pub body: Option<String>,
    pub headers: HashMap<String, String>,
}

/// Result of a single batch operation
#[derive(Debug, Clone)]
pub struct BatchOperationResult {
    pub id: String,
    pub status: u16,
    pub body: String,
    pub headers: HashMap<String, String>,
    pub error: Option<String>,
}

/// Configuration for batch execution
#[derive(Debug, Clone)]
pub struct BatchConfig {
    pub max_operations: usize,
    pub timeout_ms: u64,
    pub atomic: bool,
    pub continue_on_error: bool,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_operations: MAX_BATCH_OPERATIONS,
            timeout_ms: 30000,
            atomic: false,
            continue_on_error: true,
        }
    }
}

/// Parse and validate a batch request
pub fn parse_batch_request(body: &str) -> Result<(Vec<BatchOperation>, BatchConfig), String> {
    let json: Value = serde_json::from_str(body).map_err(|e| format!("Invalid JSON: {}", e))?;

    let operations_arr = json
        .get("operations")
        .and_then(|v| v.as_array())
        .ok_or("Missing 'operations' array")?;

    // Check max operations
    if operations_arr.len() > MAX_BATCH_OPERATIONS {
        return Err(format!(
            "Too many operations: {}, max is {MAX_BATCH_OPERATIONS}",
            operations_arr.len(),
        ));
    }

    let mut operations = Vec::new();

    for (idx, op) in operations_arr.iter().enumerate() {
        let id = op
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or(&format!("op_{}", idx))
            .to_string();

        let method = op
            .get("method")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("Operation {} missing method", idx))?
            .to_string();

        let path = op
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("Operation {} missing path", idx))?
            .to_string();

        // Validate method
        if !matches!(
            method.as_str(),
            "GET" | "POST" | "PUT" | "DELETE" | "PATCH" | "HEAD" | "OPTIONS"
        ) {
            return Err(format!("Operation {} has invalid method: {}", idx, method));
        }

        // Validate path
        if !path.starts_with('/') {
            return Err(format!("Operation {} has invalid path: {}", idx, path));
        }

        let body = op
            .get("body")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let headers = op
            .get("headers")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
                    .collect()
            })
            .unwrap_or_default();

        operations.push(BatchOperation {
            id,
            method,
            path,
            body,
            headers,
        });
    }

    // Parse batch config
    let config = if let Some(batch_config) = json.get("config").and_then(|v| v.as_object()) {
        BatchConfig {
            max_operations: batch_config
                .get("maxOperations")
                .and_then(|v| v.as_u64())
                .map(|value| usize::try_from(value).unwrap_or(usize::MAX))
                .unwrap_or(MAX_BATCH_OPERATIONS),
            timeout_ms: batch_config
                .get("timeoutMs")
                .and_then(|v| v.as_u64())
                .unwrap_or(30000),
            atomic: batch_config
                .get("atomic")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            continue_on_error: batch_config
                .get("continueOnError")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
        }
    } else {
        BatchConfig::default()
    };

    if !(1..=MAX_BATCH_OPERATIONS).contains(&config.max_operations) {
        return Err(format!(
            "maxOperations must be between 1 and {MAX_BATCH_OPERATIONS}"
        ));
    }
    if operations.len() > config.max_operations {
        return Err(format!(
            "Too many operations: {}, configured max is {}",
            operations.len(),
            config.max_operations
        ));
    }

    Ok((operations, config))
}

/// Validate batch operations
pub fn validate_batch_operations(operations: &[BatchOperation]) -> Result<(), String> {
    if operations.is_empty() {
        return Err("No operations in batch".to_string());
    }

    // Check for duplicate IDs
    let mut ids = std::collections::HashSet::new();
    for op in operations {
        if !ids.insert(&op.id) {
            return Err(format!("Duplicate operation ID: {}", op.id));
        }
    }

    // Validate individual operations
    for op in operations {
        // Check for POST/PUT/PATCH without body on paths that need it
        if matches!(op.method.as_str(), "POST" | "PUT" | "PATCH")
            && op.body.is_none()
            && !op.path.ends_with("/cancel")
            && !op.path.ends_with("/resume")
        {
            // Some endpoints might not require body, so we be lenient here
        }
    }

    Ok(())
}

/// Format batch results as JSON
pub fn format_batch_response(results: Vec<BatchOperationResult>) -> String {
    let response = json!({
        "results": results.iter().map(|r| {
            json!({
                "id": r.id,
                "status": r.status,
                "body": r.body,
                "error": r.error
            })
        }).collect::<Vec<_>>(),
        "count": results.len(),
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    });

    response.to_string()
}

/// Create an error result for a batch operation
pub fn create_error_result(id: String, error: String) -> BatchOperationResult {
    BatchOperationResult {
        id,
        status: 400,
        body: "".to_string(),
        headers: HashMap::new(),
        error: Some(error),
    }
}

/// Create a successful result for a batch operation
pub fn create_success_result(id: String, status: u16, body: String) -> BatchOperationResult {
    BatchOperationResult {
        id,
        status,
        body,
        headers: HashMap::new(),
        error: None,
    }
}

/// Check if operation would cause side effects that shouldn't be atomic
pub fn is_safe_operation(method: &str, path: &str) -> bool {
    // GET and HEAD are always safe
    if matches!(method, "GET" | "HEAD") {
        return true;
    }

    // POST/PUT/PATCH/DELETE to test paths are safe
    if path.contains("/test") || path.contains("/validate") {
        return true;
    }

    // Most operations are safe - only prevent atomic execution if explicitly marked
    !path.contains("/atomic-unsafe")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_batch_request_simple() {
        let body = r#"{
            "operations": [
                {
                    "id": "op1",
                    "method": "GET",
                    "path": "/api/health"
                }
            ]
        }"#;

        let (ops, config) = parse_batch_request(body).unwrap();
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].id, "op1");
        assert_eq!(ops[0].method, "GET");
        assert_eq!(ops[0].path, "/api/health");
        assert!(!config.atomic);
    }

    #[test]
    fn test_parse_batch_request_multiple() {
        let body = r#"{
            "operations": [
                {
                    "id": "op1",
                    "method": "GET",
                    "path": "/api/health"
                },
                {
                    "id": "op2",
                    "method": "POST",
                    "path": "/api/searches",
                    "body": "{\"query\":\"test\"}"
                }
            ]
        }"#;

        let (ops, _config) = parse_batch_request(body).unwrap();
        assert_eq!(ops.len(), 2);
    }

    #[test]
    fn test_parse_batch_request_with_config() {
        let body = r#"{
            "operations": [
                {
                    "id": "op1",
                    "method": "GET",
                    "path": "/api/health"
                }
            ],
            "config": {
                "atomic": true,
                "timeoutMs": 60000,
                "continueOnError": false
            }
        }"#;

        let (_ops, config) = parse_batch_request(body).unwrap();
        assert!(config.atomic);
        assert_eq!(config.timeout_ms, 60000);
        assert!(!config.continue_on_error);
    }

    #[test]
    fn test_batch_rejects_invalid_or_exceeded_configured_limit() {
        for max_operations in [0, 101] {
            let body = format!(
                r#"{{"operations":[{{"id":"op1","method":"GET","path":"/api/health"}}],"config":{{"maxOperations":{max_operations}}}}}"#
            );
            let error = parse_batch_request(&body).expect_err("invalid limit must fail");
            assert!(error.contains("maxOperations"), "{error}");
        }

        let body = r#"{
            "operations": [
                {"id":"op1","method":"GET","path":"/api/health"},
                {"id":"op2","method":"GET","path":"/api/stats"}
            ],
            "config": {"maxOperations": 1}
        }"#;
        let error = parse_batch_request(body).expect_err("partial batch must fail");
        assert!(error.contains("configured max is 1"), "{error}");
    }

    #[test]
    fn test_parse_batch_invalid_json() {
        let body = "not json";
        let result = parse_batch_request(body);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_batch_missing_operations() {
        let body = r#"{"config": {}}"#;
        let result = parse_batch_request(body);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_batch_operations_empty() {
        let ops = vec![];
        let result = validate_batch_operations(&ops);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_batch_duplicate_ids() {
        let ops = vec![
            BatchOperation {
                id: "op1".to_string(),
                method: "GET".to_string(),
                path: "/api/health".to_string(),
                body: None,
                headers: HashMap::new(),
            },
            BatchOperation {
                id: "op1".to_string(),
                method: "GET".to_string(),
                path: "/api/version".to_string(),
                body: None,
                headers: HashMap::new(),
            },
        ];

        let result = validate_batch_operations(&ops);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_batch_valid() {
        let ops = vec![
            BatchOperation {
                id: "op1".to_string(),
                method: "GET".to_string(),
                path: "/api/health".to_string(),
                body: None,
                headers: HashMap::new(),
            },
            BatchOperation {
                id: "op2".to_string(),
                method: "POST".to_string(),
                path: "/api/searches".to_string(),
                body: Some("{}".to_string()),
                headers: HashMap::new(),
            },
        ];

        let result = validate_batch_operations(&ops);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_error_result() {
        let result = create_error_result("op1".to_string(), "Test error".to_string());
        assert_eq!(result.id, "op1");
        assert_eq!(result.status, 400);
        assert_eq!(result.error, Some("Test error".to_string()));
    }

    #[test]
    fn test_create_success_result() {
        let result = create_success_result("op1".to_string(), 200, "OK".to_string());
        assert_eq!(result.id, "op1");
        assert_eq!(result.status, 200);
        assert_eq!(result.body, "OK");
        assert!(result.error.is_none());
    }

    #[test]
    fn test_format_batch_response() {
        let results = vec![
            create_success_result("op1".to_string(), 200, "OK".to_string()),
            create_error_result("op2".to_string(), "Failed".to_string()),
        ];

        let response = format_batch_response(results);
        // Parse JSON to verify structure
        let json: Result<serde_json::Value, _> = serde_json::from_str(&response);
        assert!(json.is_ok());
        let json = json.unwrap();
        assert_eq!(json["count"], 2);
        assert_eq!(json["results"].as_array().unwrap().len(), 2);
        assert!(response.contains("op1"));
        assert!(response.contains("op2"));
    }

    #[test]
    fn test_is_safe_operation() {
        assert!(is_safe_operation("GET", "/api/health"));
        assert!(is_safe_operation("HEAD", "/api/health"));
        assert!(is_safe_operation("POST", "/api/test"));
        assert!(is_safe_operation("POST", "/api/validate"));
        assert!(!is_safe_operation(
            "POST",
            "/api/transfers/123/atomic-unsafe"
        ));
    }

    #[test]
    fn test_batch_config_defaults() {
        let config = BatchConfig::default();
        assert_eq!(config.max_operations, 100);
        assert_eq!(config.timeout_ms, 30000);
        assert!(!config.atomic);
        assert!(config.continue_on_error);
    }
}
