//! OpenAPI/Swagger documentation generator for Phase 12
//!
//! Generates OpenAPI 3.0.0 specification for all API endpoints.

use serde_json::{json, Value};
use std::collections::BTreeMap;

const CHECKED_IN_OPENAPI_JSON: &str = include_str!("openapi.json");

/// OpenAPI spec generator
pub struct OpenApiSpec {
    version: String,
    title: String,
    description: String,
}

impl OpenApiSpec {
    /// Create a new OpenAPI spec
    pub fn new(title: &str, version: &str) -> Self {
        Self {
            title: title.to_string(),
            version: version.to_string(),
            description: "slskr - independent Soulseek network client and REST API server"
                .to_string(),
        }
    }

    /// Generate complete OpenAPI specification
    pub fn generate(&self) -> Value {
        json!({
            "openapi": "3.0.0",
            "info": {
                "title": self.title,
                "version": self.version,
                "description": self.description,
                "contact": {
                    "name": "slskr contributors",
                    "url": "https://github.com/snapetech/slskr"
                },
                "license": {
                    "name": "AGPL-3.0-only",
                    "url": "https://www.gnu.org/licenses/agpl-3.0.html"
                }
            },
            "servers": [
                {
                    "url": "http://localhost:5030",
                    "description": "Development server"
                },
                {
                    "url": "http://localhost:5030/api/v1",
                    "description": "API v1 (current stable)"
                },
                {
                    "url": "http://localhost:5030/api/v2",
                    "description": "API v2 (future features)"
                }
            ],
            "paths": self.generate_paths(),
            "components": self.generate_components(),
            "tags": self.generate_tags()
        })
    }

