/// Response enrichment system for adding metadata and computed fields
///
/// Provides request/response enrichment, field computation, and metadata injection
/// for API responses at application level.

use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Response enrichment metadata
#[derive(Debug, Clone)]
pub struct EnrichmentMetadata {
    pub request_id: String,
    pub timestamp: u64,
    pub processing_time_ms: u64,
    pub version: String,
    pub cached: bool,
}

impl EnrichmentMetadata {
    /// Create new enrichment metadata
    pub fn new(request_id: &str, processing_time_ms: u64) -> Self {
        Self {
            request_id: request_id.to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            processing_time_ms,
            version: "1.0.1".to_string(),
            cached: false,
        }
    }

    /// Convert to JSON value
    pub fn to_json(&self) -> Value {
        json!({
            "request_id": self.request_id,
            "timestamp": self.timestamp,
            "processing_time_ms": self.processing_time_ms,
            "version": self.version,
            "cached": self.cached
        })
    }
}

/// Response enricher
pub struct ResponseEnricher;

impl ResponseEnricher {
    /// Enrich response with metadata
    pub fn enrich_with_metadata(
        response: &Value,
        metadata: &EnrichmentMetadata,
        include_meta: bool,
    ) -> Value {
        if !include_meta {
            return response.clone();
        }

        match response {
            Value::Object(map) => {
                let mut enriched = map.clone();
                enriched.insert("_meta".to_string(), metadata.to_json());
                Value::Object(enriched)
            }
            _ => response.clone(),
        }
    }

    /// Add pagination metadata
    pub fn add_pagination(
        response: &Value,
        total: usize,
        limit: usize,
        offset: usize,
    ) -> Value {
        match response {
            Value::Object(map) => {
                let mut enriched = map.clone();
                enriched.insert(
                    "_pagination".to_string(),
                    json!({
                        "total": total,
                        "limit": limit,
                        "offset": offset,
                        "pages": (total + limit - 1) / limit,
                        "has_next": offset + limit < total,
                        "has_prev": offset > 0
                    }),
                );
                Value::Object(enriched)
            }
            _ => response.clone(),
        }
    }

    /// Add computed fields to response
    pub fn compute_fields(response: &Value, computations: &[(String, String)]) -> Value {
        match response {
            Value::Object(map) => {
                let mut enriched = map.clone();

                for (_field_name, _computation) in computations {
                    // In production, implement actual field computation logic
                    // For now, this is a placeholder
                }

                Value::Object(enriched)
            }
            _ => response.clone(),
        }
    }

    /// Add links to response (HATEOAS)
    pub fn add_links(response: &Value, links: &[(String, String)]) -> Value {
        match response {
            Value::Object(map) => {
                let mut enriched = map.clone();

                let mut links_obj = serde_json::Map::new();
                for (rel, href) in links {
                    links_obj.insert(rel.clone(), Value::String(href.clone()));
                }

                enriched.insert("_links".to_string(), Value::Object(links_obj));
                Value::Object(enriched)
            }
            _ => response.clone(),
        }
    }

    /// Add error context to error response
    pub fn enrich_error(
        error_code: &str,
        message: &str,
        details: Option<&str>,
        request_id: &str,
    ) -> Value {
        json!({
            "error": {
                "code": error_code,
                "message": message,
                "details": details,
                "request_id": request_id,
                "timestamp": SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            }
        })
    }
}

/// Request context enrichment
pub struct RequestEnricher;

impl RequestEnricher {
    /// Extract and enrich request context
    pub fn extract_context(
        method: &str,
        path: &str,
        headers: &HashMap<String, String>,
    ) -> Value {
        json!({
            "method": method,
            "path": path,
            "user_agent": headers.get("User-Agent"),
            "accept": headers.get("Accept"),
            "content_type": headers.get("Content-Type"),
            "auth_type": if headers.contains_key("Authorization") {
                "bearer"
            } else {
                "none"
            }
        })
    }

    /// Validate request prerequisites
    pub fn validate_prerequisites(
        method: &str,
        path: &str,
        has_auth: bool,
    ) -> Result<(), String> {
        // Validate that mutations have authentication
        if matches!(method, "POST" | "PUT" | "DELETE" | "PATCH") && !has_auth {
            return Err("Authentication required for mutations".to_string());
        }

        // Validate path format
        if !path.starts_with('/') {
            return Err("Invalid path format".to_string());
        }

        Ok(())
    }
}

/// Response summarization
pub struct ResponseSummarizer;

