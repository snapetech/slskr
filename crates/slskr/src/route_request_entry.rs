use super::*;

#[cfg(not(feature = "legacy-route-dispatch"))]
#[allow(
    dead_code,
    reason = "used by focused and bounded in-process route tests"
)]
pub(super) async fn route_http_request_with_headers(
    method: &str,
    path: &str,
    authorization: Option<&str>,
    body: &str,
    state: &AppState,
    headers: RequestSecurityHeaders,
) -> Result<HttpResponse, String> {
    route_dispatch::route_http_request_with_headers(
        method,
        path,
        authorization,
        body,
        state,
        &headers,
    )
    .await
}

#[cfg(feature = "legacy-route-dispatch")]
pub(super) async fn route_http_request_with_headers(
    method: &str,
    path: &str,
    authorization: Option<&str>,
    body: &str,
    state: &AppState,
    headers: RequestSecurityHeaders,
) -> Result<HttpResponse, String> {
    let route = routing::parse_route(method, path);
    let normalized_path = route
        .normalized_path
        .strip_prefix("/api/v0/")
        .or_else(|| route.normalized_path.strip_prefix("/api/v1/"))
        .or_else(|| route.normalized_path.strip_prefix("/api/v2/"))
        .map_or_else(
            || route.normalized_path.to_owned(),
            |versioned_path| format!("/api/{versioned_path}"),
        );
    // The historical dispatcher future is intentionally retained for
    // differential coverage, but its monolithic match can overflow the
    // default Tokio worker stack. Keep this high-frequency read on a small
    // entry path so production and focused tests can use the real share tree.
    // The legacy merge paths use bounded route dispatcher: they are also kept
    // on the split dispatcher because they parse caller-sized batches and must
    // remain stack-safe while exercising the same bounded route contract in
    // both dispatcher configurations.
    if method == "POST"
        && matches!(
            normalized_path.as_str(),
            "/api/hashdb/sync/merge" | "/api/virtualsoulfind/shadow-index/sync/merge"
        )
    {
        return route_dispatch::route_http_request_with_headers(
            method,
            path,
            authorization,
            body,
            state,
            &headers,
        )
        .await;
    }

    if method == "GET" && normalized_path == "/api/library/items/browser" {
        let span = tracing::RequestSpan::new(
            method.to_owned(),
            path.to_owned(),
            None,
            headers.remote_addr.map(|address| address.ip().to_string()),
        );
        tracing::set_request_span(span);

        if request_uses_revoked_jwt(state, authorization).await {
            tracing::complete_request_span(401);
            return Ok(routing::unauthorized_response());
        }
        if let Err(error) =
            routing::check_route_auth(&state.config, method, route.path, authorization, &headers)
        {
            let status = if error == "unauthorized" { 401 } else { 403 };
            tracing::complete_request_span(status);
            return Ok(match error {
                "unauthorized" => routing::unauthorized_response(),
                "csrf" => routing::forbidden_response("cross-site mutating request rejected"),
                _ => routing::forbidden_response("insufficient permissions for this route"),
            });
        }

        let response = library_browser_response(state, route.query).await;
        let status_code = response
            .status
            .split(' ')
            .next()
            .and_then(|value| value.parse().ok())
            .unwrap_or(500);
        tracing::complete_request_span(status_code);
        return Ok(response);
    }

    if normalized_path.starts_with("/api/streams/")
        && (matches!(method, "GET" | "HEAD")
            || (method == "POST" && normalized_path.ends_with("/ticket")))
    {
        return route_dispatch::route_http_request_with_headers(
            method,
            path,
            authorization,
            body,
            state,
            &headers,
        )
        .await;
    }

    Box::pin(legacy_route_http_request_with_headers_inner(
        method,
        path,
        authorization,
        body,
        state,
        headers,
    ))
    .await
}
