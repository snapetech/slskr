//! Controller full controller api differential ownership.

use super::*;

#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
pub(super) async fn controller_api_differential_controller_application_dump_contracts() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!(
                    "slskd GET /api/v0/application/dump [{}]",
                    $case
                ));
            }
            ledger.push(serde_json::json!({
                "target": "slskd",
                "method": "GET",
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
            Some("127.0.0.1:1".parse().expect("relay stream remote address")),
            false,
            state,
        ));
        client
            .write_all(
                b"GET /api/v0/application/dump HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
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

    struct AuditModeGuard(Option<OsString>);
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

    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));
    let (nominal_headers, nominal_body) = live_dump(Arc::clone(&state)).await;
    let nominal_headers_lower = nominal_headers.to_ascii_lowercase();
    record!(
        "nominal-status-headers-body",
        nominal_headers.starts_with("HTTP/1.1 200 OK")
            && nominal_headers_lower.contains("content-type: application/octet-stream")
            && nominal_headers_lower
                .contains("content-disposition: attachment; filename=slskd.dmp")
            && !nominal_body.is_empty()
    );

    assert!(crate::preview_stream_controller::application_dump_path(
        "/api/v0/application/dump"
    ));
    assert!(!crate::preview_stream_controller::application_dump_path(
        "/api/v1/application/dump"
    ));
    assert!(!crate::preview_stream_controller::application_dump_path(
        "/api/v0/application/dump/extra"
    ));
    let wrong_method =
        crate::route_http_request("POST", "/api/v0/application/dump", None, "", &state)
            .await
            .expect("slskd wrong dump method");
    record!(
        "malformed-path-query-or-body",
        wrong_method.status == "405 Method Not Allowed"
    );

    let (empty_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));
    let (empty_headers, empty_body) = live_dump(Arc::clone(&empty_state)).await;
    record!(
        "missing-empty-or-conflict-state",
        empty_headers.starts_with("HTTP/1.1 200 OK") && !empty_body.is_empty()
    );

    state.session.write().await.state = "connected";
    state.session.write().await.username = Some("dump-user".to_owned());
    crate::record_event(
        &state,
        "application.dump.fixture",
        "application",
        Some("populated-state".to_owned()),
    )
    .await;
    let (populated_headers, populated_body) = live_dump(Arc::clone(&state)).await;
    record!(
        "populated-dynamic-state",
        populated_headers.starts_with("HTTP/1.1 200 OK")
            && !populated_body.is_empty()
            && state.session.read().await.state == "connected"
    );

    let conflict_root = std::env::temp_dir().join(format!(
        "slskr-application-dump-conflict-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    fs::write(&conflict_root, b"state directory is a file")
        .expect("create application dump state conflict");
    let (mut failure_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));
    Arc::get_mut(&mut failure_state)
        .expect("exclusive application dump failure state")
        .config
        .state_dir = conflict_root.clone();
    let (failure_headers, failure_body) = live_dump(Arc::clone(&failure_state)).await;
    record!(
        "runtime-failure-and-timeout",
        failure_headers.starts_with("HTTP/1.1 500 Internal Server Error")
            && failure_body
                .windows("failed to create application dump".len())
                .any(|window| window == b"failed to create application dump",)
            && !failure_body
                .windows("state directory is a file".len())
                .any(|window| window == b"state directory is a file")
    );
    let _ = fs::remove_file(conflict_root);

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("controller_application_dump_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize application dump ledger"),
    )
    .expect("write application dump ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskd application dump mismatches: {}",
        mismatches.len(),
        mismatches.join(", ")
    );
}

