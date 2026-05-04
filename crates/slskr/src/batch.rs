//! Batch operation support for HTTP API

use serde::{Deserialize, Serialize};

/// Batch request operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchOperation {
    pub id: String,
    pub method: String,
    pub path: String,
    pub body: Option<String>,
}

/// Batch operation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResult {
    pub id: String,
    pub status: u16,
    pub body: String,
}

/// Batch request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchRequest {
    pub operations: Vec<BatchOperation>,
}

/// Batch response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResponse {
    pub results: Vec<BatchResult>,
    pub total_time_ms: u128,
}

impl BatchRequest {
    /// Create new batch request
    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
        }
    }

    /// Add operation to batch
    pub fn add_operation(&mut self, op: BatchOperation) {
        self.operations.push(op);
    }

    /// Get operation count
    pub fn len(&self) -> usize {
        self.operations.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }
}

impl Default for BatchRequest {
    fn default() -> Self {
        Self::new()
    }
}

impl BatchResponse {
    /// Create new batch response
    pub fn new(total_time_ms: u128) -> Self {
        Self {
            results: Vec::new(),
            total_time_ms,
        }
    }

    /// Add result to response
    pub fn add_result(&mut self, result: BatchResult) {
        self.results.push(result);
    }

    /// Get all successful results
    pub fn successful_results(&self) -> Vec<&BatchResult> {
        self.results
            .iter()
            .filter(|r| r.status >= 200 && r.status < 300)
            .collect()
    }

    /// Get all failed results
    pub fn failed_results(&self) -> Vec<&BatchResult> {
        self.results
            .iter()
            .filter(|r| r.status >= 400)
            .collect()
    }

    /// Convert to JSON representation
    pub fn to_json(&self) -> String {
        let mut json = String::from("{\"results\":[");
        for (i, result) in self.results.iter().enumerate() {
            if i > 0 {
                json.push(',');
            }
            json.push_str(&format!(
                "{{\"id\":\"{}\",\"status\":{},\"body\":{}}}",
                result.id, result.status, result.body
            ));
        }
        json.push_str(&format!("],\"total_time_ms\":{}}}", self.total_time_ms));
        json
    }
}

