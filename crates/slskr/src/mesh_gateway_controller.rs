use super::*;

pub(super) fn mesh_gateway_disabled_response() -> HttpResponse {
    HttpResponse {
        status: "404 Not Found",
        content_type: "application/json",
        body: r#"{"error":"gateway_disabled"}"#.to_owned(),
    }
}

fn mesh_gateway_error_response(
    status: &'static str,
    error: &str,
    message: Option<&str>,
) -> HttpResponse {
    let mut response = serde_json::Map::new();
    response.insert("error".to_owned(), serde_json::json!(error));
    if let Some(message) = message {
        response.insert("message".to_owned(), serde_json::json!(message));
    }
    HttpResponse {
        status,
        content_type: "application/json",
        body: serde_json::Value::Object(response).to_string(),
    }
}

pub(super) fn mesh_gateway_auth_failure(
    state: &AppState,
    headers: &RequestSecurityHeaders,
) -> Option<HttpResponse> {
    let settings = &state.config.mesh_gateway;
    let is_local_request = headers.remote_addr.is_some_and(|remote| {
        remote.ip().is_loopback() || remote.ip() == state.config.http_bind.ip()
    });

    if !is_local_request {
        let Some(configured_api_key) = settings
            .api_key
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        else {
            return Some(mesh_gateway_error_response(
                "401 Unauthorized",
                "unauthorized",
                Some("X-Slskdn-ApiKey must be configured for non-localhost gateway access"),
            ));
        };
        if !headers
            .x_gateway_api_key
            .as_deref()
            .is_some_and(|provided| {
                constant_time_bytes_equal(provided.as_bytes(), configured_api_key.as_bytes())
            })
        {
            return Some(mesh_gateway_error_response(
                "401 Unauthorized",
                "unauthorized",
                Some("Valid X-Slskdn-ApiKey header is required"),
            ));
        }
    }

    if is_local_request {
        if let Some(configured_csrf) = settings
            .csrf_token
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            if !headers.x_gateway_csrf.as_deref().is_some_and(|provided| {
                constant_time_bytes_equal(provided.as_bytes(), configured_csrf.as_bytes())
            }) {
                return Some(mesh_gateway_error_response(
                    "403 Forbidden",
                    "csrf_required",
                    Some("Valid X-Slskdn-Csrf header is required for localhost access"),
                ));
            }
        }
    }

    if let Some(origin) = headers.origin.as_deref() {
        if !settings.allowed_origins.is_empty() {
            if !settings
                .allowed_origins
                .iter()
                .any(|allowed| allowed == origin)
            {
                return Some(mesh_gateway_error_response(
                    "403 Forbidden",
                    "origin_not_allowed",
                    Some("Origin not in allowed list"),
                ));
            }
        } else if is_local_request && !mesh_gateway_localhost_origin(origin) {
            return Some(mesh_gateway_error_response(
                "403 Forbidden",
                "origin_not_allowed",
                Some("Cross-origin requests to localhost are not allowed by default"),
            ));
        }
    }

    None
}

fn mesh_gateway_localhost_origin(origin: &str) -> bool {
    utils::origin_host(origin)
        .and_then(utils::parse_authority)
        .is_some_and(|(host, _)| matches!(host.as_str(), "localhost" | "127.0.0.1" | "::1"))
}

pub(super) async fn mesh_http_services_response(state: &AppState) -> HttpResponse {
    if !state.config.mesh_gateway.enabled {
        return mesh_gateway_disabled_response();
    }
    let provider_count = usize::from(state.private_gateway.is_some());
    let services = state
        .config
        .mesh_gateway
        .allowed_services
        .iter()
        .map(|service_name| {
            serde_json::json!({
                "serviceName": service_name,
                "providerCount": provider_count,
                "available": provider_count > 0,
            })
        })
        .collect::<Vec<_>>();
    routing::ok_response(
        serde_json::json!({
            "gateway": {
                "enabled": true,
                "bindAddress": state.config.mesh_gateway.bind_address,
                "requestTimeoutSeconds": state.config.mesh_gateway.request_timeout_seconds,
                "maxRequestBodyBytes": state.config.mesh_gateway.max_request_body_bytes,
            },
            "services": services,
        })
        .to_string(),
    )
}

pub(super) async fn mesh_http_service_response(
    path: &str,
    body: &str,
    state: &AppState,
) -> HttpResponse {
    let Some(segments) = decoded_segments_after(path, "/mesh/http/") else {
        return routing::not_found_response();
    };
    let [service, operation] = segments.as_slice() else {
        return routing::not_found_response();
    };
    let service = service.trim();
    let operation = operation.trim();
    if service.is_empty() || operation.is_empty() {
        return mesh_gateway_error_response(
            "400 Bad Request",
            "invalid_request",
            Some("Service name and method are required"),
        );
    }
    if !state
        .config
        .mesh_gateway
        .allowed_services
        .iter()
        .any(|allowed| allowed == service)
    {
        return mesh_gateway_error_response(
            "403 Forbidden",
            "service_not_allowed",
            Some("Requested service is not allowed"),
        );
    }
    if body.len() > state.config.mesh_gateway.max_request_body_bytes {
        return mesh_gateway_error_response(
            "413 Payload Too Large",
            "payload_too_large",
            Some("Request body exceeds the configured size limit"),
        );
    }
    let Some(gateway) = state.private_gateway.as_ref() else {
        return mesh_gateway_error_response(
            "503 Service Unavailable",
            "service_unavailable",
            Some("No providers found for the requested service"),
        );
    };
    let call = match slskr_client::overlay::MeshServiceCall::new(
        uuid::Uuid::new_v4().to_string(),
        service,
        operation,
        body.as_bytes().to_vec(),
    ) {
        Ok(call) => call,
        Err(_) => {
            return mesh_gateway_error_response(
                "400 Bad Request",
                "invalid_request",
                Some("Service name, method, or payload is invalid"),
            )
        }
    };
    let username = state
        .config
        .username
        .as_deref()
        .filter(|username| !username.trim().is_empty())
        .unwrap_or("slskr");
    let reply = match time::timeout(
        Duration::from_secs(state.config.mesh_gateway.request_timeout_seconds),
        gateway.call_http_service(call, username, "http-gateway", state),
    )
    .await
    {
        Ok(reply) => reply,
        Err(_) => {
            return mesh_gateway_error_response(
                "504 Gateway Timeout",
                "gateway_timeout",
                Some("Service call timed out"),
            )
        }
    };
    if reply.status_code != 0 {
        let status = match reply.status_code {
            2 | 3 => "404 Not Found",
            4 => "400 Bad Request",
            8 => "403 Forbidden",
            9 => "413 Payload Too Large",
            10 => "500 Internal Server Error",
            _ => "502 Bad Gateway",
        };
        let mut response = serde_json::Map::new();
        response.insert("error".to_owned(), serde_json::json!("service_error"));
        response.insert(
            "statusCode".to_owned(),
            serde_json::json!(reply.status_code),
        );
        response.insert(
            "message".to_owned(),
            serde_json::json!("Service returned an error"),
        );
        return HttpResponse {
            status,
            content_type: "application/json",
            body: serde_json::Value::Object(response).to_string(),
        };
    }
    if reply.payload.is_empty() {
        return routing::ok_response(r#"{"success":true}"#.to_owned());
    }
    if let Ok(payload) = serde_json::from_slice::<serde_json::Value>(&reply.payload) {
        return routing::ok_response(payload.to_string());
    }
    routing::ok_response(
        serde_json::json!({
            "payload": STANDARD.encode(reply.payload),
            "encoding": "base64",
        })
        .to_string(),
    )
}
