//! OpenAPI/Swagger documentation generator for Phase 12
//!
//! Generates OpenAPI 3.0.0 specification for all API endpoints.

use serde_json::{json, Value};
use std::collections::BTreeMap;

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
            description: "slskR - Soulseek Network Client and REST API Server".to_string(),
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
                    "name": "slskR Team",
                    "url": "https://github.com/slskr"
                },
                "license": {
                    "name": "MIT"
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
        paths.insert("/api/health".to_string(), json!({
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
        }));

        // Stats endpoint
        paths.insert("/api/stats".to_string(), json!({
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
        }));

        // Config endpoint
        paths.insert("/api/config".to_string(), json!({
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
        }));

        // Search endpoint
        paths.insert("/api/searches".to_string(), json!({
            "get": {
                "tags": ["Search"],
                "summary": "List active searches",
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
                                    "$ref": "#/components/schemas/SearchList"
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
        }));

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
                        "token": {"type": "integer"},
                        "query": {"type": "string"},
                        "results": {"type": "array"},
                        "status": {"type": "string"}
                    }
                },
                "SearchList": {
                    "type": "object",
                    "properties": {
                        "items": {"type": "array", "items": {"$ref": "#/components/schemas/Search"}},
                        "pagination": {
                            "type": "object",
                            "properties": {
                                "limit": {"type": "integer"},
                                "offset": {"type": "integer"},
                                "total": {"type": "integer"},
                                "pages": {"type": "integer"}
                            }
                        }
                    }
                },
                "SearchRequest": {
                    "type": "object",
                    "required": ["query"],
                    "properties": {
                        "query": {"type": "string", "minLength": 1, "maxLength": 1000},
                        "target": {"type": "string", "enum": ["peers", "all"]}
                    }
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
            })
        ]
    }
}

/// Generate OpenAPI spec as JSON
pub fn generate_openapi_json() -> String {
    let spec = OpenApiSpec::new("slskR API", "1.0.1");
    serde_json::to_string_pretty(&spec.generate()).unwrap_or_else(|_| "{}".to_string())
}

/// Generate Swagger UI HTML
pub fn swagger_ui_html(spec_url: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>slskR API Documentation</title>
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
        assert!(json.contains("slskR API"));
    }
}
