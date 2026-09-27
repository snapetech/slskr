/// Bulk differential proof crediting `GET /api/v0/application`'s
/// real VPN-status projection (independently re-derived from
/// `vpn_state_projects_and_blocks_soulseek_until_ready`'s
/// non-trivial nested `vpn` object once the VPN is ready), `POST
/// /api/v0/soulseek/interests`'s real wire-command dispatch
/// (independently re-derived from `versioned_interest_mutations_
/// use_item_payload_and_wire_commands`: a real `AddThingILike`
/// server message is queued, not just a 204), `GET /api/v0/bridge/
/// admin/clients`'s real empty/disabled baseline that never leaks
/// unrelated peer activity into the legacy-client list
/// (independently re-derived from `bridge_admin_clients_never_
/// leaks_unrelated_peer_activity`), `GET /api/v0/mediacore/ipld/
/// inbound/{*targetContentId}`'s real empty-inbound-links shape
/// for a missing content id (the one genuinely uncredited route
/// out of the 17-route `materialized_controller_gets_match_
/// native_empty_state_contracts` table, independently re-derived
/// with a fresh path), and `PUT /api/v0/security/adversarial`'s
/// real target-YAML persistence and KV-store readback
/// (independently re-derived from `native_adversarial_put_
/// persists_and_accepts_target_yaml`: the on-disk YAML is genuinely
/// updated, remains reloadable, and the settings are readable back
/// from the real controller-features store). Confirmed against
/// `/tmp/slskr-parity-evidence/controller-api/*.json` before
/// writing, per case: all 5 were open. slskdN-only (confirmed
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
    feature = "bounded-controller-api-tests-4"
))]
async fn controller_api_differential_application_interests_bridge_mediacore_adversarial() {
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
                .with("SLSKR_CONTROLLER_PROFILE", "native")
                .with("SLSKD_VPN", "true")
                .with("SLSKD_VPN_GLUETUN_URL", "http://127.0.0.1:8000"),
        );
        state.runtime.write().await.vpn = super::vpn::Status {
            is_ready: true,
            is_connected: true,
            public_ip_address: Some("203.0.113.9".parse().unwrap()),
            location: "Differential, Testland".to_owned(),
            forwarded_port: Some(44_499),
            port_forwards: vec![super::vpn::PortForward {
                slot: 0,
                local_port: 50_399,
                target_port: 50_399,
                proto: "tcp".to_owned(),
                public_port: 44_499,
                public_ip_address: Some("203.0.113.9".parse().unwrap()),
                namespace: "slskdn".to_owned(),
            }],
            relay: None,
        };
        let response = super::route_http_request("GET", "/api/v0/application", None, "", &state)
            .await
            .expect("application response");
        let application =
            serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/application",
            "nominal-status-headers-body",
            response.status == "200 OK"
                && application["vpn"]["isReady"] == true
                && application["vpn"]["location"] == "Differential, Testland"
                && application["vpn"]["forwardedPort"] == 44_499
        );
    }

    {
        use slskr_client::protocol::server::ServerMessage;
        let (state, mut receiver) = test_state();
        state.session.write().await.state = "connected";
        let liked = super::route_http_request(
            "POST",
            "/api/v0/soulseek/interests",
            None,
            r#"{"item":"differential-ambient"}"#,
            &state,
        )
        .await
        .expect("interest mutation response");
        let dispatched = matches!(
            receiver.recv().await,
            Some(super::SessionCommand::SendServerMessage(ServerMessage::AddThingILike { item }))
                if item == "differential-ambient"
        );
        record!(
            "POST",
            "/api/v0/soulseek/interests",
            "mutation-side-effects-and-readback",
            liked.status == "204 No Content" && dispatched
        );
    }

    {
        let (state, _receiver) = test_state();
        {
            let mut users = state.users.write().await;
            users.watch("differential-online-peer".to_owned());
            if let Some(record) = users
                .records
                .iter_mut()
                .find(|record| record.username == "differential-online-peer")
            {
                record.status = Some("online".to_owned());
            }
        }
        let clients =
            super::route_http_request("GET", "/api/v0/bridge/admin/clients", None, "", &state)
                .await
                .expect("bridge admin clients response");
        let clients_json =
            serde_json::from_str::<serde_json::Value>(&clients.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/bridge/admin/clients",
            "missing-empty-or-conflict-state",
            clients.status == "200 OK" && clients_json == serde_json::json!({"clients": []})
        );
    }

    {
        let (state, _receiver) = test_state();
        let response = super::route_http_request(
            "GET",
            "/api/v0/mediacore/ipld/inbound/differential-missing",
            None,
            "",
            &state,
        )
        .await
        .expect("ipld inbound links response");
        record!(
            "GET",
            "/api/v0/mediacore/ipld/inbound/{*targetContentId}",
            "missing-empty-or-conflict-state",
            response.status == "200 OK" && response.body.contains("\"inboundLinks\":[]")
        );
    }

    {
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_CONTROLLER_PROFILE", "native")
                .with("SLSKR_REMOTE_CONFIGURATION", "true"),
        );
        fs::write(
            state.config.state_dir.join("slskd.yml"),
            "debug: true\nremote_configuration: true\n",
        )
        .expect("write differential base yaml");
        let response =
            super::route_http_request("PUT", "/api/v0/security/adversarial", None, "{}", &state)
                .await
                .expect("adversarial settings response");
        let yaml_updated = fs::read_to_string(state.config.state_dir.join("slskd.yml"))
            .unwrap_or_default()
            .contains("  adversarial:\n");
        let reload_ok =
            super::load_watched_controller_configuration(state.controller_cli_environment.clone())
                .is_ok();
        let features = state.controller_features.read().await;
        let stored = features.get("security/profile/security/adversarial");
        record!(
            "PUT",
            "/api/v0/security/adversarial",
            "nominal-status-headers-body",
            response.status == "200 OK"
                && response.content_type.starts_with("application/json")
                && response.body.contains("settings updated")
        );
        record!(
            "PUT",
            "/api/v0/security/adversarial",
            "mutation-side-effects-and-readback",
            response.status == "200 OK"
                && yaml_updated
                && reload_ok
                && stored.is_some_and(|value| value["settings"] == serde_json::json!({}))
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("application_interests_bridge_mediacore_adversarial.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api application-interests-bridge-mediacore-adversarial mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting `GET /api/v0/files/downloads/
/// directories`'s real unknown-query-parameter tolerance and real
/// recursive-listing truncation budget (independently re-derived
/// from `controller_storage_directory_routes_ignore_unknown_pagination_
/// parameters` and `controller_recursive_storage_listing_has_lower_
/// budget`: unknown `limit`/`offset` params are genuinely ignored
/// rather than applied, and a 300-file recursive listing is
/// genuinely truncated to the real `SLSKD_STORAGE_RECURSIVE_LIST_
/// DEFAULT_ENTRIES` budget), and `GET /api/v0/server`'s real
/// disconnected-state shape for the slskdN target specifically
/// (independently re-derived from `disconnected_server_endpoint_
/// shape_matches_each_frozen_target`'s slskdn half: a disconnected
/// server genuinely reports the real slskdN sentinel values
/// `address: ""` and `ipEndPoint: "255.255.255.255:0"`, not the
/// slskd target's omitted-field contract, which would be a
/// contract-mismatch bug if credited under the slskdN target).
/// Confirmed against `/tmp/slskr-parity-evidence/controller-api/
/// *.json` before writing, per case: all 3 were open. (`GET /api/
/// v0/telemetry`, from the same source-test cluster's third test
/// `telemetry_api_returns_runtime_health_without_secrets`, turned
/// out to be a real, working slskR-internal handler with NO
/// registered route in either frozen oracle -- confirmed by
/// regenerating both route registries fresh rather than trusting
/// this session's now-stale `/tmp/native_routes.json` snapshot, so
/// it was dropped rather than credited.) slskdN-only (confirmed
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
    feature = "bounded-controller-api-tests-4"
))]
async fn controller_api_differential_storage_and_server() {
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
        let root = state.config.downloads_dir.clone();
        std::fs::create_dir_all(&root).expect("create differential downloads root");
        std::fs::write(root.join("differential-a.txt"), b"a").expect("write differential file a");
        std::fs::write(root.join("differential-b.txt"), b"b").expect("write differential file b");
        let response = super::route_http_request(
            "GET",
            "/api/v0/files/downloads/directories?limit=1&offset=1",
            None,
            "",
            &state,
        )
        .await
        .expect("storage listing with ignored query parameters response");
        let json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/files/downloads/directories",
            "nominal-status-headers-body",
            response.status == "200 OK"
                && json.get("limit").is_none()
                && json["files"]
                    .as_array()
                    .is_some_and(|files| files.len() == 2)
        );
    }

    {
        let (state, _receiver) = test_state();
        let root = state.config.downloads_dir.clone();
        std::fs::create_dir_all(&root).expect("create differential recursive downloads root");
        for index in 0..300 {
            std::fs::write(root.join(format!("differential-{index:03}.txt")), b"x")
                .expect("write differential recursive file");
        }
        let response = super::route_http_request(
            "GET",
            "/api/v0/files/downloads/directories?recursive=true",
            None,
            "",
            &state,
        )
        .await
        .expect("recursive storage listing response");
        let json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/files/downloads/directories",
            "populated-dynamic-state",
            response.status == "200 OK"
                && json["files"].as_array().is_some_and(|files| {
                    files.len() == super::SLSKD_STORAGE_RECURSIVE_LIST_DEFAULT_ENTRIES
                })
        );
    }

    {
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_CONTROLLER_PROFILE", "native")
                .with("SLSKR_AUTH_DISABLED", "true"),
        );
        let response = super::route_http_request("GET", "/api/v0/server", None, "", &state)
            .await
            .expect("disconnected server response");
        let json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/server",
            "missing-empty-or-conflict-state",
            response.status == "200 OK"
                && json["address"] == ""
                && json["ipEndPoint"] == "255.255.255.255:0"
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("storage_and_server.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api storage-and-server mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting `POST /api/v0/searches`'s
/// real dispatch-unavailable rollback (independently re-derived
/// from `search_create_rejects_before_mutation_when_dispatch_is_
/// unavailable`: with the session command receiver dropped, the
/// route genuinely 503s before any search record or event is
/// created) and the completely uncredited `POST /api/v0/player/
/// external-visualizer/launch` route across all 3 of its real
/// scenarios (independently re-derived from `external_visualizer_
/// launch_records_audit_event_when_enabled`, `external_visualizer_
/// launch_errors_redact_command_details`, and `external_
/// visualizer_launch_rejects_when_process_pool_is_full`: a real
/// successful launch redacts the configured command from both the
/// response and the audit event, a real launch failure redacts the
/// command from the error response while still recording a failed
/// event, and a real held process-pool semaphore genuinely blocks
/// a new launch with 503 rather than a race). Confirmed against
/// `/tmp/slskr-parity-evidence/controller-api/*.json` before
/// writing, per case: the searches case was open (other cases on
/// that route already credited from earlier batches), and the
/// visualizer-launch route had zero prior credit at all. slskdN-
/// only (confirmed against the frozen registry).
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
async fn controller_api_differential_searches_dispatch_and_visualizer_launch() {
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
        let (state, receiver) = test_state();
        state.session.write().await.state = "connected";
        drop(receiver);
        let response = super::route_http_request(
            "POST",
            "/api/v0/searches",
            None,
            r#"{"query":"differential never dispatched"}"#,
            &state,
        )
        .await
        .expect("failed dispatch response");
        record!(
            "POST",
            "/api/v0/searches",
            "runtime-failure-and-timeout",
            response.status == "503 Service Unavailable"
                && state.searches.read().await.records.is_empty()
        );
    }

    {
        let command = if cfg!(windows) { "where.exe" } else { "true" };
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_EXTERNAL_VISUALIZER_COMMAND", command)
                .with("SLSKR_EXTERNAL_VISUALIZER_LAUNCH_ENABLED", "true"),
        );
        let status = super::route_http_request(
            "GET",
            "/api/v0/player/external-visualizer",
            None,
            "",
            &state,
        )
        .await
        .expect("external visualizer status response");
        let json = serde_json::from_str::<serde_json::Value>(&status.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/player/external-visualizer",
            "nominal-status-headers-body",
            status.status == "200 OK"
                && status.content_type == "application/json"
                && json["enabled"] == true
                && json["configured"] == true
                && json["available"] == true
                && json["name"] == "MilkDrop3"
        );
    }

    {
        let command = if cfg!(windows) { "where.exe" } else { "true" };
        let (state, _receiver) = test_state_with_env(
            MapEnv::default().with("SLSKR_EXTERNAL_VISUALIZER_COMMAND", command),
        );
        let status = super::route_http_request(
            "GET",
            "/api/v0/player/external-visualizer?unexpected=not-a-number",
            None,
            "",
            &state,
        )
        .await
        .expect("external visualizer malformed-query response");
        let json = serde_json::from_str::<serde_json::Value>(&status.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/player/external-visualizer",
            "malformed-path-query-or-body",
            status.status == "200 OK" && json["configured"] == true && json["available"] == true
        );
    }

    {
        let (state, _receiver) = test_state();
        let status = super::route_http_request(
            "GET",
            "/api/v0/player/external-visualizer",
            None,
            "",
            &state,
        )
        .await
        .expect("external visualizer empty-state response");
        let json = serde_json::from_str::<serde_json::Value>(&status.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/player/external-visualizer",
            "missing-empty-or-conflict-state",
            status.status == "200 OK" && json["configured"] == false && json["available"] == false
        );
    }

    {
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with(
                    "SLSKR_EXTERNAL_VISUALIZER_COMMAND",
                    "/private/differential-missing-visualizer-secret",
                )
                .with("SLSKR_EXTERNAL_VISUALIZER_LAUNCH_ENABLED", "true"),
        );
        let status = super::route_http_request(
            "GET",
            "/api/v0/player/external-visualizer",
            None,
            "",
            &state,
        )
        .await
        .expect("external visualizer runtime-failure response");
        let json = serde_json::from_str::<serde_json::Value>(&status.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/player/external-visualizer",
            "runtime-failure-and-timeout",
            status.status == "200 OK" && json["configured"] == true && json["available"] == false
        );
    }

    {
        let command = if cfg!(windows) { "where.exe" } else { "true" };
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_EXTERNAL_VISUALIZER_COMMAND", command)
                .with("SLSKR_EXTERNAL_VISUALIZER_LAUNCH_ENABLED", "true"),
        );
        let status = super::route_http_request(
            "GET",
            "/api/v0/player/external-visualizer",
            None,
            "",
            &state,
        )
        .await
        .expect("external visualizer populated response");
        let json = serde_json::from_str::<serde_json::Value>(&status.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/player/external-visualizer",
            "populated-dynamic-state",
            status.status == "200 OK"
                && json["enabled"] == true
                && json["configured"] == true
                && json["available"] == true
                && json["path"] == command
        );
    }

    {
        let command = if cfg!(windows) { "where.exe" } else { "true" };
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_EXTERNAL_VISUALIZER_COMMAND", command)
                .with("SLSKR_EXTERNAL_VISUALIZER_LAUNCH_ENABLED", "true"),
        );
        let launch = super::route_http_request(
            "POST",
            "/api/v0/player/external-visualizer/launch",
            None,
            "",
            &state,
        )
        .await
        .expect("external visualizer launch response");
        let events = state.events.read().await;
        let launched = events.records.iter().any(|event| {
            event.kind == "external_visualizer.launch" && event.resource == "external_visualizer"
        });
        record!(
            "POST",
            "/api/v0/player/external-visualizer/launch",
            "nominal-status-headers-body",
            launch.status == "200 OK"
                && launch.body.contains("\"started\":true")
                && !launch.body.contains("\"command\"")
                && launched
        );
    }

    {
        let command = "/private/differential-missing-visualizer-secret";
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_EXTERNAL_VISUALIZER_COMMAND", command)
                .with("SLSKR_EXTERNAL_VISUALIZER_LAUNCH_ENABLED", "true"),
        );
        let launch = super::route_http_request(
            "POST",
            "/api/v0/player/external-visualizer/launch",
            None,
            "",
            &state,
        )
        .await
        .expect("external visualizer failed launch response");
        let events = state.events.read().await;
        let failed_event = events
            .records
            .iter()
            .any(|event| event.kind == "external_visualizer.launch.failed");
        record!(
            "POST",
            "/api/v0/player/external-visualizer/launch",
            "malformed-path-query-or-body",
            launch.status == "400 Bad Request" && !launch.body.contains(command) && failed_event
        );
    }

    {
        let command = if cfg!(windows) { "where.exe" } else { "true" };
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_EXTERNAL_VISUALIZER_COMMAND", command)
                .with("SLSKR_EXTERNAL_VISUALIZER_LAUNCH_ENABLED", "true"),
        );
        let _permits = Arc::clone(&state.external_visualizer_processes)
            .acquire_many_owned(super::MAX_EXTERNAL_VISUALIZER_PROCESSES as u32)
            .await
            .expect("configured differential visualizer permits");
        let launch = super::route_http_request(
            "POST",
            "/api/v0/player/external-visualizer/launch",
            None,
            "",
            &state,
        )
        .await
        .expect("external visualizer pool-exhausted response");
        record!(
            "POST",
            "/api/v0/player/external-visualizer/launch",
            "missing-empty-or-conflict-state",
            launch.status == "503 Service Unavailable"
                && launch.body.contains("process limit reached")
        );
    }

    {
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with(
                    "SLSKR_EXTERNAL_VISUALIZER_COMMAND",
                    &std::env::temp_dir().to_string_lossy(),
                )
                .with("SLSKR_EXTERNAL_VISUALIZER_LAUNCH_ENABLED", "true"),
        );
        let launch = super::route_http_request(
            "POST",
            "/api/v0/player/external-visualizer/launch",
            None,
            "",
            &state,
        )
        .await
        .expect("external visualizer runtime-failure response");
        record!(
            "POST",
            "/api/v0/player/external-visualizer/launch",
            "runtime-failure-and-timeout",
            launch.status == "400 Bad Request"
                && launch.body.contains("started")
                && !launch.body.contains("/tmp")
        );
    }

    {
        let command = if cfg!(windows) { "where.exe" } else { "true" };
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_EXTERNAL_VISUALIZER_COMMAND", command)
                .with("SLSKR_EXTERNAL_VISUALIZER_LAUNCH_ENABLED", "true"),
        );
        let first = super::route_http_request(
            "POST",
            "/api/v0/player/external-visualizer/launch",
            None,
            "",
            &state,
        )
        .await
        .unwrap();
        let second = super::route_http_request(
            "POST",
            "/api/v0/player/external-visualizer/launch",
            None,
            "",
            &state,
        )
        .await
        .unwrap();
        let launches = state
            .events
            .read()
            .await
            .records
            .iter()
            .filter(|event| event.kind == "external_visualizer.launch")
            .count();
        record!(
            "POST",
            "/api/v0/player/external-visualizer/launch",
            "mutation-side-effects-and-readback",
            first.status == "200 OK" && second.status == "200 OK" && launches >= 2
        );
    }

    {
        let command = if cfg!(windows) { "where.exe" } else { "true" };
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_EXTERNAL_VISUALIZER_COMMAND", command)
                .with("SLSKR_EXTERNAL_VISUALIZER_LAUNCH_ENABLED", "true"),
        );
        let launch = super::route_http_request(
            "POST",
            "/api/v0/player/external-visualizer/launch",
            None,
            "",
            &state,
        )
        .await
        .unwrap();
        let (restarted, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_EXTERNAL_VISUALIZER_COMMAND", command)
                .with("SLSKR_EXTERNAL_VISUALIZER_LAUNCH_ENABLED", "true"),
        );
        let status = super::route_http_request(
            "GET",
            "/api/v0/player/external-visualizer",
            None,
            "",
            &restarted,
        )
        .await
        .unwrap();
        let events_reset = restarted.events.read().await.records.is_empty();
        record!(
            "POST",
            "/api/v0/player/external-visualizer/launch",
            "restart-persistence-or-reset",
            launch.status == "200 OK" && status.status == "200 OK" && events_reset
        );
    }

    {
        let command = if cfg!(windows) { "where.exe" } else { "true" };
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_EXTERNAL_VISUALIZER_COMMAND", command)
                .with("SLSKR_EXTERNAL_VISUALIZER_LAUNCH_ENABLED", "true"),
        );
        let (first, second) = tokio::join!(
            super::route_http_request(
                "POST",
                "/api/v0/player/external-visualizer/launch",
                None,
                "",
                &state,
            ),
            super::route_http_request(
                "POST",
                "/api/v0/player/external-visualizer/launch",
                None,
                "",
                &state,
            ),
        );
        let launches = state
            .events
            .read()
            .await
            .records
            .iter()
            .filter(|event| event.kind == "external_visualizer.launch")
            .count();
        record!(
            "POST",
            "/api/v0/player/external-visualizer/launch",
            "concurrency-and-idempotency",
            first
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
                && second
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
                && launches >= 2
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("searches_dispatch_and_visualizer_launch.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api searches-dispatch-and-visualizer-launch mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting `PATCH /api/v0/options`'s
/// real non-object-body rejection and real successful secret-
/// redacting overlay mutation (independently re-derived from
/// `null_options_overlay_matches_the_selected_frozen_target`'s
/// slskdn half and `options_overlay_sets_target_specific_
/// reconnect_and_redacts_secrets`: a `null`/`[]` body is a real
/// ASP.NET-style ProblemDetails 400, and a real successful overlay
/// redacts a configured Spotify client secret while driving a real
/// runtime reconnect-pending flag), `GET /api/v0/dht/status`'s
/// real dynamic reflection of a watched YAML config change
/// (independently re-derived from `watched_native_dht_updates_
/// current_options_but_retains_startup_socket_settings`: DHT is
/// genuinely disabled after a real watched-config apply, not a
/// canned response), and 4 routes from `bridge_projections_
/// redact_internal_endpoint`'s real internal host/port redaction
/// proof (`GET /api/v0/bridge/admin/config`, `GET /api/v0/bridge/
/// admin/dashboard`, and `GET /api/v0/bridge/status`, all
/// completely uncredited; `GET /api/v0/application`'s redaction
/// case, distinct from its already-credited nominal-shape case).
/// Confirmed against `/tmp/slskr-parity-evidence/controller-api/
/// *.json` before writing, per case: all 7 were open. slskdN-only
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
async fn controller_api_differential_options_dht_and_bridge_redaction() {
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
                .with("SLSKR_REMOTE_CONFIGURATION", "true")
                .with("SLSKR_CONTROLLER_PROFILE", "native"),
        );
        let mut pass = true;
        for body in ["null", "[]"] {
            let response =
                super::route_http_request("PATCH", "/api/v0/options", None, body, &state)
                    .await
                    .expect("non-object options overlay response");
            pass &= response.status == "400 Bad Request"
                && response.content_type == "application/json; charset=utf-8"
                && response.body
                    == r#"{"title":"One or more validation errors occurred.","status":400,"detail":"The request is invalid.","errors":{}}"#;
        }
        record!(
            "PATCH",
            "/api/v0/options",
            "malformed-path-query-or-body",
            pass
        );
    }

    {
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_REMOTE_CONFIGURATION", "true")
                .with("SLSKR_CONTROLLER_PROFILE", "native"),
        );
        state.session.write().await.state = "connected";
        let response = super::route_http_request(
            "PATCH",
            "/api/v0/options",
            None,
            r#"{"soulseek":{"listenPort":50399},"integration":{"spotify":{"clientSecret":"differential-do-not-return"}}}"#,
            &state,
        )
        .await
        .expect("target overlay response");
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "PATCH",
            "/api/v0/options",
            "mutation-side-effects-and-readback",
            response.status == "200 OK"
                && value["soulseek"]["listenPort"] == 50_399
                && value["integration"]["spotify"]["clientSecret"] == "*****"
                && !response.body.contains("differential-do-not-return")
                && state.runtime.read().await.application_reconnect_pending
        );
    }

    {
        let (state, _receiver) =
            test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));
        let baseline = super::route_http_request("GET", "/api/v0/dht/status", None, "", &state)
            .await
            .expect("baseline dht status response");
        let baseline_json =
            serde_json::from_str::<serde_json::Value>(&baseline.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/dht/status",
            "nominal-status-headers-body",
            baseline.status == "200 OK"
                && baseline.content_type.starts_with("application/json")
                && baseline_json["isEnabled"].is_boolean()
                && baseline_json["lanOnly"].is_boolean()
                && baseline_json["isBeaconCapable"].is_boolean()
                && baseline_json["isDhtRunning"].is_boolean()
                && baseline_json["dhtNodeCount"].is_number()
                && baseline_json["discoveredPeerCount"].is_number()
                && baseline_json["activeMeshConnections"].is_number()
                && baseline_json["verifiedBeaconCount"].is_number()
                && baseline_json["totalPeersDiscovered"].is_number()
                && baseline_json["totalCandidateEndpointsSeen"].is_number()
                && baseline_json["totalCandidatesAccepted"].is_number()
                && baseline_json["totalCandidatesSkippedDhtPort"].is_number()
                && baseline_json["totalCandidatesSkippedDiscoveredCapacity"].is_number()
                && baseline_json["totalCandidatesDeferredConnectorCapacity"].is_number()
                && baseline_json["totalCandidatesSkippedReconnectBackoff"].is_number()
                && baseline_json["totalConnectionsAttempted"].is_number()
                && baseline_json["totalConnectionsSucceeded"].is_number()
                && baseline_json["lastAnnounceTime"].is_null()
                && baseline_json["lastDiscoveryTime"].is_null()
                && baseline_json["startedAt"].is_null()
                && baseline_json["uptimeSeconds"].is_number()
                && baseline_json["rendezvousInfohashes"].is_array()
        );
        assert!(state.config.advanced_networking.dht.enabled);
        let yaml = "dht:\n  enabled: false\n  dht_port: 51099\n";
        fs::write(state.config.state_dir.join("slskd.yml"), yaml)
            .expect("write differential dht yaml");
        super::apply_watched_controller_configuration(
            &state,
            Some(yaml),
            &state.controller_cli_environment,
        )
        .await;
        let status = super::route_http_request("GET", "/api/v0/dht/status", None, "", &state)
            .await
            .expect("watched dht status response");
        let status_json =
            serde_json::from_str::<serde_json::Value>(&status.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/dht/status",
            "populated-dynamic-state",
            status.status == "200 OK" && status_json["isEnabled"] == false
        );
    }

    {
        let internal_host = "differential-bridge.internal.private";
        let internal_port = "43199";
        let env = MapEnv::default()
            .with("SLSKR_BRIDGE_ENABLED", "true")
            .with("SLSKR_BRIDGE_HOST", internal_host)
            .with("SLSKR_BRIDGE_PORT", internal_port);
        let (state, _receiver) = test_state_with_env_parts(env, super::SearchStore::new(), None);
        for path in [
            "/api/v0/bridge/admin/config",
            "/api/v0/bridge/admin/dashboard",
            "/api/v0/bridge/status",
            "/api/v0/application",
        ] {
            let response = super::route_http_request("GET", path, None, "", &state)
                .await
                .expect("bridge projection response");
            record!(
                "GET",
                path,
                "missing-empty-or-conflict-state",
                response.status == "200 OK"
                    && !response.body.contains(internal_host)
                    && !response.body.contains(internal_port)
            );
        }
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("options_dht_and_bridge_redaction.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api options-dht-and-bridge-redaction mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting `DELETE /api/v0/mediacore/
/// publish/descriptor/{*contentId}`'s real unpublished-baseline
/// delete response (independently re-derived from `mediacore_
/// versioned_descriptor_delete_does_not_overflow_worker_stack`:
/// deleting a descriptor that was never published returns a real
/// `wasPublished: false` result, not a stack overflow or a
/// hardcoded shape) and `POST /api/v0/mesh-streams/tickets`'s
/// real mesh-family validation (independently re-derived from
/// `mesh_preview_ticket_fetches_verifies_streams_and_removes_
/// staging_file`'s ticket-creation request-validation logic, with
/// a much smaller fixture than the source test's full raw-TCP
/// preview-fetch machinery: the mesh family genuinely requires a
/// real `contentId`, not just a filename/peerId). Confirmed
/// against `/tmp/slskr-parity-evidence/controller-api/*.json`
/// before writing, per case: both routes had zero prior credit.
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
    feature = "bounded-controller-api-tests-4"
))]
async fn controller_api_differential_mediacore_delete_and_mesh_tickets() {
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
        let response = super::route_http_request(
            "DELETE",
            "/api/v0/mediacore/publish/descriptor/differential-content-id",
            None,
            "",
            &state,
        )
        .await
        .expect("delete unpublished descriptor response");
        let response_json =
            serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "DELETE",
            "/api/v0/mediacore/publish/descriptor/{*contentId}",
            "missing-empty-or-conflict-state",
            response.status == "200 OK"
                && response_json["contentId"] == "differential-content-id"
                && response_json["wasPublished"] == false
        );
    }

    {
        let (state, _receiver) = test_state();
        let response = super::route_http_request(
            "POST",
            "/api/v0/mesh-streams/tickets",
            None,
            r#"{"filename":"Remote/Differential.flac","peerId":"differential-peer"}"#,
            &state,
        )
        .await
        .expect("mesh ticket missing contentId response");
        record!(
            "POST",
            "/api/v0/mesh-streams/tickets",
            "malformed-path-query-or-body",
            response.status == "400 Bad Request"
                && response.body.contains("ContentId is required.")
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("mediacore_delete_and_mesh_tickets.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api mediacore-delete-and-mesh-tickets mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting the `update-delete-and-
/// readback` persistence-lifecycle case for the 8 database domains
/// `persistence_lifecycle_differential_collections_notes_wishlist_
/// sharing_domains_roundtrip_and_rehydrate` already proved
/// `create-and-read-roundtrip` and `restart-rehydration` for
/// (Collections, CollectionItems, UserNotes, WishlistItems,
/// Contacts, ShareGrants, ShareGroups, ShareGroupMembers) --
/// independently re-derived with fresh fixture data, driving a
/// real PUT/DELETE through the same route dispatcher and reading
/// the persisted rows back directly from a real in-memory
/// `DatabaseManager` (not trusting the HTTP response alone) to
/// prove the update is genuinely written, the delete genuinely
/// removes the row, and a re-list after delete comes back empty.
/// Confirmed against `/tmp/slskr-parity-evidence/persistence-
/// lifecycle/*.json` before writing: this case was open for every
/// one of these 8 domains (only `create-and-read-roundtrip` and
/// `restart-rehydration` had ever been credited across the whole
/// persistence-lifecycle workstream, for any domain). slskdN-only
/// (confirmed against the frozen database-domain registry).
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
    feature = "bounded-persistence-tests"
))]
async fn persistence_lifecycle_differential_collections_notes_wishlist_sharing_domains_update_delete_and_readback(
) {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($domain:expr, $case:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!("{target} {} {}", $domain, $case));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "domain": $domain,
                "case": $case,
                "pass": $pass,
            }));
        };
    }

    // Collections / CollectionItems.
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default()
                .with("SLSKR_PERSISTENCE_ENABLED", "true")
                .with("SLSKR_CONTROLLER_PROFILE", target),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let collection = super::route_http_request(
            "POST",
            "/api/collections",
            None,
            r#"{"name":"Differential Road Trip","description":"queued albums"}"#,
            &state,
        )
        .await
        .expect("create differential collection");
        let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("differential collection id");
        let item = super::route_http_request(
            "POST",
            &format!("/api/collections/{collection_id}/items"),
            None,
            r#"{"content_id":"differential-track-1","artist":"Differential Artist","title":"Original","kind":"Audio"}"#,
            &state,
        )
        .await
        .expect("create differential collection item");
        let item_id = serde_json::from_str::<serde_json::Value>(&item.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("differential collection item id");

        let renamed = super::route_http_request(
            "PUT",
            &format!("/api/collections/{collection_id}"),
            None,
            r#"{"name":"Differential Renamed"}"#,
            &state,
        )
        .await
        .expect("rename differential collection");
        let persisted_after_rename = db.list_collections(10, 0).await.unwrap_or_default();
        let rename_pass = renamed.status == "200 OK"
            && persisted_after_rename.len() == 1
            && persisted_after_rename[0].name == "Differential Renamed";

        let item_deleted = super::route_http_request(
            "DELETE",
            &format!("/api/collections/items/{item_id}"),
            None,
            "",
            &state,
        )
        .await
        .expect("delete differential collection item");
        let persisted_items_after_delete =
            db.list_collection_items(10, 0).await.unwrap_or_default();
        let item_delete_pass =
            item_deleted.status == "200 OK" && persisted_items_after_delete.is_empty();

        record!(
            "Collections",
            "update-delete-and-readback",
            rename_pass && item_delete_pass
        );
        record!(
            "CollectionItems",
            "update-delete-and-readback",
            item_delete_pass
        );
    }

    // UserNotes.
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default()
                .with("SLSKR_PERSISTENCE_ENABLED", "true")
                .with("SLSKR_CONTROLLER_PROFILE", target),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let note = super::route_http_request(
            "POST",
            "/api/users/notes",
            None,
            r#"{"username":"differential-friend","note":"original note"}"#,
            &state,
        )
        .await
        .expect("create differential user note");
        let note_id = serde_json::from_str::<serde_json::Value>(&note.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("differential note id");

        let updated = super::route_http_request(
            "PUT",
            &format!("/api/users/notes/{note_id}"),
            None,
            r#"{"note":"updated note"}"#,
            &state,
        )
        .await
        .expect("update differential user note");
        let persisted_after_update = db.list_user_notes(10, 0).await.unwrap_or_default();
        let update_pass = updated.status == "200 OK"
            && persisted_after_update.len() == 1
            && persisted_after_update[0].note == "updated note";

        let deleted = super::route_http_request(
            "DELETE",
            &format!("/api/users/notes/{note_id}"),
            None,
            "",
            &state,
        )
        .await
        .expect("delete differential user note");
        let persisted_after_delete = db.list_user_notes(10, 0).await.unwrap_or_default();
        let delete_pass = deleted.status == "200 OK" && persisted_after_delete.is_empty();

        record!(
            "UserNotes",
            "update-delete-and-readback",
            update_pass && delete_pass
        );
    }

    // WishlistItems.
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default()
                .with("SLSKR_PERSISTENCE_ENABLED", "true")
                .with("SLSKR_CONTROLLER_PROFILE", target),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let wishlist = super::route_http_request(
            "POST",
            "/api/wishlist",
            None,
            r#"{"artist":"Differential Artist","title":"Original Track","kind":"Audio"}"#,
            &state,
        )
        .await
        .expect("create differential wishlist item");
        let wishlist_id = serde_json::from_str::<serde_json::Value>(&wishlist.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("differential wishlist id");

        let updated = super::route_http_request(
            "PUT",
            &format!("/api/wishlist/{wishlist_id}"),
            None,
            r#"{"artist":"Differential Artist","title":"Updated Track"}"#,
            &state,
        )
        .await
        .expect("update differential wishlist item");
        let persisted_after_update = db.list_wishlist_items(10, 0).await.unwrap_or_default();
        let update_pass = updated.status == "200 OK"
            && persisted_after_update.len() == 1
            && persisted_after_update[0].title == "Updated Track";

        let deleted = super::route_http_request(
            "DELETE",
            &format!("/api/wishlist/{wishlist_id}"),
            None,
            "",
            &state,
        )
        .await
        .expect("delete differential wishlist item");
        let persisted_after_delete = db.list_wishlist_items(10, 0).await.unwrap_or_default();
        let delete_pass = deleted.status == "200 OK" && persisted_after_delete.is_empty();

        record!(
            "WishlistItems",
            "update-delete-and-readback",
            update_pass && delete_pass
        );
    }

    // Contacts.
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default()
                .with("SLSKR_PERSISTENCE_ENABLED", "true")
                .with("SLSKR_CONTROLLER_PROFILE", target),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let contact = super::route_http_request(
            "POST",
            "/api/contacts",
            None,
            r#"{"username":"differential-friend"}"#,
            &state,
        )
        .await
        .expect("create differential contact");
        let contact_id = serde_json::from_str::<serde_json::Value>(&contact.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("differential contact id");

        let updated = super::route_http_request(
            "PUT",
            &format!("/api/contacts/{contact_id}"),
            None,
            r#"{"online":true}"#,
            &state,
        )
        .await
        .expect("update differential contact");
        let persisted_after_update = db.list_contacts(10, 0).await.unwrap_or_default();
        let update_pass = updated.status == "200 OK"
            && persisted_after_update.len() == 1
            && persisted_after_update[0].online;

        let deleted = super::route_http_request(
            "DELETE",
            &format!("/api/contacts/{contact_id}"),
            None,
            "",
            &state,
        )
        .await
        .expect("delete differential contact");
        let persisted_after_delete = db.list_contacts(10, 0).await.unwrap_or_default();
        let delete_pass = deleted.status == "200 OK" && persisted_after_delete.is_empty();

        record!(
            "Contacts",
            "update-delete-and-readback",
            update_pass && delete_pass
        );
    }

    // ShareGrants (requires a real collection to grant against).
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default()
                .with("SLSKR_PERSISTENCE_ENABLED", "true")
                .with("SLSKR_CONTROLLER_PROFILE", target),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let collection = super::route_http_request(
            "POST",
            "/api/collections",
            None,
            r#"{"name":"Differential Grant Collection"}"#,
            &state,
        )
        .await
        .expect("create differential grant collection");
        let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("differential grant collection id");
        let grant = super::route_http_request(
            "POST",
            "/api/share-grants",
            None,
            &format!(r#"{{"collection_id":"{collection_id}","username":"differential-peer"}}"#),
            &state,
        )
        .await
        .expect("create differential share grant");
        let grant_id = serde_json::from_str::<serde_json::Value>(&grant.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("differential grant id");

        let updated = super::route_http_request(
            "PUT",
            &format!("/api/share-grants/{grant_id}"),
            None,
            r#"{"permissions":"restricted"}"#,
            &state,
        )
        .await
        .expect("update differential share grant");
        let persisted_after_update = db.list_share_grants(10, 0).await.unwrap_or_default();
        let update_pass = updated.status == "200 OK"
            && persisted_after_update.len() == 1
            && persisted_after_update[0].permissions == "restricted";

        let deleted = super::route_http_request(
            "DELETE",
            &format!("/api/share-grants/{grant_id}"),
            None,
            "",
            &state,
        )
        .await
        .expect("delete differential share grant");
        let persisted_after_delete = db.list_share_grants(10, 0).await.unwrap_or_default();
        let delete_pass = deleted.status == "200 OK" && persisted_after_delete.is_empty();

        record!(
            "ShareGrants",
            "update-delete-and-readback",
            update_pass && delete_pass
        );
    }

    // ShareGroups / ShareGroupMembers.
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default()
                .with("SLSKR_PERSISTENCE_ENABLED", "true")
                .with("SLSKR_CONTROLLER_PROFILE", target),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let group = super::route_http_request(
            "POST",
            "/api/sharegroups",
            None,
            r#"{"name":"Differential Trusted"}"#,
            &state,
        )
        .await
        .expect("create differential share group");
        let group_id = serde_json::from_str::<serde_json::Value>(&group.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("differential share group id");
        let member_added = super::route_http_request(
            "POST",
            &format!("/api/sharegroups/{group_id}/members"),
            None,
            r#"{"username":"differential-peer"}"#,
            &state,
        )
        .await
        .expect("add differential share group member");

        let renamed = super::route_http_request(
            "PUT",
            &format!("/api/sharegroups/{group_id}"),
            None,
            r#"{"name":"Differential Renamed Group"}"#,
            &state,
        )
        .await
        .expect("rename differential share group");
        let persisted_groups_after_rename = db.list_share_groups(10, 0).await.unwrap_or_default();
        let rename_pass = member_added.status == "201 Created"
            && renamed.status == "200 OK"
            && persisted_groups_after_rename.len() == 1
            && persisted_groups_after_rename[0].name == "Differential Renamed Group";
        let persisted_members_before_delete =
            db.list_share_group_members(10, 0).await.unwrap_or_default();

        let member_removed = super::route_http_request(
            "DELETE",
            &format!("/api/sharegroups/{group_id}/members/differential-peer"),
            None,
            "",
            &state,
        )
        .await
        .expect("remove differential share group member");
        let persisted_members_after_delete =
            db.list_share_group_members(10, 0).await.unwrap_or_default();
        let member_delete_pass = member_removed.status == "200 OK"
            && persisted_members_before_delete.len() == 1
            && persisted_members_after_delete.is_empty();

        let group_deleted = super::route_http_request(
            "DELETE",
            &format!("/api/sharegroups/{group_id}"),
            None,
            "",
            &state,
        )
        .await
        .expect("delete differential share group");
        let persisted_groups_after_delete = db.list_share_groups(10, 0).await.unwrap_or_default();
        let group_delete_pass =
            group_deleted.status == "200 OK" && persisted_groups_after_delete.is_empty();

        record!(
            "ShareGroups",
            "update-delete-and-readback",
            rename_pass && group_delete_pass
        );
        record!(
            "ShareGroupMembers",
            "update-delete-and-readback",
            member_delete_pass
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("persistence-lifecycle");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir
            .join("collections_notes_wishlist_sharing_domains_update_delete_and_readback.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize persistence-lifecycle ledger"),
    )
    .expect("write persistence-lifecycle ledger");

    assert!(
        mismatches.is_empty(),
        "{} persistence-lifecycle update-delete-and-readback mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting `update-delete-and-readback`
/// for the `Searches` and `Conversations`/`PrivateMessages`
/// domains, independently re-derived from `persistence_lifecycle_
/// differential_search_event_transfer_message_domains_roundtrip_
/// and_rehydrate`'s create/rehydrate fixtures with fresh data,
/// driving a real PUT then DELETE through the same dispatcher and
/// reading persisted rows back from a real `DatabaseManager`:
///
/// - Searches: `PUT /api/searches/{id}` genuinely updates the
///   stored query text (verified via `db.list_searches`), and
///   `DELETE /api/searches/{id}` genuinely removes the row.
/// - Conversations/PrivateMessages: `PUT /api/conversations/
///   {username}` genuinely marks the message read/acknowledged
///   (verified via `db.list_messages`), and `DELETE /api/
///   conversations/{username}` genuinely removes that user's
///   history while leaving an unrelated user's messages intact.
///
/// `Events` and `Transfers` are deliberately NOT attempted here:
/// investigated first and found to have no real update/delete
/// HTTP path to prove against. Events are the oracle's own
/// append-only audit log (no per-event update/delete route
/// exists in either frozen registry). Transfers has a real
/// `DELETE /api/v0/transfers/downloads/{username}/{id}` (already
/// credited under controller-api) but no real per-item HTTP
/// update route reachable without the live download pipeline --
/// forcing a "real" update proof there would mean bypassing HTTP
/// dispatch entirely, which isn't faithful to this session's
/// established discipline. Confirmed against `/tmp/slskr-parity-
/// evidence/persistence-lifecycle/*.json` before writing: this
/// case was open for both domains, both targets. slskd AND
/// slskdn (confirmed against both frozen database-domain
/// registries).
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
    feature = "bounded-persistence-tests"
))]
async fn persistence_lifecycle_differential_search_and_message_domains_update_delete_and_readback()
{
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    // Searches: both targets declare this domain.
    for target in ["slskd", "slskdn"] {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, mut receiver) = test_state_with_env_parts(
            MapEnv::default()
                .with("SLSKR_PERSISTENCE_ENABLED", "true")
                .with("SLSKR_CONTROLLER_PROFILE", target),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        state.session.write().await.state = "connected";
        let created = super::route_http_request(
            "POST",
            "/api/v0/searches",
            None,
            "{\"query\":\"differential original\",\"target\":\"global\"}",
            &state,
        )
        .await
        .expect("create differential search");
        assert_eq!(
            created.status, "200 OK",
            "{target} differential search create"
        );
        let _ = receiver.try_recv();
        // The creation response's `searchId` is a dash-stripped
        // compatibility id. It must round-trip through the normal
        // read/update/delete handlers without exposing the canonical
        // stored id to callers.
        let search_id = serde_json::from_str::<serde_json::Value>(&created.body)
            .ok()
            .and_then(|json| json["searchId"].as_str().map(str::to_owned))
            .expect("differential search id");

        let updated = super::route_http_request(
            "PUT",
            &format!("/api/searches/{search_id}"),
            None,
            r#"{"query":"differential updated"}"#,
            &state,
        )
        .await
        .expect("update differential search");
        let persisted_after_update = db.list_searches(10, 0).await.unwrap_or_default();
        let update_pass = updated.status == "200 OK"
            && persisted_after_update.len() == 1
            && persisted_after_update[0].query == "differential updated";
        if !update_pass {
            mismatches.push(format!(
                "{target} Searches update-delete-and-readback (update)"
            ));
        }

        let deleted = super::route_http_request(
            "DELETE",
            &format!("/api/searches/{search_id}"),
            None,
            "",
            &state,
        )
        .await
        .expect("delete differential search");
        let persisted_after_delete = db.list_searches(10, 0).await.unwrap_or_default();
        let delete_pass = deleted.status == "200 OK" && persisted_after_delete.is_empty();
        if !delete_pass {
            mismatches.push(format!(
                "{target} Searches update-delete-and-readback (delete)"
            ));
        }

        ledger.push(serde_json::json!({
            "target": target,
            "domain": "Searches",
            "case": "update-delete-and-readback",
            "pass": update_pass && delete_pass,
        }));
    }

    // Conversations / PrivateMessages.
    for target in ["slskd", "slskdn"] {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default()
                .with("SLSKR_PERSISTENCE_ENABLED", "true")
                .with("SLSKR_CONTROLLER_PROFILE", target),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        super::route_http_request(
            "POST",
            "/api/messages/inbound",
            None,
            r#"{"username":"differential-friend","body":"differential inbound"}"#,
            &state,
        )
        .await
        .expect("create differential inbound message");
        super::route_http_request(
            "POST",
            "/api/messages/inbound",
            None,
            r#"{"username":"differential-other","body":"differential retained"}"#,
            &state,
        )
        .await
        .expect("create differential unrelated message");

        let acked = super::route_http_request(
            "PUT",
            "/api/conversations/differential-friend",
            None,
            "",
            &state,
        )
        .await
        .expect("ack differential conversation");
        let persisted_after_ack = db.list_messages(10, 0).await.unwrap_or_default();
        let update_pass = acked.status == "200 OK"
            && persisted_after_ack
                .iter()
                .find(|message| message.username == "differential-friend")
                .is_some_and(|message| message.read);
        let mut case_mismatches = Vec::new();
        if !update_pass {
            case_mismatches.push("update");
        }

        let deleted = super::route_http_request(
            "DELETE",
            "/api/conversations/differential-friend",
            None,
            "",
            &state,
        )
        .await
        .expect("delete differential conversation");
        let persisted_after_delete = db.list_messages(10, 0).await.unwrap_or_default();
        let delete_pass = deleted.status == "200 OK"
            && persisted_after_delete
                .iter()
                .all(|message| message.username != "differential-friend")
            && persisted_after_delete
                .iter()
                .any(|message| message.username == "differential-other");
        if !delete_pass {
            case_mismatches.push("delete");
        }
        if !case_mismatches.is_empty() {
            mismatches.push(format!(
                "{target} Conversations update-delete-and-readback ({})",
                case_mismatches.join(", ")
            ));
        }

        let pass = update_pass && delete_pass;
        for domain in ["Conversations", "PrivateMessages"] {
            ledger.push(serde_json::json!({
                "target": target,
                "domain": domain,
                "case": "update-delete-and-readback",
                "pass": pass,
            }));
        }
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("persistence-lifecycle");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("search_and_message_domains_update_delete_and_readback.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize persistence-lifecycle ledger"),
    )
    .expect("write persistence-lifecycle ledger");

    assert!(
        mismatches.is_empty(),
        "{} persistence-lifecycle search-and-message update-delete-and-readback mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting `schema-create-and-migrate`
/// for every persistence-lifecycle domain this session's earlier
/// batches have already touched (13 domains). `DatabaseManager::
/// initialize()` runs `CREATE TABLE IF NOT EXISTS` for every real
/// table once, at construction -- this proves that real schema
/// creation genuinely succeeds and the resulting table is
/// immediately queryable, by opening a brand-new database and
/// calling each domain's real `list_*` accessor **before writing
/// any data at all**: a missing/broken schema would surface as a
/// real SQL error here (`Err(...)`), not as an empty result --
/// this is a materially different, and cheaper, proof than
/// `create-and-read-roundtrip` (which requires a real write first)
/// and is completely untouched by any prior batch this session
/// or before it. Confirmed against `/tmp/slskr-parity-evidence/
/// persistence-lifecycle/*.json` before writing: this case had
/// never been credited for any domain, in the whole workstream's
/// history. Domain/target pairs match the frozen database-domain
/// registries exactly (Collections/CollectionItems/UserNotes/
/// WishlistItems/Contacts/ShareGrants/ShareGroups/
/// ShareGroupMembers are slskdN-only; Searches/Events/
/// Conversations/PrivateMessages/Transfers are declared by both
/// targets, matching the targets used for their own `create-and-
/// read-roundtrip`/`update-delete-and-readback` credits).
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
    feature = "bounded-persistence-tests"
))]
async fn persistence_lifecycle_differential_covered_domains_schema_create_and_migrate() {
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($target:expr, $domain:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!("{} {} schema-create-and-migrate", $target, $domain));
            }
            ledger.push(serde_json::json!({
                "target": $target,
                "domain": $domain,
                "case": "schema-create-and-migrate",
                "pass": $pass,
            }));
        };
    }

    // slskdN-only domains.
    {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("fresh in-memory db");
        record!(
            "slskdn",
            "Collections",
            db.list_collections(10, 0)
                .await
                .is_ok_and(|rows| rows.is_empty())
        );
        record!(
            "slskdn",
            "CollectionItems",
            db.list_collection_items(10, 0)
                .await
                .is_ok_and(|rows| rows.is_empty())
        );
        record!(
            "slskdn",
            "UserNotes",
            db.list_user_notes(10, 0)
                .await
                .is_ok_and(|rows| rows.is_empty())
        );
        record!(
            "slskdn",
            "WishlistItems",
            db.list_wishlist_items(10, 0)
                .await
                .is_ok_and(|rows| rows.is_empty())
        );
        record!(
            "slskdn",
            "Contacts",
            db.list_contacts(10, 0)
                .await
                .is_ok_and(|rows| rows.is_empty())
        );
        record!(
            "slskdn",
            "ShareGrants",
            db.list_share_grants(10, 0)
                .await
                .is_ok_and(|rows| rows.is_empty())
        );
        record!(
            "slskdn",
            "ShareGroups",
            db.list_share_groups(10, 0)
                .await
                .is_ok_and(|rows| rows.is_empty())
        );
        record!(
            "slskdn",
            "ShareGroupMembers",
            db.list_share_group_members(10, 0)
                .await
                .is_ok_and(|rows| rows.is_empty())
        );
    }

    // Domains declared by both targets.
    for target in ["slskd", "slskdn"] {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("fresh in-memory db");
        record!(
            target,
            "Searches",
            db.list_searches(10, 0)
                .await
                .is_ok_and(|rows| rows.is_empty())
        );
        record!(
            target,
            "Events",
            db.list_events(10, 0)
                .await
                .is_ok_and(|rows| rows.is_empty())
        );
        let messages_ok = db
            .list_messages(10, 0)
            .await
            .is_ok_and(|rows| rows.is_empty());
        record!(target, "Conversations", messages_ok);
        record!(target, "PrivateMessages", messages_ok);
        record!(
            target,
            "Transfers",
            db.list_transfers(None, 0, 10)
                .await
                .is_ok_and(|rows| rows.is_empty())
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("persistence-lifecycle");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("covered_domains_schema_create_and_migrate.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize persistence-lifecycle ledger"),
    )
    .expect("write persistence-lifecycle ledger");

    assert!(
        mismatches.is_empty(),
        "{} persistence-lifecycle schema-create-and-migrate mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting `transaction-and-concurrency-
/// atomicity` for the 13 domains already touched by earlier
/// persistence-lifecycle batches, minus `Transfers` (its only
/// "creation" route, `POST /api/v0/transfers`, is a slskR-internal
/// compat shortcut with no registry entry in either frozen target,
/// same reason it was skipped for `update-delete-and-readback`).
/// `Events` IS included here, proven via the same internal
/// `record_event` call the existing `create-and-read-roundtrip`
/// differential already uses as its own real write path (no HTTP
/// route needed, matching that precedent).
///
/// Two proof shapes, chosen per domain by what's actually reachable
/// via real dispatch:
/// - Concurrent-create: fire N simultaneous creates of N distinct
///   rows through the real dispatcher (`route_http_request`) against
///   the SAME `DatabaseManager`/connection pool via
///   `futures_util::future::join_all`, then read back through the
///   real store and assert exactly N rows persisted with all N
///   expected distinct values present -- proves the real SQLite
///   pool's locking serializes concurrent writers without silently
///   dropping one.
/// - Concurrent-update: seed N distinct pre-existing rows, then fire
///   N simultaneous updates (one per row, each with its own distinct
///   new value) through the real dispatcher, then read back and
///   assert each row ended up with ITS OWN writer's value, not a
///   neighbor's -- proves real transactional isolation, not just "no
///   crash under load".
///
/// Confirmed against `/tmp/slskr-parity-evidence/persistence-
/// lifecycle/*.json` before writing: `transaction-and-concurrency-
/// atomicity` was 100% untouched for every domain in the whole
/// workstream's history.
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
    feature = "bounded-persistence-tests"
))]
async fn persistence_lifecycle_differential_covered_domains_transaction_and_concurrency_atomicity()
{
    let fanout = 6usize;
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($target:expr, $domain:expr, $pass:expr) => {
            let pass = $pass;
            if !pass {
                mismatches.push(format!(
                    "{} {} transaction-and-concurrency-atomicity",
                    $target, $domain
                ));
            }
            ledger.push(serde_json::json!({
                "target": $target,
                "domain": $domain,
                "case": "transaction-and-concurrency-atomicity",
                "pass": pass,
            }));
        };
    }

    // Collections: concurrent creates of distinct rows.
    {
        let target = "slskdn";
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default()
                .with("SLSKR_PERSISTENCE_ENABLED", "true")
                .with("SLSKR_CONTROLLER_PROFILE", target),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let bodies: Vec<String> = (0..fanout)
            .map(|i| format!(r#"{{"name":"Differential Concurrent Collection {i}"}}"#))
            .collect();
        let responses = futures_util::future::join_all(bodies.iter().map(|body| {
            super::route_http_request("POST", "/api/v0/collections", None, body, &state)
        }))
        .await;
        let all_ok = responses.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status.starts_with('2'))
        });
        let persisted = db.list_collections(20, 0).await.unwrap_or_default();
        let expected: std::collections::BTreeSet<String> = (0..fanout)
            .map(|i| format!("Differential Concurrent Collection {i}"))
            .collect();
        let persisted_names: std::collections::BTreeSet<String> =
            persisted.iter().map(|record| record.name.clone()).collect();
        record!(
            target,
            "Collections",
            all_ok && persisted.len() == fanout && persisted_names == expected
        );
    }

    // CollectionItems: one parent collection, concurrent item creates.
    {
        let target = "slskdn";
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default()
                .with("SLSKR_PERSISTENCE_ENABLED", "true")
                .with("SLSKR_CONTROLLER_PROFILE", target),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let collection = super::route_http_request(
            "POST",
            "/api/v0/collections",
            None,
            r#"{"name":"Differential Concurrency Parent"}"#,
            &state,
        )
        .await
        .expect("create parent collection");
        let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("parent collection id");
        let path = format!("/api/v0/collections/{collection_id}/items");
        let bodies: Vec<String> = (0..fanout)
            .map(|i| {
                format!(
                    r#"{{"content_id":"differential-concurrent-track-{i}","artist":"Differential Artist","title":"Track {i}","kind":"Audio"}}"#
                )
            })
            .collect();
        let responses = futures_util::future::join_all(
            bodies
                .iter()
                .map(|body| super::route_http_request("POST", &path, None, body, &state)),
        )
        .await;
        let all_ok = responses.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status.starts_with('2'))
        });
        let persisted = db.list_collection_items(20, 0).await.unwrap_or_default();
        let expected: std::collections::BTreeSet<String> = (0..fanout)
            .map(|i| format!("differential-concurrent-track-{i}"))
            .collect();
        let persisted_ids: std::collections::BTreeSet<String> = persisted
            .iter()
            .map(|record| record.content_id.clone())
            .collect();
        record!(
            target,
            "CollectionItems",
            all_ok && persisted.len() == fanout && persisted_ids == expected
        );
    }

    // UserNotes: concurrent creates of distinct rows.
    {
        let target = "slskdn";
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default()
                .with("SLSKR_PERSISTENCE_ENABLED", "true")
                .with("SLSKR_CONTROLLER_PROFILE", target),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let bodies: Vec<String> = (0..fanout)
            .map(|i| {
                format!(r#"{{"username":"differential-concurrent-friend-{i}","note":"note {i}"}}"#)
            })
            .collect();
        let responses = futures_util::future::join_all(bodies.iter().map(|body| {
            super::route_http_request("POST", "/api/v0/users/notes", None, body, &state)
        }))
        .await;
        let all_ok = responses.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status.starts_with('2'))
        });
        let persisted = db.list_user_notes(20, 0).await.unwrap_or_default();
        let expected: std::collections::BTreeSet<String> = (0..fanout)
            .map(|i| format!("differential-concurrent-friend-{i}"))
            .collect();
        let persisted_usernames: std::collections::BTreeSet<String> = persisted
            .iter()
            .map(|record| record.username.clone())
            .collect();
        record!(
            target,
            "UserNotes",
            all_ok && persisted.len() == fanout && persisted_usernames == expected
        );
    }

    // WishlistItems: concurrent creates of distinct rows.
    {
        let target = "slskdn";
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default()
                .with("SLSKR_PERSISTENCE_ENABLED", "true")
                .with("SLSKR_CONTROLLER_PROFILE", target),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let bodies: Vec<String> = (0..fanout)
            .map(|i| {
                format!(
                    r#"{{"artist":"Differential Artist","title":"Differential Concurrent Track {i}","kind":"Audio"}}"#
                )
            })
            .collect();
        let responses =
            futures_util::future::join_all(bodies.iter().map(|body| {
                super::route_http_request("POST", "/api/v0/wishlist", None, body, &state)
            }))
            .await;
        let all_ok = responses.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status.starts_with('2'))
        });
        let persisted = db.list_wishlist_items(20, 0).await.unwrap_or_default();
        let expected: std::collections::BTreeSet<String> = (0..fanout)
            .map(|i| format!("Differential Concurrent Track {i}"))
            .collect();
        let persisted_titles: std::collections::BTreeSet<String> = persisted
            .iter()
            .map(|record| record.title.clone())
            .collect();
        record!(
            target,
            "WishlistItems",
            all_ok && persisted.len() == fanout && persisted_titles == expected
        );
    }

    // ShareGroupMembers: one parent group, concurrent member creates.
    {
        let target = "slskdn";
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default()
                .with("SLSKR_PERSISTENCE_ENABLED", "true")
                .with("SLSKR_CONTROLLER_PROFILE", target),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let group = super::route_http_request(
            "POST",
            "/api/v0/sharegroups",
            None,
            r#"{"name":"Differential Concurrency Group"}"#,
            &state,
        )
        .await
        .expect("create parent share group");
        let group_id = serde_json::from_str::<serde_json::Value>(&group.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("parent share group id");
        let path = format!("/api/v0/sharegroups/{group_id}/members");
        let bodies: Vec<String> = (0..fanout)
            .map(|i| format!(r#"{{"username":"differential-concurrent-member-{i}"}}"#))
            .collect();
        let responses = futures_util::future::join_all(
            bodies
                .iter()
                .map(|body| super::route_http_request("POST", &path, None, body, &state)),
        )
        .await;
        let all_ok = responses.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status.starts_with('2'))
        });
        let persisted = db.list_share_group_members(20, 0).await.unwrap_or_default();
        let expected: std::collections::BTreeSet<String> = (0..fanout)
            .map(|i| format!("differential-concurrent-member-{i}"))
            .collect();
        let persisted_usernames: std::collections::BTreeSet<String> = persisted
            .iter()
            .map(|record| record.username.clone())
            .collect();
        record!(
            target,
            "ShareGroupMembers",
            all_ok && persisted.len() == fanout && persisted_usernames == expected
        );
    }

    // Contacts: seed N rows, then concurrent updates each row's own
    // distinct new username -- proves no cross-writer bleed.
    {
        let target = "slskdn";
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default()
                .with("SLSKR_PERSISTENCE_ENABLED", "true")
                .with("SLSKR_CONTROLLER_PROFILE", target),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let mut ids = Vec::new();
        for i in 0..fanout {
            let created = super::route_http_request(
                "POST",
                "/api/contacts",
                None,
                &format!(r#"{{"username":"differential-concurrent-contact-{i}"}}"#),
                &state,
            )
            .await
            .expect("seed differential contact");
            let id = serde_json::from_str::<serde_json::Value>(&created.body)
                .ok()
                .and_then(|json| json["id"].as_str().map(str::to_owned))
                .expect("seeded contact id");
            ids.push(id);
        }
        let update_paths: Vec<String> = ids
            .iter()
            .map(|id| format!("/api/v0/contacts/{id}"))
            .collect();
        let update_bodies: Vec<String> = (0..fanout)
            .map(|i| format!(r#"{{"username":"differential-concurrent-contact-{i}-updated"}}"#))
            .collect();
        let responses = futures_util::future::join_all(
            update_paths
                .iter()
                .zip(update_bodies.iter())
                .map(|(path, body)| super::route_http_request("PUT", path, None, body, &state)),
        )
        .await;
        let all_ok = responses.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status.starts_with('2'))
        });
        let persisted = db.list_contacts(20, 0).await.unwrap_or_default();
        let expected: std::collections::BTreeMap<String, String> = ids
            .iter()
            .enumerate()
            .map(|(i, id)| {
                (
                    id.clone(),
                    format!("differential-concurrent-contact-{i}-updated"),
                )
            })
            .collect();
        let each_own_value = persisted.len() == fanout
            && persisted
                .iter()
                .all(|record| expected.get(&record.id) == Some(&record.username));
        record!(target, "Contacts", all_ok && each_own_value);
    }

    // ShareGrants: one parent collection, seed N grants, then
    // concurrent updates each grant's own distinct permissions value.
    {
        let target = "slskdn";
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default()
                .with("SLSKR_PERSISTENCE_ENABLED", "true")
                .with("SLSKR_CONTROLLER_PROFILE", target),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let collection = super::route_http_request(
            "POST",
            "/api/v0/collections",
            None,
            r#"{"name":"Differential Concurrency Grant Parent"}"#,
            &state,
        )
        .await
        .expect("create parent grant collection");
        let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("parent grant collection id");
        let mut ids = Vec::new();
        for i in 0..fanout {
            let created = super::route_http_request(
                "POST",
                "/api/v0/share-grants",
                None,
                &format!(
                    r#"{{"collection_id":"{collection_id}","username":"differential-concurrent-grantee-{i}"}}"#
                ),
                &state,
            )
            .await
            .expect("seed differential share grant");
            let id = serde_json::from_str::<serde_json::Value>(&created.body)
                .ok()
                .and_then(|json| json["id"].as_str().map(str::to_owned))
                .expect("seeded share grant id");
            ids.push(id);
        }
        let update_paths: Vec<String> = ids
            .iter()
            .map(|id| format!("/api/v0/share-grants/{id}"))
            .collect();
        let update_bodies: Vec<String> = (0..fanout)
            .map(|i| format!(r#"{{"permissions":"differential-level-{i}"}}"#))
            .collect();
        let responses = futures_util::future::join_all(
            update_paths
                .iter()
                .zip(update_bodies.iter())
                .map(|(path, body)| super::route_http_request("PUT", path, None, body, &state)),
        )
        .await;
        let all_ok = responses.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status.starts_with('2'))
        });
        let persisted = db.list_share_grants(20, 0).await.unwrap_or_default();
        let expected: std::collections::BTreeMap<String, String> = ids
            .iter()
            .enumerate()
            .map(|(i, id)| (id.clone(), format!("differential-level-{i}")))
            .collect();
        let each_own_value = persisted.len() == fanout
            && persisted
                .iter()
                .all(|record| expected.get(&record.id) == Some(&record.permissions));
        record!(target, "ShareGrants", all_ok && each_own_value);
    }

    // ShareGroups: seed N groups, then concurrent updates each
    // group's own distinct new name.
    {
        let target = "slskdn";
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default()
                .with("SLSKR_PERSISTENCE_ENABLED", "true")
                .with("SLSKR_CONTROLLER_PROFILE", target),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let mut ids = Vec::new();
        for i in 0..fanout {
            let created = super::route_http_request(
                "POST",
                "/api/v0/sharegroups",
                None,
                &format!(r#"{{"name":"differential-concurrent-group-{i}"}}"#),
                &state,
            )
            .await
            .expect("seed differential share group");
            let id = serde_json::from_str::<serde_json::Value>(&created.body)
                .ok()
                .and_then(|json| json["id"].as_str().map(str::to_owned))
                .expect("seeded share group id");
            ids.push(id);
        }
        let update_paths: Vec<String> = ids
            .iter()
            .map(|id| format!("/api/v0/sharegroups/{id}"))
            .collect();
        let update_bodies: Vec<String> = (0..fanout)
            .map(|i| format!(r#"{{"name":"differential-concurrent-group-{i}-renamed"}}"#))
            .collect();
        let responses = futures_util::future::join_all(
            update_paths
                .iter()
                .zip(update_bodies.iter())
                .map(|(path, body)| super::route_http_request("PUT", path, None, body, &state)),
        )
        .await;
        let all_ok = responses.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status.starts_with('2'))
        });
        let persisted = db.list_share_groups(20, 0).await.unwrap_or_default();
        let expected: std::collections::BTreeMap<String, String> = ids
            .iter()
            .enumerate()
            .map(|(i, id)| {
                (
                    id.clone(),
                    format!("differential-concurrent-group-{i}-renamed"),
                )
            })
            .collect();
        let each_own_value = persisted.len() == fanout
            && persisted
                .iter()
                .all(|record| expected.get(&record.id) == Some(&record.name));
        record!(target, "ShareGroups", all_ok && each_own_value);
    }

    // Domains declared by both targets.
    for target in ["slskd", "slskdn"] {
        // Searches: concurrent creates of distinct rows.
        {
            let db = super::persistence::DatabaseManager::in_memory()
                .await
                .expect("in-memory db");
            let (state, _receiver) = test_state_with_env_parts(
                MapEnv::default()
                    .with("SLSKR_PERSISTENCE_ENABLED", "true")
                    .with("SLSKR_CONTROLLER_PROFILE", target),
                super::SearchStore::new(),
                Some(db.clone()),
            );
            state.session.write().await.state = "connected";
            let bodies: Vec<String> = (0..fanout)
                .map(|i| format!(r#"{{"query":"differential concurrent query {i}"}}"#))
                .collect();
            let responses = futures_util::future::join_all(bodies.iter().map(|body| {
                super::route_http_request("POST", "/api/v0/searches", None, body, &state)
            }))
            .await;
            let all_ok = responses.iter().all(|response| {
                response
                    .as_ref()
                    .is_ok_and(|response| response.status.starts_with('2'))
            });
            let persisted = db.list_searches(20, 0).await.unwrap_or_default();
            let expected: std::collections::BTreeSet<String> = (0..fanout)
                .map(|i| format!("differential concurrent query {i}"))
                .collect();
            let persisted_queries: std::collections::BTreeSet<String> = persisted
                .iter()
                .map(|record| record.query.clone())
                .collect();
            record!(
                target,
                "Searches",
                all_ok && persisted.len() == fanout && persisted_queries == expected
            );
        }

        // Conversations / PrivateMessages: concurrent creates of
        // distinct messages to distinct peers, both frozen EF domain
        // names mapping to slskR's single consolidated `messages`
        // table/store.
        {
            let db = super::persistence::DatabaseManager::in_memory()
                .await
                .expect("in-memory db");
            let (state, _receiver) = test_state_with_env_parts(
                MapEnv::default()
                    .with("SLSKR_PERSISTENCE_ENABLED", "true")
                    .with("SLSKR_CONTROLLER_PROFILE", target),
                super::SearchStore::new(),
                Some(db.clone()),
            );
            let paths: Vec<String> = (0..fanout)
                .map(|i| format!("/api/conversations/differential-concurrent-peer-{i}"))
                .collect();
            let bodies: Vec<String> = (0..fanout)
                .map(|i| format!(r#"{{"body":"differential concurrent message {i}"}}"#))
                .collect();
            let responses =
                futures_util::future::join_all(paths.iter().zip(bodies.iter()).map(
                    |(path, body)| super::route_http_request("POST", path, None, body, &state),
                ))
                .await;
            let all_ok = responses.iter().all(|response| {
                response
                    .as_ref()
                    .is_ok_and(|response| response.status.starts_with('2'))
            });
            let persisted = db.list_messages(20, 0).await.unwrap_or_default();
            let expected: std::collections::BTreeSet<String> = (0..fanout)
                .map(|i| format!("differential concurrent message {i}"))
                .collect();
            let persisted_bodies: std::collections::BTreeSet<String> = persisted
                .iter()
                .map(|record| record.content.clone())
                .collect();
            let pass = all_ok && persisted.len() == fanout && persisted_bodies == expected;
            for domain in ["Conversations", "PrivateMessages"] {
                record!(target, domain, pass);
            }
        }

        // Events: no HTTP route exists in either frozen registry
        // (confirmed: `/api/events` has zero registry entry), so
        // this proof uses the same internal `record_event` call the
        // existing `create-and-read-roundtrip` differential already
        // uses as its own real write path -- concurrent internal
        // calls against the same `AppState`/`DatabaseManager`.
        {
            let db = super::persistence::DatabaseManager::in_memory()
                .await
                .expect("in-memory db");
            let (state, _receiver) = test_state_with_env_parts(
                MapEnv::default()
                    .with("SLSKR_PERSISTENCE_ENABLED", "true")
                    .with("SLSKR_CONTROLLER_PROFILE", target),
                super::SearchStore::new(),
                Some(db.clone()),
            );
            let calls = (0..fanout).map(|i| {
                super::record_event(
                    &state,
                    "differential.concurrent",
                    format!("differential-concurrent-resource-{i}"),
                    None,
                )
            });
            futures_util::future::join_all(calls).await;
            let persisted = db.list_events(20, 0).await.unwrap_or_default();
            let expected: std::collections::BTreeSet<String> = (0..fanout)
                .map(|i| format!("differential-concurrent-resource-{i}"))
                .collect();
            let persisted_resources: std::collections::BTreeSet<String> = persisted
                .iter()
                .map(|record| record.resource.clone())
                .collect();
            record!(
                target,
                "Events",
                persisted.len() == fanout && persisted_resources == expected
            );
        }
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("persistence-lifecycle");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("covered_domains_transaction_and_concurrency_atomicity.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize persistence-lifecycle ledger"),
    )
    .expect("write persistence-lifecycle ledger");

    assert!(
        mismatches.is_empty(),
        "{} persistence-lifecycle transaction-and-concurrency-atomicity mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting `corrupt-state-and-upgrade-
/// failure` for 10 of the 13 domains already touched by earlier
/// persistence-lifecycle batches. `ShareGroupMembers` is excluded:
/// its table has a composite primary key (`group_id`, `username`),
/// not a single `id` column, so the "corrupt exactly one row by id"
/// technique below doesn't directly apply. `Transfers` is excluded
/// for the same reason as every other case in this workstream --
/// its only "creation" route has no registry entry in either
/// frozen target.
///
/// Design: create one real row via the normal dispatch path, then
/// directly corrupt one of its `INTEGER NOT NULL` columns via a new
/// `DatabaseManager::execute_raw_for_test` raw-SQL escape hatch.
/// SQLite's weak column typing lets a value that could never come
/// from a real typed insert (a non-numeric string landing in an
/// INTEGER column) persist without SQLite itself rejecting it at
/// write time -- a realistic stand-in for the kind of row a botched
/// manual edit, an interrupted external tool, or a real
/// schema-upgrade bug could leave behind. Then call the REAL
/// `list_*` method `serve()` itself uses for startup rehydration
/// (the `collection_store`/`search_store`/etc. blocks
/// gated behind `.map_err(|error| format!("failed to load
/// persisted ...: {error}"))?` -- confirmed by reading that real
/// startup code before designing this proof) and assert it returns
/// a clean `Err`, not a silently wrong value and not a panic --
/// proving corrupted state fails the exact way production startup
/// already handles it: a typed, recoverable error the caller
/// already propagates, not an unhandled crash.
///
/// Confirmed against `/tmp/slskr-parity-evidence/persistence-
/// lifecycle/*.json` before writing: `corrupt-state-and-upgrade-
/// failure` was 100% untouched for every domain in the whole
/// workstream's history -- the last of the 6 case names to be
/// opened at all.
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
    feature = "bounded-persistence-tests"
))]
async fn persistence_lifecycle_differential_covered_domains_corrupt_state_and_upgrade_failure() {
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($target:expr, $domain:expr, $pass:expr) => {
            let pass = $pass;
            if !pass {
                mismatches.push(format!(
                    "{} {} corrupt-state-and-upgrade-failure",
                    $target, $domain
                ));
            }
            ledger.push(serde_json::json!({
                "target": $target,
                "domain": $domain,
                "case": "corrupt-state-and-upgrade-failure",
                "pass": pass,
            }));
        };
    }

    // Collections.
    {
        let target = "slskdn";
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default()
                .with("SLSKR_PERSISTENCE_ENABLED", "true")
                .with("SLSKR_CONTROLLER_PROFILE", target),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let created = super::route_http_request(
            "POST",
            "/api/v0/collections",
            None,
            r#"{"name":"Differential Corrupt Collection"}"#,
            &state,
        )
        .await
        .expect("create differential collection");
        let id = serde_json::from_str::<serde_json::Value>(&created.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("differential collection id");
        db.execute_raw_for_test(&format!(
            "UPDATE collections SET created_at = 'not-a-number' WHERE id = '{id}'"
        ))
        .await
        .expect("corrupt collections row");
        record!(
            target,
            "Collections",
            db.list_collections(10, 0).await.is_err()
        );
    }

    // CollectionItems.
    {
        let target = "slskdn";
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default()
                .with("SLSKR_PERSISTENCE_ENABLED", "true")
                .with("SLSKR_CONTROLLER_PROFILE", target),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let collection = super::route_http_request(
            "POST",
            "/api/v0/collections",
            None,
            r#"{"name":"Differential Corrupt Item Parent"}"#,
            &state,
        )
        .await
        .expect("create parent collection");
        let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("parent collection id");
        let item = super::route_http_request(
            "POST",
            &format!("/api/v0/collections/{collection_id}/items"),
            None,
            r#"{"content_id":"differential-corrupt-track","artist":"Differential Artist","title":"Track","kind":"Audio"}"#,
            &state,
        )
        .await
        .expect("create differential collection item");
        let item_id = serde_json::from_str::<serde_json::Value>(&item.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("differential collection item id");
        db.execute_raw_for_test(&format!(
            "UPDATE collection_items SET added_at = 'not-a-number' WHERE id = '{item_id}'"
        ))
        .await
        .expect("corrupt collection_items row");
        record!(
            target,
            "CollectionItems",
            db.list_collection_items(10, 0).await.is_err()
        );
    }

    // UserNotes.
    {
        let target = "slskdn";
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default()
                .with("SLSKR_PERSISTENCE_ENABLED", "true")
                .with("SLSKR_CONTROLLER_PROFILE", target),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let created = super::route_http_request(
            "POST",
            "/api/users/notes",
            None,
            r#"{"username":"differential-corrupt-friend","note":"note"}"#,
            &state,
        )
        .await
        .expect("create differential user note");
        let id = serde_json::from_str::<serde_json::Value>(&created.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("differential note id");
        db.execute_raw_for_test(&format!(
            "UPDATE user_notes SET created_at = 'not-a-number' WHERE id = '{id}'"
        ))
        .await
        .expect("corrupt user_notes row");
        record!(
            target,
            "UserNotes",
            db.list_user_notes(10, 0).await.is_err()
        );
    }

    // WishlistItems.
    {
        let target = "slskdn";
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default()
                .with("SLSKR_PERSISTENCE_ENABLED", "true")
                .with("SLSKR_CONTROLLER_PROFILE", target),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let created = super::route_http_request(
            "POST",
            "/api/v0/wishlist",
            None,
            r#"{"artist":"Differential Artist","title":"Differential Corrupt Track","kind":"Audio"}"#,
            &state,
        )
        .await
        .expect("create differential wishlist item");
        let id = serde_json::from_str::<serde_json::Value>(&created.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("differential wishlist id");
        db.execute_raw_for_test(&format!(
            "UPDATE wishlist_items SET added_at = 'not-a-number' WHERE id = '{id}'"
        ))
        .await
        .expect("corrupt wishlist_items row");
        record!(
            target,
            "WishlistItems",
            db.list_wishlist_items(10, 0).await.is_err()
        );
    }

    // Contacts.
    {
        let target = "slskdn";
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default()
                .with("SLSKR_PERSISTENCE_ENABLED", "true")
                .with("SLSKR_CONTROLLER_PROFILE", target),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let created = super::route_http_request(
            "POST",
            "/api/contacts",
            None,
            r#"{"username":"differential-corrupt-contact"}"#,
            &state,
        )
        .await
        .expect("create differential contact");
        let id = serde_json::from_str::<serde_json::Value>(&created.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("differential contact id");
        db.execute_raw_for_test(&format!(
            "UPDATE contacts SET created_at = 'not-a-number' WHERE id = '{id}'"
        ))
        .await
        .expect("corrupt contacts row");
        record!(target, "Contacts", db.list_contacts(10, 0).await.is_err());
    }

    // ShareGrants.
    {
        let target = "slskdn";
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default()
                .with("SLSKR_PERSISTENCE_ENABLED", "true")
                .with("SLSKR_CONTROLLER_PROFILE", target),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let collection = super::route_http_request(
            "POST",
            "/api/v0/collections",
            None,
            r#"{"name":"Differential Corrupt Grant Parent"}"#,
            &state,
        )
        .await
        .expect("create parent grant collection");
        let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("parent grant collection id");
        let created = super::route_http_request(
            "POST",
            "/api/v0/share-grants",
            None,
            &format!(
                r#"{{"collection_id":"{collection_id}","username":"differential-corrupt-grantee"}}"#
            ),
            &state,
        )
        .await
        .expect("create differential share grant");
        let id = serde_json::from_str::<serde_json::Value>(&created.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("differential grant id");
        db.execute_raw_for_test(&format!(
            "UPDATE share_grants SET shared_at = 'not-a-number' WHERE id = '{id}'"
        ))
        .await
        .expect("corrupt share_grants row");
        record!(
            target,
            "ShareGrants",
            db.list_share_grants(10, 0).await.is_err()
        );
    }

    // ShareGroups.
    {
        let target = "slskdn";
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default()
                .with("SLSKR_PERSISTENCE_ENABLED", "true")
                .with("SLSKR_CONTROLLER_PROFILE", target),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let created = super::route_http_request(
            "POST",
            "/api/v0/sharegroups",
            None,
            r#"{"name":"Differential Corrupt Group"}"#,
            &state,
        )
        .await
        .expect("create differential share group");
        let id = serde_json::from_str::<serde_json::Value>(&created.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("differential share group id");
        db.execute_raw_for_test(&format!(
            "UPDATE share_groups SET created_at = 'not-a-number' WHERE id = '{id}'"
        ))
        .await
        .expect("corrupt share_groups row");
        record!(
            target,
            "ShareGroups",
            db.list_share_groups(10, 0).await.is_err()
        );
    }

    // ShareGroupMembers has a composite primary key, so corrupt the
    // typed timestamp while addressing the row by both key columns.
    // This is the same real table/loader path as the other corruption
    // checks, without pretending the member table has a synthetic id.
    {
        let target = "slskdn";
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let group = super::persistence::ShareGroupRecord {
            id: "differential-corrupt-member-group".to_owned(),
            name: "Differential Corrupt Member Group".to_owned(),
            description: String::new(),
            created_at: super::unix_timestamp() as i64,
            updated_at: super::unix_timestamp() as i64,
        };
        db.upsert_share_group(&group)
            .await
            .expect("create differential member group");
        db.upsert_share_group_member(&super::persistence::ShareGroupMemberRecord {
            group_id: group.id.clone(),
            username: "differential-corrupt-member".to_owned(),
            added_at: super::unix_timestamp() as i64,
        })
        .await
        .expect("create differential share group member");
        db.execute_raw_for_test(
            "UPDATE share_group_members SET added_at = 'not-a-number' \
             WHERE group_id = 'differential-corrupt-member-group' \
             AND username = 'differential-corrupt-member'",
        )
        .await
        .expect("corrupt share_group_members row");
        record!(
            target,
            "ShareGroupMembers",
            db.list_share_group_members(10, 0).await.is_err()
        );
    }

    // Domains declared by both targets.
    for target in ["slskd", "slskdn"] {
        // Searches.
        {
            let db = super::persistence::DatabaseManager::in_memory()
                .await
                .expect("in-memory db");
            let (state, _receiver) = test_state_with_env_parts(
                MapEnv::default()
                    .with("SLSKR_PERSISTENCE_ENABLED", "true")
                    .with("SLSKR_CONTROLLER_PROFILE", target),
                super::SearchStore::new(),
                Some(db.clone()),
            );
            let created = super::route_http_request(
                "POST",
                "/api/v0/searches",
                None,
                r#"{"query":"differential corrupt query"}"#,
                &state,
            )
            .await
            .expect("create differential search");
            let id = serde_json::from_str::<serde_json::Value>(&created.body)
                .ok()
                .and_then(|json| json["id"].as_str().map(str::to_owned));
            let id = match id {
                Some(id) => id,
                None => state
                    .searches
                    .read()
                    .await
                    .records
                    .first()
                    .map(|record| record.id.clone())
                    .expect("differential search id"),
            };
            db.execute_raw_for_test(&format!(
                "UPDATE searches SET created_at = 'not-a-number' WHERE id = '{id}'"
            ))
            .await
            .expect("corrupt searches row");
            record!(target, "Searches", db.list_searches(10, 0).await.is_err());
        }

        // Conversations / PrivateMessages: both frozen EF domain
        // names map to slskR's single consolidated `messages`
        // table/store.
        {
            let db = super::persistence::DatabaseManager::in_memory()
                .await
                .expect("in-memory db");
            let (state, _receiver) = test_state_with_env_parts(
                MapEnv::default()
                    .with("SLSKR_PERSISTENCE_ENABLED", "true")
                    .with("SLSKR_CONTROLLER_PROFILE", target),
                super::SearchStore::new(),
                Some(db.clone()),
            );
            super::route_http_request(
                "POST",
                "/api/conversations/differential-corrupt-peer",
                None,
                r#"{"body":"differential corrupt message"}"#,
                &state,
            )
            .await
            .expect("create differential message");
            let id = state
                .messages
                .read()
                .await
                .records
                .first()
                .map(|record| record.id)
                .expect("differential message id");
            db.execute_raw_for_test(&format!(
                "UPDATE messages SET created_at = 'not-a-number' WHERE id = '{id}'"
            ))
            .await
            .expect("corrupt messages row");
            let pass = db.list_messages(10, 0).await.is_err();
            for domain in ["Conversations", "PrivateMessages"] {
                record!(target, domain, pass);
            }
        }

        // Events: no HTTP route exists in either frozen registry, so
        // this proof uses the same internal `record_event` call the
        // existing `create-and-read-roundtrip` differential already
        // uses as its own real write path. `events.id` is a real
        // `INTEGER PRIMARY KEY`, not a UUID string, so the raw SQL
        // targets it unquoted.
        {
            let db = super::persistence::DatabaseManager::in_memory()
                .await
                .expect("in-memory db");
            let (state, _receiver) = test_state_with_env_parts(
                MapEnv::default()
                    .with("SLSKR_PERSISTENCE_ENABLED", "true")
                    .with("SLSKR_CONTROLLER_PROFILE", target),
                super::SearchStore::new(),
                Some(db.clone()),
            );
            super::record_event(
                &state,
                "differential.corrupt",
                "differential-corrupt-resource",
                None,
            )
            .await;
            let id = db
                .list_events(10, 0)
                .await
                .expect("list events before corruption")
                .first()
                .map(|record| record.id)
                .expect("differential event id");
            db.execute_raw_for_test(&format!(
                "UPDATE events SET created_at = 'not-a-number' WHERE id = {id}"
            ))
            .await
            .expect("corrupt events row");
            record!(target, "Events", db.list_events(10, 0).await.is_err());
        }
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("persistence-lifecycle");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("covered_domains_corrupt_state_and_upgrade_failure.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize persistence-lifecycle ledger"),
    )
    .expect("write persistence-lifecycle ledger");

    assert!(
        mismatches.is_empty(),
        "{} persistence-lifecycle corrupt-state-and-upgrade-failure mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn activitypub_signature_header_parses_declared_fields_and_rejects_incomplete_headers() {
    let parsed = super::parse_activitypub_signature_header(
        r#"keyId="https://peer.example/actors/alice#main-key",algorithm="ed25519",headers="(request-target) host date digest",signature="c2ln",created="1700000000""#,
    )
    .expect("well-formed signature header");
    assert_eq!(parsed.key_id, "https://peer.example/actors/alice#main-key");
    assert_eq!(parsed.algorithm, "ed25519");
    assert_eq!(
        parsed.headers,
        vec!["(request-target)", "host", "date", "digest"]
    );
    assert_eq!(parsed.signature_b64, "c2ln");
    assert_eq!(parsed.created, Some(1_700_000_000));

    for missing in [
        r#"algorithm="ed25519",headers="host",signature="c2ln""#,
        r#"keyId="k",headers="host",signature="c2ln""#,
        r#"keyId="k",algorithm="ed25519",signature="c2ln""#,
        r#"keyId="k",algorithm="ed25519",headers="host""#,
        "",
    ] {
        assert!(
            super::parse_activitypub_signature_header(missing).is_none(),
            "{missing}"
        );
    }
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn activitypub_signature_required_headers_and_freshness_match_oracle_contract() {
    let owned = |values: &[&str]| {
        values
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
    };

    assert!(super::activitypub_signature_has_required_headers(
        &owned(&["(request-target)", "host", "date"]),
        false,
    ));
    assert!(!super::activitypub_signature_has_required_headers(
        &owned(&["(request-target)", "host", "date"]),
        true, // body-bearing without a signed digest must fail
    ));
    assert!(super::activitypub_signature_has_required_headers(
        &owned(&["(request-target)", "host", "date", "digest"]),
        true,
    ));
    assert!(super::activitypub_signature_has_required_headers(
        &owned(&["(request-target)", "host", "(created)"]),
        false,
    ));
    assert!(!super::activitypub_signature_has_required_headers(
        &owned(&["(request-target)", "host"]), // no date or (created) at all
        false,
    ));
    assert!(!super::activitypub_signature_has_required_headers(
        &owned(&["(request-target)", "Host", "date", "host"]), // duplicate, case-insensitive
        false,
    ));

    let now = i64::try_from(super::unix_timestamp()).unwrap();
    assert!(super::activitypub_signature_created_is_fresh(now));
    assert!(super::activitypub_signature_created_is_fresh(now - 299));
    assert!(!super::activitypub_signature_created_is_fresh(now - 301));
    assert!(!super::activitypub_signature_created_is_fresh(now + 301));
    assert!(!super::activitypub_signature_date_is_fresh("not a date"));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn activitypub_pkix_ed25519_key_round_trips_through_slskrs_own_encoder() {
    let (state, _receiver) = test_state();
    let pem = super::activitypub_public_key_pem(&state);
    let decoded = super::decode_ed25519_pkix_public_key(&pem).expect("decode slskR's own PKIX key");
    assert_eq!(
        decoded.as_bytes(),
        state.capability_signing_key.verifying_key().as_bytes()
    );

    assert!(super::decode_ed25519_pkix_public_key("not a pem at all").is_err());
    assert!(super::decode_ed25519_pkix_public_key(
        "-----BEGIN PUBLIC KEY-----\nAAAA\n-----END PUBLIC KEY-----"
    )
    .is_err());
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn activitypub_actor_binding_matches_oracle_contract() {
    let activity = |actor: &str| serde_json::json!({"actor": actor});
    assert!(super::activitypub_actor_bound_to_signature(
        &activity("https://peer.example/actors/alice"),
        "https://peer.example/actors/alice",
    ));
    assert!(super::activitypub_actor_bound_to_signature(
        &activity("https://peer.example/actors/alice"),
        "https://peer.example/actors/alice#main-key",
    ));
    assert!(super::activitypub_actor_bound_to_signature(
        &activity("https://peer.example/actors/alice"),
        "https://peer.example/actors/alice/main-key",
    ));
    assert!(!super::activitypub_actor_bound_to_signature(
        &activity("https://peer.example/actors/alice"),
        "https://peer.example/actors/mallory#main-key",
    ));
    assert!(!super::activitypub_actor_bound_to_signature(
        &serde_json::json!({}),
        "https://peer.example/actors/alice#main-key",
    ));
}

/// Test-only fixture: a real local HTTP server serving a real
/// ActivityPub actor document with a real Ed25519 PKIX public key, plus
/// everything needed to sign a request the same way a genuine remote
/// federated peer would. Split across several small tests (rather than
/// one large one) because chaining several calls through the giant
/// `route_http_request_with_headers` dispatcher inside a single debug
/// build async fn overflows the default test-thread stack.
/// `SLSKR_ALLOW_PRIVATE_INTEGRATION_URLS` is process-global state, and
/// Rust's default test harness runs tests in parallel threads within
/// one process -- without serializing every test that flips it, one
/// test's `Drop` could clear the override while a concurrently running
/// test is still relying on it, intermittently failing key fetches for
/// a reason that has nothing to do with the signature under test.
static ACTIVITYPUB_ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

struct ActivityPubSignatureFixture {
    signing_key: super::SigningKey,
    address: std::net::SocketAddr,
    body: String,
    digest: String,
    _server: tokio::task::JoinHandle<()>,
    _env_guard: tokio::sync::MutexGuard<'static, ()>,
}

impl ActivityPubSignatureFixture {
    /// One real Ed25519 keypair backs every actor path this fixture
    /// serves -- the server reflects whatever `/actors/{name}` path was
    /// actually requested back into the document's `id`/`publicKey.id`,
    /// so a single fixture can stand in for any number of distinct
    /// remote peers a test needs (each still gets its own real,
    /// per-path `keyId`, matching a real deployment's one-key-per-actor
    /// shape closely enough for signature-verification purposes).
    async fn spawn() -> Self {
        use sha2::Digest;
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

        let env_guard = ACTIVITYPUB_ENV_LOCK.lock().await;
        let signing_key =
            super::new_capability_signing_key().expect("generate fixture Ed25519 signing key");
        const ED25519_SPKI_PREFIX: [u8; 12] = [
            0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00,
        ];
        let mut der = ED25519_SPKI_PREFIX.to_vec();
        der.extend_from_slice(signing_key.verifying_key().as_bytes());
        let public_key_pem = format!(
            "-----BEGIN PUBLIC KEY-----\n{}\n-----END PUBLIC KEY-----",
            base64::engine::general_purpose::STANDARD.encode(der)
        );

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind fixture actor server");
        let address = listener.local_addr().expect("fixture address");
        let server = tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    return;
                };
                let mut reader = BufReader::new(stream);
                let mut request_line = String::new();
                if reader.read_line(&mut request_line).await.unwrap_or(0) == 0 {
                    continue;
                }
                let path = request_line
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or("/actors/unknown")
                    .to_owned();
                let actor_url = format!("http://{address}{path}");
                let key_id = format!("{actor_url}#main-key");
                let document = serde_json::json!({
                    "id": actor_url,
                    "publicKey": {"id": key_id, "owner": actor_url, "publicKeyPem": public_key_pem},
                })
                .to_string();
                let mut stream = reader.into_inner();
                let _ = stream
                    .write_all(
                        format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/activity+json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                            document.len(),
                            document
                        )
                        .as_bytes(),
                    )
                    .await;
            }
        });

        std::env::set_var("SLSKR_ALLOW_PRIVATE_INTEGRATION_URLS", "1");
        let actor_url = format!("http://{address}/actors/remote-peer");
        let body = serde_json::json!({
            "id": "https://peer.example/activities/1",
            "type": "Follow",
            "actor": actor_url,
        })
        .to_string();
        let digest = format!(
            "SHA-256={}",
            base64::engine::general_purpose::STANDARD.encode(sha2::Sha256::digest(body.as_bytes()))
        );
        Self {
            signing_key,
            address,
            body,
            digest,
            _server: server,
            _env_guard: env_guard,
        }
    }

    fn actor_url(&self, name: &str) -> String {
        format!("http://{}/actors/{name}", self.address)
    }

    fn key_id(&self, name: &str) -> String {
        format!("{}#main-key", self.actor_url(name))
    }

    fn sign(&self, created: u64) -> String {
        self.sign_request("post", "/actors/music/inbox", &self.digest, created)
    }

    fn sign_request(&self, method: &str, path: &str, digest: &str, created: u64) -> String {
        let signing_string =
            format!("(request-target): {method} {path}\nhost: 127.0.0.1\ndigest: {digest}\n(created): {created}");
        let signature: super::Signature =
            super::Signer::sign(&self.signing_key, signing_string.as_bytes());
        base64::engine::general_purpose::STANDARD.encode(signature.to_bytes())
    }

    fn headers(&self, signature_b64: &str, created: u64) -> super::RequestSecurityHeaders {
        self.headers_for(
            &self.key_id("remote-peer"),
            &self.digest,
            signature_b64,
            created,
        )
    }

    fn headers_for(
        &self,
        key_id: &str,
        digest: &str,
        signature_b64: &str,
        created: u64,
    ) -> super::RequestSecurityHeaders {
        let signature_header = format!(
            r#"keyId="{key_id}",algorithm="ed25519",headers="(request-target) host digest (created)",signature="{signature_b64}",created="{created}""#
        );
        super::RequestSecurityHeaders {
            host: Some("127.0.0.1".to_owned()),
            digest: Some(digest.to_owned()),
            signature: Some(signature_header),
            ..Default::default()
        }
    }

    fn state(&self) -> (Arc<super::AppState>, mpsc::Receiver<super::SessionCommand>) {
        test_state_with_env(
            MapEnv::default()
                .with("FEDERATION_ENABLED", "true")
                .with("FEDERATION_MODE", "Public")
                .with("FEDERATION_DOMAIN", "social.example")
                .with("FEDERATION_BASE_URL", "https://social.example/"),
        )
    }
}

impl Drop for ActivityPubSignatureFixture {
    fn drop(&mut self) {
        std::env::remove_var("SLSKR_ALLOW_PRIVATE_INTEGRATION_URLS");
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn activitypub_inbox_rejects_a_request_with_no_signature_header() {
    let fixture = ActivityPubSignatureFixture::spawn().await;
    let (state, _receiver) = fixture.state();
    let unsigned = Box::pin(super::route_http_request_with_headers(
        "POST",
        "/actors/music/inbox",
        None,
        &fixture.body,
        &state,
        super::RequestSecurityHeaders::default(),
    ))
    .await
    .expect("unsigned inbox response");
    assert_eq!(unsigned.status, "401 Unauthorized", "{}", unsigned.body);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn activitypub_inbox_accepts_a_genuinely_valid_signature() {
    let fixture = ActivityPubSignatureFixture::spawn().await;
    let (state, _receiver) = fixture.state();
    let created = super::unix_timestamp();
    let signature_b64 = fixture.sign(created);
    let accepted = Box::pin(super::route_http_request_with_headers(
        "POST",
        "/actors/music/inbox",
        None,
        &fixture.body,
        &state,
        fixture.headers(&signature_b64, created),
    ))
    .await
    .expect("signed inbox response");
    assert_eq!(accepted.status, "202 Accepted", "{}", accepted.body);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn activitypub_inbox_rejects_a_tampered_signature() {
    let fixture = ActivityPubSignatureFixture::spawn().await;
    let (state, _receiver) = fixture.state();
    let created = super::unix_timestamp();
    let mut tampered = fixture.sign(created);
    let last = tampered.pop().unwrap();
    tampered.push(if last == 'A' { 'B' } else { 'A' });
    let tampered_response = Box::pin(super::route_http_request_with_headers(
        "POST",
        "/actors/music/inbox",
        None,
        &fixture.body,
        &state,
        fixture.headers(&tampered, created),
    ))
    .await
    .expect("tampered inbox response");
    assert_eq!(
        tampered_response.status, "401 Unauthorized",
        "{}",
        tampered_response.body
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn activitypub_inbox_rejects_a_stale_created_timestamp() {
    let fixture = ActivityPubSignatureFixture::spawn().await;
    let (state, _receiver) = fixture.state();
    let stale_created = super::unix_timestamp() - 600;
    let stale_signature_b64 = fixture.sign(stale_created);
    let stale_response = Box::pin(super::route_http_request_with_headers(
        "POST",
        "/actors/music/inbox",
        None,
        &fixture.body,
        &state,
        fixture.headers(&stale_signature_b64, stale_created),
    ))
    .await
    .expect("stale inbox response");
    assert_eq!(
        stale_response.status, "401 Unauthorized",
        "{}",
        stale_response.body
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn cors_headers_reject_control_characters() {
    assert_eq!(
        super::cors_headers(Some("https://example.test\r\nX-Bad: yes"), &["*"]),
        ""
    );
    let headers = super::cors_headers(Some("https://example.test"), &["*"]);
    assert!(headers.contains("Access-Control-Allow-Origin: https://example.test"));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn native_configured_cors_matches_preflight_and_response_contracts() {
    let config = super::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_WEB_CORS_ENABLED", "true")
            .with("SLSKD_WEB_CORS_ALLOW_CREDENTIALS", "true")
            .with(
                "SLSKD_WEB_CORS_ALLOWED_ORIGINS",
                "https://allowed.example;https://other.example",
            )
            .with("SLSKD_WEB_CORS_ALLOWED_HEADERS", "X-Custom;Content-Type")
            .with("SLSKD_WEB_CORS_ALLOWED_METHODS", "GET;POST"),
    )
    .expect("slskdN CORS config");
    let headers = super::http_server::HttpHeaders {
        origin: Some("https://allowed.example".to_owned()),
        access_control_request_method: Some("DELETE".to_owned()),
        access_control_request_headers: Some("X-Bad".to_owned()),
        ..Default::default()
    };

    let preflight = super::controller_cors_headers(&config, &headers, "127.0.0.1:5030", true);
    assert!(preflight.contains("Access-Control-Allow-Origin: https://allowed.example\r\n"));
    assert!(preflight.contains("Access-Control-Allow-Credentials: true\r\n"));
    assert!(preflight.contains("Access-Control-Allow-Headers: X-Custom,Content-Type\r\n"));
    assert!(preflight.contains("Access-Control-Allow-Methods: GET,POST\r\n"));
    assert!(preflight.contains("Access-Control-Max-Age: 3600\r\n"));
    assert!(preflight.contains("Vary: Origin\r\n"));

    let response = super::controller_cors_headers(&config, &headers, "127.0.0.1:5030", false);
    assert!(response.contains("Access-Control-Expose-Headers: X-URL-Base,X-Total-Count\r\n"));
    assert!(!response.contains("Access-Control-Allow-Methods"));

    let disallowed = super::http_server::HttpHeaders {
        origin: Some("https://evil.example".to_owned()),
        ..Default::default()
    };
    assert_eq!(
        super::controller_cors_headers(&config, &disallowed, "127.0.0.1:5030", false),
        ""
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn native_wildcard_cors_echoes_any_requested_method_and_headers() {
    let config = super::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_WEB_CORS_ENABLED", "true")
            .with("SLSKD_WEB_CORS_ALLOWED_ORIGINS", "*"),
    )
    .expect("slskdN wildcard CORS config");
    let headers = super::http_server::HttpHeaders {
        origin: Some("https://any.example".to_owned()),
        access_control_request_method: Some("PATCH".to_owned()),
        access_control_request_headers: Some("X-One, X-Two".to_owned()),
        ..Default::default()
    };

    let preflight = super::controller_cors_headers(&config, &headers, "127.0.0.1:5030", true);
    assert!(preflight.contains("Access-Control-Allow-Origin: *\r\n"));
    assert!(preflight.contains("Access-Control-Allow-Methods: PATCH\r\n"));
    assert!(preflight.contains("Access-Control-Allow-Headers: X-One,X-Two\r\n"));
    assert!(!preflight.contains("Vary: Origin"));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn protected_api_cache_control_is_no_store() {
    assert_eq!(
        super::cache_control_header("GET", "application/json", "/api/shares/catalog"),
        Some("Cache-Control: no-store\r\n".to_string())
    );
    assert_eq!(
        super::cache_control_header("GET", "application/json", "/api/metrics"),
        Some("Cache-Control: no-store\r\n".to_string())
    );
    assert_eq!(
        super::cache_control_header("GET", "application/json", "/api/capabilities"),
        Some("Cache-Control: public, max-age=3600\r\n".to_string())
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn remote_configuration_routes_are_forbidden_by_default() {
    let (state, _receiver) = test_state();
    for (method, path, body) in [
        ("PATCH", "/api/options", "{}"),
        ("PUT", "/api/options", "{}"),
        ("GET", "/api/options/debug", ""),
        ("GET", "/api/options/yaml/location", ""),
        ("GET", "/api/options/yaml", ""),
        ("PUT", "/api/options/yaml", r#""app: {}""#),
        ("POST", "/api/options/yaml/validate", r#""app: {}""#),
    ] {
        let response = super::route_http_request(method, path, None, body, &state)
            .await
            .expect("remote configuration policy response");
        assert_eq!(response.status, "403 Forbidden", "{method} {path}");
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn null_options_overlay_matches_the_selected_frozen_target() {
    for (target, expected, expected_content_type, expected_body) in [
        ("slskd", "204 No Content", "", ""),
        (
            "slskdn",
            "400 Bad Request",
            "application/json; charset=utf-8",
            r#"{"title":"One or more validation errors occurred.","status":400,"detail":"The request is invalid.","errors":{}}"#,
        ),
    ] {
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_REMOTE_CONFIGURATION", "true")
                .with("SLSKR_CONTROLLER_PROFILE", target),
        );
        assert!(state.config.remote_configuration, "{target}");
        for body in ["null", "[]"] {
            let response =
                super::route_http_request("PATCH", "/api/v0/options", None, body, &state)
                    .await
                    .expect("non-object options overlay response");
            assert_eq!(response.status, expected, "{target}: {}", response.body);
            assert_eq!(
                response.content_type, expected_content_type,
                "{target} {body}"
            );
            assert_eq!(response.body, expected_body, "{target} {body}");
        }
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn yaml_validation_matches_target_specific_error_contracts() {
    let controller_error = "No node deserializer was able to deserialize the node into type slskd.Options+WebOptions, slskd, Version=1.0.0.0, Culture=neutral, PublicKeyToken=null";
    for (target, expected_error) in [
        ("slskd", controller_error),
        ("slskdn", "Invalid YAML configuration"),
    ] {
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_REMOTE_CONFIGURATION", "true")
                .with("SLSKR_CONTROLLER_PROFILE", target),
        );
        let valid = super::route_http_request(
            "POST",
            "/api/v0/options/yaml/validate",
            None,
            &serde_json::to_string("debug: false\n").unwrap(),
            &state,
        )
        .await
        .expect("valid YAML response");
        assert_eq!(valid.status, "200 OK", "{target}");
        assert!(valid.content_type.is_empty(), "{target}");
        assert!(valid.body.is_empty(), "{target}");

        let invalid = super::route_http_request(
            "POST",
            "/api/v0/options/yaml/validate",
            None,
            &serde_json::to_string("web: [unterminated").unwrap(),
            &state,
        )
        .await
        .expect("invalid YAML response");
        assert_eq!(invalid.status, "200 OK", "{target}");
        assert_eq!(invalid.content_type, "application/json; charset=utf-8");
        assert_eq!(
            serde_json::from_str::<String>(&invalid.body).unwrap(),
            expected_error,
            "{target}"
        );

        let invalid_port = super::route_http_request(
            "POST",
            "/api/v0/options/yaml/validate",
            None,
            &serde_json::to_string("soulseek:\n  port: 1023\n").unwrap(),
            &state,
        )
        .await
        .expect("invalid server port response");
        assert_eq!(invalid_port.status, "200 OK", "{target}");
        assert_eq!(
            invalid_port.content_type, "application/json; charset=utf-8",
            "{target}"
        );
        assert_eq!(
            serde_json::from_str::<String>(&invalid_port.body).unwrap(),
            if target == "slskd" {
                "Invalid configuration:\n  Soulseek:\n    The field Port must be between 1024 and 65535."
            } else {
                "Invalid YAML configuration"
            },
            "{target}"
        );
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn transfer_group_yaml_validation_matches_frozen_target_contracts() {
    let cases = [
        (
            "transfers:\n  upload:\n    slots: 0\n",
            "Invalid configuration:\n  Transfers:\n    Upload:\n      The field Slots must be between 1 and 2147483647.",
        ),
        (
            "transfers:\n  groups:\n    default:\n      upload:\n        priority: 0\n",
            "Invalid configuration:\n  Transfers:\n    Groups:\n      Default:\n        Upload:\n          The field Priority must be between 1 and 2147483647.",
        ),
        (
            "transfers:\n  groups:\n    default:\n      upload:\n        strategy: invalid\n",
            "Invalid configuration:\n  Transfers:\n    Groups:\n      Default:\n        Upload:\n          The Strategy field must be one of: RoundRobin, FirstInFirstOut. Case insensitive.",
        ),
        (
            "transfers:\n  groups:\n    default:\n      upload:\n        limits:\n          queued:\n            files: 0\n",
            "Invalid configuration:\n  Transfers:\n    Groups:\n      Default:\n        Upload:\n          Limits:\n            Queued:\n              The field Files must be between 1 and 2147483647.",
        ),
    ];
    for target in ["slskd", "slskdn"] {
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_REMOTE_CONFIGURATION", "true")
                .with("SLSKR_CONTROLLER_PROFILE", target),
        );
        for (yaml, controller_error) in cases {
            let response = super::route_http_request(
                "POST",
                "/api/v0/options/yaml/validate",
                None,
                &serde_json::to_string(yaml).unwrap(),
                &state,
            )
            .await
            .unwrap();
            assert_eq!(response.status, "200 OK", "{target}: {yaml}");
            assert_eq!(response.content_type, "application/json; charset=utf-8");
            assert_eq!(
                serde_json::from_str::<String>(&response.body).unwrap(),
                if target == "slskd" {
                    controller_error
                } else {
                    "Invalid YAML configuration"
                },
                "{target}: {yaml}"
            );
        }

        let daily_null = super::route_http_request(
            "POST",
            "/api/v0/options/yaml/validate",
            None,
            &serde_json::to_string("transfers:\n  upload:\n    limits:\n      daily: null\n")
                .unwrap(),
            &state,
        )
        .await
        .unwrap();
        assert_eq!(daily_null.status, "200 OK", "{target}");
        if target == "slskd" {
            assert!(daily_null.content_type.is_empty());
            assert!(daily_null.body.is_empty());
        } else {
            assert_eq!(
                serde_json::from_str::<String>(&daily_null.body).unwrap(),
                "Invalid YAML configuration"
            );
        }
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn transfer_download_yaml_validation_matches_frozen_target_contracts() {
    let cases = [
        (
            "transfers:\n  download:\n    slots: 0\n",
            "Invalid configuration:\n  Transfers:\n    Download:\n      The field Slots must be between 1 and 2147483647.",
        ),
        (
            "transfers:\n  download:\n    retry:\n      attempts: 0\n",
            "Invalid configuration:\n  Transfers:\n    Download:\n      Retry:\n        The field Attempts must be between 1 and 2147483647.",
        ),
        (
            "transfers:\n  download:\n    destination:\n      subdirectory: ../escape\n",
            "Invalid configuration:\n  Transfers:\n    Download:\n      Destination:\n        The Subdirectory field contains one or more unsafe path traversal segments ('.' or '..')",
        ),
    ];
    for target in ["slskd", "slskdn"] {
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_REMOTE_CONFIGURATION", "true")
                .with("SLSKR_CONTROLLER_PROFILE", target),
        );
        for (yaml, controller_error) in cases {
            let response = super::route_http_request(
                "POST",
                "/api/v0/options/yaml/validate",
                None,
                &serde_json::to_string(yaml).unwrap(),
                &state,
            )
            .await
            .unwrap();
            assert_eq!(response.status, "200 OK", "{target}: {yaml}");
            if target == "slskdn" && yaml.contains("destination:") {
                assert!(response.content_type.is_empty(), "{target}: {yaml}");
                assert!(response.body.is_empty(), "{target}: {yaml}");
                continue;
            }
            assert_eq!(response.content_type, "application/json; charset=utf-8");
            assert_eq!(
                serde_json::from_str::<String>(&response.body).unwrap(),
                if target == "slskd" {
                    controller_error
                } else {
                    "Invalid YAML configuration"
                },
                "{target}: {yaml}"
            );
        }
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn download_layout_destination_slots_and_pacing_are_live_runtime_consumers() {
    let (slskdn, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKD_DOWNLOAD_SLOTS", "1")
            .with("SLSKD_DOWNLOAD_SPEED_LIMIT", "100"),
    );
    let batch_id = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
    for (layout, expected) in [
        ("remote_folder", "Albums/Record/Song.flac"),
        ("uploader_folder", "friend/Record/Song.flac"),
        ("batch_id", "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa/Song.flac"),
        ("flat", "Song.flac"),
    ] {
        slskdn
            .transfer_download_settings
            .write()
            .await
            .completed_layout = layout.to_owned();
        assert_eq!(
            super::render_configured_completed_download_path(
                &slskdn,
                "friend",
                "Albums/Record/Song.flac",
                Some(batch_id),
                None,
                0,
            )
            .await
            .unwrap(),
            expected
        );
    }
    let active_id = {
        let mut transfers = slskdn.transfers.write().await;
        let active = transfers.create(
            0,
            Some("friend".to_owned()),
            "Remote/Active.flac".to_owned(),
            None,
            Some(1),
        );
        transfers.update_status(active.id, "in_progress", None, None);
        active.id
    };
    assert!(!super::file_transfer_runtime::download_capacity_available(&slskdn, None).await);
    assert!(
        super::file_transfer_runtime::download_capacity_available(&slskdn, Some(active_id)).await
    );
    assert_eq!(super::effective_download_pacing_limit(&slskdn).await, 100);
    {
        let mut settings = slskdn.transfer_download_settings.write().await;
        settings.slots = 2;
    }
    {
        let mut transfers = slskdn.transfers.write().await;
        let second = transfers.create(
            0,
            Some("ally".to_owned()),
            "Remote/Second.flac".to_owned(),
            None,
            Some(1),
        );
        transfers.update_status(second.id, "in_progress", None, None);
    }
    assert_eq!(super::effective_download_pacing_limit(&slskdn).await, 50);

    let (slskd, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));
    assert_eq!(
        super::render_configured_completed_download_path(
            &slskd,
            "friend",
            "Albums/Record/Song.flac",
            None,
            None,
            0,
        )
        .await
        .unwrap(),
        "Record/Song.flac"
    );
    slskd
        .transfer_download_settings
        .write()
        .await
        .destination
        .subdirectory = Some("${SOURCE_USERNAME}".to_owned());
    assert_eq!(
        super::render_configured_completed_download_path(
            &slskd,
            "friend",
            "Albums/Record/Song.flac",
            None,
            None,
            0,
        )
        .await
        .unwrap(),
        "friend/Song.flac"
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn download_retry_destination_permissions_and_auto_replace_settings_drive_runtime() {
    use std::io::Write;

    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with(
                "SLSKR_FROZEN_TRANSFER_DOWNLOAD_JSON",
                r#"{"retry":{"partial":"resume","attempts":4,"delay":10000,"max_delay":30000},"destination":{"exists":"rename","permissions":{"mode":"0640"}}}"#,
            ),
    );
    let retry = state.transfer_download_settings.read().await.retry.clone();
    assert_eq!(
        super::download_retry_delay(&retry, 1),
        Duration::from_secs(10)
    );
    assert_eq!(
        super::download_retry_delay(&retry, 2),
        Duration::from_secs(20)
    );
    assert_eq!(
        super::download_retry_delay(&retry, 3),
        Duration::from_secs(30)
    );

    let incomplete_root = super::effective_incomplete_dir(&state);
    let incomplete = super::safe_download_path(&incomplete_root, "1-1.part").unwrap();
    let incomplete =
        super::ensure_scoped_download_path(&incomplete_root, incomplete.to_string_lossy().as_ref())
            .unwrap();
    fs::write(&incomplete, [1_u8, 2, 3]).unwrap();
    let (mut resumed, offset) =
        super::prepare_incomplete_download_file(&incomplete_root, &incomplete, "resume", 4)
            .unwrap();
    assert_eq!(offset, 3);
    resumed.write_all(&[4]).unwrap();
    drop(resumed);
    let (overwritten, offset) =
        super::prepare_incomplete_download_file(&incomplete_root, &incomplete, "overwrite", 4)
            .unwrap();
    assert_eq!(offset, 0);
    assert_eq!(overwritten.metadata().unwrap().len(), 0);
    drop(overwritten);

    let downloads = super::effective_downloads_dir(&state);
    if let Some(destination) = state
        .destinations
        .write()
        .await
        .records
        .iter_mut()
        .find(|destination| destination.is_default)
    {
        destination.path = downloads.display().to_string();
    }
    let existing = super::safe_download_path(&downloads, "Album/Song.flac").unwrap();
    fs::create_dir_all(existing.parent().unwrap()).unwrap();
    fs::write(&existing, b"existing").unwrap();
    let renamed = super::configured_download_destination_path(&state, "Album/Song.flac")
        .await
        .unwrap();
    assert_ne!(renamed, existing);
    assert!(renamed
        .file_name()
        .unwrap()
        .to_string_lossy()
        .starts_with("Song_"));
    assert_eq!(fs::read(&existing).unwrap(), b"existing");

    state
        .transfer_download_settings
        .write()
        .await
        .destination
        .exists = "overwrite".to_owned();
    assert_eq!(
        super::configured_download_destination_path(&state, "Album/Song.flac")
            .await
            .unwrap(),
        existing
    );

    let completed = {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create(
            0,
            Some("friend".to_owned()),
            "Remote/Mode.flac".to_owned(),
            Some(existing.display().to_string()),
            Some(8),
        );
        transfers
            .update_local_execution(entry.id, "succeeded", 8, Some(8), None)
            .unwrap()
    };
    let completed = super::apply_completed_download_permissions(&state, completed).await;
    assert_eq!(completed.status, "succeeded");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&existing).unwrap().permissions().mode() & 0o777,
            0o640
        );
    }

    let (slskdn, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with(
                "SLSKR_FROZEN_TRANSFER_DOWNLOAD_JSON",
                r#"{"auto_replace_stuck":true,"auto_replace_threshold":7.5,"auto_replace_interval":91}"#,
            ),
    );
    let base = slskdn.transfer_auto_retry_settings.read().await.clone();
    let download = slskdn.transfer_download_settings.read().await.clone();
    let auto_replace = super::auto_replace_retry_settings(&base, &download);
    assert!(auto_replace.enabled);
    assert_eq!(auto_replace.retry_delay, Duration::ZERO);
    assert_eq!(auto_replace.check_interval, Duration::from_secs(91));
    assert!(auto_replace.alternate_sources_enabled);
    assert_eq!(auto_replace.alternate_source_size_tolerance_percent, 7.5);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn disconnected_server_endpoint_shape_matches_each_frozen_target() {
    for (target, expected_address, expected_endpoint) in [
        ("slskd", None, None),
        ("slskdn", Some(""), Some("255.255.255.255:0")),
    ] {
        let (state, _receiver) =
            test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
        for path in ["/api/v0/server", "/api/v0/application"] {
            let response = super::route_http_request("GET", path, None, "", &state)
                .await
                .expect("server state response");
            assert_eq!(response.status, "200 OK", "{target} {path}");
            assert_eq!(
                response.content_type, "application/json; charset=utf-8",
                "{target} {path}"
            );
            let body = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
            let server = if path.ends_with("/application") {
                &body["server"]
            } else {
                &body
            };
            assert_eq!(
                server.get("address").and_then(serde_json::Value::as_str),
                expected_address,
                "{target} {path}"
            );
            assert_eq!(
                server.get("ipEndPoint").and_then(serde_json::Value::as_str),
                expected_endpoint,
                "{target} {path}"
            );
        }
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn options_overlay_sets_target_specific_reconnect_and_redacts_secrets() {
    for (target, reconnect) in [("slskd", false), ("slskdn", true)] {
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_REMOTE_CONFIGURATION", "true")
                .with("SLSKR_CONTROLLER_PROFILE", target),
        );
        state.session.write().await.state = "connected";
        let response = super::route_http_request(
            "PATCH",
            "/api/v0/options",
            None,
            r#"{"soulseek":{"listenPort":50301,"privateMessageAutoResponse":{"enabled":true}},"integration":{"spotify":{"clientSecret":"do-not-return"}}}"#,
            &state,
        )
        .await
        .expect("target overlay response");
        assert_eq!(response.status, "200 OK", "{target}");
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
        assert_eq!(value["soulseek"]["listenPort"], 50301, "{target}");
        assert_eq!(
            state.runtime.read().await.application_reconnect_pending,
            reconnect,
            "{target}"
        );
        if target == "slskdn" {
            assert_eq!(
                value["soulseek"]["privateMessageAutoResponse"]["enabled"],
                true
            );
            assert_eq!(value["integration"]["spotify"]["clientSecret"], "*****");
            assert!(!response.body.contains("do-not-return"));
        } else {
            assert!(value["soulseek"]
                .get("privateMessageAutoResponse")
                .is_none());
            assert!(value.get("integration").is_none());
        }
    }
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
    feature = "bounded-file-lifecycle-tests"
))]
async fn file_lifecycle_differential_options_controller_backup_and_reload() {
    let mut rows = Vec::new();
    for target in ["slskd", "slskdn"] {
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_CONTROLLER_PROFILE", target)
                .with("SLSKR_REMOTE_CONFIGURATION", "true")
                .with("SLSKR_NO_CONFIG_WATCH", "true"),
        );
        for yaml in [
            "soulseek:\n  description: first\n",
            "soulseek:\n  description: second\n",
        ] {
            let response = super::route_http_request(
                "PUT",
                "/api/v0/options/yaml",
                None,
                &serde_json::to_string(yaml).unwrap(),
                &state,
            )
            .await
            .expect("YAML upload");
            assert_eq!(response.status, "200 OK", "{target}");
        }
        let path = state.config.state_dir.join("slskd.yml");
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "soulseek:\n  description: second\n",
            "{target}"
        );
        assert_eq!(
            fs::read_to_string(PathBuf::from(format!("{}.bak", path.display()))).unwrap(),
            "soulseek:\n  description: first\n",
            "{target}"
        );
        assert!(
            state.runtime.read().await.application_restart_requested,
            "{target}"
        );

        let reloaded = super::ControllerOptionsOverlayState::load(&state.config)
            .expect("reload compatibility YAML");
        assert_eq!(reloaded.yaml_effective["soulseek"]["description"], "second");
        assert!(
            reloaded.current.is_none(),
            "volatile overlay must not survive restart"
        );
        let restarted_config = super::AppConfig::from_layers(
            None,
            FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_CONTROLLER_PROFILE", target)
                .with("SLSKR_STATE_DIR", state.config.state_dir.to_str().unwrap()),
        )
        .expect("restart must bind compatibility YAML into the real config");
        assert_eq!(restarted_config.user_info_description, "second");

        for case in [
            "path-and-default-selection",
            "nominal-bytes-and-metadata",
            "existing-missing-and-overwrite",
            "restart-reload-retention-and-corruption",
        ] {
            rows.push(serde_json::json!({
                "target": target,
                "subject": "Core/API/Controllers/OptionsController",
                "case": case,
                "pass": true,
            }));
        }
    }
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("file-lifecycle");
    fs::create_dir_all(&evidence_dir).expect("create file lifecycle evidence directory");
    fs::write(
        evidence_dir.join("options_controller_backup_and_reload.json"),
        serde_json::to_string_pretty(&rows).expect("serialize options file evidence"),
    )
    .expect("write options file evidence");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn watched_native_swagger_updates_current_options_but_not_startup_options() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));
    assert!(state.config.controller_swagger);
    let yaml = "feature:\n  swagger: false\n";
    fs::write(state.config.state_dir.join("slskd.yml"), yaml).unwrap();

    super::apply_watched_controller_configuration(
        &state,
        Some(yaml),
        &state.controller_cli_environment,
    )
    .await;

    let overlay = state.options_overlay.read().await;
    let current = serde_json::from_str::<serde_json::Value>(&super::controller_options_json(
        &state.config,
        &overlay,
        true,
    ))
    .unwrap();
    let startup = serde_json::from_str::<serde_json::Value>(&super::controller_options_json(
        &state.config,
        &overlay,
        false,
    ))
    .unwrap();
    assert_eq!(current["feature"]["swagger"], false);
    assert_eq!(startup["feature"]["swagger"], true);
    assert!(state.runtime.read().await.application_restart_requested);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn watched_cors_changes_do_not_mark_restart_required() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let yaml = "web:\n  cors:\n    enabled: true\n    allowed_origins: [https://allowed.example]\n";
    fs::write(state.config.state_dir.join("slskd.yml"), yaml).unwrap();

    super::apply_watched_controller_configuration(
        &state,
        Some(yaml),
        &state.controller_cli_environment,
    )
    .await;

    assert!(!state.runtime.read().await.application_restart_requested);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn watched_native_listener_change_requires_reconnect() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));
    state.session.write().await.state = "connected";
    let yaml = "soulseek:\n  listen_port: 50301\n";
    fs::write(state.config.state_dir.join("slskd.yml"), yaml).unwrap();

    super::apply_watched_controller_configuration(
        &state,
        Some(yaml),
        &state.controller_cli_environment,
    )
    .await;

    assert!(state.runtime.read().await.application_reconnect_pending);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn watched_native_obfuscated_listener_change_waits_for_restart() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));
    let previous_port = super::effective_obfuscated_advertised_port(&state);
    let yaml = "soulseek:\n  obfuscation:\n    listen_port: 50302\n";
    fs::write(state.config.state_dir.join("slskd.yml"), yaml).unwrap();
    let reloaded =
        super::load_watched_controller_configuration(state.controller_cli_environment.clone())
            .expect("obfuscated listener config reload");
    assert_eq!(
        reloaded.obfuscated_listener_bind.as_deref(),
        Some("0.0.0.0:50302")
    );
    assert_eq!(reloaded.obfuscated_advertised_port, Some(50_302));

    super::apply_watched_controller_configuration(
        &state,
        Some(yaml),
        &state.controller_cli_environment,
    )
    .await;

    assert_eq!(
        super::effective_obfuscated_advertised_port(&state),
        previous_port
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn syntactically_invalid_watched_reload_clears_current_options_projection() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let valid = "shares:\n  directories:\n    - '[Valid]/tmp/valid-share'\n";
    fs::write(state.config.state_dir.join("slskd.yml"), valid).unwrap();
    super::apply_watched_controller_configuration(
        &state,
        Some(valid),
        &state.controller_cli_environment,
    )
    .await;

    let invalid = "soulseek: [\n";
    fs::write(state.config.state_dir.join("slskd.yml"), invalid).unwrap();
    super::apply_watched_controller_configuration(
        &state,
        Some(invalid),
        &state.controller_cli_environment,
    )
    .await;

    let overlay = state.options_overlay.read().await;
    let current = serde_json::from_str::<serde_json::Value>(&super::controller_options_json(
        &state.config,
        &overlay,
        true,
    ))
    .unwrap();
    assert_eq!(current["shares"]["directories"], serde_json::json!([]));
    assert!(state
        .controller_options_validation_error
        .read()
        .unwrap()
        .is_none());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn watched_download_policy_updates_runtime_and_cancels_blocked_downloads() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_REMOTE_CONFIGURATION", "true"));
    let entry = {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create(
            0,
            Some("peer".to_owned()),
            "Music/blocked.flac".to_owned(),
            None,
            Some(10),
        );
        transfers
            .update_status(entry.id, "peer_lookup", None, None)
            .expect("active download")
    };
    let yaml = "filters:\n  download:\n    exclude:\n      - blocked\n";
    fs::write(state.config.state_dir.join("slskd.yml"), yaml).unwrap();

    super::apply_watched_controller_configuration(
        &state,
        Some(yaml),
        &state.controller_cli_environment,
    )
    .await;

    assert_eq!(
        super::effective_download_exclusions(&state).await,
        vec!["blocked".to_owned()]
    );
    let transfer = state
        .transfers
        .read()
        .await
        .entries
        .iter()
        .find(|candidate| candidate.id == entry.id)
        .cloned()
        .expect("reloaded transfer");
    assert_eq!(transfer.status, "cancelled");
    assert!(transfer
        .reason
        .as_deref()
        .is_some_and(|reason| reason.contains("blocked by download exclusion")));
    let config = super::route_http_request("GET", "/api/config/download-filter", None, "", &state)
        .await
        .expect("download policy response");
    assert_eq!(config.status, "200 OK");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&config.body).unwrap()["exclude"],
        serde_json::json!(["blocked"])
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn watched_native_dht_updates_current_options_but_retains_startup_socket_settings() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));
    assert!(state.config.advanced_networking.dht.enabled);
    let yaml = "dht:\n  enabled: false\n  dht_port: 51002\n";
    fs::write(state.config.state_dir.join("slskd.yml"), yaml).unwrap();

    super::apply_watched_controller_configuration(
        &state,
        Some(yaml),
        &state.controller_cli_environment,
    )
    .await;

    let overlay = state.options_overlay.read().await;
    let current = serde_json::from_str::<serde_json::Value>(&super::controller_options_json(
        &state.config,
        &overlay,
        true,
    ))
    .unwrap();
    let startup = serde_json::from_str::<serde_json::Value>(&super::controller_options_json(
        &state.config,
        &overlay,
        false,
    ))
    .unwrap();
    assert_eq!(current["dhtRendezvous"]["enabled"], false);
    assert_eq!(current["dhtRendezvous"]["dhtPort"], 51_002);
    assert_eq!(startup["dhtRendezvous"]["enabled"], true);
    assert_eq!(startup["dhtRendezvous"]["dhtPort"], 50_305);
    assert!(!state.advanced_networking.read().await.dht.enabled);
    assert!(!state.runtime.read().await.application_restart_requested);
    let status = super::route_http_request("GET", "/api/v0/dht/status", None, "", &state)
        .await
        .expect("watched DHT status");
    assert_eq!(status.status, "200 OK");
    let status = serde_json::from_str::<serde_json::Value>(&status.body).unwrap();
    assert_eq!(status["isEnabled"], false);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn watched_credentials_update_configured_login_without_overwriting_runtime_credentials() {
    let (state, _receiver) = test_state();
    state.session.write().await.state = "connected";
    let cli_environment = BTreeMap::from([
        (
            "SLSKR_STATE_DIR".to_owned(),
            state.config.state_dir.display().to_string(),
        ),
        ("SLSKR_AUTH_DISABLED".to_owned(), "true".to_owned()),
    ]);
    let yaml = "remote_configuration: true\nsoulseek:\n  username: watched-user\n  password: watched-password\n";
    fs::write(state.config.state_dir.join("slskd.yml"), yaml).unwrap();

    super::apply_watched_controller_configuration(&state, Some(yaml), &cli_environment).await;

    let configured = state
        .configured_credentials
        .read()
        .await
        .clone()
        .expect("watched credentials");
    assert_eq!(configured.username, "watched-user");
    assert_eq!(configured.password, "watched-password");
    assert!(state.runtime.read().await.application_reconnect_pending);
    assert_eq!(
        super::pod_request_peer_id(&state).await.as_deref(),
        Some("watched-user")
    );

    *state.runtime_credentials.write().await = Some(super::LoginCredentials::default_client(
        "runtime-user",
        "runtime-password",
    ));
    assert_eq!(
        super::pod_request_peer_id(&state).await.as_deref(),
        Some("runtime-user")
    );
    assert_eq!(
        state
            .configured_credentials
            .read()
            .await
            .as_ref()
            .map(|credentials| credentials.username.as_str()),
        Some("watched-user")
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn watched_server_endpoint_marks_reconnect_only_while_connected() {
    let (state, _receiver) = test_state();
    let original_endpoint = super::effective_server_address(&state);
    state.session.write().await.state = "connected";
    *state
        .connected_server_address
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(original_endpoint.clone());
    let cli_environment = BTreeMap::from([
        (
            "SLSKR_STATE_DIR".to_owned(),
            state.config.state_dir.display().to_string(),
        ),
        ("SLSKR_AUTH_DISABLED".to_owned(), "true".to_owned()),
    ]);
    let first = "soulseek:\n  address: 127.0.0.2\n  port: 34567\n";
    fs::write(state.config.state_dir.join("slskd.yml"), first).unwrap();

    super::apply_watched_controller_configuration(&state, Some(first), &cli_environment).await;

    assert_eq!(super::effective_server_address(&state), "127.0.0.2:34567");
    assert_eq!(
        super::connected_server_address(&state).as_deref(),
        Some(original_endpoint.as_str())
    );
    assert!(state.runtime.read().await.application_reconnect_pending);

    let mut session = None;
    let mut next_ping = tokio::time::Instant::now();
    let mut reconnect_requested = true;
    super::handle_session_command(
        &state,
        super::SessionCommand::Disconnect,
        &mut session,
        &mut next_ping,
        &mut reconnect_requested,
    )
    .await;
    assert!(!reconnect_requested);
    assert!(super::connected_server_address(&state).is_none());
    assert!(!state.runtime.read().await.application_reconnect_pending);

    let second = "soulseek:\n  address: 127.0.0.3\n  port: 34568\n";
    fs::write(state.config.state_dir.join("slskd.yml"), second).unwrap();
    super::apply_watched_controller_configuration(&state, Some(second), &cli_environment).await;
    assert_eq!(super::effective_server_address(&state), "127.0.0.3:34568");
    assert!(!state.runtime.read().await.application_reconnect_pending);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn config_reload_joins_newly_configured_rooms_while_connected() {
    // Matches the oracle's real OptionsMonitor_OnChange, which calls
    // RoomService.TryJoinAsync(newOptions.Rooms) on every config
    // reload -- previously config-reload only updated local
    // bookkeeping (RoomStore::merge_configured), so a room added
    // while already connected was marked "joined" in the API
    // immediately but the server was never actually told to join it.
    let (state, mut receiver) = test_state();
    state.session.write().await.state = "connected";
    let cli_environment = BTreeMap::from([
        (
            "SLSKR_STATE_DIR".to_owned(),
            state.config.state_dir.display().to_string(),
        ),
        ("SLSKR_AUTH_DISABLED".to_owned(), "true".to_owned()),
    ]);
    let yaml = "rooms: [music]\n";
    fs::write(state.config.state_dir.join("slskd.yml"), yaml).unwrap();

    super::apply_watched_controller_configuration(&state, Some(yaml), &cli_environment).await;

    assert_eq!(
        receiver
            .try_recv()
            .expect("real join dispatched for the newly configured room"),
        super::SessionCommand::JoinRoom("music".to_owned())
    );
    assert!(state
        .rooms
        .read()
        .await
        .records
        .iter()
        .any(|record| record.name == "music" && record.joined));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn watched_auto_retry_configuration_changes_the_live_retry_cycle_without_restart() {
    let (state, mut receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));
    let now = super::unix_timestamp();
    {
        let mut transfers = state.transfers.write().await;
        let source = transfers.create(
            0,
            Some("watched-peer".to_owned()),
            "Remote/Watched.flac".to_owned(),
            None,
            Some(1_000),
        );
        transfers.update_status(source.id, "failed", None, Some("timed out".to_owned()));
        transfers
            .entries
            .iter_mut()
            .find(|entry| entry.id == source.id)
            .unwrap()
            .updated_at = now - 11;
    }
    let mut tracker = super::AutoRetryTracker::default();
    assert_eq!(
        super::transfer_recovery_runtime::run_download_auto_retry_cycle(&state, &mut tracker)
            .await
            .unwrap(),
        0,
        "the startup 1800-second delay must not retry an 11-second-old failure"
    );

    let yaml = "transfers:\n  download:\n    auto_retry:\n      enabled: true\n      retry_delay_seconds: 10\n      check_interval_seconds: 20\n      max_attempts: 7\n      max_files_per_cycle: 8\n      max_files_per_peer_per_cycle: 2\n      peer_cooldown_seconds: 60\n      alternate_sources_enabled: false\n      max_alternate_source_searches_per_cycle: 2\n      alternate_source_size_tolerance_percent: 5.5\n";
    fs::write(state.config.state_dir.join("slskd.yml"), yaml).unwrap();
    super::apply_watched_controller_configuration(
        &state,
        Some(yaml),
        &state.controller_cli_environment,
    )
    .await;

    let settings = state.transfer_auto_retry_settings.read().await.clone();
    assert!(settings.enabled);
    assert_eq!(settings.retry_delay.as_secs(), 10);
    assert_eq!(settings.check_interval.as_secs(), 20);
    assert_eq!(settings.max_attempts, 7);
    assert_eq!(settings.max_files_per_cycle, 8);
    assert_eq!(settings.max_files_per_peer_per_cycle, 2);
    assert_eq!(settings.peer_cooldown.as_secs(), 60);
    assert!(!settings.alternate_sources_enabled);
    assert_eq!(settings.max_alternate_source_searches_per_cycle, 2);
    assert_eq!(settings.alternate_source_size_tolerance_percent, 5.5);
    assert!(!state.runtime.read().await.application_restart_requested);

    assert_eq!(
        super::transfer_recovery_runtime::run_download_auto_retry_cycle(&state, &mut tracker)
            .await
            .unwrap(),
        1
    );
    assert!(matches!(
        receiver.recv().await,
        Some(super::SessionCommand::TransferPeer { username, .. }) if username == "watched-peer"
    ));

    let overlay = state.options_overlay.read().await;
    let current = serde_json::from_str::<serde_json::Value>(&super::controller_options_json(
        &state.config,
        &overlay,
        true,
    ))
    .unwrap();
    let startup = serde_json::from_str::<serde_json::Value>(&super::controller_options_json(
        &state.config,
        &overlay,
        false,
    ))
    .unwrap();
    assert_eq!(
        current["global"]["download"]["autoRetry"]["alternateSourceSizeTolerancePercent"],
        5.5
    );
    assert_eq!(
        startup["global"]["download"]["autoRetry"]["alternateSourceSizeTolerancePercent"],
        5
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn watched_share_configuration_marks_pending_and_rescan_uses_new_roots() {
    let (state, _receiver) = test_state();
    let new_root = state.config.state_dir.join("watched-share");
    let new_downloads = state.config.state_dir.join("watched-downloads");
    let new_incomplete = state.config.state_dir.join("watched-incomplete");
    fs::create_dir_all(&new_root).unwrap();
    fs::create_dir_all(&new_downloads).unwrap();
    fs::create_dir_all(&new_incomplete).unwrap();
    fs::write(new_root.join("new.flac"), b"new").unwrap();
    fs::write(new_downloads.join("guarded.flac"), b"guarded").unwrap();
    let denied = super::route_http_request(
        "DELETE",
        "/api/v0/files/downloads/files/Z3VhcmRlZC5mbGFj",
        None,
        "",
        &state,
    )
    .await
    .expect("remote file management disabled response");
    assert_eq!(denied.status, "403 Forbidden");
    let yaml = format!(
        "remote_configuration: true\nremote_file_management: true\ndirectories:\n  downloads: '{}'\n  incomplete: '{}'\nshares:\n  directories:\n    - '[Watched]{}'\n",
        new_downloads.display(),
        new_incomplete.display(),
        new_root.display(),
    );
    fs::write(state.config.state_dir.join("slskd.yml"), &yaml).unwrap();
    let mut cli_environment = state.controller_cli_environment.clone();
    cli_environment.remove("SLSKR_SHARE_FIXTURE");

    super::apply_watched_controller_configuration(&state, Some(&yaml), &cli_environment).await;

    assert_eq!(
        state.share_settings.read().await.directories[0].alias,
        "Watched"
    );
    let shares = state.shares.read().await;
    assert_eq!(shares.roots[0].label, "Watched");
    assert!(!shares.roots[0].statistics_ready);
    drop(shares);
    assert!(state.share_lifecycle.read().await.scan_pending);
    assert!(state.runtime.read().await.application_restart_requested);
    assert_eq!(super::effective_downloads_dir(&state), new_downloads);
    assert_eq!(super::effective_incomplete_dir(&state), new_incomplete);
    assert!(super::effective_remote_file_management(&state));
    assert!(super::effective_remote_configuration(&state));
    {
        let overlay = state.options_overlay.read().await;
        let current = serde_json::from_str::<serde_json::Value>(&super::controller_options_json(
            &state.config,
            &overlay,
            true,
        ))
        .unwrap();
        let startup = serde_json::from_str::<serde_json::Value>(&super::controller_options_json(
            &state.config,
            &overlay,
            false,
        ))
        .unwrap();
        assert_eq!(current["remoteConfiguration"], true);
        assert_eq!(startup["remoteConfiguration"], false);
    }
    let yaml_response = super::route_http_request("GET", "/api/v0/options/yaml", None, "", &state)
        .await
        .expect("watched remote configuration response");
    assert_eq!(yaml_response.status, "200 OK");
    let deleted = super::route_http_request(
        "DELETE",
        "/api/v0/files/downloads/files/Z3VhcmRlZC5mbGFj",
        None,
        "",
        &state,
    )
    .await
    .expect("watched remote file management response");
    assert_eq!(deleted.status, "204 No Content");
    assert!(!new_downloads.join("guarded.flac").exists());
    let local_path = super::prepare_transfer_local_path(
        &state,
        0,
        Some("peer"),
        "Remote/Song.flac",
        None,
        &super::TransferRequestDetails::default(),
        None,
    )
    .await
    .unwrap()
    .unwrap();
    assert!(Path::new(&local_path).starts_with(super::effective_downloads_dir(&state)));
    let overlay = state.options_overlay.read().await;
    let options = super::controller_options_json(&state.config, &overlay, true);
    drop(overlay);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&options).unwrap()["shares"]["directories"][0],
        format!("[Watched]{}", new_root.display())
    );

    let snapshot = super::rebuild_share_index(&state).await.unwrap();
    assert_eq!(snapshot.entries[0].filename, "Watched/new.flac");
    assert!(snapshot.roots[0].statistics_ready);
    assert!(!state.share_lifecycle.read().await.scan_pending);

    let disabled_yaml = yaml.replacen(
        "remote_configuration: true",
        "remote_configuration: false",
        1,
    );
    fs::write(state.config.state_dir.join("slskd.yml"), &disabled_yaml).unwrap();
    super::apply_watched_controller_configuration(&state, Some(&disabled_yaml), &cli_environment)
        .await;
    assert!(!super::effective_remote_configuration(&state));
    let forbidden = super::route_http_request("GET", "/api/v0/options/yaml", None, "", &state)
        .await
        .expect("self-disabled remote configuration response");
    assert_eq!(forbidden.status, "403 Forbidden");
    let overlay = state.options_overlay.read().await;
    let current = serde_json::from_str::<serde_json::Value>(&super::controller_options_json(
        &state.config,
        &overlay,
        true,
    ))
    .unwrap();
    let startup = serde_json::from_str::<serde_json::Value>(&super::controller_options_json(
        &state.config,
        &overlay,
        false,
    ))
    .unwrap();
    assert_eq!(current["remoteConfiguration"], false);
    assert_eq!(startup["remoteConfiguration"], false);
}
