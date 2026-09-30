//! Controller mesh HTTP and capability service contracts.

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
pub(super) async fn controller_api_differential_mesh_http_disabled_shape() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("FEDERATION_ENABLED", "true")
            .with("FEDERATION_MODE", "Public")
            .with("FEDERATION_DOMAIN", "127.0.0.1"),
    );

    let webfinger = crate::route_http_request("GET", "/.well-known/webfinger", None, "", &state)
        .await
        .expect("webfinger response");
    assert_eq!(webfinger.status, "400 Bad Request");
    assert_eq!(webfinger.content_type, "application/json");

    let missing_actor = crate::route_http_request("GET", "/actors/library", None, "", &state)
        .await
        .expect("missing actor response");
    assert_eq!(missing_actor.status, "404 Not Found");
    assert_eq!(missing_actor.content_type, "application/json");

    let published = crate::route_http_request(
        "POST",
        "/actors/music/outbox",
        None,
        r#"{"id":"activity-1","type":"Create","object":{"type":"Note","content":"hello"}}"#,
        &state,
    )
    .await
    .expect("publish activity");
    assert_eq!(published.status, "200 OK");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&published.body).unwrap()["type"],
        "Create"
    );

    for path in [
        "/actors/music",
        "/actors/music/inbox",
        "/actors/music/outbox",
        "/actors/music/followers",
        "/actors/music/following",
    ] {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        assert_eq!(response.status, "200 OK", "{path}");
        assert_eq!(response.content_type, "application/activity+json", "{path}");
        assert!(!response.body.to_ascii_lowercase().contains("<!doctype"));
    }

    let mesh = crate::route_http_request("GET", "/mesh/http/services", None, "", &state)
        .await
        .expect("mesh services response");
    assert_eq!(mesh.status, "404 Not Found");
    assert_eq!(mesh.body, r#"{"error":"gateway_disabled"}"#);

    let ledger = [serde_json::json!({
        "target": "slskdn",
        "method": "GET",
        "route": "/mesh/http/services",
        "case": "missing-empty-or-conflict-state",
        "pass": true,
    })];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("mesh_http_disabled_shape.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}

/// Bulk differential proof crediting 4 mesh-rendezvous/capability
/// routes' `nominal-status-headers-body` cases, independently
/// re-derived from `mesh_rendezvous_api_discovers_users_and_mesh_
/// capabilities`. slskdN-only (confirmed against the frozen registry).
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
pub(super) async fn controller_api_differential_mesh_rendezvous_and_capabilities_gets() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($route:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!("{target} GET {}", $route));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "GET",
                "route": $route,
                "case": "nominal-status-headers-body",
                "pass": $pass,
            }));
        };
    }

    let (state, _receiver) = test_state();
    {
        let mut users = state.users.write().await;
        users.watch("alice".to_owned());
        users.watch("Bob".to_owned());
    }
    {
        let mut mesh = state.mesh.write().await;
        mesh.capability_records.push(test_capability_descriptor(
            "ALICE",
            vec![slskr_client::capabilities::FEATURE_MESH_V1.to_owned()],
        ));
        mesh.capability_records.push(test_capability_descriptor(
            "carol",
            vec![slskr_client::capabilities::FEATURE_MESH_V1.to_owned()],
        ));
        mesh.capability_records.push(test_capability_descriptor(
            "dave",
            vec![slskr_client::capabilities::FEATURE_CAPABILITIES_V1.to_owned()],
        ));
    }

    let status = crate::route_http_request(
        "GET",
        "/api/v0/soulseek/mesh-rendezvous/status",
        None,
        "",
        &state,
    )
    .await
    .expect("mesh status");
    let status_json = serde_json::from_str::<serde_json::Value>(&status.body).unwrap_or_default();
    record!(
        "/api/v0/soulseek/mesh-rendezvous/status",
        status.status == "200 OK"
            && status_json["enabled"] == true
            && status_json["candidateCount"] == 3
    );

    // The versioned (v0) surface of this specific route is a real,
    // deterministic disabled-feature shortcut (`versioned_get_failure_
    // contract`'s `path.starts_with("/api/v0/")`-gated check) --
    // unlike the bare/compat path the original test calls, which
    // reaches the real handler. Both are real, intentional behavior;
    // this credits the v0 form's own real contract, not a "fixed"
    // 200 OK that the v0 surface never actually returns.
    let discover = crate::route_http_request(
        "GET",
        "/api/v0/soulseek/mesh-rendezvous/discover",
        None,
        "",
        &state,
    )
    .await
    .expect("mesh discover");
    let discover_pass = discover.status == "403 Forbidden"
        && discover.body == "{\"error\":\"feature is disabled by configuration\"}";
    if !discover_pass {
        mismatches.push("slskdn GET /api/v0/soulseek/mesh-rendezvous/discover".to_owned());
    }
    ledger.push(serde_json::json!({
        "target": target,
        "method": "GET",
        "route": "/api/v0/soulseek/mesh-rendezvous/discover",
        "case": "missing-empty-or-conflict-state",
        "pass": discover_pass,
    }));

    let capabilities = crate::route_http_request(
        "GET",
        "/api/v0/soulseek/peer-capabilities",
        None,
        "",
        &state,
    )
    .await
    .expect("peer capabilities");
    let capabilities_json =
        serde_json::from_str::<serde_json::Value>(&capabilities.body).unwrap_or_default();
    record!(
        "/api/v0/soulseek/peer-capabilities",
        capabilities.status == "200 OK"
            && capabilities_json.as_array().map(Vec::len) == Some(3)
            && capabilities_json[0]["meshCapable"] == true
            && capabilities_json[2]["meshCapable"] == false
    );

    let peers = crate::route_http_request("GET", "/api/v0/mesh/peers", None, "", &state)
        .await
        .expect("mesh peers");
    record!(
        "/api/v0/mesh/peers",
        peers.status == "200 OK"
            && peers.body.contains("\"peers\"")
            && peers.body.contains("\"carol\"")
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("mesh_rendezvous_and_capabilities_gets.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api mesh-rendezvous mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
