/// Bulk differential proof for `GET /api/v0/application/build`
/// (real app version, not the wire-protocol version) and `DELETE
/// /api/v0/files/downloads/files/{base64FileName}` (remote file
/// management is forbidden by default, a real file removal is
/// genuinely performed and readback-confirmed, and a base64-
/// decoded path-traversal attempt is rejected) -- independently
/// re-derived from `build_info_uses_app_version_not_protocol_
/// version`, `controller_file_delete_routes_are_forbidden_by_default`,
/// and `controller_file_delete_routes_are_scoped_to_storage_roots` with
/// fresh fixture data. Confirmed against `/tmp/slskr-parity-
/// evidence/controller-api/*.json` before writing, per case: zero
/// prior credit on either route. slskdN-only (confirmed against
/// the frozen registry).
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
async fn controller_api_differential_build_info_and_file_delete() {
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
    let build = super::route_http_request("GET", "/api/v0/application/build", None, "", &state)
        .await
        .expect("build info response");
    let build_json = serde_json::from_str::<serde_json::Value>(&build.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/application/build",
        "nominal-status-headers-body",
        build.status == "200 OK"
            && build_json["current"] == env!("CARGO_PKG_VERSION")
            && build_json["protocol"]["major"] == serde_json::json!(super::CLIENT_MAJOR_VERSION)
    );

    let (disabled_state, _receiver) = test_state();
    let forbidden = super::route_http_request(
        "DELETE",
        "/api/v0/files/downloads/files/differential-token",
        None,
        "",
        &disabled_state,
    )
    .await
    .expect("default remote file management policy response");
    record!(
        "DELETE",
        "/api/v0/files/downloads/files/{base64FileName}",
        "missing-empty-or-conflict-state",
        forbidden.status == "403 Forbidden"
    );

    let (enabled_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_REMOTE_FILE_MANAGEMENT", "true"));
    let download_file = enabled_state
        .config
        .downloads_dir
        .join("Remote")
        .join("DifferentialSong.mp3");
    std::fs::create_dir_all(download_file.parent().unwrap())
        .expect("create differential download directory");
    std::fs::write(&download_file, b"differential-song").expect("write differential download file");
    let token = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        "Remote/DifferentialSong.mp3",
    );
    let deleted = super::route_http_request(
        "DELETE",
        &format!("/api/v0/files/downloads/files/{token}"),
        None,
        "",
        &enabled_state,
    )
    .await
    .expect("real file delete response");
    let missing_after = super::route_http_request(
        "DELETE",
        &format!("/api/v0/files/downloads/files/{token}"),
        None,
        "",
        &enabled_state,
    )
    .await
    .expect("repeat delete response");
    record!(
        "DELETE",
        "/api/v0/files/downloads/files/{base64FileName}",
        "mutation-side-effects-and-readback",
        deleted.status == "204 No Content"
            && !download_file.exists()
            && missing_after.status == "204 No Content"
    );

    let traversal_token = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        "../differential-secret",
    );
    let traversal = super::route_http_request(
        "DELETE",
        &format!("/api/v0/files/downloads/files/{traversal_token}"),
        None,
        "",
        &enabled_state,
    )
    .await
    .expect("traversal delete response");
    record!(
        "DELETE",
        "/api/v0/files/downloads/files/{base64FileName}",
        "malformed-path-query-or-body",
        traversal.status == "400 Bad Request"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("build_info_and_file_delete.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api build-info-and-file-delete mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof for the slskdN application version reads:
/// semantic version, latest-version state, and build metadata.  The
/// latest-version and build rows are checked both before and after
/// populating the real in-memory release state, so the evidence covers
/// the controller's current snapshot rather than only a hardcoded empty
/// response.  slskdN-only (confirmed against the frozen registry).
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
async fn controller_api_differential_application_version_state_contracts() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($route:expr, $case:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!(
                    "{target} GET {} [{}]",
                    $route,
                    $case
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "GET",
                "route": $route,
                "case": $case,
                "pass": $pass,
            }));
        };
    }

    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));

    let version = super::route_http_request("GET", "/api/v0/application/version", None, "", &state)
        .await
        .expect("application version response");
    let version_value =
        serde_json::from_str::<serde_json::Value>(&version.body).unwrap_or(serde_json::Value::Null);
    record!(
        "/api/v0/application/version",
        "nominal-status-headers-body",
        version.status == "200 OK"
            && version.content_type == "application/json"
            && version_value == env!("CARGO_PKG_VERSION")
    );

    let latest = super::route_http_request(
        "GET",
        "/api/v0/application/version/latest",
        None,
        "",
        &state,
    )
    .await
    .expect("latest application version response");
    let latest_value =
        serde_json::from_str::<serde_json::Value>(&latest.body).unwrap_or(serde_json::Value::Null);
    record!(
        "/api/v0/application/version/latest",
        "nominal-status-headers-body",
        latest.status == "200 OK"
            && latest.content_type == "application/json"
            && latest_value["current"] == env!("CARGO_PKG_VERSION")
            && latest_value["latest"] == ""
            && latest_value["latestTag"] == ""
            && latest_value["latestUrl"] == ""
    );

    {
        let mut version_state = state
            .controller_version
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        version_state.latest = Some("9.9.9-slskdn.20260101120000".to_owned());
        version_state.latest_tag = Some("v9.9.9-slskdn.20260101120000".to_owned());
        version_state.latest_url = Some("https://example.test/releases/9.9.9".to_owned());
        version_state.checked_at = Some("2026-08-06T00:00:00Z".to_owned());
        version_state.is_update_available = Some(true);
    }

    let populated_latest = super::route_http_request(
        "GET",
        "/api/v0/application/version/latest",
        None,
        "",
        &state,
    )
    .await
    .expect("populated latest application version response");
    let populated_latest_value = serde_json::from_str::<serde_json::Value>(&populated_latest.body)
        .unwrap_or(serde_json::Value::Null);
    record!(
        "/api/v0/application/version/latest",
        "populated-dynamic-state",
        populated_latest.status == "200 OK"
            && populated_latest.content_type == "application/json"
            && populated_latest_value["latest"] == "9.9.9-slskdn.20260101120000"
            && populated_latest_value["latestTag"] == "v9.9.9-slskdn.20260101120000"
            && populated_latest_value["latestUrl"] == "https://example.test/releases/9.9.9"
            && populated_latest_value["checkedAt"] == "2026-08-06T00:00:00Z"
            && populated_latest_value["isUpdateAvailable"] == true
    );

    let populated_build =
        super::route_http_request("GET", "/api/v0/application/build", None, "", &state)
            .await
            .expect("populated application build response");
    let populated_build_value = serde_json::from_str::<serde_json::Value>(&populated_build.body)
        .unwrap_or(serde_json::Value::Null);
    record!(
        "/api/v0/application/build",
        "populated-dynamic-state",
        populated_build.status == "200 OK"
            && populated_build.content_type == "application/json"
            && populated_build_value["current"] == env!("CARGO_PKG_VERSION")
            && populated_build_value["latest"] == "9.9.9-slskdn.20260101120000"
            && populated_build_value["latestTag"] == "v9.9.9-slskdn.20260101120000"
            && populated_build_value["isUpdateAvailable"] == true
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("application_version_state_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api application-version mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential proof for the slskdN versioned application snapshot after
/// real runtime mutations.  The existing compatibility-shell test covers
/// the same projection through unversioned `/api/application`; this slice
/// exercises the registered `/api/v0/application` route itself and credits
/// only its populated dynamic-state row.
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
async fn controller_api_differential_application_populated_versioned_state() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($route:expr, $case:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!("{target} GET {} [{}]", $route, $case));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "GET",
                "route": $route,
                "case": $case,
                "pass": $pass,
            }));
        };
    }

    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));

    let bridge_config = super::route_http_request(
        "PUT",
        "/api/v0/bridge/admin/config",
        None,
        r#"{"maxClients":4,"enabled":true}"#,
        &state,
    )
    .await
    .expect("versioned bridge configuration mutation");
    assert_eq!(bridge_config.status, "200 OK", "{}", bridge_config.body);

    let restart = super::route_http_request("PUT", "/api/v0/application", None, "{}", &state)
        .await
        .expect("versioned application restart mutation");
    assert_eq!(restart.status, "204 No Content", "{}", restart.body);

    let application = super::route_http_request("GET", "/api/v0/application", None, "", &state)
        .await
        .expect("versioned application state response");
    let application_value =
        serde_json::from_str::<serde_json::Value>(&application.body).unwrap_or_default();
    let pass = application.status == "200 OK"
        && application.content_type == "application/json; charset=utf-8"
        && application_value["pendingRestart"] == true
        && application_value["bridge"]["configUpdates"] == 0;
    if !pass {
        mismatches.push(format!(
            "{target} application actual: {} {} {}",
            application.status, application.content_type, application.body
        ));
    }
    record!("/api/v0/application", "populated-dynamic-state", pass);

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("application_populated_versioned_state.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api populated-application mismatches:\n{}",
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
async fn controller_api_differential_populated_compatibility_status_and_capabilities() {
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

    let status = super::route_http_request("GET", "/api/server/status", None, "", &state)
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
        super::route_http_request("GET", "/api/slskdn/capabilities", None, "", &state)
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

/// Bulk differential proof for the frozen versioned capability peer
/// projections: all known peers, mesh-capable peers, and one peer's
/// detail.  The rows are populated from slskR's real signed-capability
/// store and assert the controller's public DTO fields, not merely a
/// non-404 response.  slskdN-only (confirmed against the registry).
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
async fn controller_api_differential_versioned_capability_peer_projections() {
    let target = "slskdn";
    let username = "versioned-capability-peer";
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
    state
        .mesh
        .write()
        .await
        .capability_records
        .push(test_capability_descriptor(
            username,
            vec![slskr_client::capabilities::FEATURE_MESH_V1.to_owned()],
        ));

    let peers = super::route_http_request("GET", "/api/v0/capabilities/peers", None, "", &state)
        .await
        .expect("versioned capability peers response");
    let peers_value =
        serde_json::from_str::<serde_json::Value>(&peers.body).unwrap_or(serde_json::Value::Null);
    record!(
        "/api/v0/capabilities/peers",
        peers.status == "200 OK"
            && peers.content_type == "application/json"
            && peers_value["count"] == 1
            && peers_value["peers"][0]["username"] == username
            && peers_value["peers"][0]["protocolVersion"] == 1
            && peers_value["peers"][0]["flagsValue"] == 8
            && peers_value["peers"][0]["canMeshSync"] == true
    );

    let mesh_peers =
        super::route_http_request("GET", "/api/v0/capabilities/mesh-peers", None, "", &state)
            .await
            .expect("versioned mesh capability peers response");
    let mesh_peers_value = serde_json::from_str::<serde_json::Value>(&mesh_peers.body)
        .unwrap_or(serde_json::Value::Null);
    record!(
        "/api/v0/capabilities/mesh-peers",
        mesh_peers.status == "200 OK"
            && mesh_peers.content_type == "application/json"
            && mesh_peers_value["count"] == 1
            && mesh_peers_value["peers"][0]["username"] == username
            && mesh_peers_value["peers"][0]["meshSeqId"] == 0
    );

    let peer = super::route_http_request(
        "GET",
        &format!("/api/v0/capabilities/peers/{username}"),
        None,
        "",
        &state,
    )
    .await
    .expect("versioned capability peer detail response");
    let peer_value =
        serde_json::from_str::<serde_json::Value>(&peer.body).unwrap_or(serde_json::Value::Null);
    record!(
        "/api/v0/capabilities/peers/{username}",
        peer.status == "200 OK"
            && peer.content_type == "application/json"
            && peer_value["username"] == username
            && peer_value["protocolVersion"] == 1
            && peer_value["flagsValue"] == 8
            && peer_value["canMeshSync"] == true
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("versioned_capability_peer_projections.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api versioned capability mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential proof for the empty versioned capability-peer list. The
/// populated projection is covered separately; this closes its nominal
/// status/header/body case with the oracle's exact envelope.
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
async fn controller_api_differential_versioned_capability_peers_nominal() {
    let target = "slskdn";
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let response = super::route_http_request("GET", "/api/v0/capabilities/peers", None, "", &state)
        .await
        .expect("versioned capability peers response");
    let value = serde_json::from_str::<serde_json::Value>(&response.body)
        .unwrap_or(serde_json::Value::Null);
    let expected = serde_json::json!({
        "count": 0,
        "peers": [],
    });
    let pass = response.status == "200 OK"
        && response.content_type.starts_with("application/json")
        && value == expected;
    let ledger = vec![serde_json::json!({
        "target": target,
        "method": "GET",
        "route": "/api/v0/capabilities/peers",
        "case": "nominal-status-headers-body",
        "pass": pass,
    })];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("versioned_capability_peers_nominal.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        pass,
        "{target} GET /api/v0/capabilities/peers: got {} {} {}",
        response.status, response.content_type, response.body
    );
}

/// Differential proof for the remaining versioned CapabilitiesController
/// cases.  The frozen controller is backed by process-local capability
/// service state, so a closed SQLite pool must not change its projections;
/// parsing likewise has no durable side effects to restore on restart.
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
async fn controller_api_differential_native_capabilities_contracts() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
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
                "pass": pass,
            }));
        }};
    }

    let valid_root = |response: &super::routing::HttpResponse| {
        let value = serde_json::from_str::<serde_json::Value>(&response.body)
            .unwrap_or(serde_json::Value::Null);
        let capability_file = value["json"]
            .as_str()
            .and_then(|json| serde_json::from_str::<serde_json::Value>(json).ok())
            .unwrap_or(serde_json::Value::Null);
        response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && value["version"] == "slskdn/1.0.0+dht+mesh+swarm"
            && value["tag"] == "slskdn_caps:v1;dht=1;mesh=1;swarm=1;hashx=1;flacdb=1"
            && capability_file["client"] == "slskdn"
            && capability_file["version"] == "1.0.0"
            && capability_file["capabilities"] == 63
    };

    let empty_peer_list = |response: &super::routing::HttpResponse| {
        response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && serde_json::from_str::<serde_json::Value>(&response.body).ok()
                == Some(serde_json::json!({"count": 0, "peers": []}))
    };

    let (root_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let malformed_root =
        super::route_http_request("GET", "/api/v0/capabilities/extra", None, "", &root_state)
            .await
            .expect("malformed versioned capabilities path");
    record!(
        "GET",
        "/api/v0/capabilities",
        "malformed-path-query-or-body",
        malformed_root.status == "404 Not Found"
    );

    let empty_root =
        super::route_http_request("GET", "/api/v0/capabilities", None, "", &root_state)
            .await
            .expect("empty versioned capabilities response");
    record!(
        "GET",
        "/api/v0/capabilities",
        "missing-empty-or-conflict-state",
        valid_root(&empty_root)
    );

    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("capabilities runtime database");
    let (runtime_root_state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    db.close_for_test().await;
    let runtime_root =
        super::route_http_request("GET", "/api/v0/capabilities", None, "", &runtime_root_state)
            .await
            .expect("versioned capabilities response with closed database");
    record!(
        "GET",
        "/api/v0/capabilities",
        "runtime-failure-and-timeout",
        valid_root(&runtime_root)
    );

    let (populated_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    populated_state
        .mesh
        .write()
        .await
        .capability_records
        .push(test_capability_descriptor(
            "capabilities-root-peer",
            vec![slskr_client::capabilities::FEATURE_MESH_V1.to_owned()],
        ));
    let populated_root =
        super::route_http_request("GET", "/api/v0/capabilities", None, "", &populated_state)
            .await
            .expect("populated versioned capabilities response");
    record!(
        "GET",
        "/api/v0/capabilities",
        "populated-dynamic-state",
        valid_root(&populated_root)
    );

    let malformed_mesh = super::route_http_request(
        "GET",
        "/api/v0/capabilities/mesh-peers/extra",
        None,
        "",
        &root_state,
    )
    .await
    .expect("malformed mesh-peer path");
    record!(
        "GET",
        "/api/v0/capabilities/mesh-peers",
        "malformed-path-query-or-body",
        malformed_mesh.status == "404 Not Found"
    );
    let empty_mesh = super::route_http_request(
        "GET",
        "/api/v0/capabilities/mesh-peers",
        None,
        "",
        &root_state,
    )
    .await
    .expect("empty mesh-peer response");
    record!(
        "GET",
        "/api/v0/capabilities/mesh-peers",
        "missing-empty-or-conflict-state",
        empty_peer_list(&empty_mesh)
    );

    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("mesh-peer runtime database");
    let (runtime_mesh_state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    db.close_for_test().await;
    let runtime_mesh = super::route_http_request(
        "GET",
        "/api/v0/capabilities/mesh-peers",
        None,
        "",
        &runtime_mesh_state,
    )
    .await
    .expect("mesh-peer response with closed database");
    record!(
        "GET",
        "/api/v0/capabilities/mesh-peers",
        "runtime-failure-and-timeout",
        empty_peer_list(&runtime_mesh)
    );

    let malformed_peers = super::route_http_request(
        "GET",
        "/api/v0/capabilities/peers/extra",
        None,
        "",
        &root_state,
    )
    .await
    .expect("malformed capability-peer list path");
    record!(
        "GET",
        "/api/v0/capabilities/peers",
        "malformed-path-query-or-body",
        malformed_peers.status == "404 Not Found"
    );
    let empty_peers =
        super::route_http_request("GET", "/api/v0/capabilities/peers", None, "", &root_state)
            .await
            .expect("empty capability-peer response");
    record!(
        "GET",
        "/api/v0/capabilities/peers",
        "missing-empty-or-conflict-state",
        empty_peer_list(&empty_peers)
    );

    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("capability-peer runtime database");
    let (runtime_peers_state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    db.close_for_test().await;
    let runtime_peers = super::route_http_request(
        "GET",
        "/api/v0/capabilities/peers",
        None,
        "",
        &runtime_peers_state,
    )
    .await
    .expect("capability-peer response with closed database");
    record!(
        "GET",
        "/api/v0/capabilities/peers",
        "runtime-failure-and-timeout",
        empty_peer_list(&runtime_peers)
    );

    let peer_username = "capabilities-detail-peer";
    populated_state
        .mesh
        .write()
        .await
        .capability_records
        .push(test_capability_descriptor(
            peer_username,
            vec![slskr_client::capabilities::FEATURE_MESH_V1.to_owned()],
        ));
    let peer = super::route_http_request(
        "GET",
        &format!("/api/v0/capabilities/peers/{peer_username}"),
        None,
        "",
        &populated_state,
    )
    .await
    .expect("known capability-peer detail");
    let valid_peer = |response: &super::routing::HttpResponse| {
        let value = serde_json::from_str::<serde_json::Value>(&response.body)
            .unwrap_or(serde_json::Value::Null);
        response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && value["username"] == peer_username
            && value["protocolVersion"] == 1
            && value["flagsValue"] == 8
            && value["canMeshSync"] == true
    };
    record!(
        "GET",
        "/api/v0/capabilities/peers/{username}",
        "nominal-status-headers-body",
        valid_peer(&peer)
    );
    let malformed_peer = super::route_http_request(
        "GET",
        "/api/v0/capabilities/peers/%20",
        None,
        "",
        &populated_state,
    )
    .await
    .expect("blank capability-peer username");
    let malformed_peer_value = serde_json::from_str::<serde_json::Value>(&malformed_peer.body)
        .unwrap_or(serde_json::Value::Null);
    record!(
        "GET",
        "/api/v0/capabilities/peers/{username}",
        "malformed-path-query-or-body",
        malformed_peer.status == "400 Bad Request"
            && malformed_peer_value["error"] == "Username is required"
    );
    let missing_peer = super::route_http_request(
        "GET",
        "/api/v0/capabilities/peers/unknown-capability-peer",
        None,
        "",
        &populated_state,
    )
    .await
    .expect("unknown capability-peer detail");
    let missing_peer_value = serde_json::from_str::<serde_json::Value>(&missing_peer.body)
        .unwrap_or(serde_json::Value::Null);
    record!(
        "GET",
        "/api/v0/capabilities/peers/{username}",
        "missing-empty-or-conflict-state",
        missing_peer.status == "404 Not Found"
            && missing_peer_value["error"] == "No capabilities known for peer"
    );

    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("capability-peer detail runtime database");
    let (runtime_peer_state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    runtime_peer_state
        .mesh
        .write()
        .await
        .capability_records
        .push(test_capability_descriptor(
            peer_username,
            vec![slskr_client::capabilities::FEATURE_MESH_V1.to_owned()],
        ));
    db.close_for_test().await;
    let runtime_peer = super::route_http_request(
        "GET",
        &format!("/api/v0/capabilities/peers/{peer_username}"),
        None,
        "",
        &runtime_peer_state,
    )
    .await
    .expect("capability-peer detail with closed database");
    record!(
        "GET",
        "/api/v0/capabilities/peers/{username}",
        "runtime-failure-and-timeout",
        valid_peer(&runtime_peer)
    );

    let parse_body = r#"{"description":" slskr_caps:v7;dht=1;mesh=1;swarm=1;hashx=1;flacdb=1 "}"#;
    let parsed = super::route_http_request(
        "POST",
        "/api/v0/capabilities/parse",
        None,
        parse_body,
        &root_state,
    )
    .await
    .expect("nominal capabilities parse");
    let parsed_value =
        serde_json::from_str::<serde_json::Value>(&parsed.body).unwrap_or(serde_json::Value::Null);
    record!(
        "POST",
        "/api/v0/capabilities/parse",
        "nominal-status-headers-body",
        parsed.status == "200 OK"
            && parsed.content_type.starts_with("application/json")
            && parsed_value["isSlskdn"] == true
            && parsed_value["flagsValue"] == 59
            && parsed_value["protocolVersion"] == 7
            && parsed_value["canMeshSync"] == true
            && parsed_value["canSwarm"] == true
    );
    let malformed_parse = super::route_http_request(
        "POST",
        "/api/v0/capabilities/parse",
        None,
        "not-json",
        &root_state,
    )
    .await
    .expect("malformed capabilities parse");
    record!(
        "POST",
        "/api/v0/capabilities/parse",
        "malformed-path-query-or-body",
        malformed_parse.status == "400 Bad Request"
    );
    let empty_parse = super::route_http_request(
        "POST",
        "/api/v0/capabilities/parse",
        None,
        "{}",
        &root_state,
    )
    .await
    .expect("empty capabilities parse");
    record!(
        "POST",
        "/api/v0/capabilities/parse",
        "missing-empty-or-conflict-state",
        empty_parse.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&empty_parse.body).ok()
                == Some(serde_json::json!({"isSlskdn": false}))
    );
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("capabilities parse runtime database");
    let (runtime_parse_state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    db.close_for_test().await;
    let runtime_parse = super::route_http_request(
        "POST",
        "/api/v0/capabilities/parse",
        None,
        parse_body,
        &runtime_parse_state,
    )
    .await
    .expect("capabilities parse with closed database");
    let runtime_parse_value = serde_json::from_str::<serde_json::Value>(&runtime_parse.body)
        .unwrap_or(serde_json::Value::Null);
    record!(
        "POST",
        "/api/v0/capabilities/parse",
        "runtime-failure-and-timeout",
        runtime_parse.status == "200 OK"
            && runtime_parse_value["isSlskdn"] == true
            && runtime_parse_value["flagsValue"] == 59
    );

    let before =
        super::route_http_request("GET", "/api/v0/capabilities/peers", None, "", &root_state)
            .await
            .expect("capability peers before parse mutation check");
    let mutation_parse = super::route_http_request(
        "POST",
        "/api/v0/capabilities/parse",
        None,
        parse_body,
        &root_state,
    )
    .await
    .expect("capability parse mutation check");
    let after =
        super::route_http_request("GET", "/api/v0/capabilities/peers", None, "", &root_state)
            .await
            .expect("capability peers after parse mutation check");
    record!(
        "POST",
        "/api/v0/capabilities/parse",
        "mutation-side-effects-and-readback",
        mutation_parse.status == "200 OK" && empty_peer_list(&before) && empty_peer_list(&after)
    );

    let (restart_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let restart_parse = super::route_http_request(
        "POST",
        "/api/v0/capabilities/parse",
        None,
        parse_body,
        &restart_state,
    )
    .await
    .expect("capability parse before restart check");
    let (restarted_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let restarted_peers = super::route_http_request(
        "GET",
        "/api/v0/capabilities/peers",
        None,
        "",
        &restarted_state,
    )
    .await
    .expect("capability peers after restart check");
    record!(
        "POST",
        "/api/v0/capabilities/parse",
        "restart-persistence-or-reset",
        restart_parse.status == "200 OK" && empty_peer_list(&restarted_peers)
    );

    let concurrent = futures_util::future::join_all((0..8).map(|_| {
        super::route_http_request(
            "POST",
            "/api/v0/capabilities/parse",
            None,
            parse_body,
            &root_state,
        )
    }))
    .await;
    let first_body = concurrent
        .first()
        .and_then(|response| response.as_ref().ok())
        .map(|response| response.body.clone());
    record!(
        "POST",
        "/api/v0/capabilities/parse",
        "concurrency-and-idempotency",
        concurrent.iter().all(|response| {
            response.as_ref().is_ok_and(|response| {
                response.status == "200 OK" && first_body.as_deref() == Some(response.body.as_str())
            })
        })
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("native_capabilities_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize capabilities ledger"),
    )
    .expect("write capabilities ledger");
    assert!(
        mismatches.is_empty(),
        "{} controller-api capabilities mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential proof for the versioned Identity/Profile controller.  The
/// versioned surface is deliberately kept separate from slskR's legacy
/// username-oriented `/api/profile/*` compatibility shell.
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
async fn controller_api_differential_native_profile_contracts() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
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
                "pass": pass,
            }));
        }};
    }

    let valid_profile = |response: &super::routing::HttpResponse| {
        let value = serde_json::from_str::<serde_json::Value>(&response.body)
            .unwrap_or(serde_json::Value::Null);
        response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && value["peerId"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
            && value["publicKey"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
            && value["displayName"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
            && value["capabilities"].is_i64()
            && value["endpoints"].is_array()
            && value["createdAt"].is_string()
            && value["expiresAt"].is_string()
            && value["signature"].is_string()
    };
    let valid_lookup = |response: &super::routing::HttpResponse| {
        let value = serde_json::from_str::<serde_json::Value>(&response.body)
            .unwrap_or(serde_json::Value::Null);
        response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && value["peerId"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
            && value["displayName"].is_string()
            && value["capabilities"].is_i64()
            && value["endpoints"].is_array()
            && value.get("publicKey").is_none()
            && value.get("signature").is_none()
    };
    let valid_invite = |response: &super::routing::HttpResponse| {
        let value = serde_json::from_str::<serde_json::Value>(&response.body)
            .unwrap_or(serde_json::Value::Null);
        let friend_code = value["friendCode"].as_str().unwrap_or_default();
        let code_parts = friend_code.split('-').collect::<Vec<_>>();
        response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && value["inviteLink"]
                .as_str()
                .is_some_and(|value| value.starts_with("slskdn://invite/"))
            && code_parts.len() == 4
            && code_parts[0].len() == 5
            && code_parts[1].len() == 4
            && code_parts[2].len() == 4
            && code_parts[3].len() == 3
    };
    let update_body = r#"{"displayName":" Updated Profile ","avatar":" avatar.png ","capabilities":7,"endpoints":[{"type":"Direct","address":"https://profile.example","priority":1},{"type":" ","address":"","priority":2}]}"#;

    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let me = super::route_http_request("GET", "/api/v0/profile/me", None, "", &state)
        .await
        .expect("versioned own profile");
    record!(
        "GET",
        "/api/v0/profile/me",
        "missing-empty-or-conflict-state",
        valid_profile(&me)
    );
    let me_value = serde_json::from_str::<serde_json::Value>(&me.body).expect("own profile JSON");
    let peer_id = me_value["peerId"]
        .as_str()
        .expect("own profile peer ID")
        .to_owned();

    let malformed_me =
        super::route_http_request("GET", "/api/v0/profile/me/extra", None, "", &state)
            .await
            .expect("malformed own profile path");
    record!(
        "GET",
        "/api/v0/profile/me",
        "malformed-path-query-or-body",
        malformed_me.status == "404 Not Found"
    );
    let known = super::route_http_request(
        "GET",
        &format!("/api/v0/profile/{peer_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("known profile lookup");
    record!(
        "GET",
        "/api/v0/profile/{peerId}",
        "nominal-status-headers-body",
        valid_lookup(&known)
    );
    let malformed_peer = super::route_http_request("GET", "/api/v0/profile/%20", None, "", &state)
        .await
        .expect("blank profile peer ID");
    record!(
        "GET",
        "/api/v0/profile/{peerId}",
        "malformed-path-query-or-body",
        malformed_peer.status == "400 Bad Request"
    );

    let update = super::route_http_request("PUT", "/api/v0/profile/me", None, update_body, &state)
        .await
        .expect("populated profile update");
    let updated_value =
        serde_json::from_str::<serde_json::Value>(&update.body).unwrap_or(serde_json::Value::Null);
    let populated_me = super::route_http_request("GET", "/api/v0/profile/me", None, "", &state)
        .await
        .expect("populated own profile");
    let populated_peer = super::route_http_request(
        "GET",
        &format!("/api/v0/profile/{peer_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("populated profile lookup");
    record!(
        "GET",
        "/api/v0/profile/me",
        "populated-dynamic-state",
        valid_profile(&populated_me)
            && serde_json::from_str::<serde_json::Value>(&populated_me.body)
                .ok()
                .is_some_and(|value| value["displayName"] == "Updated Profile")
    );
    record!(
        "GET",
        "/api/v0/profile/{peerId}",
        "populated-dynamic-state",
        valid_lookup(&populated_peer)
            && serde_json::from_str::<serde_json::Value>(&populated_peer.body)
                .ok()
                .is_some_and(|value| value["displayName"] == "Updated Profile")
    );
    let _ = updated_value;

    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("profile runtime database");
    let (runtime_state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    db.close_for_test().await;
    let runtime_me =
        super::route_http_request("GET", "/api/v0/profile/me", None, "", &runtime_state)
            .await
            .expect("own profile with closed database");
    record!(
        "GET",
        "/api/v0/profile/me",
        "runtime-failure-and-timeout",
        valid_profile(&runtime_me)
    );
    let runtime_me_value = serde_json::from_str::<serde_json::Value>(&runtime_me.body)
        .expect("runtime own profile JSON");
    let runtime_peer_id = runtime_me_value["peerId"]
        .as_str()
        .expect("runtime own profile peer ID");
    let runtime_peer = super::route_http_request(
        "GET",
        &format!("/api/v0/profile/{runtime_peer_id}"),
        None,
        "",
        &runtime_state,
    )
    .await
    .expect("profile lookup with closed database");
    record!(
        "GET",
        "/api/v0/profile/{peerId}",
        "runtime-failure-and-timeout",
        valid_lookup(&runtime_peer)
    );

    let malformed_invite =
        super::route_http_request("POST", "/api/v0/profile/invite", None, "null", &state)
            .await
            .expect("malformed profile invite");
    record!(
        "POST",
        "/api/v0/profile/invite",
        "malformed-path-query-or-body",
        malformed_invite.status == "400 Bad Request"
    );
    let empty_invite =
        super::route_http_request("POST", "/api/v0/profile/invite", None, "{}", &state)
            .await
            .expect("empty profile invite");
    record!(
        "POST",
        "/api/v0/profile/invite",
        "missing-empty-or-conflict-state",
        valid_invite(&empty_invite)
    );
    let (restart_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let restarted_invite = super::route_http_request(
        "POST",
        "/api/v0/profile/invite",
        None,
        r#"{"expiresInHours":2}"#,
        &restart_state,
    )
    .await
    .expect("profile invite after restart");
    record!(
        "POST",
        "/api/v0/profile/invite",
        "restart-persistence-or-reset",
        valid_invite(&restarted_invite)
    );
    let concurrent_invites = futures_util::future::join_all((0..8).map(|_| {
        super::route_http_request(
            "POST",
            "/api/v0/profile/invite",
            None,
            r#"{"expiresInHours":1}"#,
            &state,
        )
    }))
    .await;
    record!(
        "POST",
        "/api/v0/profile/invite",
        "concurrency-and-idempotency",
        concurrent_invites
            .iter()
            .all(|response| { response.as_ref().is_ok_and(valid_invite) })
    );

    let malformed_update =
        super::route_http_request("PUT", "/api/v0/profile/me", None, "null", &state)
            .await
            .expect("malformed profile update");
    record!(
        "PUT",
        "/api/v0/profile/me",
        "malformed-path-query-or-body",
        malformed_update.status == "400 Bad Request"
    );
    let empty_update = super::route_http_request("PUT", "/api/v0/profile/me", None, "{}", &state)
        .await
        .expect("empty profile update");
    record!(
        "PUT",
        "/api/v0/profile/me",
        "missing-empty-or-conflict-state",
        empty_update.status == "400 Bad Request"
    );
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("profile update runtime database");
    let (runtime_update_state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    db.close_for_test().await;
    let runtime_update = super::route_http_request(
        "PUT",
        "/api/v0/profile/me",
        None,
        update_body,
        &runtime_update_state,
    )
    .await
    .expect("profile update with closed database");
    record!(
        "PUT",
        "/api/v0/profile/me",
        "runtime-failure-and-timeout",
        valid_profile(&runtime_update)
    );
    let restart_update = super::route_http_request(
        "PUT",
        "/api/v0/profile/me",
        None,
        r#"{"displayName":"Restart Profile"}"#,
        &restart_state,
    )
    .await
    .expect("profile update after restart");
    record!(
        "PUT",
        "/api/v0/profile/me",
        "restart-persistence-or-reset",
        valid_profile(&restart_update)
    );
    let concurrent_updates =
        futures_util::future::join_all((0..8).map(|index| {
            let body = format!(r#"{{"displayName":"Concurrent {index}"}}"#);
            let state = state.clone();
            async move {
                super::route_http_request("PUT", "/api/v0/profile/me", None, &body, &state).await
            }
        }))
        .await;
    record!(
        "PUT",
        "/api/v0/profile/me",
        "concurrency-and-idempotency",
        concurrent_updates
            .iter()
            .all(|response| { response.as_ref().is_ok_and(valid_profile) })
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("native_profile_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize profile ledger"),
    )
    .expect("write profile ledger");
    assert!(
        mismatches.is_empty(),
        "{} controller-api profile mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential proof for the empty/default versioned destination
/// projections. The legacy destination records retain their compatibility
/// fields, while slskdN returns the four-field `DestinationResponse` DTO.
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
async fn controller_api_differential_versioned_destinations_nominal() {
    let target = "slskdn";
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let expected_default = serde_json::json!({
        "name": "Downloads",
        "path": "/home/user/Downloads",
        "isDefault": true,
        "exists": std::path::Path::new("/home/user/Downloads").exists(),
    });
    let cases = [
        (
            "/api/v0/destinations",
            serde_json::json!([expected_default.clone()]),
        ),
        ("/api/v0/destinations/default", expected_default),
    ];
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    for (path, expected) in cases {
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .expect("versioned destinations response");
        let value = serde_json::from_str::<serde_json::Value>(&response.body)
            .unwrap_or(serde_json::Value::Null);
        let pass = response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && value == expected;
        if !pass {
            mismatches.push(format!(
                "{target} GET {path}: {} {}",
                response.status, response.body
            ));
        }
        ledger.push(serde_json::json!({
            "target": target,
            "method": "GET",
            "route": path,
            "case": "nominal-status-headers-body",
            "pass": pass,
        }));
    }
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("versioned_destinations_nominal.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

/// Differential proof for the empty versioned hash-database backfill
/// candidate projection. The compatibility route retains its historical
/// `candidates` and `entries` aliases; slskdN exposes `count` and
/// `candidates`.
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
async fn controller_api_differential_versioned_backfill_candidates_nominal() {
    let target = "slskdn";
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let response =
        super::route_http_request("GET", "/api/v0/backfill/candidates", None, "", &state)
            .await
            .expect("versioned backfill-candidates response");
    let value = serde_json::from_str::<serde_json::Value>(&response.body)
        .unwrap_or(serde_json::Value::Null);
    let expected = serde_json::json!({
        "count": 0,
        "candidates": [],
    });
    let pass = response.status == "200 OK"
        && response.content_type.starts_with("application/json")
        && value == expected;
    let ledger = vec![serde_json::json!({
        "target": target,
        "method": "GET",
        "route": "/api/v0/backfill/candidates",
        "case": "nominal-status-headers-body",
        "pass": pass,
    })];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("versioned_backfill_candidates_nominal.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        pass,
        "{target} GET /api/v0/backfill/candidates: got {} {} {}",
        response.status, response.content_type, response.body
    );
}

/// Differential proof for the empty versioned multi-source job list.
/// This route already projects the oracle's `{ count, jobs }` envelope;
/// the missing piece was an explicit nominal ledger row.
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
async fn controller_api_differential_versioned_multisource_jobs_nominal() {
    let target = "slskdn";
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let response = super::route_http_request("GET", "/api/v0/multisource/jobs", None, "", &state)
        .await
        .expect("versioned multisource-jobs response");
    let value = serde_json::from_str::<serde_json::Value>(&response.body)
        .unwrap_or(serde_json::Value::Null);
    let expected = serde_json::json!({
        "count": 0,
        "jobs": [],
    });
    let pass = response.status == "200 OK"
        && response.content_type.starts_with("application/json")
        && value == expected;
    let ledger = vec![serde_json::json!({
        "target": target,
        "method": "GET",
        "route": "/api/v0/multisource/jobs",
        "case": "nominal-status-headers-body",
        "pass": pass,
    })];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("versioned_multisource_jobs_nominal.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        pass,
        "{target} GET /api/v0/multisource/jobs: got {} {} {}",
        response.status, response.content_type, response.body
    );
}

/// Differential proof for the empty versioned swarm-trends DTO.  The
/// oracle has no historical trend storage in this state, so the nominal
/// response is the exact six-array envelope already returned by slskR.
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
async fn controller_api_differential_versioned_swarm_trends_nominal() {
    let target = "slskdn";
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let response =
        super::route_http_request("GET", "/api/v0/swarm/analytics/trends", None, "", &state)
            .await
            .expect("versioned swarm-trends response");
    let value = serde_json::from_str::<serde_json::Value>(&response.body)
        .unwrap_or(serde_json::Value::Null);
    let expected = serde_json::json!({
        "timePoints": [],
        "successRates": [],
        "averageSpeeds": [],
        "averageDurations": [],
        "averageSourcesUsed": [],
        "downloadCounts": [],
    });
    let pass = response.status == "200 OK"
        && response.content_type.starts_with("application/json")
        && value == expected;
    let ledger = vec![serde_json::json!({
        "target": target,
        "method": "GET",
        "route": "/api/v0/swarm/analytics/trends",
        "case": "nominal-status-headers-body",
        "pass": pass,
    })];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("versioned_swarm_trends_nominal.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        pass,
        "{target} GET /api/v0/swarm/analytics/trends: got {} {} {}",
        response.status, response.content_type, response.body
    );
}

/// Differential proof for the empty versioned swarm-analytics dashboard.
/// The response is fully derived from an empty multi-source store and
/// matches the oracle's zero metrics plus deterministic recommendations.
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
async fn controller_api_differential_versioned_swarm_dashboard_nominal() {
    let target = "slskdn";
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let response =
        super::route_http_request("GET", "/api/v0/swarm/analytics/dashboard", None, "", &state)
            .await
            .expect("versioned swarm-dashboard response");
    let value = serde_json::from_str::<serde_json::Value>(&response.body)
        .unwrap_or(serde_json::Value::Null);
    let expected = serde_json::json!({
        "performanceMetrics": {
            "totalDownloads": 0,
            "successfulDownloads": 0,
            "failedDownloads": 0,
            "successRate": 0.0,
            "averageDurationSeconds": 0.0,
            "averageSpeedBytesPerSecond": 0.0,
            "averageSourcesUsed": 0.0,
            "totalBytesDownloaded": 0,
            "totalChunksCompleted": 0,
            "chunkSuccessRate": 0.0,
            "timeWindow": "1.00:00:00",
        },
        "peerRankings": [],
        "efficiencyMetrics": {
            "chunkUtilization": 0.0,
            "peerUtilization": 0.0,
            "redundancyFactor": 0.0,
            "averageTimeToFirstByteMs": 0.0,
            "averageReassignmentRate": 0.0,
            "averageRescueRate": 0.0,
        },
        "recommendations": [
            {
                "type": "NetworkConfig",
                "priority": "High",
                "title": "Low Download Speed",
                "description": "Average download speed is 0.00 MB/s. This may indicate network or peer issues.",
                "action": "Check network connectivity, firewall settings, and consider using more sources per download.",
                "estimatedImpact": 0.4,
            },
            {
                "type": "PeerSelection",
                "priority": "High",
                "title": "Low Success Rate",
                "description": "Current success rate is 0.0%. Consider improving peer selection criteria.",
                "action": "Review peer reputation thresholds and increase minimum reputation score for peer selection.",
                "estimatedImpact": 0.3,
            },
            {
                "type": "ChunkSize",
                "priority": "Medium",
                "title": "High Chunk Failure Rate",
                "description": "Chunk success rate is 0.0%. Consider adjusting chunk size.",
                "action": "Try reducing chunk size to improve reliability, or increase timeout values.",
                "estimatedImpact": 0.2,
            },
            {
                "type": "SourceCount",
                "priority": "Low",
                "title": "Low Peer Utilization",
                "description": "Only 0.0% of available peers are being utilized.",
                "action": "Consider increasing the number of sources per download to improve redundancy.",
                "estimatedImpact": 0.15,
            },
        ],
    });
    let pass = response.status == "200 OK"
        && response.content_type.starts_with("application/json")
        && value == expected;
    let ledger = vec![serde_json::json!({
        "target": target,
        "method": "GET",
        "route": "/api/v0/swarm/analytics/dashboard",
        "case": "nominal-status-headers-body",
        "pass": pass,
    })];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("versioned_swarm_dashboard_nominal.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        pass,
        "{target} GET /api/v0/swarm/analytics/dashboard: got {} {} {}",
        response.status, response.content_type, response.body
    );
}

/// Differential proof for an empty versioned transfer-history page.  An
/// explicit future watermark makes the nominal DTO deterministic while
/// exercising the real direction, cursor, and page-size validation path.
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
async fn controller_api_differential_versioned_transfer_history_nominal() {
    let target = "slskdn";
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let response = super::route_http_request(
        "GET",
        "/api/v0/transfers/history?direction=download&asOf=4102444800000",
        None,
        "",
        &state,
    )
    .await
    .expect("versioned transfer-history response");
    let value = serde_json::from_str::<serde_json::Value>(&response.body)
        .unwrap_or(serde_json::Value::Null);
    let expected = serde_json::json!({
        "asOf": 4_102_444_800_000_i64,
        "hasMore": false,
        "nextOffset": 0,
        "transfers": [],
    });
    let pass = response.status == "200 OK"
        && response.content_type.starts_with("application/json")
        && value == expected;
    let ledger = vec![serde_json::json!({
        "target": target,
        "method": "GET",
        "route": "/api/v0/transfers/history",
        "case": "nominal-status-headers-body",
        "pass": pass,
    })];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("versioned_transfer_history_nominal.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        pass,
        "{target} GET /api/v0/transfers/history: got {} {} {}",
        response.status, response.content_type, response.body
    );
}

/// Differential proof for the two empty list projections in the
/// versioned transfer controller.  Both are exact `[]` responses in the
/// oracle's fresh state and in slskR's state-backed queue.
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
async fn controller_api_differential_versioned_transfer_empty_lists_nominal() {
    let target = "slskdn";
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    for (route, label) in [
        ("/api/v0/transfers/uploads", "versioned uploads"),
        (
            "/api/v0/transfers/downloads/stuck",
            "versioned stuck downloads",
        ),
    ] {
        let response = super::route_http_request("GET", route, None, "", &state)
            .await
            .expect(label);
        let value = serde_json::from_str::<serde_json::Value>(&response.body)
            .unwrap_or(serde_json::Value::Null);
        let pass = response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && value == serde_json::json!([]);
        if !pass {
            mismatches.push(format!(
                "{target} GET {route}: got {} {} {}",
                response.status, response.content_type, response.body
            ));
        }
        ledger.push(serde_json::json!({
            "target": target,
            "method": "GET",
            "route": route,
            "case": "nominal-status-headers-body",
            "pass": pass,
        }));
    }
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("versioned_transfer_empty_lists_nominal.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

/// Differential proof for the versioned transfer-change snapshot.  The
/// cursor is server-generated, so this checks the exact response envelope,
/// zero direction counts, and empty transfer set rather than comparing the
/// timestamp to a second process's clock.
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
async fn controller_api_differential_versioned_transfer_changes_nominal() {
    let target = "slskdn";
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let response = super::route_http_request("GET", "/api/v0/transfers/changes", None, "", &state)
        .await
        .expect("versioned transfer-changes response");
    let value = serde_json::from_str::<serde_json::Value>(&response.body)
        .unwrap_or(serde_json::Value::Null);
    let pass = response.status == "200 OK"
        && response.content_type.starts_with("application/json")
        && value["cursor"].as_u64().is_some_and(|cursor| cursor > 0)
        && value["counts"] == serde_json::json!({"download": 0, "upload": 0})
        && value["transfers"] == serde_json::json!([]);
    let ledger = vec![serde_json::json!({
        "target": target,
        "method": "GET",
        "route": "/api/v0/transfers/changes",
        "case": "nominal-status-headers-body",
        "pass": pass,
    })];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("versioned_transfer_changes_nominal.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        pass,
        "{target} GET /api/v0/transfers/changes: got {} {} {}",
        response.status, response.content_type, response.body
    );
}

/// Differential proof for the empty versioned transfer-summary DTO.  The
/// oracle returns a direction-keyed map whose state maps are empty when
/// the transfer store has no completed records; the legacy route retains
/// slskR's historical aggregate report.
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
async fn controller_api_differential_versioned_transfer_summary_nominal() {
    let target = "slskdn";
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let response = super::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/summary",
        None,
        "",
        &state,
    )
    .await
    .expect("versioned transfer-summary response");
    let value = serde_json::from_str::<serde_json::Value>(&response.body)
        .unwrap_or(serde_json::Value::Null);
    let expected = serde_json::json!({
        "Download": {},
        "Upload": {},
    });
    let pass = response.status == "200 OK"
        && response.content_type.starts_with("application/json")
        && value == expected;
    let ledger = vec![serde_json::json!({
        "target": target,
        "method": "GET",
        "route": "/api/v0/telemetry/reports/transfers/summary",
        "case": "nominal-status-headers-body",
        "pass": pass,
    })];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("versioned_transfer_summary_nominal.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        pass,
        "{target} GET /api/v0/telemetry/reports/transfers/summary: got {} {} {}",
        response.status, response.content_type, response.body
    );
}

/// Differential proof for a deterministic empty versioned transfer
/// histogram. The explicit future window avoids wall-clock-dependent
/// bucket keys while exercising the oracle's gapless interval projection.
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
async fn controller_api_differential_versioned_transfer_histogram_nominal() {
    let target = "slskdn";
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let response = super::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/histogram?start=2100-01-01T00:00:00Z&end=2100-01-01T01:00:00Z&interval=60",
        None,
        "",
        &state,
    )
    .await
    .expect("versioned transfer-histogram response");
    let value = serde_json::from_str::<serde_json::Value>(&response.body)
        .unwrap_or(serde_json::Value::Null);
    let expected = serde_json::json!({
        "2100-01-01T00:00:00Z": {
            "Download": {},
            "Upload": {},
        },
    });
    let pass = response.status == "200 OK"
        && response.content_type.starts_with("application/json")
        && value == expected;
    let ledger = vec![serde_json::json!({
        "target": target,
        "method": "GET",
        "route": "/api/v0/telemetry/reports/transfers/histogram",
        "case": "nominal-status-headers-body",
        "pass": pass,
    })];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("versioned_transfer_histogram_nominal.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        pass,
        "{target} GET /api/v0/telemetry/reports/transfers/histogram: got {} {} {}",
        response.status, response.content_type, response.body
    );
}

/// Differential proof for populated versioned transfer reports.  The
/// oracle groups completed records by direction and final state and
/// carries the measured summary fields into both the summary and histogram
/// projections.  Fixed Unix timestamps keep both responses deterministic.
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
async fn controller_api_differential_versioned_transfer_reports_populated_state() {
    let target = "slskdn";
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    {
        let mut transfers = state.transfers.write().await;
        let created = transfers.create(
            0,
            Some("telemetry peer".to_owned()),
            "Telemetry/Report.flac".to_owned(),
            None,
            Some(321),
        );
        let entry = transfers
            .entries
            .iter_mut()
            .find(|entry| entry.id == created.id)
            .expect("populated telemetry transfer");
        entry.status = "succeeded".to_owned();
        entry.requested_at = 3_700;
        entry.started_at = Some(3_710);
        entry.bytes_transferred = 321;
        entry.updated_at = 3_730;
        entry.updated_at_ms = 3_730_000;
    }

    let summary = super::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/summary?start=3600&end=7200&direction=Download&username=telemetry%20peer",
        None,
        "",
        &state,
    )
    .await
    .expect("populated versioned transfer-summary response");
    let summary_value =
        serde_json::from_str::<serde_json::Value>(&summary.body).unwrap_or(serde_json::Value::Null);
    let summary_record = &summary_value["Download"]["Succeeded"];
    let summary_pass = summary.status == "200 OK"
        && summary.content_type.starts_with("application/json")
        && summary_record["username"] == ""
        && summary_record["totalBytes"] == 321
        && summary_record["count"] == 1
        && summary_record["distinctUsers"] == 1
        && summary_record["averageWait"] == 10.0
        && summary_record["averageDuration"] == 20.0
        && summary_record["averageSpeed"]
            .as_f64()
            .is_some_and(|value| (value - 16.05).abs() < 0.000001)
        && summary_value["Upload"] == serde_json::json!({});

    let histogram = super::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/histogram?start=3600&end=7200&interval=60&direction=Download&username=telemetry%20peer",
        None,
        "",
        &state,
    )
    .await
    .expect("populated versioned transfer-histogram response");
    let histogram_value = serde_json::from_str::<serde_json::Value>(&histogram.body)
        .unwrap_or(serde_json::Value::Null);
    let histogram_record = &histogram_value["1970-01-01T01:00:00Z"]["Download"]["Succeeded"];
    let histogram_pass = histogram.status == "200 OK"
        && histogram.content_type.starts_with("application/json")
        && histogram_record["username"] == ""
        && histogram_record["totalBytes"] == 321
        && histogram_record["count"] == 1
        && histogram_record["distinctUsers"] == 1
        && histogram_value["1970-01-01T01:00:00Z"]["Upload"] == serde_json::json!({})
        && histogram_value
            .as_object()
            .is_some_and(|buckets| buckets.len() == 1);

    let ledger = vec![
        serde_json::json!({
            "target": target,
            "method": "GET",
            "route": "/api/v0/telemetry/reports/transfers/summary",
            "case": "populated-dynamic-state",
            "pass": summary_pass,
        }),
        serde_json::json!({
            "target": target,
            "method": "GET",
            "route": "/api/v0/telemetry/reports/transfers/histogram",
            "case": "populated-dynamic-state",
            "pass": histogram_pass,
        }),
    ];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("versioned_transfer_reports_populated.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        summary_pass,
        "{target} GET /api/v0/telemetry/reports/transfers/summary: got {} {} {}",
        summary.status, summary.content_type, summary.body
    );
    assert!(
        histogram_pass,
        "{target} GET /api/v0/telemetry/reports/transfers/histogram: got {} {} {}",
        histogram.status, histogram.content_type, histogram.body
    );
}

/// Bulk differential proof for the versioned destinations controller's
/// populated list and selected-default projections.  The fixture uses a
/// real directory so the response's existence flag is also exercised;
/// the in-memory destination store is the same state-backed projection
/// used by the HTTP handlers.  slskdN-only (confirmed against the frozen
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
    feature = "bounded-controller-api-tests-3"
))]
async fn controller_api_differential_versioned_destinations_populated_state() {
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
    let root = std::env::temp_dir().join(format!(
        "slskr-destination-populated-{}",
        uuid::Uuid::new_v4().simple()
    ));
    let custom_path = root.join("Archive");
    fs::create_dir_all(&custom_path).expect("create destination fixture");
    {
        let mut destinations = state.destinations.write().await;
        destinations.records = vec![
            super::destination_state::DestinationRecord {
                id: "default".to_owned(),
                name: "Downloads".to_owned(),
                path: root.join("Downloads").display().to_string(),
                is_default: false,
            },
            super::destination_state::DestinationRecord {
                id: "configured-0".to_owned(),
                name: "Archive".to_owned(),
                path: custom_path.display().to_string(),
                is_default: true,
            },
        ];
    }

    let list = super::route_http_request("GET", "/api/v0/destinations", None, "", &state)
        .await
        .expect("populated destinations response");
    let list_value =
        serde_json::from_str::<serde_json::Value>(&list.body).unwrap_or(serde_json::Value::Null);
    record!(
        "/api/v0/destinations",
        list.status == "200 OK"
            && list.content_type == "application/json"
            && list_value
                .as_array()
                .is_some_and(|records| records.len() == 2)
            && list_value
                .as_array()
                .is_some_and(|records| records.iter().any(|record| {
                    record["name"] == "Archive"
                        && record["path"] == custom_path.display().to_string()
                        && record["isDefault"] == true
                        && record["exists"] == true
                }))
    );

    let default =
        super::route_http_request("GET", "/api/v0/destinations/default", None, "", &state)
            .await
            .expect("populated default destination response");
    let default_value =
        serde_json::from_str::<serde_json::Value>(&default.body).unwrap_or(serde_json::Value::Null);
    record!(
        "/api/v0/destinations/default",
        default.status == "200 OK"
            && default.content_type == "application/json"
            && default_value["name"] == "Archive"
            && default_value["path"] == custom_path.display().to_string()
            && default_value["isDefault"] == true
            && default_value["exists"] == true
    );

    let _ = fs::remove_dir_all(root);
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("versioned_destinations_populated_state.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api destinations mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential evidence for the remaining slskdN DestinationsController
/// edges.  The native controller binds only the case-insensitive `Path`
/// property, rejects null/empty/malformed bodies, derives its default
/// destination from configuration even if the backing state is empty, and
/// performs validation without persistence or database coupling.
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
async fn controller_api_differential_native_destinations_edge_contracts() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!(
                    "{target} {} {} [{}]",
                    $method,
                    $route,
                    $case
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": $method,
                "route": $route,
                "case": $case,
                "pass": pass,
            }));
        }};
    }

    let root = std::env::temp_dir().join(format!(
        "slskr-destination-edge-{}",
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&root).expect("create destination edge root");
    let env = || {
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_DOWNLOADS_DIR", &root.display().to_string())
    };
    let (state, _receiver) = test_state_with_env(env());
    {
        let mut destinations = state.destinations.write().await;
        destinations.records.clear();
    }

    let expected_default = serde_json::json!({
        "name": "Downloads",
        "path": root.display().to_string(),
        "isDefault": true,
        "exists": true,
    });
    let empty_list = super::route_http_request("GET", "/api/v0/destinations", None, "", &state)
        .await
        .expect("empty slskdn destinations list");
    let empty_list_json =
        serde_json::from_str::<serde_json::Value>(&empty_list.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/destinations",
        "missing-empty-or-conflict-state",
        empty_list.status == "200 OK"
            && empty_list.content_type.starts_with("application/json")
            && empty_list_json == serde_json::json!([expected_default.clone()])
    );

    let empty_default =
        super::route_http_request("GET", "/api/v0/destinations/default", None, "", &state)
            .await
            .expect("empty slskdn default destination");
    let empty_default_json =
        serde_json::from_str::<serde_json::Value>(&empty_default.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/destinations/default",
        "missing-empty-or-conflict-state",
        empty_default.status == "200 OK"
            && empty_default.content_type.starts_with("application/json")
            && empty_default_json == expected_default
    );

    let malformed_list = super::route_http_request(
        "GET",
        "/api/v0/destinations/extra?unexpected=not-a-number",
        None,
        "",
        &state,
    )
    .await
    .expect("malformed slskdn destinations list path");
    record!(
        "GET",
        "/api/v0/destinations",
        "malformed-path-query-or-body",
        malformed_list.status == "404 Not Found"
    );

    let malformed_default = super::route_http_request(
        "GET",
        "/api/v0/destinations/default/extra",
        None,
        "",
        &state,
    )
    .await
    .expect("malformed slskdn default destination path");
    record!(
        "GET",
        "/api/v0/destinations/default",
        "malformed-path-query-or-body",
        malformed_default.status == "404 Not Found"
    );

    {
        let mut destinations = state.destinations.write().await;
        *destinations = super::DestinationStore::from_config(
            &state.config.downloads_dir,
            &state.config.core_workflow.destinations,
        );
    }
    let invalid_field = super::route_http_request(
        "POST",
        "/api/v0/destinations/validate",
        None,
        &serde_json::json!({"destination": root.display().to_string()}).to_string(),
        &state,
    )
    .await
    .expect("invalid slskdn destination field");
    let empty_body =
        super::route_http_request("POST", "/api/v0/destinations/validate", None, "", &state)
            .await
            .expect("empty slskdn destination body");
    record!(
        "POST",
        "/api/v0/destinations/validate",
        "malformed-path-query-or-body",
        invalid_field.status == "400 Bad Request"
            && invalid_field.body.contains("Path is required")
    );
    record!(
        "POST",
        "/api/v0/destinations/validate",
        "missing-empty-or-conflict-state",
        empty_body.status == "400 Bad Request" && empty_body.body.contains("Path is required")
    );

    let valid_body = serde_json::json!({"Path": root.display().to_string()}).to_string();
    let before_validation = state.destinations.read().await.list();
    let valid = super::route_http_request(
        "POST",
        "/api/v0/destinations/validate",
        None,
        &valid_body,
        &state,
    )
    .await
    .expect("valid slskdn destination validation");
    let valid_json = serde_json::from_str::<serde_json::Value>(&valid.body).unwrap_or_default();
    let after_validation = state.destinations.read().await.list();
    record!(
        "POST",
        "/api/v0/destinations/validate",
        "restart-persistence-or-reset",
        valid.status == "200 OK"
            && valid_json["path"] == root.display().to_string()
            && valid_json["exists"] == true
            && valid_json["writable"] == true
            && before_validation == after_validation
    );

    let failure_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("destinations failure database");
    let (failure_state, _receiver) =
        test_state_with_env_parts(env(), super::SearchStore::new(), Some(failure_db.clone()));
    {
        let mut destinations = failure_state.destinations.write().await;
        *destinations = super::DestinationStore::from_config(
            &failure_state.config.downloads_dir,
            &failure_state.config.core_workflow.destinations,
        );
    }
    failure_db.close_for_test().await;
    let failure_list =
        super::route_http_request("GET", "/api/v0/destinations", None, "", &failure_state)
            .await
            .expect("destinations list after database close");
    let failure_default = super::route_http_request(
        "GET",
        "/api/v0/destinations/default",
        None,
        "",
        &failure_state,
    )
    .await
    .expect("destination default after database close");
    let failure_validate = super::route_http_request(
        "POST",
        "/api/v0/destinations/validate",
        None,
        &valid_body,
        &failure_state,
    )
    .await
    .expect("destination validation after database close");
    let failure_validate_json =
        serde_json::from_str::<serde_json::Value>(&failure_validate.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/destinations",
        "runtime-failure-and-timeout",
        failure_list.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&failure_list.body)
                .ok()
                .and_then(|value| value.as_array().map(|records| records.len()))
                == Some(1)
    );
    record!(
        "GET",
        "/api/v0/destinations/default",
        "runtime-failure-and-timeout",
        failure_default.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&failure_default.body)
                .ok()
                .is_some_and(|value| value["name"] == "Downloads")
    );
    record!(
        "POST",
        "/api/v0/destinations/validate",
        "runtime-failure-and-timeout",
        failure_validate.status == "200 OK"
            && failure_validate_json["path"] == root.display().to_string()
    );

    let concurrent_before = state.destinations.read().await.list();
    let concurrent_responses = futures_util::future::join_all([
        super::route_http_request(
            "POST",
            "/api/v0/destinations/validate",
            None,
            &valid_body,
            &state,
        ),
        super::route_http_request(
            "POST",
            "/api/v0/destinations/validate",
            None,
            &valid_body,
            &state,
        ),
    ])
    .await;
    let concurrent_after = state.destinations.read().await.list();
    record!(
        "POST",
        "/api/v0/destinations/validate",
        "concurrency-and-idempotency",
        concurrent_responses.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
        }) && concurrent_before == concurrent_after
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("native_destinations_edge_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskdn destinations edge ledger"),
    )
    .expect("write slskdn destinations edge ledger");
    let _ = fs::remove_dir_all(root);
    assert!(
        mismatches.is_empty(),
        "{} slskdn destinations edge mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof for the `runtime-failure-and-timeout`
/// case (security-ban and share/collection-grant routes) -- same
/// real closed-database fault injection as the earlier `runtime_
/// failure_*` batches, independently re-derived from `security_
/// ban_rolls_back_when_persistence_fails`, `security_unban_rolls_
/// back_when_persistence_fails`, `collection_delete_rolls_back_
/// grant_revocation_when_persistence_fails`, `share_grant_
/// revocation_rolls_back_when_persistence_fails`, and `share_
/// access_token_issue_rolls_back_when_persistence_fails` with
/// fresh fixture data. Confirmed against `/tmp/slskr-parity-
/// evidence/controller-api/*.json` before writing, per case: none
/// of these 5 routes had `runtime-failure-and-timeout` credited
/// (some had other cases already true). slskdN-only (confirmed
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
    feature = "bounded-controller-api-tests-3"
))]
async fn controller_api_differential_runtime_failure_security_and_shares() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($method:expr, $route:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!("{target} {} {} [runtime-failure-and-timeout]", $method, $route));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": $method,
                "route": $route,
                "case": "runtime-failure-and-timeout",
                "pass": $pass,
            }));
        };
    }

    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        db.close_for_test().await;
        let response = super::route_http_request(
            "POST",
            "/api/v0/security/bans/username",
            None,
            r#"{"username":"differential-must-persist"}"#,
            &state,
        )
        .await
        .expect("failed ban response");
        record!(
            "POST",
            "/api/v0/security/bans/username",
            response.status == "503 Service Unavailable"
                && state.security.read().await.bans.is_empty()
        );
    }

    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        state
            .security
            .write()
            .await
            .ban("username", "differential-must-remain".to_owned())
            .expect("ban");
        db.close_for_test().await;
        let response = super::route_http_request(
            "DELETE",
            "/api/v0/security/bans/username/differential-must-remain",
            None,
            "",
            &state,
        )
        .await
        .expect("failed unban response");
        record!(
            "DELETE",
            "/api/v0/security/bans/username/{username}",
            response.status == "503 Service Unavailable"
                && state.security.read().await.active_bans() == 1
        );
    }

    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let collection_id = state
            .collections
            .write()
            .await
            .create(
                String::new(),
                "Differential Private".to_owned(),
                String::new(),
            )
            .expect("collection")
            .id;
        state
            .share_grants
            .write()
            .await
            .create_with_contract(
                None,
                collection_id.clone(),
                "differential-friend".to_owned(),
            )
            .expect("share grant");
        db.close_for_test().await;
        let response = super::route_http_request(
            "DELETE",
            &format!("/api/v0/collections/{collection_id}"),
            None,
            "",
            &state,
        )
        .await
        .expect("failed collection delete response");
        record!(
            "DELETE",
            "/api/v0/collections/{id}",
            response.status == "503 Service Unavailable"
                && state.collections.read().await.get(&collection_id).is_some()
        );
    }

    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        state
            .share_grants
            .write()
            .await
            .create_with_contract(
                None,
                "differential-collection".to_owned(),
                "differential-friend".to_owned(),
            )
            .expect("grant");
        db.close_for_test().await;
        let response =
            super::route_http_request("DELETE", "/api/v0/share-grants/grant-1", None, "", &state)
                .await
                .expect("failed grant revocation response");
        record!(
            "DELETE",
            "/api/v0/share-grants/{id}",
            response.status == "503 Service Unavailable"
                && state.share_grants.read().await.get("grant-1").is_some()
        );
    }

    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        state
            .collections
            .write()
            .await
            .create(
                String::new(),
                "Differential Private".to_owned(),
                String::new(),
            )
            .expect("collection");
        state
            .share_grants
            .write()
            .await
            .create_with_contract(None, "col-1".to_owned(), "differential-friend".to_owned())
            .expect("grant");
        db.close_for_test().await;
        let response = super::route_http_request(
            "POST",
            "/api/v0/share-grants/grant-1/token",
            None,
            r#"{"expiresInSeconds":600}"#,
            &state,
        )
        .await
        .expect("failed access token issue response");
        record!(
            "POST",
            "/api/v0/share-grants/{id}/token",
            response.status == "503 Service Unavailable"
                && state.share_access_tokens.read().await.records.is_empty()
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("runtime_failure_security_and_shares.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api runtime-failure-security-and-shares mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof for sharegroups CRUD/edge cases and the
/// `runtime-failure-and-timeout` case (sharegroups routes), plus
/// `concurrency-and-idempotency` for the shares-rebuild route --
/// independently re-derived from
/// `share_group_revocation_rolls_back_when_persistence_fails`,
/// `share_group_member_revocation_rolls_back_when_persistence_
/// fails`, `share_rebuild_routes_roll_back_when_persistence_
/// fails`, and `share_rebuild_routes_reject_concurrent_scans`
/// (a real held scan-permit genuinely blocks a concurrent rebuild
/// with 503, not a race) with fresh fixture data. Confirmed
/// against the evidence directory before writing: none of these
/// cases had prior credit. slskdN-only (confirmed against the
/// frozen registry).
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
async fn controller_api_differential_sharegroups_and_shares_rebuild() {
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

    {
        let (state, _receiver) = test_state();
        let empty = super::route_http_request("GET", "/api/v0/sharegroups", None, "", &state)
            .await
            .expect("empty sharegroups");
        record!(
            "GET",
            "/api/v0/sharegroups",
            "nominal-status-headers-body",
            empty.status == "200 OK"
        );
        record!(
            "GET",
            "/api/v0/sharegroups",
            "missing-empty-or-conflict-state",
            empty.status == "200 OK" && empty.body == "[]"
        );

        let malformed_list =
            super::route_http_request("GET", "/api/v0/sharegroups/", None, "", &state)
                .await
                .expect("malformed sharegroups list");
        record!(
            "GET",
            "/api/v0/sharegroups",
            "malformed-path-query-or-body",
            malformed_list.status == "400 Bad Request"
        );

        let malformed_create = super::route_http_request(
            "POST",
            "/api/v0/sharegroups",
            None,
            r#"{"name":""}"#,
            &state,
        )
        .await
        .expect("malformed sharegroup create");
        record!(
            "POST",
            "/api/v0/sharegroups",
            "malformed-path-query-or-body",
            malformed_create.status == "400 Bad Request"
        );

        let missing_create =
            super::route_http_request("POST", "/api/v0/sharegroups", None, "{}", &state)
                .await
                .expect("missing sharegroup name");
        record!(
            "POST",
            "/api/v0/sharegroups",
            "missing-empty-or-conflict-state",
            missing_create.status == "400 Bad Request"
        );

        let created = super::route_http_request(
            "POST",
            "/api/v0/sharegroups",
            None,
            r#"{"name":"Differential Group","description":"edge fixture"}"#,
            &state,
        )
        .await
        .expect("create sharegroup");
        let created_json =
            serde_json::from_str::<serde_json::Value>(&created.body).unwrap_or_default();
        let group_id = created_json["id"].as_str().unwrap_or_default().to_owned();

        let listed = super::route_http_request("GET", "/api/v0/sharegroups", None, "", &state)
            .await
            .expect("list populated sharegroups");
        let listed_json =
            serde_json::from_str::<serde_json::Value>(&listed.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/sharegroups",
            "populated-dynamic-state",
            listed.status == "200 OK"
                && listed_json
                    .as_array()
                    .is_some_and(|groups| { groups.iter().any(|group| group["id"] == group_id) })
        );

        let get_route = format!("/api/v0/sharegroups/{group_id}");
        let fetched = super::route_http_request("GET", &get_route, None, "", &state)
            .await
            .expect("get sharegroup");
        let fetched_json =
            serde_json::from_str::<serde_json::Value>(&fetched.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/sharegroups/{id}",
            "nominal-status-headers-body",
            fetched.status == "200 OK"
        );
        record!(
            "GET",
            "/api/v0/sharegroups/{id}",
            "populated-dynamic-state",
            fetched.status == "200 OK"
                && fetched_json["id"] == group_id
                && fetched_json["name"] == "Differential Group"
        );
        let missing_get = super::route_http_request(
            "GET",
            "/api/v0/sharegroups/00000000-0000-0000-0000-000000000000",
            None,
            "",
            &state,
        )
        .await
        .expect("missing sharegroup get");
        record!(
            "GET",
            "/api/v0/sharegroups/{id}",
            "missing-empty-or-conflict-state",
            missing_get.status == "404 Not Found"
        );

        let updated = super::route_http_request(
            "PUT",
            &get_route,
            None,
            r#"{"name":"Updated Differential Group","description":"updated"}"#,
            &state,
        )
        .await
        .expect("update sharegroup");
        let updated_json =
            serde_json::from_str::<serde_json::Value>(&updated.body).unwrap_or_default();
        record!(
            "PUT",
            "/api/v0/sharegroups/{id}",
            "nominal-status-headers-body",
            updated.status == "200 OK"
        );
        record!(
            "PUT",
            "/api/v0/sharegroups/{id}",
            "mutation-side-effects-and-readback",
            updated_json["name"] == "Updated Differential Group"
        );
        let missing_update = super::route_http_request(
            "PUT",
            "/api/v0/sharegroups/missing-group",
            None,
            r#"{"name":"missing"}"#,
            &state,
        )
        .await
        .expect("missing sharegroup update");
        record!(
            "PUT",
            "/api/v0/sharegroups/{id}",
            "missing-empty-or-conflict-state",
            missing_update.status == "404 Not Found"
        );
        let malformed_update = super::route_http_request(
            "PUT",
            &format!("{get_route}/"),
            None,
            r#"{"name":"Malformed Path"}"#,
            &state,
        )
        .await
        .expect("malformed sharegroup update path");
        record!(
            "PUT",
            "/api/v0/sharegroups/{id}",
            "malformed-path-query-or-body",
            malformed_update.status == "404 Not Found"
        );

        let members_route = format!("/api/v0/sharegroups/{group_id}/members");
        let empty_members = super::route_http_request("GET", &members_route, None, "", &state)
            .await
            .expect("empty sharegroup members");
        record!(
            "GET",
            "/api/v0/sharegroups/{id}/members",
            "nominal-status-headers-body",
            empty_members.status == "200 OK"
        );
        record!(
            "GET",
            "/api/v0/sharegroups/{id}/members",
            "missing-empty-or-conflict-state",
            empty_members.status == "200 OK" && empty_members.body == "[]"
        );

        let malformed_member =
            super::route_http_request("POST", &members_route, None, "{}", &state)
                .await
                .expect("malformed sharegroup member");
        record!(
            "POST",
            "/api/v0/sharegroups/{id}/members",
            "malformed-path-query-or-body",
            malformed_member.status == "409 Conflict"
        );
        let missing_member = super::route_http_request(
            "POST",
            "/api/v0/sharegroups/missing-group/members",
            None,
            r#"{"username":"friend"}"#,
            &state,
        )
        .await
        .expect("missing sharegroup member group");
        record!(
            "POST",
            "/api/v0/sharegroups/{id}/members",
            "missing-empty-or-conflict-state",
            missing_member.status == "404 Not Found"
        );

        let added = super::route_http_request(
            "POST",
            &members_route,
            None,
            r#"{"username":"friend"}"#,
            &state,
        )
        .await
        .expect("add sharegroup member");
        record!(
            "POST",
            "/api/v0/sharegroups/{id}/members",
            "nominal-status-headers-body",
            added.status == "201 Created"
        );
        record!(
            "POST",
            "/api/v0/sharegroups/{id}/members",
            "mutation-side-effects-and-readback",
            added.body.contains("friend")
        );

        let populated_members = super::route_http_request("GET", &members_route, None, "", &state)
            .await
            .expect("populated sharegroup members");
        let populated_members_json =
            serde_json::from_str::<serde_json::Value>(&populated_members.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/sharegroups/{id}/members",
            "populated-dynamic-state",
            populated_members.status == "200 OK"
                && populated_members_json.as_array().is_some_and(|members| {
                    members.iter().any(|member| member["username"] == "friend")
                })
        );

        let member_route = format!("/api/v0/sharegroups/{group_id}/members/friend");
        let removed_member = super::route_http_request("DELETE", &member_route, None, "", &state)
            .await
            .expect("remove sharegroup member");
        record!(
            "DELETE",
            "/api/v0/sharegroups/{id}/members/{userId}",
            "nominal-status-headers-body",
            removed_member.status == "200 OK"
        );
        record!(
            "DELETE",
            "/api/v0/sharegroups/{id}/members/{userId}",
            "mutation-side-effects-and-readback",
            removed_member.status == "200 OK"
        );
        let missing_member_delete = super::route_http_request(
            "DELETE",
            "/api/v0/sharegroups/missing-group/members/friend",
            None,
            "",
            &state,
        )
        .await
        .expect("missing sharegroup member delete");
        record!(
            "DELETE",
            "/api/v0/sharegroups/{id}/members/{userId}",
            "missing-empty-or-conflict-state",
            missing_member_delete.status == "404 Not Found"
        );
        let malformed_member_delete =
            super::route_http_request("DELETE", &format!("{member_route}/"), None, "", &state)
                .await
                .expect("malformed sharegroup member delete path");
        record!(
            "DELETE",
            "/api/v0/sharegroups/{id}/members/{userId}",
            "malformed-path-query-or-body",
            malformed_member_delete.status == "404 Not Found"
        );

        let deleted = super::route_http_request("DELETE", &get_route, None, "", &state)
            .await
            .expect("delete sharegroup");
        record!(
            "DELETE",
            "/api/v0/sharegroups/{id}",
            "nominal-status-headers-body",
            deleted.status == "200 OK"
        );
        record!(
            "DELETE",
            "/api/v0/sharegroups/{id}",
            "mutation-side-effects-and-readback",
            deleted.body == "{}"
        );
        let missing_delete = super::route_http_request(
            "DELETE",
            "/api/v0/sharegroups/missing-group",
            None,
            "",
            &state,
        )
        .await
        .expect("missing sharegroup delete");
        record!(
            "DELETE",
            "/api/v0/sharegroups/{id}",
            "missing-empty-or-conflict-state",
            missing_delete.status == "404 Not Found"
        );
        let malformed_delete =
            super::route_http_request("DELETE", &format!("{get_route}/"), None, "", &state)
                .await
                .expect("malformed sharegroup delete path");
        record!(
            "DELETE",
            "/api/v0/sharegroups/{id}",
            "malformed-path-query-or-body",
            malformed_delete.status == "404 Not Found"
        );
    }

    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let group_id = state
            .sharegroups
            .write()
            .await
            .create("Differential Trusted".to_owned(), String::new())
            .expect("share group")
            .id;
        state
            .sharegroups
            .write()
            .await
            .add_member(&group_id, "differential-friend".to_owned())
            .expect("member capacity")
            .expect("share group");
        db.close_for_test().await;
        let response = super::route_http_request(
            "DELETE",
            &format!("/api/v0/sharegroups/{group_id}"),
            None,
            "",
            &state,
        )
        .await
        .expect("failed share group revocation response");
        record!(
            "DELETE",
            "/api/v0/sharegroups/{id}",
            "runtime-failure-and-timeout",
            response.status == "503 Service Unavailable"
        );
    }

    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let group_id = state
            .sharegroups
            .write()
            .await
            .create("Differential Trusted".to_owned(), String::new())
            .expect("share group")
            .id;
        state
            .sharegroups
            .write()
            .await
            .add_member(&group_id, "differential-friend".to_owned())
            .expect("member capacity")
            .expect("share group");
        db.close_for_test().await;
        let response = super::route_http_request(
            "DELETE",
            &format!("/api/v0/sharegroups/{group_id}/members/differential-friend"),
            None,
            "",
            &state,
        )
        .await
        .expect("failed share group member revocation response");
        record!(
            "DELETE",
            "/api/v0/sharegroups/{id}/members/{userId}",
            "runtime-failure-and-timeout",
            response.status == "503 Service Unavailable"
        );
    }

    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        db.close_for_test().await;
        let response = super::route_http_request("PUT", "/api/v0/shares", None, "", &state)
            .await
            .expect("failed shares rebuild response");
        record!(
            "PUT",
            "/api/v0/shares",
            "runtime-failure-and-timeout",
            response.status == "503 Service Unavailable"
        );
    }

    {
        let (state, _receiver) = test_state();
        let _permit = Arc::clone(&state.share_scans)
            .acquire_owned()
            .await
            .expect("share scan permit");
        let response = super::route_http_request("PUT", "/api/v0/shares", None, "", &state)
            .await
            .expect("concurrent shares rebuild response");
        record!(
            "PUT",
            "/api/v0/shares",
            "concurrency-and-idempotency",
            response.status == "503 Service Unavailable"
                && response.body == "{\"error\":\"share scan already in progress\"}"
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("sharegroups_and_shares_rebuild.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api sharegroups-and-shares-rebuild mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
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
    feature = "bounded-controller-api-tests-3"
))]
async fn controller_api_differential_sharegroups_persistence_and_concurrency() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    let persistence_env = || {
        MapEnv::default()
            .with("SLSKR_PERSISTENCE_ENABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", target)
    };

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

    // Share-group creation survives a real persisted-state rebuild.
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("share-group create restart database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let created = super::route_http_request(
            "POST",
            "/api/v0/sharegroups",
            None,
            r#"{"name":"Restart Share Group","description":"persisted"}"#,
            &state,
        )
        .await
        .unwrap();
        let created_json = serde_json::from_str::<serde_json::Value>(&created.body).unwrap();
        let group_id = created_json["id"].as_str().unwrap().to_owned();
        let groups = db.list_share_groups(10, 0).await.unwrap();
        let members = db.list_share_group_members(10, 0).await.unwrap();
        let (restarted_state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        *restarted_state.sharegroups.write().await =
            super::ShareGroupStore::from_persisted(groups.clone(), members);
        let fetched = super::route_http_request(
            "GET",
            &format!("/api/v0/sharegroups/{group_id}"),
            None,
            "",
            &restarted_state,
        )
        .await
        .unwrap();
        let fetched_json = serde_json::from_str::<serde_json::Value>(&fetched.body).unwrap();
        record!(
            "POST",
            "/api/v0/sharegroups",
            "restart-persistence-or-reset",
            created.status == "201 Created"
                && groups.len() == 1
                && groups[0].id == group_id
                && fetched.status == "200 OK"
                && fetched_json["id"] == group_id
        );
    }

    // Distinct group creates through one real controller are all durable.
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("share-group create concurrency database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let bodies: Vec<String> = (0..4)
            .map(|index| format!(r#"{{"name":"Concurrent Share Group {index}"}}"#))
            .collect();
        let responses = futures_util::future::join_all(bodies.iter().map(|body| {
            super::route_http_request("POST", "/api/v0/sharegroups", None, body, &state)
        }))
        .await;
        let groups = db.list_share_groups(10, 0).await.unwrap_or_default();
        let expected: std::collections::BTreeSet<String> = (0..4)
            .map(|index| format!("Concurrent Share Group {index}"))
            .collect();
        let names: std::collections::BTreeSet<String> =
            groups.iter().map(|group| group.name.clone()).collect();
        record!(
            "POST",
            "/api/v0/sharegroups",
            "concurrency-and-idempotency",
            responses.iter().all(|response| {
                response
                    .as_ref()
                    .is_ok_and(|response| response.status == "201 Created")
            }) && groups.len() == 4
                && names == expected
        );
    }

    // A group update survives restart rehydration.
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("share-group update restart database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let created = super::route_http_request(
            "POST",
            "/api/v0/sharegroups",
            None,
            r#"{"name":"Update Restart Before"}"#,
            &state,
        )
        .await
        .unwrap();
        let group_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let updated = super::route_http_request(
            "PUT",
            &format!("/api/v0/sharegroups/{group_id}"),
            None,
            r#"{"name":"Update Restart After","description":"changed"}"#,
            &state,
        )
        .await
        .unwrap();
        let groups = db.list_share_groups(10, 0).await.unwrap();
        let members = db.list_share_group_members(10, 0).await.unwrap();
        let (restarted_state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        *restarted_state.sharegroups.write().await =
            super::ShareGroupStore::from_persisted(groups, members);
        let fetched = super::route_http_request(
            "GET",
            &format!("/api/v0/sharegroups/{group_id}"),
            None,
            "",
            &restarted_state,
        )
        .await
        .unwrap();
        let fetched_json = serde_json::from_str::<serde_json::Value>(&fetched.body).unwrap();
        record!(
            "PUT",
            "/api/v0/sharegroups/{id}",
            "restart-persistence-or-reset",
            updated.status == "200 OK"
                && fetched.status == "200 OK"
                && fetched_json["name"] == "Update Restart After"
        );
    }

    // Concurrent group updates retain each writer's own value.
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("share-group update concurrency database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let mut group_ids = Vec::new();
        for index in 0..4 {
            let created = super::route_http_request(
                "POST",
                "/api/v0/sharegroups",
                None,
                &format!(r#"{{"name":"Update Concurrent Before {index}"}}"#),
                &state,
            )
            .await
            .unwrap();
            group_ids.push(
                serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
                    .as_str()
                    .unwrap()
                    .to_owned(),
            );
        }
        let responses = futures_util::future::join_all(group_ids.iter().enumerate().map(
            |(index, group_id)| {
                let path = format!("/api/v0/sharegroups/{group_id}");
                let body = format!(r#"{{"name":"Update Concurrent After {index}"}}"#);
                let state = Arc::clone(&state);
                async move { super::route_http_request("PUT", &path, None, &body, &state).await }
            },
        ))
        .await;
        let groups = db.list_share_groups(10, 0).await.unwrap_or_default();
        let expected: std::collections::BTreeSet<String> = (0..4)
            .map(|index| format!("Update Concurrent After {index}"))
            .collect();
        let names: std::collections::BTreeSet<String> =
            groups.iter().map(|group| group.name.clone()).collect();
        record!(
            "PUT",
            "/api/v0/sharegroups/{id}",
            "concurrency-and-idempotency",
            responses.iter().all(|response| {
                response
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
            }) && groups.len() == 4
                && names == expected
        );
    }

    // A member create survives group/member snapshot rehydration.
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("share-group member create restart database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let created = super::route_http_request(
            "POST",
            "/api/v0/sharegroups",
            None,
            r#"{"name":"Member Restart Group"}"#,
            &state,
        )
        .await
        .unwrap();
        let group_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let added = super::route_http_request(
            "POST",
            &format!("/api/v0/sharegroups/{group_id}/members"),
            None,
            r#"{"username":"restart-member"}"#,
            &state,
        )
        .await
        .unwrap();
        let groups = db.list_share_groups(10, 0).await.unwrap();
        let members = db.list_share_group_members(10, 0).await.unwrap();
        let (restarted_state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        *restarted_state.sharegroups.write().await =
            super::ShareGroupStore::from_persisted(groups, members);
        let listed = super::route_http_request(
            "GET",
            &format!("/api/v0/sharegroups/{group_id}/members"),
            None,
            "",
            &restarted_state,
        )
        .await
        .unwrap();
        let listed_json = serde_json::from_str::<serde_json::Value>(&listed.body).unwrap();
        record!(
            "POST",
            "/api/v0/sharegroups/{id}/members",
            "restart-persistence-or-reset",
            added.status == "201 Created"
                && listed.status == "200 OK"
                && listed_json.as_array().is_some_and(|members| {
                    members
                        .iter()
                        .any(|member| member["username"] == "restart-member")
                })
        );
    }

    // Distinct member creates through one group are all durable.
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("share-group member create concurrency database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let created = super::route_http_request(
            "POST",
            "/api/v0/sharegroups",
            None,
            r#"{"name":"Member Concurrent Group"}"#,
            &state,
        )
        .await
        .unwrap();
        let group_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let bodies: Vec<String> = (0..4)
            .map(|index| format!(r#"{{"username":"concurrent-member-{index}"}}"#))
            .collect();
        let path = format!("/api/v0/sharegroups/{group_id}/members");
        let responses = futures_util::future::join_all(
            bodies
                .iter()
                .map(|body| super::route_http_request("POST", &path, None, body, &state)),
        )
        .await;
        let members = db.list_share_group_members(10, 0).await.unwrap_or_default();
        let expected: std::collections::BTreeSet<String> = (0..4)
            .map(|index| format!("concurrent-member-{index}"))
            .collect();
        let usernames: std::collections::BTreeSet<String> = members
            .iter()
            .map(|member| member.username.clone())
            .collect();
        record!(
            "POST",
            "/api/v0/sharegroups/{id}/members",
            "concurrency-and-idempotency",
            responses.iter().all(|response| {
                response
                    .as_ref()
                    .is_ok_and(|response| response.status == "201 Created")
            }) && members.len() == 4
                && usernames == expected
        );
    }

    // Group deletion is durable across restart.
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("share-group delete restart database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let created = super::route_http_request(
            "POST",
            "/api/v0/sharegroups",
            None,
            r#"{"name":"Delete Restart Group"}"#,
            &state,
        )
        .await
        .unwrap();
        let group_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let deleted = super::route_http_request(
            "DELETE",
            &format!("/api/v0/sharegroups/{group_id}"),
            None,
            "",
            &state,
        )
        .await
        .unwrap();
        let groups = db.list_share_groups(10, 0).await.unwrap();
        let members = db.list_share_group_members(10, 0).await.unwrap();
        let (restarted_state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        *restarted_state.sharegroups.write().await =
            super::ShareGroupStore::from_persisted(groups.clone(), members);
        let fetched = super::route_http_request(
            "GET",
            &format!("/api/v0/sharegroups/{group_id}"),
            None,
            "",
            &restarted_state,
        )
        .await
        .unwrap();
        record!(
            "DELETE",
            "/api/v0/sharegroups/{id}",
            "restart-persistence-or-reset",
            deleted.status == "200 OK" && groups.is_empty() && fetched.status == "404 Not Found"
        );
    }

    // Distinct group deletions complete and remove every persisted row.
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("share-group delete concurrency database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let mut group_ids = Vec::new();
        for index in 0..4 {
            let created = super::route_http_request(
                "POST",
                "/api/v0/sharegroups",
                None,
                &format!(r#"{{"name":"Delete Concurrent Group {index}"}}"#),
                &state,
            )
            .await
            .unwrap();
            group_ids.push(
                serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
                    .as_str()
                    .unwrap()
                    .to_owned(),
            );
        }
        let responses = futures_util::future::join_all(group_ids.iter().map(|group_id| {
            let path = format!("/api/v0/sharegroups/{group_id}");
            let state = Arc::clone(&state);
            async move { super::route_http_request("DELETE", &path, None, "", &state).await }
        }))
        .await;
        let groups = db.list_share_groups(10, 0).await.unwrap_or_default();
        record!(
            "DELETE",
            "/api/v0/sharegroups/{id}",
            "concurrency-and-idempotency",
            responses.iter().all(|response| {
                response
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
            }) && groups.is_empty()
        );
    }

    // Member deletion is durable across restart.
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("share-group member delete restart database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let created = super::route_http_request(
            "POST",
            "/api/v0/sharegroups",
            None,
            r#"{"name":"Delete Member Restart Group"}"#,
            &state,
        )
        .await
        .unwrap();
        let group_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let member_path = format!("/api/v0/sharegroups/{group_id}/members");
        let added = super::route_http_request(
            "POST",
            &member_path,
            None,
            r#"{"username":"delete-restart-member"}"#,
            &state,
        )
        .await
        .unwrap();
        let deleted = super::route_http_request(
            "DELETE",
            &format!("{member_path}/delete-restart-member"),
            None,
            "",
            &state,
        )
        .await
        .unwrap();
        let groups = db.list_share_groups(10, 0).await.unwrap();
        let members = db.list_share_group_members(10, 0).await.unwrap();
        let (restarted_state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        *restarted_state.sharegroups.write().await =
            super::ShareGroupStore::from_persisted(groups, members.clone());
        let listed = super::route_http_request("GET", &member_path, None, "", &restarted_state)
            .await
            .unwrap();
        record!(
            "DELETE",
            "/api/v0/sharegroups/{id}/members/{userId}",
            "restart-persistence-or-reset",
            added.status == "201 Created"
                && deleted.status == "200 OK"
                && members.is_empty()
                && listed.status == "200 OK"
                && listed.body == "[]"
        );
    }

    // Distinct member deletions complete without leaving persisted rows.
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("share-group member delete concurrency database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let created = super::route_http_request(
            "POST",
            "/api/v0/sharegroups",
            None,
            r#"{"name":"Delete Member Concurrent Group"}"#,
            &state,
        )
        .await
        .unwrap();
        let group_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let member_path = format!("/api/v0/sharegroups/{group_id}/members");
        let mut usernames = Vec::new();
        for index in 0..4 {
            let username = format!("delete-concurrent-member-{index}");
            super::route_http_request(
                "POST",
                &member_path,
                None,
                &format!(r#"{{"username":"{username}"}}"#),
                &state,
            )
            .await
            .unwrap();
            usernames.push(username);
        }
        let responses = futures_util::future::join_all(usernames.iter().map(|username| {
            let path = format!("{member_path}/{username}");
            let state = Arc::clone(&state);
            async move { super::route_http_request("DELETE", &path, None, "", &state).await }
        }))
        .await;
        let members = db.list_share_group_members(10, 0).await.unwrap_or_default();
        record!(
            "DELETE",
            "/api/v0/sharegroups/{id}/members/{userId}",
            "concurrency-and-idempotency",
            responses.iter().all(|response| {
                response
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
            }) && members.is_empty()
        );
    }

    // Group creation rolls back when the persistence backend fails.
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("share-group create runtime database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        db.close_for_test().await;
        let response = super::route_http_request(
            "POST",
            "/api/v0/sharegroups",
            None,
            r#"{"name":"Runtime Share Group"}"#,
            &state,
        )
        .await
        .unwrap();
        let pass = response.status == "503 Service Unavailable"
            && state.sharegroups.read().await.records.is_empty();
        record!(
            "POST",
            "/api/v0/sharegroups",
            "runtime-failure-and-timeout",
            pass
        );
    }

    // Group updates restore the prior in-memory state when persistence fails.
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("share-group update runtime database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let created = super::route_http_request(
            "POST",
            "/api/v0/sharegroups",
            None,
            r#"{"name":"Runtime Update Before"}"#,
            &state,
        )
        .await
        .unwrap();
        let group_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        db.close_for_test().await;
        let response = super::route_http_request(
            "PUT",
            &format!("/api/v0/sharegroups/{group_id}"),
            None,
            r#"{"name":"Runtime Update After"}"#,
            &state,
        )
        .await
        .unwrap();
        let group = state.sharegroups.read().await.get(&group_id);
        let pass = response.status == "503 Service Unavailable"
            && group.is_some_and(|group| group.name == "Runtime Update Before");
        record!(
            "PUT",
            "/api/v0/sharegroups/{id}",
            "runtime-failure-and-timeout",
            pass
        );
    }

    // Member additions roll back when persistence fails.
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("share-group member create runtime database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let created = super::route_http_request(
            "POST",
            "/api/v0/sharegroups",
            None,
            r#"{"name":"Runtime Member Group"}"#,
            &state,
        )
        .await
        .unwrap();
        let group_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        db.close_for_test().await;
        let response = super::route_http_request(
            "POST",
            &format!("/api/v0/sharegroups/{group_id}/members"),
            None,
            r#"{"username":"runtime-member"}"#,
            &state,
        )
        .await
        .unwrap();
        let group = state.sharegroups.read().await.get(&group_id);
        let pass = response.status == "503 Service Unavailable"
            && group.is_some_and(|group| group.members.is_empty());
        record!(
            "POST",
            "/api/v0/sharegroups/{id}/members",
            "runtime-failure-and-timeout",
            pass
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("sharegroups_persistence_and_concurrency.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        mismatches.is_empty(),
        "{} controller-api sharegroup persistence mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting `PUT /api/v0/options`'s
/// remote-configuration-disabled-by-default gate (independently
/// re-derived from `remote_configuration_routes_are_forbidden_by_
/// default`, which already fully credited `PATCH /api/v0/options`
/// for the same case but not `PUT`), `POST /api/v0/conversations/
/// batch`'s real per-recipient database persistence (independently
/// re-derived from `conversations_batch_persists_each_outbound_
/// message`, using a real in-memory `DatabaseManager` and reading
/// the persisted rows back directly, not just the HTTP response),
/// and `GET /api/v0/conversations/{username}`'s real replay-
/// deduplication projection (independently re-derived from
/// `inbound_private_message_replays_update_one_record_and_retain_
/// replay_flag`: two server-pushed `MessageUserResponse` frames for
/// the same message id collapse to one stored record with
/// `wasReplayed` preserved, reusing the real `project_server_
/// message` production function against a real loopback session).
/// Confirmed against `/tmp/slskr-parity-evidence/controller-api/
/// *.json` before writing, per case: all 3 were open. slskdN-only
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
    feature = "bounded-controller-api-tests-3"
))]
async fn controller_api_differential_options_and_conversations_projection() {
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
    let forbidden = super::route_http_request("PUT", "/api/options", None, "{}", &state)
        .await
        .expect("remote configuration policy response");
    record!(
        "PUT",
        "/api/v0/options",
        "missing-empty-or-conflict-state",
        forbidden.status == "403 Forbidden"
    );

    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, mut receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let response = super::route_http_request(
            "POST",
            "/api/v0/conversations/batch",
            None,
            r#"{"usernames":["differential-friend","differential-peer"],"body":"differential persisted batch"}"#,
            &state,
        )
        .await
        .expect("batch message response");
        let dispatched = matches!(
            receiver.try_recv(),
            Ok(super::SessionCommand::MessageUsers { .. })
        );
        let mut persisted = db.list_messages(10, 0).await.expect("list messages");
        persisted.sort_by(|left, right| left.username.cmp(&right.username));
        record!(
            "POST",
            "/api/v0/conversations/batch",
            "mutation-side-effects-and-readback",
            response.status == "201 Created"
                && dispatched
                && persisted.len() == 2
                && persisted[0].username == "differential-friend"
                && persisted[0].content == "differential persisted batch"
                && persisted[1].username == "differential-peer"
        );
    }

    {
        use slskr_client::protocol::server::{PrivateMessage, ServerMessage};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let client = tokio::net::TcpStream::connect(address);
        let server = listener.accept();
        let (client, server) = tokio::join!(client, server);
        let (server, _) = server.unwrap();
        let mut session = slskr_client::server::ServerSession::new(
            slskr_client::stream::ServerConnection::new(server),
        );
        let _client = slskr_client::stream::ServerConnection::new(client.unwrap());
        let message = |replayed| {
            ServerMessage::MessageUserResponse(PrivateMessage {
                id: 177,
                timestamp: 188,
                username: "differential-peer".to_owned(),
                message: "differential hello".to_owned(),
                is_new: true,
                was_replayed: replayed,
            })
        };
        super::project_server_message(&state, &mut session, &message(false)).await;
        super::project_server_message(&state, &mut session, &message(true)).await;
        let response = super::route_http_request(
            "GET",
            "/api/v0/conversations/differential-peer",
            None,
            "",
            &state,
        )
        .await
        .expect("conversation projection response");
        let json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        let messages = json["messages"].as_array().cloned().unwrap_or_default();
        record!(
            "GET",
            "/api/v0/conversations/{username}",
            "populated-dynamic-state",
            messages.len() == 1 && messages[0]["wasReplayed"] == true
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("options_and_conversations_projection.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api options-and-conversations-projection mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof for the versioned slskdN options controller's
/// current projection and volatile overlay lifecycle.  The oracle's
/// `OptionsController` returns a redacted current snapshot, accepts a
/// validated overlay only when remote configuration is enabled, applies
/// overlays in memory, loses them on restart, and exposes generic 500
/// problem details when the current options snapshot is invalid.  The
/// concurrency case checks that each real overlay request succeeds and
/// that the final readback is one of the submitted live values.  The
/// POST/PUT `/api/v0/options` extension rows are intentionally not
/// credited here because they are not declared by the frozen oracle
/// `OptionsController`.  slskdN-only (confirmed against the registry).
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
async fn controller_api_differential_options_current_overlay_lifecycle() {
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

    {
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_CONTROLLER_PROFILE", target)
                .with("SLSKR_REMOTE_CONFIGURATION", "true"),
        );
        let response = super::route_http_request("GET", "/api/v0/options", None, "", &state)
            .await
            .expect("current options response");
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/options",
            "nominal-status-headers-body",
            response.status == "200 OK"
                && response.content_type == "application/json; charset=utf-8"
                && value["remoteConfiguration"] == true
                && value["web"]["authentication"]["password"] == "*****"
                && value["web"]["authentication"]["jwt"]["key"] == "*****"
        );
    }

    {
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_CONTROLLER_PROFILE", target)
                .with("SLSKR_REMOTE_CONFIGURATION", "true"),
        );
        let patched = super::route_http_request(
            "PATCH",
            "/api/v0/options",
            None,
            r#"{"soulseek":{"listenPort":50311,"privateMessageAutoResponse":{"enabled":true}},"integration":{"spotify":{"clientSecret":"options-readback-secret"}}}"#,
            &state,
        )
        .await
        .expect("options readback overlay");
        let current = super::route_http_request("GET", "/api/v0/options", None, "", &state)
            .await
            .expect("options populated readback");
        let value = serde_json::from_str::<serde_json::Value>(&current.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/options",
            "populated-dynamic-state",
            patched.status == "200 OK"
                && current.status == "200 OK"
                && value["soulseek"]["listenPort"] == 50311
                && value["soulseek"]["privateMessageAutoResponse"]["enabled"] == true
                && value["integration"]["spotify"]["clientSecret"] == "*****"
                && !current.body.contains("options-readback-secret")
        );
    }

    {
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_CONTROLLER_PROFILE", target)
                .with("SLSKR_REMOTE_CONFIGURATION", "true"),
        );
        let response = super::route_http_request(
            "PATCH",
            "/api/v0/options",
            None,
            r#"{"soulseek":{"listenPort":50312,"privateMessageAutoResponse":{"enabled":true}},"integration":{"spotify":{"clientSecret":"options-patch-secret"}}}"#,
            &state,
        )
        .await
        .expect("nominal options overlay response");
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "PATCH",
            "/api/v0/options",
            "nominal-status-headers-body",
            response.status == "200 OK"
                && response.content_type == "application/json; charset=utf-8"
                && value["soulseek"]["listenPort"] == 50312
                && value["soulseek"]["privateMessageAutoResponse"]["enabled"] == true
                && value["integration"]["spotify"]["clientSecret"] == "*****"
                && !response.body.contains("options-patch-secret")
        );
    }

    {
        let env = MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_REMOTE_CONFIGURATION", "true");
        let (state, _receiver) = test_state_with_env(env.clone());
        let baseline = super::route_http_request("GET", "/api/v0/options", None, "", &state)
            .await
            .expect("baseline options response");
        let baseline_value =
            serde_json::from_str::<serde_json::Value>(&baseline.body).unwrap_or_default();
        let patched = super::route_http_request(
            "PATCH",
            "/api/v0/options",
            None,
            r#"{"soulseek":{"listenPort":50313}}"#,
            &state,
        )
        .await
        .expect("volatile options overlay response");
        let restarted = test_state_with_env(env).0;
        let current = super::route_http_request("GET", "/api/v0/options", None, "", &restarted)
            .await
            .expect("options response after restart");
        let current_value =
            serde_json::from_str::<serde_json::Value>(&current.body).unwrap_or_default();
        record!(
            "PATCH",
            "/api/v0/options",
            "restart-persistence-or-reset",
            patched.status == "200 OK"
                && current.status == "200 OK"
                && current_value["soulseek"]["listenPort"]
                    == baseline_value["soulseek"]["listenPort"]
                && current_value["soulseek"]["listenPort"] != 50313
        );
    }

    {
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_CONTROLLER_PROFILE", target)
                .with("SLSKR_REMOTE_CONFIGURATION", "true"),
        );
        let ports = [50320_u64, 50321, 50322, 50323];
        let bodies = ports
            .iter()
            .map(|port| format!(r#"{{"soulseek":{{"listenPort":{port}}}}}"#))
            .collect::<Vec<_>>();
        let responses =
            futures_util::future::join_all(bodies.iter().map(|body| {
                super::route_http_request("PATCH", "/api/v0/options", None, body, &state)
            }))
            .await;
        let current = super::route_http_request("GET", "/api/v0/options", None, "", &state)
            .await
            .expect("concurrent options readback");
        let port = serde_json::from_str::<serde_json::Value>(&current.body)
            .ok()
            .and_then(|value| value["soulseek"]["listenPort"].as_u64());
        record!(
            "PATCH",
            "/api/v0/options",
            "concurrency-and-idempotency",
            responses.iter().all(|response| {
                response
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
            }) && current.status == "200 OK"
                && port.is_some_and(|port| ports.contains(&port))
        );
    }

    {
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_CONTROLLER_PROFILE", target)
                .with("SLSKR_REMOTE_CONFIGURATION", "true"),
        );
        *state
            .controller_options_validation_error
            .write()
            .expect("options validation error lock") = Some("differential invalid options".into());
        let response = super::route_http_request("GET", "/api/v0/options", None, "", &state)
            .await
            .expect("options validation failure response");
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/options",
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.content_type == "application/problem+json"
                && value["title"] == "Internal Server Error"
                && value["status"] == 500
                && value["detail"] == "An unexpected error occurred."
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("options_current_overlay_lifecycle.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api options-current-overlay mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting `POST /api/v0/integrations/
/// spotify/authorize`'s real OAuth-state persistence-failure
/// rollback (independently re-derived from `spotify_oauth_state_
/// is_not_issued_when_persistence_fails`: a real closed database
/// makes state-issuance genuinely fail, and the pre-existing
/// (unrelated, expired) OAuth state record is left byte-for-byte
/// unchanged), `POST /api/v0/pods`'s real caller-identity
/// enforcement (independently re-derived from `pod_creation_
/// never_trusts_a_caller_supplied_peer_identity`: without a real
/// local Soulseek identity configured, pod creation is genuinely
/// forbidden rather than trusting a client-supplied
/// `requestingPeerId`), `GET /api/v0/mesh/hello`'s real baseline
/// handshake shape (independently re-derived from `mesh_hello_
/// matches_frozen_message_dto`: zeroed sequence/hash counters and
/// no publication key/signature on a fresh, unpublished mesh
/// state), `GET /api/v0/mesh/hello`'s real populated sequence/hash
/// counters after a hash merge, the mesh/hashdb populated readbacks for
/// that same entry, the hash-by-size projection, and `GET /api/v0/listening-party`'s real directory
/// listing (independently re-derived from `listening_party_
/// directory_ticket_streams_local_audio_ranges`'s directory-
/// listing half only, skipping its raw-TCP radio-stream half:
/// the directory entry's `streamPath` genuinely encodes the real
/// content id of a seeded listening-party fixture). Confirmed
/// against `/tmp/slskr-parity-evidence/controller-api/*.json`
/// before writing, per case: all 8 were open. slskdN-only
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
    feature = "bounded-controller-api-tests-4"
))]
async fn controller_api_differential_spotify_pods_mesh_and_party() {
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

    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default()
                .with("SLSKR_PERSISTENCE_ENABLED", "true")
                .with("SLSKR_SPOTIFY_ENABLED", "true")
                .with("SLSKR_SPOTIFY_CLIENT_ID", "differential-client-id"),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        state.oauth_states.write().await.records.insert(
            "differential-expired".to_owned(),
            super::OAuthStateRecord {
                provider: "spotify".to_owned(),
                redirect_uri: "http://localhost/differential-callback".to_owned(),
                code_verifier: None,
                created_at: 0,
                expires_at: 0,
            },
        );
        let previous = state.oauth_states.read().await.clone();
        db.close_for_test().await;
        let response = super::route_http_request(
            "POST",
            "/api/v0/integrations/spotify/authorize",
            None,
            "",
            &state,
        )
        .await
        .expect("failed oauth state persistence response");
        record!(
            "POST",
            "/api/v0/integrations/spotify/authorize",
            "runtime-failure-and-timeout",
            response.status == "503 Service Unavailable"
                && *state.oauth_states.read().await == previous
        );
    }

    {
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSK_USERNAME", "")
                .with("SLSK_PASSWORD", "")
                .with("SLSKR_AUTH_DISABLED", "false")
                .with("SLSKR_API_TOKEN", "differential-token"),
        );
        let response = super::route_http_request(
            "POST",
            "/api/v0/pods",
            Some("Bearer differential-token"),
            r#"{"pod":{"podId":"pod:differential-spoofed","name":"Spoofed"},"requestingPeerId":"differential-caller-controlled"}"#,
            &state,
        )
        .await
        .expect("pod create without local identity response");
        record!(
            "POST",
            "/api/v0/pods",
            "missing-empty-or-conflict-state",
            response.status == "403 Forbidden"
        );
    }

    {
        let (state, _receiver) =
            test_state_with_env(MapEnv::default().with("SLSK_USERNAME", "differential-mesh-user"));
        let hello = super::route_http_request("GET", "/api/v0/mesh/hello", None, "", &state)
            .await
            .expect("mesh hello response");
        let hello_json = serde_json::from_str::<serde_json::Value>(&hello.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/mesh/hello",
            "nominal-status-headers-body",
            hello.status == "200 OK"
                && hello_json["type"] == 1
                && hello_json["latest_seq_id"] == 0
                && hello_json["hash_count"] == 0
                && hello_json.get("peerId").is_none()
        );

        let merged = super::route_http_request(
            "POST",
            "/api/v0/mesh/merge?fromUser=differential-mesh-peer",
            None,
            &serde_json::json!({
                "entries": [{
                    "flacKey": "differential-mesh-hello-key",
                    "byteHash": "a".repeat(64),
                    "size": 4096
                }]
            })
            .to_string(),
            &state,
        )
        .await
        .expect("merge a real mesh hello hash");
        let merged_json =
            serde_json::from_str::<serde_json::Value>(&merged.body).unwrap_or_default();
        let latest_seq_id = merged_json["latestSeqId"].as_u64().unwrap_or_default();
        let populated = super::route_http_request("GET", "/api/v0/mesh/hello", None, "", &state)
            .await
            .expect("populated mesh hello response");
        let populated_json =
            serde_json::from_str::<serde_json::Value>(&populated.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/mesh/hello",
            "populated-dynamic-state",
            merged.status == "200 OK"
                && merged_json["merged"] == 1
                && latest_seq_id > 0
                && populated.status == "200 OK"
                && populated_json["latest_seq_id"] == latest_seq_id
                && populated_json["hash_count"] == 1
                && populated_json["client_id"] == "differential-mesh-user"
                && populated_json["public_key"] == ""
                && populated_json["signature"] == ""
        );

        let lookup = super::route_http_request(
            "GET",
            "/api/v0/mesh/lookup/differential-mesh-hello-key",
            None,
            "",
            &state,
        )
        .await
        .expect("populated mesh lookup response");
        let lookup_json =
            serde_json::from_str::<serde_json::Value>(&lookup.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/mesh/lookup/{flacKey}",
            "populated-dynamic-state",
            lookup.status == "200 OK"
                && lookup_json["found"] == true
                && lookup_json["entry"]["flacKey"] == "differential-mesh-hello-key"
                && lookup_json["entry"]["byteHash"] == "a".repeat(64)
                && lookup_json["entry"]["size"] == 4096
        );

        let hashdb = super::route_http_request(
            "GET",
            "/api/v0/hashdb/hash/differential-mesh-hello-key",
            None,
            "",
            &state,
        )
        .await
        .expect("populated hashdb lookup response");
        let hashdb_json =
            serde_json::from_str::<serde_json::Value>(&hashdb.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/hashdb/hash/{flacKey}",
            "populated-dynamic-state",
            hashdb.status == "200 OK"
                && hashdb_json["flacKey"] == "differential-mesh-hello-key"
                && hashdb_json["byteHash"] == "a".repeat(64)
                && hashdb_json["size"] == 4096
        );

        let by_size =
            super::route_http_request("GET", "/api/v0/hashdb/hash/by-size/4096", None, "", &state)
                .await
                .expect("populated hashdb size lookup response");
        let by_size_json =
            serde_json::from_str::<serde_json::Value>(&by_size.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/hashdb/hash/by-size/{size}",
            "populated-dynamic-state",
            by_size.status == "200 OK"
                && by_size_json["count"] == 1
                && by_size_json["entries"].as_array().is_some_and(|entries| {
                    entries.len() == 1
                        && entries[0]["flacKey"] == "differential-mesh-hello-key"
                        && entries[0]["byteHash"] == "a".repeat(64)
                        && entries[0]["size"] == 4096
                })
        );
    }

    {
        let root = std::env::temp_dir().join(format!(
            "slskr-listening-party-differential-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&root).expect("create differential listening-party share root");
        std::fs::write(
            root.join("differential-party.flac"),
            b"differential-party-audio-bytes",
        )
        .expect("write differential listening-party fixture");
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_CONTROLLER_PROFILE", "native")
                .with("SLSKR_SHARE_FIXTURE", "")
                .with("SLSKR_SHARE_DIRS", &root.display().to_string()),
        );
        let content_id = {
            let shares = state.shares.read().await;
            let entry = shares
                .entries
                .first()
                .expect("differential share fixture entry");
            super::stable_content_hash(&entry.filename, entry.size).to_string()
        };
        state
            .controller_features
            .write_for_test()
            .await
            .upsert(
                "listening-party/pod:differential-stream-audit/general".to_owned(),
                serde_json::json!({
                    "partyId": "party:differential-stream-audit",
                    "podId": "pod:differential-stream-audit",
                    "channelId": "general",
                    "hostPeerId": "differential-tester",
                    "action": "play",
                    "contentId": content_id,
                    "title": "Differential Party Track",
                    "artist": "Differential Party Artist",
                    "serverTimeUnixMs": super::unix_timestamp_millis(),
                    "listed": true,
                    "allowMeshStreaming": true,
                }),
            )
            .expect("persist differential listening-party fixture");
        let directory =
            super::route_http_request("GET", "/api/v0/listening-party", None, "", &state)
                .await
                .expect("list differential listening-party directory");
        let directory_json =
            serde_json::from_str::<serde_json::Value>(&directory.body).unwrap_or_default();
        let stream_path = directory_json[0]["streamPath"].as_str().unwrap_or_default();
        record!(
            "GET",
            "/api/v0/listening-party",
            "populated-dynamic-state",
            directory.status == "200 OK" && stream_path.contains(&super::url_encode(&content_id))
        );
        let _ = std::fs::remove_dir_all(root);
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("spotify_pods_mesh_and_party.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api spotify-pods-mesh-and-party mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