    fn generate_paths(&self) -> BTreeMap<String, Value> {
        let mut paths = BTreeMap::new();

        // Health endpoint
        paths.insert(
            "/api/health".to_string(),
            json!({
                "get": {
                    "tags": ["Health & Info"],
                    "summary": "Check server health",
                    "operationId": "getHealth",
                    "responses": {
                        "200": {
                            "description": "Server is healthy",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/Health"
                                    }
                                }
                            }
                        }
                    }
                }
            }),
        );

        // Stats endpoint
        paths.insert(
            "/api/stats".to_string(),
            json!({
                "get": {
                    "tags": ["Session"],
                    "summary": "Get server statistics",
                    "operationId": "getStats",
                    "security": [{"bearerAuth": []}],
                    "responses": {
                        "200": {
                            "description": "Server statistics",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/Stats"
                                    }
                                }
                            }
                        },
                        "401": {"description": "Unauthorized"}
                    }
                }
            }),
        );

        // Config endpoint
        paths.insert(
            "/api/config".to_string(),
            json!({
                "get": {
                    "tags": ["Session"],
                    "summary": "Get server configuration",
                    "operationId": "getConfig",
                    "security": [{"bearerAuth": []}],
                    "responses": {
                        "200": {
                            "description": "Server configuration",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/Config"
                                    }
                                }
                            }
                        },
                        "401": {"description": "Unauthorized"}
                    }
                }
            }),
        );

        // Search endpoint
        paths.insert(
            "/api/searches".to_string(),
            json!({
                "get": {
                    "tags": ["Search"],
                    "summary": "List active searches as a slskd-compatible array",
                    "operationId": "listSearches",
                    "parameters": [
                        {
                            "name": "limit",
                            "in": "query",
                            "description": "Items per page (1-100)",
                            "schema": {"type": "integer", "default": 20}
                        },
                        {
                            "name": "offset",
                            "in": "query",
                            "description": "Page offset",
                            "schema": {"type": "integer", "default": 0}
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "List of searches",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "array",
                                        "items": {"$ref": "#/components/schemas/Search"}
                                    }
                                }
                            }
                        }
                    }
                },
                "post": {
                    "tags": ["Search"],
                    "summary": "Start new search",
                    "operationId": "startSearch",
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/SearchRequest"
                                }
                            }
                        }
                    },
                    "responses": {
                        "201": {
                            "description": "Search started",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/Search"
                                    }
                                }
                            }
                        },
                        "400": {"description": "Bad request"}
                    }
                }
            }),
        );

        paths.insert(
            "/api/searches/records".to_string(),
            json!({
                "get": {
                    "tags": ["Search"],
                    "summary": "List active searches with slskr metadata envelope",
                    "operationId": "listSearchRecords",
                    "security": [{"bearerAuth": []}],
                    "responses": {
                        "200": {
                            "description": "Search list envelope",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/SearchRecordList"
                                    }
                                }
                            }
                        },
                        "401": {"description": "Unauthorized"}
                    }
                }
            }),
        );

        paths.insert(
            "/api/events".to_string(),
            json!({
                "get": {
                    "tags": ["Events"],
                    "summary": "List events as a slskd-compatible array",
                    "operationId": "listEvents",
                    "security": [{"bearerAuth": []}],
                    "responses": {
                        "200": {
                            "description": "Event array",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "array",
                                        "items": {"$ref": "#/components/schemas/Event"}
                                    }
                                }
                            }
                        },
                        "401": {"description": "Unauthorized"}
                    }
                }
            }),
        );

        paths.insert(
            "/api/events/records".to_string(),
            json!({
                "get": {
                    "tags": ["Events"],
                    "summary": "List events with slskr metadata envelope",
                    "operationId": "listEventRecords",
                    "security": [{"bearerAuth": []}],
                    "responses": {
                        "200": {
                            "description": "Event list envelope",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/EventRecordList"
                                    }
                                }
                            }
                        },
                        "401": {"description": "Unauthorized"}
                    }
                }
            }),
        );

        paths
    }

    fn generate_components(&self) -> Value {
        json!({
            "schemas": {
                "Health": {
                    "type": "object",
                    "properties": {
                        "status": {"type": "string", "example": "ok"},
                        "service": {"type": "string", "example": "slskr"},
                        "timestamp": {"type": "string", "format": "date-time"}
                    }
                },
                "Stats": {
                    "type": "object",
                    "properties": {
                        "session": {"type": "object"},
                        "searches": {"type": "object"},
                        "transfers": {"type": "object"},
                        "users": {"type": "object"}
                    }
                },
                "Config": {
                    "type": "object",
                    "properties": {
                        "http_bind": {"type": "string"},
                        "server_address": {"type": "string"},
                        "listen_port": {"type": "integer"},
                        "share_roots": {"type": "array", "items": {"type": "string"}}
                    }
                },
                "Search": {
                    "type": "object",
                    "properties": {
                        "id": {"type": "string"},
                        "token": {"type": "integer"},
                        "query": {"type": "string"},
                        "searchText": {"type": "string"},
                        "state": {"type": "string"},
                        "isComplete": {"type": "boolean"},
                        "fileCount": {"type": "integer"},
                        "lockedFileCount": {"type": "integer"},
                        "responseCount": {"type": "integer"},
                        "responses": {"type": "array"},
                        "results": {"type": "array"},
                        "status": {"type": "string"},
                        "startedAt": {"type": "string"},
                        "endedAt": {"type": "string", "nullable": true}
                    },
                    "required": ["id", "token", "query", "searchText", "state", "isComplete", "fileCount", "lockedFileCount", "responseCount", "responses", "startedAt"]
                },
                "SearchList": {
                    "type": "array",
                    "items": {"$ref": "#/components/schemas/Search"}
                },
                "SearchRecordList": {
                    "type": "object",
                    "properties": {
                        "entries": {"type": "array", "items": {"$ref": "#/components/schemas/Search"}},
                        "count": {"type": "integer"},
                        "filtered_count": {"type": "integer"},
                        "offset": {"type": "integer"},
                        "limit": {"type": "integer", "nullable": true},
                        "next_token": {"type": "integer"}
                    },
                    "required": ["entries", "count", "filtered_count", "offset", "next_token"]
                },
                "SearchRequest": {
                    "type": "object",
                    "description": "Provide either query or slskd-compatible searchText.",
                    "properties": {
                        "query": {"type": "string", "minLength": 1, "maxLength": 1000},
                        "searchText": {"type": "string", "minLength": 1, "maxLength": 1000},
                        "target": {"type": "string", "enum": ["peers", "all"]}
                    }
                },
                "Event": {
                    "type": "object",
                    "properties": {
                        "id": {"type": "integer"},
                        "type": {"type": "string"},
                        "kind": {"type": "string"},
                        "resource": {"type": "string"},
                        "detail": {"type": "string", "nullable": true},
                        "createdAt": {"type": "integer"}
                    },
                    "required": ["id", "type", "kind", "resource", "createdAt"]
                },
                "EventRecordList": {
                    "type": "object",
                    "properties": {
                        "entries": {"type": "array", "items": {"$ref": "#/components/schemas/Event"}},
                        "count": {"type": "integer"},
                        "filtered_count": {"type": "integer"},
                        "offset": {"type": "integer"},
                        "limit": {"type": "integer", "nullable": true}
                    },
                    "required": ["entries", "count", "filtered_count", "offset"]
                }
            },
            "securitySchemes": {
                "bearerAuth": {
                    "type": "http",
                    "scheme": "bearer",
                    "bearerFormat": "token"
                }
            }
        })
    }

    fn generate_tags(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "Health & Info",
                "description": "Server health and metadata"
            }),
            json!({
                "name": "Session",
                "description": "Session management and statistics"
            }),
            json!({
                "name": "Search",
                "description": "Search operations"
            }),
            json!({
                "name": "Events",
                "description": "Event feed and event history"
            }),
            json!({
                "name": "Transfers",
                "description": "File transfer management"
            }),
            json!({
                "name": "Users",
                "description": "User management"
            }),
            json!({
                "name": "Webhooks",
                "description": "Webhook management"
            }),
        ]
    }
}