/// Parse batch request from JSON
pub fn parse_batch_request(json: &str) -> Result<BatchRequest, String> {
    // Simplified JSON parsing for batch request
    if !json.contains("\"operations\"") {
        return Err("Missing 'operations' field".to_string());
    }

    let mut request = BatchRequest::new();

    // Extract operations array (simplified)
    if let Some(ops_start) = json.find("\"operations\":[") {
        let after_ops = &json[ops_start + 14..];
        let ops_end = after_ops.find(']').ok_or("Invalid operations array")?;
        let ops_str = &after_ops[..ops_end];

        // Split by operation objects
        let mut i = 0;
        loop {
            if let Some(obj_start) = ops_str[i..].find('{') {
                i += obj_start;
                if let Some(obj_end) = ops_str[i..].find('}') {
                    let obj_str = &ops_str[i..=obj_end + i];
                    if let Ok(op) = parse_operation(obj_str) {
                        request.add_operation(op);
                    }
                    i += obj_end + 1;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }

    if request.is_empty() {
        Err("No valid operations found".to_string())
    } else {
        Ok(request)
    }
}

/// Parse single operation from JSON
fn parse_operation(json: &str) -> Result<BatchOperation, String> {
    let id = extract_json_string_field(json, "id").ok_or("Missing 'id' field")?;
    let method = extract_json_string_field(json, "method").ok_or("Missing 'method' field")?;
    let path = extract_json_string_field(json, "path").ok_or("Missing 'path' field")?;
    let body = extract_json_string_field(json, "body");

    Ok(BatchOperation {
        id,
        method,
        path,
        body,
    })
}

/// Extract string field from JSON
fn extract_json_string_field(json: &str, field: &str) -> Option<String> {
    let key = format!("\"{}\":", field);
    let after_key = json.find(&key)?;
    let start = after_key + key.len();
    let after_start = &json[start..].trim_start();

    if after_start.starts_with('"') {
        let string_content = &after_start[1..];
        let end = string_content.find('"')?;
        Some(string_content[..end].to_string())
    } else {
        None
    }
}

/// Builder for batch operations
pub struct BatchBuilder {
    operations: Vec<BatchOperation>,
}

impl BatchBuilder {
    /// Create new batch builder
    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
        }
    }

    /// Add GET operation
    pub fn get(mut self, id: String, path: String) -> Self {
        self.operations.push(BatchOperation {
            id,
            method: "GET".to_string(),
            path,
            body: None,
        });
        self
    }

    /// Add POST operation
    pub fn post(mut self, id: String, path: String, body: String) -> Self {
        self.operations.push(BatchOperation {
            id,
            method: "POST".to_string(),
            path,
            body: Some(body),
        });
        self
    }

    /// Add PUT operation
    pub fn put(mut self, id: String, path: String, body: String) -> Self {
        self.operations.push(BatchOperation {
            id,
            method: "PUT".to_string(),
            path,
            body: Some(body),
        });
        self
    }

    /// Add DELETE operation
    pub fn delete(mut self, id: String, path: String) -> Self {
        self.operations.push(BatchOperation {
            id,
            method: "DELETE".to_string(),
            path,
            body: None,
        });
        self
    }

    /// Build batch request
    pub fn build(self) -> BatchRequest {
        BatchRequest {
            operations: self.operations,
        }
    }
}

impl Default for BatchBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_request_creation() {
        let req = BatchRequest::new();
        assert!(req.is_empty());
    }

    #[test]
    fn test_batch_operation_addition() {
        let mut req = BatchRequest::new();
        let op = BatchOperation {
            id: "op1".to_string(),
            method: "GET".to_string(),
            path: "/api/stats".to_string(),
            body: None,
        };
        req.add_operation(op);
        assert_eq!(req.len(), 1);
    }

    #[test]
    fn test_batch_response_creation() {
        let resp = BatchResponse::new(100);
        assert_eq!(resp.total_time_ms, 100);
        assert!(resp.results.is_empty());
    }

    #[test]
    fn test_batch_response_result_filtering() {
        let mut resp = BatchResponse::new(100);
        resp.add_result(BatchResult {
            id: "op1".to_string(),
            status: 200,
            body: "OK".to_string(),
        });
        resp.add_result(BatchResult {
            id: "op2".to_string(),
            status: 404,
            body: "Not Found".to_string(),
        });

        assert_eq!(resp.successful_results().len(), 1);
        assert_eq!(resp.failed_results().len(), 1);
    }

    #[test]
    fn test_batch_builder() {
        let batch = BatchBuilder::new()
            .get("get1".to_string(), "/api/stats".to_string())
            .post(
                "post1".to_string(),
                "/api/searches".to_string(),
                "{}".to_string(),
            )
            .build();

        assert_eq!(batch.len(), 2);
        assert_eq!(batch.operations[0].method, "GET");
        assert_eq!(batch.operations[1].method, "POST");
    }

    #[test]
    fn test_batch_response_json_serialization() {
        let mut resp = BatchResponse::new(50);
        resp.add_result(BatchResult {
            id: "op1".to_string(),
            status: 200,
            body: "OK".to_string(),
        });

        let json = resp.to_json();
        assert!(json.contains("\"op1\""));
        assert!(json.contains("\"status\":200"));
    }

    #[test]
    fn test_parse_batch_request() {
        let json = r#"{"operations":[{"id":"op1","method":"GET","path":"/api/stats"}]}"#;
        let result = parse_batch_request(json);
        assert!(result.is_ok());
        let req = result.unwrap();
        assert_eq!(req.len(), 1);
    }
}
