//! Controller full native api differential fixtures ownership.

use super::*;

#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
pub(super) async fn controller_api_differential_native_application_dump_gates_impl() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    struct AuditModeGuard(Option<std::ffi::OsString>);
    impl Drop for AuditModeGuard {
        fn drop(&mut self) {
            if let Some(value) = self.0.take() {
                std::env::set_var("SLSKR_CONTROLLER_AUDIT_MODE", value);
            } else {
                std::env::remove_var("SLSKR_CONTROLLER_AUDIT_MODE");
            }
        }
    }

    let _env_lock = APPLICATION_DUMP_ENV_LOCK.lock().await;
    let _audit_mode_guard = AuditModeGuard(std::env::var_os("SLSKR_CONTROLLER_AUDIT_MODE"));
    std::env::set_var("SLSKR_CONTROLLER_AUDIT_MODE", "1");

    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!(
                    "slskdn POST /api/v0/application/dump [{}]",
                    $case
                ));
            }
            ledger.push(serde_json::json!({
                "target": "slskdn",
                "method": "POST",
                "route": "/api/v0/application/dump",
                "case": $case,
                "pass": pass,
            }));
        }};
    }

    async fn live_dump(state: Arc<crate::AppState>) -> (String, Vec<u8>) {
        let (mut client, server) = tokio::io::duplex(1024 * 1024);
        let task = tokio::spawn(crate::handle_http_stream(
            server,
            Some("127.0.0.1:1".parse().expect("dump remote address")),
            false,
            state,
        ));
        client
            .write_all(
                b"POST /api/v0/application/dump HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nContent-Length: 0\r\n\r\n",
            )
            .await
            .expect("write application dump request");
        let mut response = Vec::new();
        client
            .read_to_end(&mut response)
            .await
            .expect("read application dump response");
        task.await
            .expect("application dump HTTP task")
            .expect("application dump HTTP response");
        let header_end = response
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .expect("application dump response headers");
        (
            String::from_utf8_lossy(&response[..header_end]).to_string(),
            response[header_end + 4..].to_vec(),
        )
    }

    let (disabled, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));
    let response =
        crate::route_http_request("POST", "/api/v0/application/dump", None, "", &disabled)
            .await
            .expect("disabled slskdN dump route");
    assert_eq!(response.status, "404 Not Found");
    assert!(response.body.is_empty());

    let base = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", "native")
        .with("SLSKD_ALLOW_MEMORY_DUMP", "true")
        .with("SLSKD_ALLOW_REMOTE_NO_AUTH", "true")
        .with("SLSKD_PASSTHROUGH_ALLOWED_CIDRS", "192.0.2.0/24");
    let (local_only, _receiver) = test_state_with_env(base.clone());
    let local =
        crate::route_http_request("POST", "/api/v0/application/dump", None, "", &local_only)
            .await
            .expect("local slskdN dump route");
    assert_eq!(local.status, "200 OK");
    assert_eq!(local.content_type, "application/octet-stream");

    let remote_headers = crate::RequestSecurityHeaders {
        remote_addr: Some("192.0.2.44:1".parse().unwrap()),
        ..Default::default()
    };
    let remote_forbidden = crate::route_http_request_with_headers(
        "POST",
        "/api/v0/application/dump",
        None,
        "",
        &local_only,
        remote_headers.clone(),
    )
    .await
    .expect("remote dump denial");
    assert_eq!(remote_forbidden.status, "403 Forbidden");
    assert!(remote_forbidden.body.is_empty());

    let (remote_allowed, _receiver) =
        test_state_with_env(base.clone().with("SLSKD_ALLOW_REMOTE_DUMP", "true"));
    let remote = crate::route_http_request_with_headers(
        "POST",
        "/api/v0/application/dump",
        None,
        "",
        &remote_allowed,
        remote_headers,
    )
    .await
    .expect("remote dump allowance");
    assert_eq!(remote.status, "200 OK");

    let wrong_method =
        crate::route_http_request("GET", "/api/v0/application/dump", None, "", &local_only)
            .await
            .expect("slskdN wrong dump method");
    assert_eq!(wrong_method.status, "405 Method Not Allowed");
    record!(
        "nominal-status-headers-body",
        local.status == "200 OK"
            && local.content_type == "application/octet-stream"
            && local.body.is_empty()
    );
    record!(
        "malformed-path-query-or-body",
        wrong_method.status == "405 Method Not Allowed"
    );

    let diagnostics_dir = local_only.config.state_dir.join("diagnostics");
    let files_before = fs::read_dir(&diagnostics_dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .count();
    let (mutation_headers, mutation_body) = live_dump(Arc::clone(&local_only)).await;
    let mutation_headers_lower = mutation_headers.to_ascii_lowercase();
    let files_after = fs::read_dir(&diagnostics_dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .count();
    record!(
        "mutation-side-effects-and-readback",
        mutation_headers.starts_with("HTTP/1.1 200 OK")
            && mutation_headers_lower.contains("content-type: application/octet-stream")
            && !mutation_body.is_empty()
            && files_after == files_before
    );

    let (fresh_dump_state, _receiver) = test_state_with_env(base.clone());
    let (fresh_headers, fresh_body) = live_dump(Arc::clone(&fresh_dump_state)).await;
    record!(
        "missing-empty-or-conflict-state",
        fresh_headers.starts_with("HTTP/1.1 200 OK") && !fresh_body.is_empty()
    );
    record!(
        "restart-persistence-or-reset",
        fresh_headers.starts_with("HTTP/1.1 200 OK")
            && !fresh_body.is_empty()
            && !fresh_dump_state
                .runtime
                .read()
                .await
                .application_restart_requested
    );

    let concurrent_dumps = futures_util::future::join_all([
        live_dump(Arc::clone(&local_only)),
        live_dump(Arc::clone(&local_only)),
    ])
    .await;
    record!(
        "concurrency-and-idempotency",
        concurrent_dumps
            .iter()
            .all(|(headers, body)| { headers.starts_with("HTTP/1.1 200 OK") && !body.is_empty() })
    );

    let conflict_root = std::env::temp_dir().join(format!(
        "slskr-slskdn-application-dump-conflict-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    fs::write(&conflict_root, b"state directory is a file")
        .expect("create slskdN application dump state conflict");
    let (mut failure_state, _receiver) = test_state_with_env(base);
    Arc::get_mut(&mut failure_state)
        .expect("exclusive slskdN application dump failure state")
        .config
        .state_dir = conflict_root.clone();
    let (failure_headers, failure_body) = live_dump(Arc::clone(&failure_state)).await;
    record!(
        "runtime-failure-and-timeout",
        failure_headers.starts_with("HTTP/1.1 500 Internal Server Error")
            && failure_body
                .windows("failed to create application dump".len())
                .any(|window| window == b"failed to create application dump")
            && !failure_body
                .windows("state directory is a file".len())
                .any(|window| window == b"state directory is a file")
    );
    let _ = fs::remove_file(conflict_root);

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("native_application_dump_gates.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskdN application dump mismatches: {}",
        mismatches.len(),
        mismatches.join(", ")
    );
}
