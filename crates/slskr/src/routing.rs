//! HTTP routing module: request dispatching and path extraction.

use crate::utils::{
    csrf_origin_allowed, is_authorized, normalize_api_path, route_requires_auth,
    split_request_target, RequestSecurityHeaders,
};
use crate::AppConfig;

// ============================================================================
// HTTP Response Type
// ============================================================================

#[derive(Debug)]
pub struct HttpResponse {
    pub status: &'static str,
    pub content_type: &'static str,
    pub body: String,
}

// ============================================================================
// Route Matching
// ============================================================================

pub struct ParsedRoute<'a> {
    pub method: &'a str,
    pub path: &'a str,
    pub normalized_path: &'a str,
    pub query: Option<&'a str>,
}

pub fn parse_route<'a>(method: &'a str, path: &'a str) -> ParsedRoute<'a> {
    let (path_only, query) = split_request_target(path);
    let normalized_path = normalize_api_path(path_only);
    ParsedRoute {
        method,
        path: path_only,
        normalized_path,
        query,
    }
}

pub fn check_route_auth(
    config: &AppConfig,
    method: &str,
    path: &str,
    auth: Option<&str>,
    headers: &RequestSecurityHeaders,
) -> Result<(), &'static str> {
    let normalized = normalize_api_path(path);
    let delegated_share_route = (method == "GET" && normalized.starts_with("/api/streams/"))
        || (method == "POST"
            && normalized.starts_with("/api/streams/")
            && normalized.ends_with("/share-ticket"))
        || (method == "GET"
            && normalized.starts_with("/api/share-grants/")
            && normalized.ends_with("/manifest"));

    if route_requires_auth(config, normalized)
        && !delegated_share_route
        && !is_authorized(config, auth, headers.cookie.as_deref())
    {
        return Err("unauthorized");
    }

    if !csrf_origin_allowed(config, method, normalized, headers) {
        return Err("forbidden");
    }

    Ok(())
}

// ============================================================================
// Response Builders
// ============================================================================

pub fn unauthorized_response() -> HttpResponse {
    HttpResponse {
        status: "401 Unauthorized",
        content_type: "application/json",
        body: "{\"error\":\"unauthorized\"}".to_owned(),
    }
}

pub fn forbidden_response(message: &str) -> HttpResponse {
    HttpResponse {
        status: "403 Forbidden",
        content_type: "application/json",
        body: format!("{{\"error\":\"{}\"}}", crate::config::json_escape(message)),
    }
}

pub fn not_found_response() -> HttpResponse {
    HttpResponse {
        status: "404 Not Found",
        content_type: "application/json",
        body: "{\"error\":\"not found\"}".to_owned(),
    }
}

pub fn bad_request_response(message: &str) -> HttpResponse {
    HttpResponse {
        status: "400 Bad Request",
        content_type: "application/json",
        body: format!("{{\"error\":\"{}\"}}", crate::config::json_escape(message)),
    }
}

pub fn created_response(body: String) -> HttpResponse {
    HttpResponse {
        status: "201 Created",
        content_type: "application/json",
        body,
    }
}

pub fn accepted_response(body: String) -> HttpResponse {
    HttpResponse {
        status: "202 Accepted",
        content_type: "application/json",
        body,
    }
}

pub fn ok_response(body: String) -> HttpResponse {
    HttpResponse {
        status: "200 OK",
        content_type: "application/json",
        body,
    }
}

pub fn conflict_response(message: &str) -> HttpResponse {
    HttpResponse {
        status: "409 Conflict",
        content_type: "application/json",
        body: format!("{{\"error\":\"{}\"}}", crate::config::json_escape(message)),
    }
}

pub fn service_unavailable_response(message: &str) -> HttpResponse {
    HttpResponse {
        status: "503 Service Unavailable",
        content_type: "application/json",
        body: format!("{{\"error\":\"{}\"}}", crate::config::json_escape(message)),
    }
}

pub fn not_implemented_response(message: &str) -> HttpResponse {
    HttpResponse {
        status: "501 Not Implemented",
        content_type: "application/json",
        body: format!("{{\"error\":\"{}\"}}", crate::config::json_escape(message)),
    }
}
