/// API versioning and compatibility management
///
/// Handles version detection, backward compatibility, deprecation warnings,
/// and version-specific response formatting.

use serde_json::{json, Value};
use std::collections::HashMap;

/// API version information
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ApiVersion {
    V0,
    V1,
    V2,
}

impl ApiVersion {
    /// Parse version from path
    pub fn from_path(path: &str) -> Self {
        if path.contains("/v0/") || path.contains("/v0.") {
            ApiVersion::V0
        } else if path.contains("/v2/") || path.contains("/v2.") {
            ApiVersion::V2
        } else {
            ApiVersion::V1 // Default
        }
    }

    /// Get current version
    pub fn current() -> Self {
        ApiVersion::V2
    }

    /// Check if version is deprecated
    pub fn is_deprecated(&self) -> bool {
        matches!(self, ApiVersion::V0)
    }

    /// Get deprecation message
    pub fn deprecation_message(&self) -> Option<String> {
        match self {
            ApiVersion::V0 => Some(
                "API v0 is deprecated and will be removed on 2026-12-01. Please upgrade to v2."
                    .to_string(),
            ),
            _ => None,
        }
    }

    /// Get version string
    pub fn as_string(&self) -> &str {
        match self {
            ApiVersion::V0 => "0.0.0",
            ApiVersion::V1 => "1.0.1",
            ApiVersion::V2 => "2.0.0",
        }
    }
}

/// Version compatibility checker
pub struct VersionCompatibility;

impl VersionCompatibility {
    /// Check if endpoint is available in version
    pub fn is_endpoint_available(version: ApiVersion, endpoint: &str) -> bool {
        match version {
            ApiVersion::V0 => {
                // V0 has limited endpoints
                matches!(
                    endpoint,
                    "/api/health"
                        | "/api/version"
                        | "/api/stats"
                        | "/api/transfers"
                        | "/api/searches"
                )
            }
            ApiVersion::V1 => {
                // V1 adds batch and graphql
                !endpoint.contains("/events/stream") && !endpoint.contains("/v2/")
            }
            ApiVersion::V2 => true, // V2 supports everything
        }
    }

    /// Transform response for version compatibility
    pub fn transform_for_version(response: &Value, version: ApiVersion) -> Value {
        match version {
            ApiVersion::V0 => {
                // V0 responses are more minimal
                Self::strip_metadata(response)
            }
            ApiVersion::V1 => {
                // V1 includes some metadata
                response.clone()
            }
            ApiVersion::V2 => {
                // V2 includes all metadata
                response.clone()
            }
        }
    }

    /// Strip metadata for older versions
    fn strip_metadata(response: &Value) -> Value {
        match response {
            Value::Object(map) => {
                let mut filtered = serde_json::Map::new();
                for (key, val) in map {
                    if !key.starts_with('_') {
                        filtered.insert(key.clone(), val.clone());
                    }
                }
                Value::Object(filtered)
            }
            _ => response.clone(),
        }
    }

    /// Add version header
    pub fn add_version_headers(version: ApiVersion) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        headers.insert("API-Version".to_string(), version.as_string().to_string());
        headers.insert("X-API-Version".to_string(), version.as_string().to_string());

        if let Some(msg) = version.deprecation_message() {
            headers.insert("Deprecation".to_string(), "true".to_string());
            headers.insert("Sunset".to_string(), "2026-12-01T00:00:00Z".to_string());
            headers.insert("Warning".to_string(), format!("299 - \"{}\"", msg));
        }

        headers
    }
}

/// Response format for different API versions
pub struct VersionedResponse;

impl VersionedResponse {
    /// Create version-specific response
    pub fn create(version: ApiVersion, data: &Value, meta: Option<&Value>) -> Value {
        match version {
            ApiVersion::V0 => {
                // V0: Simple flat structure
                data.clone()
            }
            ApiVersion::V1 => {
                // V1: Data + metadata wrapper
                json!({
                    "data": data,
                    "meta": meta
                })
            }
            ApiVersion::V2 => {
                // V2: Rich structure with metadata
                json!({
                    "status": "success",
                    "data": data,
                    "meta": meta
                })
            }
        }
    }

    /// Create error response for version
    pub fn error(
        version: ApiVersion,
        code: &str,
        message: &str,
        status: u16,
    ) -> Value {
        match version {
            ApiVersion::V0 => {
                // V0: Simple error
                json!({
                    "error": message
                })
            }
            ApiVersion::V1 => {
                // V1: Error with code
                json!({
                    "error": {
                        "code": code,
                        "message": message
                    }
                })
            }
            ApiVersion::V2 => {
                // V2: Full error response
                json!({
                    "status": "error",
                    "error": {
                        "code": code,
                        "message": message,
                        "http_status": status
                    }
                })
            }
        }
    }
}

/// Migration helper for upgrading versions
pub struct VersionMigration;

impl VersionMigration {
    /// Migrate response from V0 to V1
    pub fn v0_to_v1(response: &Value) -> Value {
        match response {
            Value::Object(_) => {
                // Wrap in data envelope
                json!({
                    "data": response,
                    "meta": {
                        "version": "1.0.0"
                    }
                })
            }
            _ => response.clone(),
        }
    }