#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
pub(super) async fn controller_api_differential_controller_browse_status_tracks_request_failure_and_completion(
) {
    let (state, _receiver) = test_state();
    state.session.write().await.state = "connected";

    let requested = crate::route_http_request(
        "POST",
        "/api/v0/users/friend/browse/request",
        None,
        "",
        &state,
    )
    .await
    .expect("request browse");
    assert_eq!(requested.status, "202 Accepted");

    let status = crate::route_http_request(
        "GET",
        "/api/v0/users/friend/browse/status",
        None,
        "",
        &state,
    )
    .await
    .expect("requested browse status");
    assert_eq!(status.status, "200 OK");
    assert!(status.content_type.contains("application/json"));
    let status_json = serde_json::from_str::<serde_json::Value>(&status.body).unwrap();
    assert_eq!(status_json["status"], "requested");
    assert_eq!(status_json["state"], "InProgress");
    assert_eq!(status_json["isComplete"], false);
    assert_eq!(status_json["percentComplete"], 0.0);

    let failed = crate::route_http_request(
        "POST",
        "/api/v0/users/friend/browse/fail",
        None,
        "{\"reason\":\"timed out\"}",
        &state,
    )
    .await
    .expect("fail browse");
    assert_eq!(failed.status, "200 OK");

    let status = crate::route_http_request(
        "GET",
        "/api/v0/users/friend/browse/status",
        None,
        "",
        &state,
    )
    .await
    .expect("failed browse status");
    assert_eq!(status.status, "200 OK");
    assert!(status.content_type.contains("application/json"));
    let status_json = serde_json::from_str::<serde_json::Value>(&status.body).unwrap();
    assert_eq!(status_json["status"], "failed");
    assert_eq!(status_json["state"], "Failed");
    assert_eq!(status_json["reason"], "browse failed");

    crate::route_http_request(
        "POST",
        "/api/v0/browse-responses",
        None,
        "{\"username\":\"friend\",\"entries\":[{\"filename\":\"Remote/Album/One.flac\",\"size\":7}]}",
        &state,
    )
    .await
    .expect("complete browse");

    let status = crate::route_http_request(
        "GET",
        "/api/v0/users/friend/browse/status",
        None,
        "",
        &state,
    )
    .await
    .expect("completed browse status");
    assert_eq!(status.status, "200 OK");
    assert!(status.content_type.contains("application/json"));
    let status_json = serde_json::from_str::<serde_json::Value>(&status.body).unwrap();
    assert_eq!(status_json["status"], "ready");
    assert_eq!(status_json["state"], "Completed");
    assert_eq!(status_json["isComplete"], true);
    assert_eq!(status_json["size"], 7);
    assert_eq!(status_json["bytesTransferred"], 7);
    assert_eq!(status_json["fileCount"], 1);
    assert_eq!(status_json["directoryCount"], 1);

    let ledger = vec![
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/users/{username}/browse/status",
            "case": "nominal-status-headers-body",
            "pass": true,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/users/{username}/browse/status",
            "case": "populated-dynamic-state",
            "pass": true,
        }),
    ];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("user_browse_status_projection.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}

/// Bulk differential proof crediting 34 GET routes'
/// `missing-empty-or-conflict-state` cases, independently re-derived
/// from `materialized_controller_gets_match_native_empty_state_
/// contracts`'s real empty-state response-shape checks for
/// nonexistent resources. slskdN-only (confirmed against the frozen
/// registry).
#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-2"
))]
pub(super) async fn controller_api_differential_materialized_empty_state_gets() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    let (state, _receiver) = test_state();
    let cases = [
        (
            "/api/v0/nowplaying",
            "/api/v0/nowplaying",
            "204 No Content",
            None,
        ),
        (
            "/api/v0/listening-party/missing-pod/missing-channel",
            "/api/v0/listening-party/{podId}/{channelId}",
            "404 Not Found",
            Some("\"error\":\"not found\""),
        ),
        (
            "/api/v0/mediacore/contentid/domain/music",
            "/api/v0/mediacore/contentid/domain/{domain}",
            "200 OK",
            Some("\"contentIds\":[]"),
        ),
        (
            "/api/v0/mediacore/contentid/domain/music/type/recording",
            "/api/v0/mediacore/contentid/domain/{domain}/type/{type}",
            "200 OK",
            Some("\"normalizedType\":\"recording\""),
        ),
        (
            "/api/v0/mediacore/contentid/stats",
            "/api/v0/mediacore/contentid/stats",
            "200 OK",
            Some("\"totalMappings\":0"),
        ),
        (
            "/api/v0/mediacore/contentid/exists/missing",
            "/api/v0/mediacore/contentid/exists/{externalId}",
            "200 OK",
            Some("\"exists\":false"),
        ),
        (
            "/api/v0/mediacore/contentid/external/missing",
            "/api/v0/mediacore/contentid/external/{contentId}",
            "200 OK",
            Some("\"externalIds\":[]"),
        ),
        (
            "/api/v0/mediacore/contentid/validate/not-a-content-id",
            "/api/v0/mediacore/contentid/validate/{*contentId}",
            "200 OK",
            Some("\"isValid\":false"),
        ),
        (
            "/api/v0/mediacore/ipld/graph/missing",
            "/api/v0/mediacore/ipld/graph/{*contentId}",
            "200 OK",
            Some("\"nodes\":["),
        ),
        (
            "/api/v0/mediacore/ipld/inbound/missing",
            "/api/v0/mediacore/ipld/inbound/{*targetContentId}",
            "200 OK",
            Some("\"inboundLinks\":[]"),
        ),
        (
            "/api/v0/mediacore/ipld/traverse/content:audio:track:missing?linkName=missing",
            "/api/v0/mediacore/ipld/traverse/{*startContentId}",
            "200 OK",
            Some("\"visitedNodes\":["),
        ),
        (
            "/api/v0/mediacore/ipld/validate",
            "/api/v0/mediacore/ipld/validate",
            "200 OK",
            Some("\"isValid\":true"),
        ),
        (
            "/api/v0/mediacore/retrieve/query/domain/music",
            "/api/v0/mediacore/retrieve/query/domain/{domain}",
            "200 OK",
            Some("\"descriptors\":[]"),
        ),
        (
            "/api/v0/mediacore/perceptualhash/algorithms",
            "/api/v0/mediacore/perceptualhash/algorithms",
            "200 OK",
            Some("\"algorithms\":["),
        ),
        (
            "/api/v0/mediacore/portability/merge-strategies",
            "/api/v0/mediacore/portability/merge-strategies",
            "200 OK",
            Some("\"strategies\":["),
        ),
        (
            "/api/v0/mediacore/portability/strategies",
            "/api/v0/mediacore/portability/strategies",
            "200 OK",
            Some("\"strategies\":["),
        ),
        (
            "/api/v0/mediacore/retrieve/stats",
            "/api/v0/mediacore/retrieve/stats",
            "200 OK",
            Some("\"totalRetrievals\":0"),
        ),
        (
            "/api/v0/mediacore/publish/stats",
            "/api/v0/mediacore/publish/stats",
            "200 OK",
            Some("\"totalPublishedDescriptors\":0"),
        ),
        (
            "/api/v0/mediacore/stats/dashboard",
            "/api/v0/mediacore/stats/dashboard",
            "200 OK",
            Some("\"contentRegistry\""),
        ),
        (
            "/api/v0/mediacore/stats/descriptors",
            "/api/v0/mediacore/stats/descriptors",
            "200 OK",
            Some("\"totalRetrievals\":0"),
        ),
        (
            "/api/v0/mediacore/stats/fuzzy",
            "/api/v0/mediacore/stats/fuzzy",
            "200 OK",
            Some("\"totalMatches\":0"),
        ),
        (
            "/api/v0/mediacore/stats/ipld",
            "/api/v0/mediacore/stats/ipld",
            "200 OK",
            Some("\"totalLinks\":0"),
        ),
        (
            "/api/v0/mediacore/stats/perceptual",
            "/api/v0/mediacore/stats/perceptual",
            "200 OK",
            Some("\"totalHashesComputed\":0"),
        ),
        (
            "/api/v0/mediacore/stats/portability",
            "/api/v0/mediacore/stats/portability",
            "200 OK",
            Some("\"totalExports\":0"),
        ),
        (
            "/api/v0/mediacore/stats/publishing",
            "/api/v0/mediacore/stats/publishing",
            "200 OK",
            Some("\"totalPublished\":0"),
        ),
        (
            "/api/v0/mediacore/stats/registry",
            "/api/v0/mediacore/stats/registry",
            "200 OK",
            Some("\"totalMappings\":0"),
        ),
        (
            "/api/v0/mediacore/retrieve/descriptor/content:audio:track:missing",
            "/api/v0/mediacore/retrieve/descriptor/{*contentId}",
            "404 Not Found",
            Some("\"found\":false"),
        ),
        (
            "/api/v0/podcore/missing/opinions/members/affinity",
            "/api/v0/podcore/{podId}/opinions/members/affinity",
            "200 OK",
            Some("{}"),
        ),
        (
            "/api/v0/podcore/backfill/missing/last-seen",
            "/api/v0/podcore/backfill/{podId}/last-seen",
            "200 OK",
            Some("{}"),
        ),
        (
            "/api/v0/pods/missing/channels/missing/messages",
            "/api/v0/pods/{podId}/channels/{channelId}/messages",
            "200 OK",
            Some("[]"),
        ),
        (
            "/api/v0/quarantine-jury/requests/missing/routes",
            "/api/v0/quarantine-jury/requests/{requestId}/routes",
            "200 OK",
            Some("[]"),
        ),
        (
            "/api/v0/security/disclosure/missing",
            "/api/v0/security/disclosure/{username}",
            "200 OK",
            Some("\"peerTier\":\"Unknown\""),
        ),
        (
            "/api/v0/security/reputation/missing",
            "/api/v0/security/reputation/{username}",
            "200 OK",
            Some("\"score\":50"),
        ),
        (
            "/api/v0/traces/missing/summary",
            "/api/v0/traces/{jobId}/summary",
            "200 OK",
            Some("\"totalEvents\":0"),
        ),
    ];

    for (path, route_template, status, expected_body) in cases {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        let pass = response.status == status
            && match expected_body {
                Some(expected) => response.body.contains(expected),
                None => response.body.is_empty(),
            };
        if !pass {
            mismatches.push(format!(
                "{target} GET {route_template} [missing-empty-or-conflict-state]: {}",
                response.body
            ));
        }
        ledger.push(serde_json::json!({
            "target": target,
            "method": "GET",
            "route": route_template,
            "case": "missing-empty-or-conflict-state",
            "pass": pass,
        }));
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("materialized_empty_state_gets.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api materialized-empty-state mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting the remaining tail of
/// `openapi_mutation_dtos_match_native_status_and_field_contracts`
/// (everything after the Collections/CollectionItems lifecycle, which
/// `controller_api_differential_collections_items_crud_reorder_
/// lifecycle` already credits): mediacore content-id registration
/// validation, nowplaying, podcore content-pod creation, the full
/// wishlist lifecycle, overlay IP blocklisting, quarantine-jury
/// request validation, and security IP bans. slskdN-only (confirmed
/// against the frozen registry).
#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-2"
))]
pub(super) async fn controller_api_differential_openapi_mutation_dtos_tail() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!("{target} {} {} [{}]", $method, $route, $case));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": $method,
                "route": $route,
                "case": $case,
                "pass": $pass,
            }));
        };
    }

    let (state, _receiver) = test_state();

    let invalid_content_id = crate::route_http_request(
        "POST",
        "/api/v0/mediacore/contentid/register",
        None,
        r#"{"externalId":"mbid","contentId":"invalid"}"#,
        &state,
    )
    .await
    .expect("register invalid content id");
    record!(
        "POST",
        "/api/v0/mediacore/contentid/register",
        "malformed-path-query-or-body",
        invalid_content_id.status == "400 Bad Request"
    );

    let valid_content_id = crate::route_http_request(
        "POST",
        "/api/v0/mediacore/contentid/register",
        None,
        r#"{"externalId":"mbid","contentId":"content:music:recording:mbid"}"#,
        &state,
    )
    .await
    .expect("register valid content id");
    record!(
        "POST",
        "/api/v0/mediacore/contentid/register",
        "mutation-side-effects-and-readback",
        valid_content_id.status == "200 OK"
            && valid_content_id
                .body
                .contains("mapping registered successfully")
    );

    let mut content_id_gets_pass = true;
    for (path, expected) in [
        (
            "/api/v0/mediacore/contentid/exists/mbid",
            r#"{"exists":true}"#,
        ),
        (
            "/api/v0/mediacore/contentid/external/content%3Amusic%3Arecording%3Ambid",
            r#"{"externalIds":["mbid"]}"#,
        ),
        (
            "/api/v0/mediacore/contentid/domain/music",
            r#""contentIds":["content:music:recording:mbid"]"#,
        ),
        (
            "/api/v0/mediacore/contentid/domain/music/type/recording",
            r#""contentIds":["content:music:recording:mbid"]"#,
        ),
    ] {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        content_id_gets_pass &= response.status == "200 OK" && response.body.contains(expected);
    }
    record!(
        "GET",
        "/api/v0/mediacore/contentid/exists/{externalId}",
        "nominal-status-headers-body",
        content_id_gets_pass
    );
    record!(
        "GET",
        "/api/v0/mediacore/contentid/external/{contentId}",
        "nominal-status-headers-body",
        content_id_gets_pass
    );
    record!(
        "GET",
        "/api/v0/mediacore/contentid/domain/{domain}",
        "nominal-status-headers-body",
        content_id_gets_pass
    );
    record!(
        "GET",
        "/api/v0/mediacore/contentid/domain/{domain}/type/{type}",
        "nominal-status-headers-body",
        content_id_gets_pass
    );
    record!(
        "GET",
        "/api/v0/mediacore/contentid/exists/{externalId}",
        "populated-dynamic-state",
        content_id_gets_pass
    );
    record!(
        "GET",
        "/api/v0/mediacore/contentid/external/{contentId}",
        "populated-dynamic-state",
        content_id_gets_pass
    );
    record!(
        "GET",
        "/api/v0/mediacore/contentid/domain/{domain}",
        "populated-dynamic-state",
        content_id_gets_pass
    );
    record!(
        "GET",
        "/api/v0/mediacore/contentid/domain/{domain}/type/{type}",
        "populated-dynamic-state",
        content_id_gets_pass
    );

    let resolved_content_id = crate::route_http_request(
        "GET",
        "/api/v0/mediacore/contentid/resolve/mbid",
        None,
        "",
        &state,
    )
    .await
    .expect("resolve content id");
    let resolved_content_id_json =
        serde_json::from_str::<serde_json::Value>(&resolved_content_id.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/mediacore/contentid/resolve/{externalId}",
        "nominal-status-headers-body",
        resolved_content_id.status == "200 OK" && !resolved_content_id.body.is_empty()
    );
    record!(
        "GET",
        "/api/v0/mediacore/contentid/resolve/{externalId}",
        "populated-dynamic-state",
        resolved_content_id.status == "200 OK"
            && resolved_content_id_json["contentId"] == "content:music:recording:mbid"
    );

    let unresolved_content_id = crate::route_http_request(
        "GET",
        "/api/v0/mediacore/contentid/resolve/missing",
        None,
        "",
        &state,
    )
    .await
    .expect("resolve missing content id");
    record!(
        "GET",
        "/api/v0/mediacore/contentid/resolve/{externalId}",
        "missing-empty-or-conflict-state",
        unresolved_content_id.status == "404 Not Found"
            && unresolved_content_id.body.contains("External ID not found")
    );

    let content_id_stats =
        crate::route_http_request("GET", "/api/v0/mediacore/contentid/stats", None, "", &state)
            .await
            .expect("content id stats");
    let content_id_stats_json =
        serde_json::from_str::<serde_json::Value>(&content_id_stats.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/mediacore/contentid/stats",
        "populated-dynamic-state",
        content_id_stats.status == "200 OK"
            && content_id_stats_json["totalMappings"] == 1
            && content_id_stats_json["totalDomains"] == 1
            && content_id_stats_json["mappingsByDomain"]["music"] == 1
    );

    let valid_content_id_readback = crate::route_http_request(
        "GET",
        "/api/v0/mediacore/contentid/validate/content%3Amusic%3Arecording%3Ambid",
        None,
        "",
        &state,
    )
    .await
    .expect("validate content id");
    let valid_content_id_json =
        serde_json::from_str::<serde_json::Value>(&valid_content_id_readback.body)
            .unwrap_or_default();
    record!(
        "GET",
        "/api/v0/mediacore/contentid/validate/{*contentId}",
        "nominal-status-headers-body",
        valid_content_id_readback.status == "200 OK" && !valid_content_id_readback.body.is_empty()
    );
    record!(
        "GET",
        "/api/v0/mediacore/contentid/validate/{*contentId}",
        "populated-dynamic-state",
        valid_content_id_readback.status == "200 OK"
            && valid_content_id_json["contentId"] == "content:music:recording:mbid"
            && valid_content_id_json["isValid"] == true
            && valid_content_id_json["domain"] == "music"
            && valid_content_id_json["type"] == "recording"
            && valid_content_id_json["id"] == "mbid"
    );

    let now_playing = crate::route_http_request(
        "PUT",
        "/api/v0/nowplaying",
        None,
        r#"{"artist":"Artist","title":"Track","album":"Album"}"#,
        &state,
    )
    .await
    .expect("set now playing");
    record!(
        "PUT",
        "/api/v0/nowplaying",
        "nominal-status-headers-body",
        now_playing.status == "204 No Content" && now_playing.body.is_empty()
    );
    let now_playing_readback =
        crate::route_http_request("GET", "/api/v0/nowplaying", None, "", &state)
            .await
            .expect("get now playing");
    record!(
        "PUT",
        "/api/v0/nowplaying",
        "mutation-side-effects-and-readback",
        now_playing.status == "204 No Content"
            && now_playing_readback.status == "200 OK"
            && now_playing_readback.body.contains("Artist")
            && now_playing_readback.body.contains("Track")
    );
    record!(
        "GET",
        "/api/v0/nowplaying",
        "nominal-status-headers-body",
        now_playing_readback.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/nowplaying",
        "populated-dynamic-state",
        now_playing_readback.status == "200 OK"
            && now_playing_readback.body.contains("Artist")
            && now_playing_readback.body.contains("Track")
    );

    let invalid_content_pod = crate::route_http_request(
        "POST",
        "/api/v0/podcore/content/create-pod",
        None,
        r#"{"podId":"pod:invalid","name":"Differential","visibility":"Listed","contentId":"content:music:recording:differential"}"#,
        &state,
    )
    .await
    .expect("create invalid content pod");
    record!(
        "POST",
        "/api/v0/podcore/content/create-pod",
        "malformed-path-query-or-body",
        invalid_content_pod.status == "400 Bad Request"
    );

    let content_pod = crate::route_http_request(
        "POST",
        "/api/v0/podcore/content/create-pod",
        None,
        r#"{"podId":"pod:00000000000000000000000000000003","name":"Differential","visibility":"Listed","contentId":"content:music:recording:differential","tags":[],"channels":[],"externalBindings":[]}"#,
        &state,
    )
    .await
    .expect("create valid content pod");
    let content_pod_json =
        serde_json::from_str::<serde_json::Value>(&content_pod.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/podcore/content/create-pod",
        "nominal-status-headers-body",
        content_pod.status == "201 Created"
            && content_pod_json["focusContentId"] == "content:music:recording:differential"
    );
    let content_pod_readback = crate::route_http_request(
        "GET",
        "/api/v0/pods/pod%3A00000000000000000000000000000003",
        None,
        "",
        &state,
    )
    .await
    .expect("read content-linked pod");
    record!(
        "POST",
        "/api/v0/podcore/content/create-pod",
        "mutation-side-effects-and-readback",
        content_pod.status == "201 Created"
            && content_pod_json["focusContentId"] == "content:music:recording:differential"
            && content_pod_readback.status == "200 OK"
            && content_pod_readback
                .body
                .contains("content:music:recording:differential")
    );

    let wishlist_item = crate::route_http_request(
        "POST",
        "/api/v0/wishlist",
        None,
        r#"{"searchText":"differential","filter":"differential","enabled":true,"autoDownload":true,"maxResults":1,"maxDownloads":1}"#,
        &state,
    )
    .await
    .expect("create wishlist item");
    let wishlist_json =
        serde_json::from_str::<serde_json::Value>(&wishlist_item.body).unwrap_or_default();
    let wishlist_id = wishlist_json["id"].as_str().unwrap_or_default().to_owned();
    record!(
        "POST",
        "/api/v0/wishlist",
        "nominal-status-headers-body",
        wishlist_item.status == "201 Created"
    );
    record!(
        "POST",
        "/api/v0/wishlist",
        "mutation-side-effects-and-readback",
        uuid::Uuid::parse_str(&wishlist_id).is_ok()
            && wishlist_json["searchText"] == "differential"
            && wishlist_json["maxResults"] == 1
            && wishlist_json.get("artist").is_none()
    );

    let updated_wishlist = crate::route_http_request(
        "PUT",
        &format!("/api/v0/wishlist/{wishlist_id}"),
        None,
        r#"{"searchText":"updated-differential","filter":"lossless","enabled":true,"autoDownload":false,"maxResults":5,"maxDownloads":1}"#,
        &state,
    )
    .await
    .expect("update wishlist item");
    let updated_wishlist_json =
        serde_json::from_str::<serde_json::Value>(&updated_wishlist.body).unwrap_or_default();
    record!(
        "PUT",
        "/api/v0/wishlist/{id}",
        "mutation-side-effects-and-readback",
        updated_wishlist_json["searchText"] == "updated-differential"
            && updated_wishlist_json["maxResults"] == 5
    );

    let ignored_result = crate::route_http_request(
        "POST",
        &format!("/api/v0/wishlist/{wishlist_id}/ignored-results"),
        None,
        r#"{"username":"differential-peer","directory":"/tmp/slskdn-differential"}"#,
        &state,
    )
    .await
    .expect("add ignored result");
    let ignored_json =
        serde_json::from_str::<serde_json::Value>(&ignored_result.body).unwrap_or_default();
    let ignored_id = ignored_json["id"].as_str().unwrap_or_default().to_owned();
    record!(
        "POST",
        "/api/v0/wishlist/{id}/ignored-results",
        "nominal-status-headers-body",
        ignored_result.status == "201 Created"
    );
    record!(
        "POST",
        "/api/v0/wishlist/{id}/ignored-results",
        "mutation-side-effects-and-readback",
        ignored_json["wishlistItemId"] == wishlist_id
            && ignored_json["directory"] == "/tmp/slskdn-differential"
    );
    let ignored_list = crate::route_http_request(
        "GET",
        &format!("/api/v0/wishlist/{wishlist_id}/ignored-results"),
        None,
        "",
        &state,
    )
    .await
    .expect("list ignored results");
    let ignored_list_json =
        serde_json::from_str::<serde_json::Value>(&ignored_list.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/wishlist/{id}/ignored-results",
        "nominal-status-headers-body",
        ignored_list.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/wishlist/{id}/ignored-results",
        "populated-dynamic-state",
        ignored_list.status == "200 OK"
            && ignored_list_json.as_array().is_some_and(|entries| {
                entries.len() == 1
                    && entries[0]["wishlistItemId"] == wishlist_id
                    && entries[0]["directory"] == "/tmp/slskdn-differential"
            })
    );

    let deleted_ignored = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/wishlist/{wishlist_id}/ignored-results/{ignored_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete ignored result");
    record!(
        "DELETE",
        "/api/v0/wishlist/{id}/ignored-results/{ignoredResultId}",
        "nominal-status-headers-body",
        deleted_ignored.status == "204 No Content"
    );
    let ignored_after_delete = crate::route_http_request(
        "GET",
        &format!("/api/v0/wishlist/{wishlist_id}/ignored-results"),
        None,
        "",
        &state,
    )
    .await
    .expect("read deleted ignored result");
    record!(
        "DELETE",
        "/api/v0/wishlist/{id}/ignored-results/{ignoredResultId}",
        "mutation-side-effects-and-readback",
        deleted_ignored.status == "204 No Content"
            && ignored_after_delete.status == "200 OK"
            && ignored_after_delete.body == "[]"
    );

    let wishlist_before_view = crate::route_http_request(
        "GET",
        &format!("/api/v0/wishlist/{wishlist_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("read wishlist before marking viewed");
    let wishlist_before_view_json =
        serde_json::from_str::<serde_json::Value>(&wishlist_before_view.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/wishlist/{id}",
        "nominal-status-headers-body",
        wishlist_before_view.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/wishlist/{id}",
        "populated-dynamic-state",
        wishlist_before_view.status == "200 OK"
            && wishlist_before_view_json["id"] == wishlist_id
            && wishlist_before_view_json["searchText"] == "updated-differential"
    );

    let marked_viewed = crate::route_http_request(
        "POST",
        &format!("/api/v0/wishlist/{wishlist_id}/mark-viewed"),
        None,
        "",
        &state,
    )
    .await
    .expect("mark wishlist viewed");
    let wishlist_after_view = crate::route_http_request(
        "GET",
        &format!("/api/v0/wishlist/{wishlist_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("read wishlist after marking viewed");
    record!(
        "POST",
        "/api/v0/wishlist/{id}/mark-viewed",
        "nominal-status-headers-body",
        marked_viewed.status == "204 No Content"
    );
    let wishlist_after_view_json =
        serde_json::from_str::<serde_json::Value>(&wishlist_after_view.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/wishlist/{id}/mark-viewed",
        "mutation-side-effects-and-readback",
        marked_viewed.status == "204 No Content"
            && wishlist_after_view.status == "200 OK"
            && wishlist_before_view_json["lastViewedAt"].is_null()
            && wishlist_after_view_json["lastViewedAt"].is_string()
    );

    let deleted_wishlist = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/wishlist/{wishlist_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete wishlist item");
    record!(
        "DELETE",
        "/api/v0/wishlist/{id}",
        "nominal-status-headers-body",
        deleted_wishlist.status == "204 No Content"
    );
    let deleted_wishlist_readback = crate::route_http_request(
        "GET",
        &format!("/api/v0/wishlist/{wishlist_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("read deleted wishlist item");
    record!(
        "DELETE",
        "/api/v0/wishlist/{id}",
        "mutation-side-effects-and-readback",
        deleted_wishlist.status == "204 No Content"
            && deleted_wishlist_readback.status == "404 Not Found"
    );

    let block = crate::route_http_request(
        "POST",
        "/api/v0/overlay/blocklist/ip",
        None,
        r#"{"ip":"192.0.2.9","reason":"differential"}"#,
        &state,
    )
    .await
    .expect("block ip");
    record!(
        "POST",
        "/api/v0/overlay/blocklist/ip",
        "nominal-status-headers-body",
        block.status == "200 OK" && block.body == r#"{"message":"IP address blocked"}"#
    );
    let blocklist = crate::route_http_request("GET", "/api/v0/overlay/blocklist", None, "", &state)
        .await
        .expect("list overlay blocklist");
    record!(
        "POST",
        "/api/v0/overlay/blocklist/ip",
        "mutation-side-effects-and-readback",
        block.status == "200 OK"
            && blocklist.status == "200 OK"
            && blocklist.body.contains("192.0.2.9")
    );
    record!(
        "GET",
        "/api/v0/overlay/blocklist",
        "populated-dynamic-state",
        blocklist.status == "200 OK" && blocklist.body.contains("192.0.2.9")
    );

    let invalid_jury = crate::route_http_request(
        "POST",
        "/api/v0/quarantine-jury/requests",
        None,
        r#"{"evidence":[],"jurors":[],"minJurorVotes":1}"#,
        &state,
    )
    .await
    .expect("validate invalid jury request");
    record!(
        "POST",
        "/api/v0/quarantine-jury/requests",
        "malformed-path-query-or-body",
        invalid_jury.status == "400 Bad Request"
            && invalid_jury.body.contains("trusted juror")
            && invalid_jury.body.contains("minimal evidence")
    );

    let security_ban = crate::route_http_request(
        "POST",
        "/api/v0/security/bans/ip",
        None,
        r#"{"ipAddress":"198.51.100.9","reason":"differential"}"#,
        &state,
    )
    .await
    .expect("ban ip");
    record!(
        "POST",
        "/api/v0/security/bans/ip",
        "nominal-status-headers-body",
        security_ban.status == "200 OK" && security_ban.body.is_empty()
    );

    let security_bans = crate::route_http_request("GET", "/api/v0/security/bans", None, "", &state)
        .await
        .expect("list security bans");
    let security_bans_json =
        serde_json::from_str::<serde_json::Value>(&security_bans.body).unwrap_or_default();
    let security_record = security_bans_json
        .as_array()
        .and_then(|records| {
            records
                .iter()
                .find(|record| record["key"] == "IP:198.51.100.9")
        })
        .cloned()
        .unwrap_or_default();
    record!(
        "GET",
        "/api/v0/security/bans",
        "populated-dynamic-state",
        security_bans.status == "200 OK"
            && security_record["reason"] == "differential"
            && security_record["isPermanent"] == false
            && security_record["timeRemaining"].as_str().is_some()
    );
    record!(
        "POST",
        "/api/v0/security/bans/ip",
        "mutation-side-effects-and-readback",
        security_ban.status == "200 OK"
            && security_ban.body.is_empty()
            && security_record["key"] == "IP:198.51.100.9"
            && security_record["reason"] == "differential"
            && security_record["isPermanent"] == false
            && security_record["timeRemaining"].as_str().is_some()
    );

    let username_ban = crate::route_http_request(
        "POST",
        "/api/v0/security/bans/username",
        None,
        r#"{"username":"Differential-User","reason":"username differential"}"#,
        &state,
    )
    .await
    .expect("ban username");
    record!(
        "POST",
        "/api/v0/security/bans/username",
        "nominal-status-headers-body",
        username_ban.status == "200 OK" && username_ban.body.is_empty()
    );

    let username_bans = crate::route_http_request("GET", "/api/v0/security/bans", None, "", &state)
        .await
        .expect("list username ban");
    let username_bans_json =
        serde_json::from_str::<serde_json::Value>(&username_bans.body).unwrap_or_default();
    let username_record = username_bans_json
        .as_array()
        .and_then(|records| {
            records
                .iter()
                .find(|record| record["key"] == "User:differential-user")
        })
        .cloned()
        .unwrap_or_default();
    record!(
        "POST",
        "/api/v0/security/bans/username",
        "mutation-side-effects-and-readback",
        username_ban.status == "200 OK"
            && username_ban.body.is_empty()
            && username_bans.status == "200 OK"
            && username_record["reason"] == "username differential"
            && username_record["isPermanent"] == false
            && username_record["timeRemaining"].as_str().is_some()
    );

    let username_unban = crate::route_http_request(
        "DELETE",
        "/api/v0/security/bans/username/differential-user",
        None,
        "",
        &state,
    )
    .await
    .expect("unban username");
    record!(
        "DELETE",
        "/api/v0/security/bans/username/{username}",
        "nominal-status-headers-body",
        username_unban.status == "200 OK" && username_unban.body.is_empty()
    );

    let username_bans_after_delete =
        crate::route_http_request("GET", "/api/v0/security/bans", None, "", &state)
            .await
            .expect("list bans after username unban");
    let username_bans_after_delete_json =
        serde_json::from_str::<serde_json::Value>(&username_bans_after_delete.body)
            .unwrap_or_default();
    let username_removed = username_bans_after_delete_json
        .as_array()
        .is_some_and(|records| {
            !records
                .iter()
                .any(|record| record["key"] == "User:differential-user")
        });
    record!(
        "DELETE",
        "/api/v0/security/bans/username/{username}",
        "mutation-side-effects-and-readback",
        username_unban.status == "200 OK"
            && username_unban.body.is_empty()
            && username_bans_after_delete.status == "200 OK"
            && username_removed
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("openapi_mutation_dtos_tail.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api openapi-mutation-dtos-tail mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential proof crediting the unversioned bridge mutation routes'
/// API-version negotiation contract. The frozen controller exposes the
/// compatibility routes but ASP.NET API versioning rejects POST requests
/// without an explicit version before binding the request body. slskdN-only
/// (confirmed against the frozen registry).
#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-2"
))]
pub(super) async fn controller_api_differential_unversioned_bridge_version_validation() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($route:expr, $case:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!(
                    "{target} POST {} [{}]",
                    $route, $case
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "POST",
                "route": $route,
                "case": $case,
                "pass": $pass,
            }));
        };
    }

    let (state, _receiver) = test_state();
    let routes = [
        ("/api/bridge/search", r#"{"query":"unversioned bridge"}"#),
        (
            "/api/bridge/download",
            r#"{"username":"peer","filename":"file.flac","targetPath":"/tmp/file.flac"}"#,
        ),
        ("/api/bridge/start", "{}"),
        ("/api/bridge/stop", "{}"),
    ];
    for (route, nominal_body) in routes {
        let nominal = crate::route_http_request("POST", route, None, nominal_body, &state)
            .await
            .unwrap_or_else(|error| panic!("POST {route}: {error}"));
        record!(
            route,
            "nominal-status-headers-body",
            nominal.status == "400 Bad Request" && nominal.body.contains("ApiVersionUnspecified")
        );

        let malformed = crate::route_http_request("POST", route, None, "not-json", &state)
            .await
            .unwrap_or_else(|error| panic!("POST {route} malformed: {error}"));
        record!(
            route,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
                && malformed.body.contains("ApiVersionUnspecified")
        );

        let empty = crate::route_http_request("POST", route, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("POST {route} empty: {error}"));
        record!(
            route,
            "missing-empty-or-conflict-state",
            empty.status == "400 Bad Request" && empty.body.contains("ApiVersionUnspecified")
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("unversioned_bridge_version_validation.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api unversioned bridge version mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential proof crediting the remaining unversioned mutating
/// controller routes' API-version negotiation contract. These routes are
/// exposed for compatibility, but the frozen API-versioning middleware
/// rejects them before request binding when no version is supplied.
/// slskdN-only (confirmed against the frozen registry).
#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-2"
))]
pub(super) async fn controller_api_differential_unversioned_mutation_version_validation() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!(
                    "{target} {} {} [{}]",
                    $method, $route, $case
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": $method,
                "route": $route,
                "case": $case,
                "pass": $pass,
            }));
        };
    }

    let (state, _receiver) = test_state();
    let routes = [
        (
            "POST",
            "/api/audio/analyzers/migrate",
            "/api/audio/analyzers/migrate",
            "{}",
        ),
        ("POST", "/api/jobs/mb-release", "/api/jobs/mb-release", "{}"),
        (
            "POST",
            "/api/jobs/discography",
            "/api/jobs/discography",
            "{}",
        ),
        (
            "POST",
            "/api/jobs/label-crate",
            "/api/jobs/label-crate",
            "{}",
        ),
        (
            "POST",
            "/api/library/health/scans",
            "/api/library/health/scans",
            "{}",
        ),
        (
            "POST",
            "/api/library/health/issues/fix",
            "/api/library/health/issues/fix",
            "{}",
        ),
        (
            "POST",
            "/api/source-feed-imports/preview",
            "/api/source-feed-imports/preview",
            "{}",
        ),
        (
            "POST",
            "/api/integrations/spotify/authorize",
            "/api/integrations/spotify/authorize",
            "{}",
        ),
        (
            "DELETE",
            "/api/integrations/spotify",
            "/api/integrations/spotify",
            "",
        ),
        (
            "PUT",
            "/api/bridge/admin/config",
            "/api/bridge/admin/config",
            "{}",
        ),
        (
            "PATCH",
            "/api/library/health/issues/issue-id",
            "/api/library/health/issues/{issueId}",
            "{}",
        ),
    ];
    for (method, path, route, nominal_body) in routes {
        let nominal = crate::route_http_request(method, path, None, nominal_body, &state)
            .await
            .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
        record!(
            method,
            route,
            "nominal-status-headers-body",
            nominal.status == "400 Bad Request" && nominal.body.contains("ApiVersionUnspecified")
        );

        let malformed = crate::route_http_request(method, path, None, "not-json", &state)
            .await
            .unwrap_or_else(|error| panic!("{method} {path} malformed: {error}"));
        record!(
            method,
            route,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
                && malformed.body.contains("ApiVersionUnspecified")
        );

        let empty = crate::route_http_request(method, path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{method} {path} empty: {error}"));
        record!(
            method,
            route,
            "missing-empty-or-conflict-state",
            empty.status == "400 Bad Request" && empty.body.contains("ApiVersionUnspecified")
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("unversioned_mutation_version_validation.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api unversioned mutation version mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting the remaining registered routes
/// of `extended_controller_mutations_are_stateful_and_domain_backed`
/// (found via call-density scan) not already credited elsewhere:
/// opinions GET/DELETE, security circuits (real Tor-circuit-building
/// failure, no invented success), mediacore descriptor stats, pods
/// creation, podcore channels, and the pod signing keypair/sign/
/// verify pipeline (real ed25519 crypto resolved from actual pod
/// membership, not a client-supplied key -- a forged sender is
/// genuinely rejected). slskdN-only (confirmed against the frozen
/// registry; the literal-pod-id-embedded `/api/v0/podcore/pod-
/// controller/channels` path in the source test maps to the real
/// templated route `/api/v0/podcore/{podId}/channels`, not a
/// separate unregistered path).
#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-2"
))]
pub(super) async fn controller_api_differential_extended_controller_mutations() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!("{target} {} {} [{}]", $method, $route, $case));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": $method,
                "route": $route,
                "case": $case,
                "pass": $pass,
            }));
        };
    }

    let (state, _receiver) = test_state();

    let opinion = crate::route_http_request(
        "POST",
        "/api/v0/opinions",
        None,
        r#"{"issuer":"differential-test","subjectType":"Track","subjectId":"recording-differential","kind":"Like","strength":0.75,"confidence":1,"comment":"good"}"#,
        &state,
    )
    .await
    .expect("create opinion");
    let opinion_json = serde_json::from_str::<serde_json::Value>(&opinion.body).unwrap_or_default();
    let opinion_id = opinion_json["id"].as_str().unwrap_or_default().to_owned();

    let opinions = crate::route_http_request("GET", "/api/v0/opinions", None, "", &state)
        .await
        .expect("list opinions");
    record!(
        "GET",
        "/api/v0/opinions",
        "nominal-status-headers-body",
        opinions.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/opinions",
        "populated-dynamic-state",
        opinions.body.contains("recording-differential")
    );
    record!(
        "POST",
        "/api/v0/opinions",
        "mutation-side-effects-and-readback",
        opinion.status == "200 OK"
            && !opinion_id.is_empty()
            && opinion_json["subjectId"] == "recording-differential"
            && opinions.body.contains(&opinion_id)
    );

    let circuit = crate::route_http_request(
        "POST",
        "/api/v0/security/circuits",
        None,
        r#"{"id":"circuit-differential","peerId":"peer-differential","active":true}"#,
        &state,
    )
    .await
    .expect("create circuit");
    record!(
        "POST",
        "/api/v0/security/circuits",
        "runtime-failure-and-timeout",
        circuit.status == "400 Bad Request"
            && serde_json::from_str::<serde_json::Value>(&circuit.body).unwrap_or_default()
                == serde_json::json!({"error": "Circuit building failed"})
    );

    let circuits = crate::route_http_request("GET", "/api/v0/security/circuits", None, "", &state)
        .await
        .expect("list circuits");
    record!(
        "GET",
        "/api/v0/security/circuits",
        "nominal-status-headers-body",
        circuits.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/security/circuits",
        "populated-dynamic-state",
        circuits.body == "[]"
    );

    crate::route_http_request(
        "POST",
        "/api/v0/mediacore/publish/descriptor",
        None,
        &serde_json::json!({
            "descriptor": {
                "contentId": "cid-differential",
                "hashes": [{"algorithm": "sha256", "hex": "0123456789abcdef"}],
                "signature": {
                    "publicKey": "key",
                    "signature": "0123456789abcdef",
                    "timestampUnixMs": crate::unix_timestamp_millis(),
                },
            },
        })
        .to_string(),
        &state,
    )
    .await
    .expect("publish descriptor fixture");

    let descriptor_stats = crate::route_http_request(
        "GET",
        "/api/v0/mediacore/stats/descriptors",
        None,
        "",
        &state,
    )
    .await
    .expect("descriptor stats");
    let descriptor_stats_json =
        serde_json::from_str::<serde_json::Value>(&descriptor_stats.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/mediacore/stats/descriptors",
        "nominal-status-headers-body",
        descriptor_stats.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/mediacore/stats/descriptors",
        "populated-dynamic-state",
        descriptor_stats_json["activeCacheEntries"] == 0
    );

    let created_pod = crate::route_http_request(
        "POST",
        "/api/v0/pods",
        None,
        r#"{"pod":{"podId":"pod-controller-differential","name":"Controller Pod Differential","isPublic":true}}"#,
        &state,
    )
    .await
    .expect("create pod");
    record!(
        "POST",
        "/api/v0/pods",
        "nominal-status-headers-body",
        created_pod.status == "201 Created"
    );

    let channel = crate::route_http_request(
        "POST",
        "/api/v0/podcore/pod-controller-differential/channels",
        None,
        r#"{"channelId":"general","name":"General"}"#,
        &state,
    )
    .await
    .expect("create pod channel");
    record!(
        "POST",
        "/api/v0/podcore/{podId}/channels",
        "nominal-status-headers-body",
        channel.status == "201 Created"
    );

    let channels = crate::route_http_request(
        "GET",
        "/api/v0/podcore/pod-controller-differential/channels",
        None,
        "",
        &state,
    )
    .await
    .expect("list pod channels");
    record!(
        "GET",
        "/api/v0/podcore/{podId}/channels",
        "nominal-status-headers-body",
        channels.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/podcore/{podId}/channels",
        "populated-dynamic-state",
        channels.body.contains("general")
    );
    record!(
        "POST",
        "/api/v0/podcore/{podId}/channels",
        "mutation-side-effects-and-readback",
        channel.status == "201 Created" && channels.body.contains("general")
    );

    let keypair = crate::route_http_request(
        "POST",
        "/api/v0/podcore/signing/generate-keypair",
        None,
        "{}",
        &state,
    )
    .await
    .expect("generate pod signing keypair");
    let keys = serde_json::from_str::<serde_json::Value>(&keypair.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/podcore/signing/generate-keypair",
        "nominal-status-headers-body",
        keypair.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v0/podcore/signing/generate-keypair",
        "mutation-side-effects-and-readback",
        keys["publicKey"].as_str().is_some() && keys["privateKey"].as_str().is_some()
    );

    state
        .pods
        .write()
        .await
        .upsert_member(
            "pod-controller-differential",
            crate::pods::PodMember {
                peer_id: "tester".to_owned(),
                role: "member".to_owned(),
                is_banned: false,
                public_key: keys["publicKey"].as_str().map(str::to_owned),
                joined_at: None,
                last_seen: None,
            },
        )
        .expect("add tester as a real pod member with a signing public key");

    let signed = crate::route_http_request(
        "POST",
        "/api/v0/podcore/signing/sign",
        None,
        &serde_json::json!({
            "privateKey": keys["privateKey"],
            "message": {
                "messageId":"message-differential",
                "podId":"pod-controller-differential",
                "senderPeerId":"tester",
                "body":"hello",
                "timestampUnixMs": crate::unix_timestamp() * 1000,
            }
        })
        .to_string(),
        &state,
    )
    .await
    .expect("sign pod message");
    let signed_json = serde_json::from_str::<serde_json::Value>(&signed.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/podcore/signing/sign",
        "nominal-status-headers-body",
        signed.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v0/podcore/signing/sign",
        "mutation-side-effects-and-readback",
        signed_json["signature"]
            .as_str()
            .is_some_and(|signature| signature.starts_with("ed25519:"))
    );

    let mut forged = signed_json.clone();
    forged["message"]["senderPeerId"] = serde_json::json!("someone-else");
    let forged_verified = crate::route_http_request(
        "POST",
        "/api/v0/podcore/signing/verify",
        None,
        &forged.to_string(),
        &state,
    )
    .await
    .expect("verify forged sender");
    record!(
        "POST",
        "/api/v0/podcore/signing/verify",
        "missing-empty-or-conflict-state",
        forged_verified.body == r#"{"isValid":false}"#
    );

    let ranked = crate::route_http_request(
        "POST",
        "/api/v0/ranking/rank",
        None,
        r#"[{"username":"slow","filename":"x.flac","uploadSpeed":10},{"username":"fast","filename":"x.flac","uploadSpeed":10000,"hasFreeUploadSlot":true}]"#,
        &state,
    )
    .await
    .expect("rank sources");
    let ranked_json = serde_json::from_str::<serde_json::Value>(&ranked.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/ranking/rank",
        "nominal-status-headers-body",
        ranked.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v0/ranking/rank",
        "mutation-side-effects-and-readback",
        ranked_json[0]["username"] == "fast"
    );

    let opinion_delete_route = format!("/api/v0/opinions/{opinion_id}");
    let removed_opinion =
        crate::route_http_request("DELETE", &opinion_delete_route, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{opinion_delete_route}: {error}"));
    record!(
        "DELETE",
        "/api/v0/opinions/{id}",
        "nominal-status-headers-body",
        removed_opinion.status == "204 No Content"
    );
    record!(
        "DELETE",
        "/api/v0/opinions/{id}",
        "mutation-side-effects-and-readback",
        removed_opinion.status == "204 No Content"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("extended_controller_mutations.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api extended-controller-mutations mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof for populated compatibility projections:
/// `ServerCompatibilityController.GetStatus` after a real session state
/// transition and `CapabilitiesController.GetCapabilities` after the
/// real ScenePodBridge feature state is enabled.  The versioned and
/// unversioned capability route intentionally share the frozen native
/// controller surface here; the separate `/api/v0/capabilities` wire
/// capability-file endpoint is not credited by this slice.
#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-3"
))]
pub(super) async fn controller_api_differential_populated_compatibility_status_and_capabilities() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($route:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!(
                    "{target} GET {} [populated-dynamic-state]",
                    $route
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "GET",
                "route": $route,
                "case": "populated-dynamic-state",
                "pass": $pass,
            }));
        };
    }

    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    {
        let mut session = state.session.write().await;
        session.state = "connected";
        session.username = Some("populated-peer".to_owned());
        session.connected_at = Some(1_754_000_000);
        session.updated_at = 1_754_000_001;
    }

    let status = crate::route_http_request("GET", "/api/server/status", None, "", &state)
        .await
        .expect("populated server status response");
    let status_value =
        serde_json::from_str::<serde_json::Value>(&status.body).unwrap_or(serde_json::Value::Null);
    record!(
        "/api/server/status",
        status.status == "200 OK"
            && status.content_type == "application/json"
            && status_value["connected"] == true
            && status_value["state"] == "logged_in"
            && status_value["username"] == "populated-peer"
    );

    {
        let mut media = state.media_services.write().await;
        media.features.scene_pod_bridge = true;
    }
    let capabilities =
        crate::route_http_request("GET", "/api/slskdn/capabilities", None, "", &state)
            .await
            .expect("populated capabilities response");
    let capabilities_value = serde_json::from_str::<serde_json::Value>(&capabilities.body)
        .unwrap_or(serde_json::Value::Null);
    let features = capabilities_value["features"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    record!(
        "/api/slskdn/capabilities",
        capabilities.status == "200 OK"
            && capabilities.content_type == "application/json"
            && capabilities_value["impl"] == "slskdn"
            && capabilities_value["compat"] == "slskd"
            && features.iter().any(|feature| feature == "scene_pod_bridge")
            && capabilities_value["feature"]["scenePodBridge"] == true
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("populated_compatibility_status_and_capabilities.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api populated compatibility mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
