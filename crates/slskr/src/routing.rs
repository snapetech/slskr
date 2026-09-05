//! HTTP routing module: request dispatching and path extraction.

use crate::utils::{
    authorize_controller_route_from, controller_route_requires_principal, csrf_origin_allowed,
    normalize_api_path, split_request_target, RequestSecurityHeaders,
};
use crate::{AppConfig, ControllerProfile};

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

/// The immutable request context shared by route-domain dispatchers.
/// Keeping request fields together prevents a route group from accidentally
/// dropping headers, the raw body, or the authorization value as the bounded
/// dispatcher is split into smaller domains.
#[derive(Clone, Copy)]
pub struct RouteRequest<'a> {
    pub method: &'a str,
    pub path: &'a str,
    pub authorization: Option<&'a str>,
    pub body: &'a str,
    pub headers: &'a RequestSecurityHeaders,
}

impl<'a> RouteRequest<'a> {
    pub fn new(
        method: &'a str,
        path: &'a str,
        authorization: Option<&'a str>,
        body: &'a str,
        headers: &'a RequestSecurityHeaders,
    ) -> Self {
        Self {
            method,
            path,
            authorization,
            body,
            headers,
        }
    }

    pub fn parsed(&self) -> ParsedRoute<'a> {
        parse_route(self.method, self.path)
    }
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
    let starts_with_path = |legacy: &str, versioned: &str| {
        normalized.starts_with(legacy) || normalized.starts_with(versioned)
    };
    let streams_path = starts_with_path("/api/streams/", "/api/v0/streams/");
    let peer_streams_path = starts_with_path("/api/peer-streams/", "/api/v0/peer-streams/");
    let mesh_streams_path = starts_with_path("/api/mesh-streams/", "/api/v0/mesh-streams/");
    let share_grants_path = starts_with_path("/api/share-grants/", "/api/v0/share-grants/");
    let delegated_share_route = (matches!(method, "GET" | "HEAD") && streams_path)
        || (matches!(method, "GET" | "HEAD") && (peer_streams_path || mesh_streams_path))
        || (method == "POST" && streams_path && normalized.ends_with("/share-ticket"))
        || (method == "POST"
            && share_grants_path
            && normalized.ends_with("/backfill")
            && (headers.x_share_token.is_some()
                || auth.is_some_and(|value| {
                    let Some((scheme, value)) = value.trim().split_once(' ') else {
                        return false;
                    };
                    let Some((prefix, token)) = value.trim().split_once(':') else {
                        return false;
                    };
                    scheme.eq_ignore_ascii_case("bearer")
                        && prefix.eq_ignore_ascii_case("share")
                        && !token.trim().is_empty()
                })))
        || (method == "GET" && share_grants_path && normalized.ends_with("/manifest"));
    let public_solid_client_id = method == "GET" && normalized == "/solid/clientid.jsonld";

    if config.controller_profile == ControllerProfile::Native
        && !config.auth_required
        && controller_route_requires_principal(config, method, path)
        && !config.controller_passthrough_allows(headers.remote_addr)
    {
        return Err("unauthorized");
    }

    if !delegated_share_route && !public_solid_client_id {
        authorize_controller_route_from(
            config,
            method,
            path,
            auth,
            headers.cookie.as_deref(),
            headers.remote_addr,
        )?;
    }

    let bearer_or_api_key = auth.is_some_and(crate::utils::has_api_authorization_scheme)
        || headers.x_gateway_api_key.is_some()
        || (delegated_share_route && headers.x_share_token.is_some());
    if !bearer_or_api_key && !csrf_origin_allowed(config, method, normalized, headers) {
        return Err("csrf");
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

pub fn method_not_allowed_response() -> HttpResponse {
    HttpResponse {
        status: "405 Method Not Allowed",
        content_type: "application/json",
        body: "{\"error\":\"method not allowed\"}".to_owned(),
    }
}

pub fn unmatched_route_response() -> HttpResponse {
    HttpResponse {
        status: "404 Not Found",
        content_type: "application/json",
        body: "{\"error\":\"route not found\"}".to_owned(),
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

pub fn no_content_response() -> HttpResponse {
    HttpResponse {
        status: "204 No Content",
        content_type: "application/json",
        body: String::new(),
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

/// Return the ASP.NET string-result shape used by frozen controllers that
/// pass a plain string as the response value.  System.Text.Json serializes
/// that value as a JSON string when the request accepts JSON; wrapping it in
/// an error object changes the public contract.
pub fn service_unavailable_string_response(message: &str) -> HttpResponse {
    HttpResponse {
        status: "503 Service Unavailable",
        content_type: "application/json",
        body: serde_json::to_string(message).unwrap_or_else(|_| "\"\"".to_owned()),
    }
}

pub fn internal_server_error_response(message: &str) -> HttpResponse {
    HttpResponse {
        status: "500 Internal Server Error",
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