/// Generate OpenAPI spec as JSON
pub fn generate_openapi_json() -> String {
    CHECKED_IN_OPENAPI_JSON.to_owned()
}

/// Generate Swagger UI HTML
pub fn swagger_ui_html(spec_url: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>slskr API Documentation</title>
    <meta charset="utf-8"/>
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/swagger-ui-dist@3/swagger-ui.css">
</head>
<body>
    <div id="swagger-ui"></div>
    <script src="https://cdn.jsdelivr.net/npm/swagger-ui-dist@3/swagger-ui-bundle.js"></script>
    <script>
        const ui = SwaggerUIBundle({{
            url: "{}",
            dom_id: '#swagger-ui',
            presets: [
                SwaggerUIBundle.presets.apis,
                SwaggerUIBundle.SwaggerUIStandalonePreset
            ],
            layout: "BaseLayout"
        }})
    </script>
</body>
</html>"#,
        spec_url
    )
}

/// Generate the frozen slskd/slskdN Swagger UI shell. The upstream targets
/// publish the same static Swashbuckle index and vary only the generated spec.
pub fn frozen_swagger_ui_html() -> String {
    r#"<!-- HTML for static distribution bundle build -->
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Swagger UI</title>
    <link rel="stylesheet" type="text/css" href="./swagger-ui.css">
    <link rel="stylesheet" type="text/css" href="./index.css">
    <link rel="icon" type="image/png" href="./favicon-32x32.png" sizes="32x32" />
    <link rel="icon" type="image/png" href="./favicon-16x16.png" sizes="16x16" />
</head>
<body>
    <div id="swagger-ui"></div>
    <script src="./swagger-ui-bundle.js" charset="utf-8"></script>
    <script src="./swagger-ui-standalone-preset.js" charset="utf-8"></script>
    <script src="index.js" charset="utf-8"></script>
</body>
</html>"#
        .to_owned()
}