    /// Migrate response from V1 to V2
    pub fn v1_to_v2(response: &Value) -> Value {
        if let Value::Object(map) = response {
            if let Some(data) = map.get("data") {
                return json!({
                    "status": "success",
                    "data": data,
                    "meta": map.get("meta")
                });
            }
        }
        response.clone()
    }

    /// Migrate request query parameters
    pub fn migrate_query_params(
        params: &mut HashMap<String, String>,
        from_version: ApiVersion,
        to_version: ApiVersion,
    ) {
        if from_version == ApiVersion::V0 && to_version == ApiVersion::V2 {
            // In V0, pagination was 'start' and 'count'
            // In V2, it's 'offset' and 'limit'
            if let Some(start) = params.remove("start") {
                params.insert("offset".to_string(), start);
            }
            if let Some(count) = params.remove("count") {
                params.insert("limit".to_string(), count);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_version_from_path() {
        assert_eq!(ApiVersion::from_path("/api/v0/health"), ApiVersion::V0);
        assert_eq!(ApiVersion::from_path("/api/v1/health"), ApiVersion::V1);
        assert_eq!(ApiVersion::from_path("/api/v2/health"), ApiVersion::V2);
        assert_eq!(ApiVersion::from_path("/api/health"), ApiVersion::V1); // default
    }

    #[test]
    fn test_api_version_deprecation() {
        assert!(ApiVersion::V0.is_deprecated());
        assert!(!ApiVersion::V1.is_deprecated());
        assert!(!ApiVersion::V2.is_deprecated());
    }

    #[test]
    fn test_version_deprecation_message() {
        let msg = ApiVersion::V0.deprecation_message();
        assert!(msg.is_some());
        assert!(msg.unwrap().contains("deprecated"));
    }

    #[test]
    fn test_endpoint_availability_v0() {
        assert!(VersionCompatibility::is_endpoint_available(
            ApiVersion::V0,
            "/api/health"
        ));
        assert!(!VersionCompatibility::is_endpoint_available(
            ApiVersion::V0,
            "/api/batch"
        ));
    }

    #[test]
    fn test_endpoint_availability_v2() {
        assert!(VersionCompatibility::is_endpoint_available(
            ApiVersion::V2,
            "/api/health"
        ));
        assert!(VersionCompatibility::is_endpoint_available(
            ApiVersion::V2,
            "/api/batch"
        ));
    }

    #[test]
    fn test_transform_for_version() {
        let response = json!({
            "data": "test",
            "_metadata": "hidden"
        });

        let v0_response = VersionCompatibility::transform_for_version(&response, ApiVersion::V0);

        assert!(v0_response.get("data").is_some());
        assert!(v0_response.get("_metadata").is_none());
    }

    #[test]
    fn test_version_headers() {
        let headers = VersionCompatibility::add_version_headers(ApiVersion::V2);

        assert_eq!(headers.get("API-Version"), Some(&"2.0.0".to_string()));
        assert_eq!(headers.get("X-API-Version"), Some(&"2.0.0".to_string()));
    }

    #[test]
    fn test_deprecated_version_headers() {
        let headers = VersionCompatibility::add_version_headers(ApiVersion::V0);

        assert_eq!(headers.get("Deprecation"), Some(&"true".to_string()));
        assert!(headers.contains_key("Sunset"));
    }

    #[test]
    fn test_versioned_response_v0() {
        let data = json!({"key": "value"});
        let response = VersionedResponse::create(ApiVersion::V0, &data, None);

        assert_eq!(response, data);
    }

    #[test]
    fn test_versioned_response_v2() {
        let data = json!({"key": "value"});
        let response = VersionedResponse::create(ApiVersion::V2, &data, None);

        assert_eq!(response["status"], "success");
        assert_eq!(response["data"]["key"], "value");
    }

    #[test]
    fn test_versioned_error_response() {
        let error = VersionedResponse::error(ApiVersion::V2, "NOT_FOUND", "Item not found", 404);

        assert_eq!(error["status"], "error");
        assert_eq!(error["error"]["code"], "NOT_FOUND");
        assert_eq!(error["error"]["http_status"], 404);
    }

    #[test]
    fn test_v0_to_v1_migration() {
        let v0_response = json!({
            "items": [],
            "count": 0
        });

        let v1_response = VersionMigration::v0_to_v1(&v0_response);

        assert!(v1_response.get("data").is_some());
        assert!(v1_response.get("meta").is_some());
    }

    #[test]
    fn test_migrate_query_params() {
        let mut params = HashMap::new();
        params.insert("start".to_string(), "10".to_string());
        params.insert("count".to_string(), "20".to_string());

        VersionMigration::migrate_query_params(&mut params, ApiVersion::V0, ApiVersion::V2);

        assert_eq!(params.get("offset"), Some(&"10".to_string()));
        assert_eq!(params.get("limit"), Some(&"20".to_string()));
        assert!(params.get("start").is_none());
    }
}
