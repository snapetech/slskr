//! Controller full web routes contracts ownership.

use super::*;

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn root_health_endpoints_are_served_anonymously() {
    // Matches the oracle's endpoints.MapHealthChecks("/health") and
    // ("/health/mesh"), both AllowAnonymous -- orchestrator/container
    // health probes hit these at the root, not under /api.
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_AUTH_DISABLED", "false"));

    let health = crate::route_http_request("GET", "/health", None, "", &state)
        .await
        .expect("root health response");
    assert_eq!(health.status, "200 OK");
    let json = serde_json::from_str::<serde_json::Value>(&health.body).unwrap();
    assert_eq!(json["status"], "ok");

    let mesh_health = crate::route_http_request("GET", "/health/mesh", None, "", &state)
        .await
        .expect("root mesh health response");
    assert_eq!(mesh_health.status, "200 OK");
    let mesh_json = serde_json::from_str::<serde_json::Value>(&mesh_health.body).unwrap();
    assert!(mesh_json.get("status").is_some());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn root_serves_webui_or_fallback_dashboard() {
    let (state, _receiver) = test_state();

    let response = crate::route_http_request("GET", "/", None, "", &state)
        .await
        .expect("root response");

    assert_eq!(response.status, "200 OK");
    assert_eq!(response.content_type, "text/html; charset=utf-8");
    if response.body.contains("id=\"root\"") && response.body.contains("/assets/") {
        assert!(response.body.contains("<script"));
        return;
    }

    assert!(response.body.contains("<h1>slskr</h1>"));
    assert!(response.body.contains("Fallback dashboard"));
    assert!(response.body.contains("href=\"/\""));
    assert!(response.body.contains("SLSKR_WEB_BUILD_DIR"));
    assert!(response.body.contains("id=\"session-actions\""));
    assert!(response
        .body
        .contains("data-session-action=\"privileges/check\""));
    assert!(response.body.contains("id=\"search-form\""));
    assert!(response.body.contains("id=\"watch-form\""));
    assert!(response.body.contains("id=\"unwatch-button\""));
    assert!(response.body.contains("id=\"browse-request-form\""));
    assert!(response.body.contains("id=\"browse-folder\""));
    assert!(response.body.contains("id=\"share-rescan-form\""));
    assert!(response.body.contains("id=\"transfer-form\""));
    assert!(response.body.contains("id=\"transfer-progress\""));
    assert!(response.body.contains("id=\"transfer-local-path\""));
    assert!(response.body.contains("id=\"message-form\""));
    assert!(response.body.contains("id=\"room-join-form\""));
    assert!(response.body.contains("id=\"room-message-form\""));
    assert!(response.body.contains("id=\"room-message-username\""));
    assert!(response.body.contains("id=\"token-form\""));
    assert!(response
        .body
        .contains("id=\"api-token\" name=\"token\" type=\"password\""));
    assert!(response.body.contains("id=\"user-table\""));
    assert!(response.body.contains("id=\"share-table\""));
    assert!(response.body.contains("id=\"message-table\""));
    assert!(response.body.contains("id=\"room-table\""));
    assert!(response.body.contains("id=\"browse-table\""));
    assert!(response.body.contains("id=\"search-filter-q\""));
    assert!(response.body.contains("id=\"transfer-filter-status\""));
    assert!(response.body.contains("id=\"share-filter-extension\""));
    assert!(response.body.contains("id=\"message-filter-direction\""));
    assert!(response.body.contains("id=\"room-filter-joined\""));
    assert!(response.body.contains("id=\"browse-filter-status\""));
    assert!(response.body.contains("/api/v0/stats"));
    assert!(response.body.contains("/api/v0/session/"));
    assert!(response.body.contains("/api/v0/searches"));
    assert!(response.body.contains("data-search-action=\"complete\""));
    assert!(response.body.contains("/api/v0/users/watch"));
    assert!(response.body.contains("data-user-action=\"browse\""));
    assert!(response.body.contains("data-user-action=\"stats\""));
    assert!(response.body.contains("/stats/request"));
    assert!(response.body.contains("/api/v0/shares/catalog"));
    assert!(response.body.contains("/browse/request"));
    assert!(response.body.contains("/api/v0/shares/rescan"));
    assert!(response.body.contains("/api/v0/transfers"));
    assert!(response.body.contains("data-transfer-action=\"complete\""));
    assert!(response.body.contains("/api/v0/messages"));
    assert!(response.body.contains("data-message-action=\"ack\""));
    assert!(response.body.contains("/api/v0/rooms/"));
    assert!(response.body.contains("/api/v0/rooms/refresh"));
    assert!(response.body.contains("data-room-action=\"leave\""));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn spa_deep_links_serve_html_shell() {
    let (state, _receiver) = test_state();

    let response = crate::route_http_request("GET", "/system/network", None, "", &state)
        .await
        .expect("SPA deep-link response");

    assert_eq!(response.status, "200 OK");
    assert_eq!(response.content_type, "text/html; charset=utf-8");
    assert!(response.body.contains("id=\"root\"") || response.body.contains("<h1>slskr</h1>"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn head_spa_routes_return_ok_without_body() {
    let (state, _receiver) = test_state();

    for path in ["/", "/system/network"] {
        let response = crate::route_http_request("HEAD", path, None, "", &state)
            .await
            .expect("HEAD SPA response");

        assert_eq!(response.status, "200 OK");
        assert_eq!(response.content_type, "text/html; charset=utf-8");
        assert!(!response.body.is_empty());
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn fallback_dashboard_is_available_on_dashboard_route() {
    let (state, _receiver) = test_state();

    let response = crate::route_http_request("GET", "/dashboard", None, "", &state)
        .await
        .expect("dashboard response");

    assert_eq!(response.status, "200 OK");
    assert_eq!(response.content_type, "text/html; charset=utf-8");
    assert!(response.body.contains("Fallback dashboard"));
    assert!(response.body.contains("href=\"/\""));
    assert!(response.body.contains("id=\"session-actions\""));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn web_static_resolver_stays_under_build_root() {
    let root = std::env::temp_dir().join(format!(
        "slskr-web-static-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(root.join("assets")).unwrap();
    std::fs::create_dir_all(root.join("static")).unwrap();
    std::fs::write(root.join("index.html"), "<html></html>").unwrap();
    std::fs::write(root.join("assets").join("app.js"), "console.log('ok')").unwrap();
    std::fs::write(root.join("static").join("app.js"), "console.log('static')").unwrap();
    std::fs::write(root.join("favicon.ico"), "ico").unwrap();

    let (asset, content_type) =
        crate::web_static_file_for_request_under_root(&root, "/assets/app.js")
            .expect("asset resolved");
    assert!(asset.ends_with("assets/app.js"));
    assert_eq!(content_type, "text/javascript; charset=utf-8");

    let (nested_asset, nested_content_type) =
        crate::web_static_file_for_request_under_root(&root, "/system/mediacore/assets/app.js")
            .expect("nested SPA asset resolved");
    assert!(nested_asset.ends_with("assets/app.js"));
    assert_eq!(nested_content_type, "text/javascript; charset=utf-8");

    let (nested_static_asset, nested_static_content_type) =
        crate::web_static_file_for_request_under_root(&root, "/system/static/app.js")
            .expect("nested static asset resolved");
    assert!(nested_static_asset.ends_with("static/app.js"));
    assert_eq!(nested_static_content_type, "text/javascript; charset=utf-8");

    let (nested_icon, nested_icon_type) =
        crate::web_static_file_for_request_under_root(&root, "/system/favicon.ico")
            .expect("nested SPA root icon resolved");
    assert!(nested_icon.ends_with("favicon.ico"));
    assert_eq!(nested_icon_type, "image/x-icon");

    let (fallback, _) = crate::web_static_file_for_request_under_root(&root, "/missing-route")
        .expect("SPA fallback resolved");
    assert!(fallback.ends_with("index.html"));
    assert!(crate::web_static_file_for_request_under_root(&root, "/../Cargo.toml").is_none());

    let _ = std::fs::remove_dir_all(root);
}

#[cfg(unix)]
#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn web_static_resolver_rejects_symlink_escape() {
    use std::os::unix::fs::symlink;

    let root = std::env::temp_dir().join(format!(
        "slskr-web-static-symlink-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    let outside = std::env::temp_dir().join(format!(
        "slskr-web-static-outside-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("index.html"), "<html></html>").unwrap();
    std::fs::write(&outside, "secret").unwrap();
    symlink(&outside, root.join("leak.txt")).unwrap();

    assert!(crate::web_static_file_for_request_under_root(&root, "/leak.txt").is_none());
    assert!(crate::read_bounded_web_static_file(&root.join("leak.txt")).is_err());

    let outside_dir = outside.with_extension("dir");
    std::fs::create_dir_all(&outside_dir).unwrap();
    std::fs::write(outside_dir.join("secret.txt"), "secret").unwrap();
    symlink(&outside_dir, root.join("linked")).unwrap();
    assert!(
        crate::read_bounded_web_static_file_under_root(&root, &root.join("linked/secret.txt"))
            .is_err()
    );

    let _ = std::fs::remove_file(outside);
    let _ = std::fs::remove_dir_all(outside_dir);
    let _ = std::fs::remove_dir_all(root);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn web_static_reader_rejects_oversized_assets() {
    let path = std::env::temp_dir().join(format!(
        "slskr-web-static-large-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    let file = std::fs::File::create(&path).unwrap();
    file.set_len(crate::MAX_WEB_STATIC_BYTES + 1).unwrap();

    let error = crate::read_bounded_web_static_file(&path)
        .expect_err("oversized static asset should be rejected");
    assert!(error.contains("static asset is too large"));

    let _ = std::fs::remove_file(path);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn web_static_oversize_errors_return_413() {
    let response = crate::web_static_error_response(
        "static asset is too large: 1 bytes at /srv/private/web/index.html",
    );
    assert_eq!(response.status, "413 Payload Too Large");
    assert!(response.body.contains("static asset is too large"));
    assert!(!response.body.contains("1 bytes"));
    assert!(!response.body.contains("/srv/private"));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn web_static_internal_errors_do_not_expose_filesystem_details() {
    let response = crate::web_static_error_response(
        "static file confined open failed: permission denied: /srv/private/web/index.html",
    );
    assert_eq!(response.status, "500 Internal Server Error");
    assert_eq!(response.body, "{\"error\":\"static asset is unavailable\"}");
    assert!(!response.body.contains("permission denied"));
    assert!(!response.body.contains("/srv/private"));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn web_static_string_reader_rejects_invalid_utf8() {
    let path = std::env::temp_dir().join(format!(
        "slskr-web-static-invalid-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::write(&path, [0xff_u8]).unwrap();

    let error = crate::read_bounded_web_static_string(&path)
        .expect_err("invalid UTF-8 static text should be rejected");
    assert!(error.contains("static asset is not UTF-8"));

    let _ = std::fs::remove_file(path);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn web_static_csp_rejects_inline_scripts_and_scopes_style_and_wasm_exceptions() {
    let root = std::env::temp_dir().join(format!(
        "slskr-web-static-csp-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&root).unwrap();
    let react_index = root.join("index.html");
    std::fs::write(&react_index, "<html></html>").unwrap();

    let react_csp = crate::web_static_content_security_policy(&react_index);
    assert!(react_csp.contains("script-src 'self'"));
    assert!(!react_csp.contains("script-src 'self' 'unsafe-inline'"));
    assert!(react_csp.contains("style-src-attr 'unsafe-inline'"));
    assert!(react_csp.contains("'sha256-AVTm08UMHPqpttgoudpSsvenKKfidtwuSnUVJLIuqcA='"));
    assert!(!react_csp.contains("wasm-unsafe-eval"));

    std::fs::write(root.join("slskr_web.wasm"), []).unwrap();
    let wasm_csp = crate::web_static_content_security_policy(&react_index);
    assert!(wasm_csp.contains("script-src 'self' 'wasm-unsafe-eval'"));
    assert!(!wasm_csp.contains("script-src 'self' 'unsafe-inline'"));
    assert!(wasm_csp.contains("style-src-attr 'unsafe-inline'"));

    let _ = std::fs::remove_dir_all(root);
}