/// Small local bootstrap that preserves the upstream asset paths while using
/// the already-supported Swagger UI distribution when the browser loads them.
pub fn frozen_swagger_index_js(spec_url: &str) -> String {
    format!(
        "window.onload=function(){{window.ui=SwaggerUIBundle({{url:{spec_url:?},dom_id:'#swagger-ui',deepLinking:true,presets:[SwaggerUIBundle.presets.apis,SwaggerUIStandalonePreset],layout:'StandaloneLayout'}});}};"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openapi_generation() {
        let spec = OpenApiSpec::new("Test API", "1.0.0");
        let generated = spec.generate();

        assert_eq!(generated["openapi"], "3.0.0");
        assert_eq!(generated["info"]["title"], "Test API");
        assert_eq!(generated["info"]["version"], "1.0.0");
        assert_eq!(generated["info"]["license"]["name"], "AGPL-3.0-only");
    }

    #[test]
    fn test_swagger_ui_html() {
        let html = swagger_ui_html("/api/openapi.json");
        assert!(html.contains("swagger-ui"));
        assert!(html.contains("/api/openapi.json"));
    }

    #[test]
    fn test_openapi_json_generation() {
        let json = generate_openapi_json();
        assert!(json.contains("openapi"));
        assert!(json.contains("slskr HTTP API"));
        assert!(json.contains("AGPL-3.0-only"));
    }

    #[test]
    fn test_runtime_openapi_matches_checked_in_spec() {
        let runtime: Value = serde_json::from_str(&generate_openapi_json()).expect("runtime spec");
        let checked_in: Value =
            serde_json::from_str(CHECKED_IN_OPENAPI_JSON).expect("checked-in spec");
        assert_eq!(runtime, checked_in);
    }

    #[test]
    fn test_openapi_documents_slskd_compatible_arrays() {
        let spec = OpenApiSpec::new("Test API", "1.0.0").generate();
        assert_eq!(
            spec["paths"]["/api/searches"]["get"]["responses"]["200"]["content"]
                ["application/json"]["schema"]["type"],
            "array"
        );
        assert_eq!(
            spec["paths"]["/api/events"]["get"]["responses"]["200"]["content"]["application/json"]
                ["schema"]["type"],
            "array"
        );
        assert_eq!(
            spec["paths"]["/api/searches/records"]["get"]["responses"]["200"]["content"]
                ["application/json"]["schema"]["$ref"],
            "#/components/schemas/SearchRecordList"
        );
        assert_eq!(
            spec["paths"]["/api/events/records"]["get"]["responses"]["200"]["content"]
                ["application/json"]["schema"]["$ref"],
            "#/components/schemas/EventRecordList"
        );
        assert!(spec["components"]["schemas"]["Search"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value == "responses"));
        assert!(
            spec["components"]["schemas"]["SearchRequest"]
                .get("required")
                .is_none(),
            "searchText-only slskd requests must be valid in the schema"
        );
    }

    #[test]
    fn test_checked_in_openapi_documents_slskd_compatible_arrays() {
        let spec: Value = serde_json::from_str(CHECKED_IN_OPENAPI_JSON).expect("openapi json");
        assert_eq!(
            spec["paths"]["/api/searches"]["get"]["responses"]["200"]["content"]
                ["application/json"]["schema"]["type"],
            "array"
        );
        assert_eq!(
            spec["paths"]["/api/events"]["get"]["responses"]["200"]["content"]["application/json"]
                ["schema"]["type"],
            "array"
        );
        assert_eq!(
            spec["paths"]["/api/searches/records"]["get"]["responses"]["200"]["content"]
                ["application/json"]["schema"]["$ref"],
            "#/components/schemas/SearchRecordList"
        );
        assert_eq!(
            spec["paths"]["/api/events/records"]["get"]["responses"]["200"]["content"]
                ["application/json"]["schema"]["$ref"],
            "#/components/schemas/EventRecordList"
        );
        assert!(spec["components"]["schemas"]["Search"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value == "responses"));
        assert!(
            spec["components"]["schemas"]["SearchCreateRequest"]
                .get("required")
                .is_none(),
            "searchText-only slskd requests must be valid in the checked-in schema"
        );
    }
}
