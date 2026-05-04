//! API documentation endpoints for Phase 12
//!
//! Provides OpenAPI spec, Swagger UI, and API documentation endpoints.

use crate::openapi::{generate_openapi_json, swagger_ui_html};

/// Route documentation requests
pub fn handle_docs_request(method: &str, path: &str) -> Option<(String, String)> {
    match (method, path) {
        // OpenAPI JSON spec
        ("GET", "/api/openapi.json") | ("GET", "/api/v1/openapi.json") | ("GET", "/api/v2/openapi.json") => {
            Some((
                "application/json".to_string(),
                generate_openapi_json()
            ))
        },
        
        // Swagger UI
        ("GET", "/api/docs") | ("GET", "/api/v1/docs") | ("GET", "/api/v2/docs") => {
            Some((
                "text/html".to_string(),
                swagger_ui_html("/api/openapi.json")
            ))
        },
        
        // API documentation index
        ("GET", "/api/docs/index") => {
            Some((
                "application/json".to_string(),
                serde_json::json!({
                    "title": "slskR API Documentation",
                    "version": "1.0.1",
                    "docs": {
                        "swagger_ui": "/api/docs",
                        "openapi_spec": "/api/openapi.json",
                        "guides": {
                            "rate_limiting": "/docs/RATE_LIMITING.md",
                            "api_versioning": "/docs/API_VERSIONING.md",
                            "webhooks": "/docs/WEBHOOK_API.md"
                        }
                    },
                    "endpoints": {
                        "total": 202,
                        "by_method": {
                            "GET": 81,
                            "POST": 67,
                            "PUT": 6,
                            "DELETE": 15,
                            "PATCH": 1,
                            "OPTIONS": 32
                        }
                    }
                }).to_string()
            ))
        },
        
        // Endpoint statistics
        ("GET", "/api/docs/stats") => {
            Some((
                "application/json".to_string(),
                serde_json::json!({
                    "total_endpoints": 202,
                    "api_versions": ["v0", "v1", "v2"],
                    "categories": {
                        "health": 7,
                        "session": 5,
                        "search": 15,
                        "transfers": 18,
                        "users": 12,
                        "messages": 8,
                        "rooms": 15,
                        "shares": 8,
                        "webhooks": 6,
                        "collections": 22,
                        "wishlist": 18,
                        "contacts": 20,
                        "share_groups": 15,
                        "user_notes": 12,
                        "interests": 12
                    },
                    "features": {
                        "rate_limiting": {
                            "anonymous": "1000 req/min",
                            "authenticated": "5000 req/min"
                        },
                        "caching": "Cache-Control + ETag",
                        "compression": "gzip",
                        "cors": "Configurable",
                        "webhooks": "HMAC-SHA256"
                    }
                }).to_string()
            ))
        },
        
        _ => None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_docs_openapi_json() {
        let result = handle_docs_request("GET", "/api/openapi.json");
        assert!(result.is_some());
        let (content_type, body) = result.unwrap();
        assert_eq!(content_type, "application/json");
        assert!(body.contains("openapi"));
    }

    #[test]
    fn test_docs_swagger_ui() {
        let result = handle_docs_request("GET", "/api/docs");
        assert!(result.is_some());
        let (content_type, body) = result.unwrap();
        assert_eq!(content_type, "text/html");
        assert!(body.contains("swagger-ui"));
    }

    #[test]
    fn test_docs_index() {
        let result = handle_docs_request("GET", "/api/docs/index");
        assert!(result.is_some());
        let (content_type, body) = result.unwrap();
        assert_eq!(content_type, "application/json");
        assert!(body.contains("slskR API Documentation"));
    }

    #[test]
    fn test_docs_stats() {
        let result = handle_docs_request("GET", "/api/docs/stats");
        assert!(result.is_some());
        let (content_type, body) = result.unwrap();
        assert_eq!(content_type, "application/json");
        assert!(body.contains("202"));  // total endpoints
    }
}