impl ResponseSummarizer {
    /// Generate summary of array response
    pub fn summarize_array(data: &[Value]) -> Value {
        let mut counts = HashMap::new();

        for item in data {
            if let Value::Object(map) = item {
                for key in map.keys() {
                    *counts.entry(key.clone()).or_insert(0) += 1;
                }
            }
        }

        json!({
            "count": data.len(),
            "fields": counts
        })
    }

    /// Generate statistics for response
    pub fn compute_stats(values: &[f64]) -> Value {
        if values.is_empty() {
            return json!(null);
        }

        let sum: f64 = values.iter().sum();
        let mean = sum / values.len() as f64;

        let variance = values
            .iter()
            .map(|v| (v - mean).powi(2))
            .sum::<f64>()
            / values.len() as f64;
        let stddev = variance.sqrt();

        let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        json!({
            "count": values.len(),
            "sum": sum,
            "mean": mean,
            "median": values[values.len() / 2],
            "min": min,
            "max": max,
            "stddev": stddev
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enrichment_metadata_creation() {
        let meta = EnrichmentMetadata::new("req_123", 45);
        assert_eq!(meta.request_id, "req_123");
        assert_eq!(meta.processing_time_ms, 45);
    }

    #[test]
    fn test_enrichment_metadata_to_json() {
        let meta = EnrichmentMetadata::new("req_123", 100);
        let json = meta.to_json();
        assert_eq!(json["request_id"], "req_123");
        assert_eq!(json["processing_time_ms"], 100);
    }

    #[test]
    fn test_enrich_with_metadata() {
        let response = json!({
            "data": "test"
        });

        let meta = EnrichmentMetadata::new("req_123", 50);
        let enriched = ResponseEnricher::enrich_with_metadata(&response, &meta, true);

        assert!(enriched.get("_meta").is_some());
        assert_eq!(enriched["data"], "test");
    }

    #[test]
    fn test_add_pagination() {
        let response = json!({
            "items": []
        });

        let paginated = ResponseEnricher::add_pagination(&response, 100, 10, 0);

        assert!(paginated.get("_pagination").is_some());
        assert_eq!(paginated["_pagination"]["total"], 100);
        assert_eq!(paginated["_pagination"]["has_next"], true);
        assert_eq!(paginated["_pagination"]["has_prev"], false);
    }

    #[test]
    fn test_add_links() {
        let response = json!({
            "id": "123"
        });

        let links = vec![
            ("self".to_string(), "/api/item/123".to_string()),
            ("next".to_string(), "/api/item/124".to_string()),
        ];

        let with_links = ResponseEnricher::add_links(&response, &links);

        assert!(with_links.get("_links").is_some());
        assert_eq!(with_links["_links"]["self"], "/api/item/123");
    }

    #[test]
    fn test_enrich_error() {
        let error = ResponseEnricher::enrich_error("NOT_FOUND", "Item not found", None, "req_123");

        assert_eq!(error["error"]["code"], "NOT_FOUND");
        assert_eq!(error["error"]["message"], "Item not found");
        assert_eq!(error["error"]["request_id"], "req_123");
    }

    #[test]
    fn test_extract_request_context() {
        let mut headers = HashMap::new();
        headers.insert("User-Agent".to_string(), "TestClient/1.0".to_string());

        let context = RequestEnricher::extract_context("GET", "/api/health", &headers);

        assert_eq!(context["method"], "GET");
        assert_eq!(context["path"], "/api/health");
        assert_eq!(context["auth_type"], "none");
    }

    #[test]
    fn test_validate_prerequisites_mutation_without_auth() {
        let result = RequestEnricher::validate_prerequisites("POST", "/api/test", false);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_prerequisites_query_without_auth() {
        let result = RequestEnricher::validate_prerequisites("GET", "/api/test", false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_summarize_array() {
        let data = vec![
            json!({"name": "A", "value": 1}),
            json!({"name": "B", "value": 2}),
        ];

        let summary = ResponseSummarizer::summarize_array(&data);

        assert_eq!(summary["count"], 2);
        assert_eq!(summary["fields"]["name"], 2);
    }

    #[test]
    fn test_compute_stats() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let stats = ResponseSummarizer::compute_stats(&values);

        assert_eq!(stats["count"], 5);
        assert_eq!(stats["sum"], 15.0);
        assert_eq!(stats["mean"], 3.0);
        assert_eq!(stats["min"], 1.0);
        assert_eq!(stats["max"], 5.0);
    }
}
