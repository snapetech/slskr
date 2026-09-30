//! Controller full peer network differential ownership.

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
pub(super) async fn controller_api_differential_portforwarding_start_readback() {
    // The oracle's real route (ASP.NET's default [controller] token for
    // PortForwardingController has no hyphen) is
    // "api/v0/portforwarding/start". A routing-table typo previously
    // mapped that exact path to an internal literal that only a
    // fake-success stub matched -- any real caller using the real
    // oracle-spelled path always got a canned "Port forwarding
    // started" message with none of the real handler's pod-membership/
    // gateway-pinning validation ever running and no forward ever
    // actually starting. This proves the fixed routing table now
    // reaches the same real handler already exercised (via the
    // internal hyphenated spelling) by the sibling tests above.
    let (state, _receiver) = test_state_with_env(MapEnv::default().with(
        "SLSKR_TEST_USER_ENDPOINT_OVERRIDES",
        "gateway=127.0.0.1:2234",
    ));
    let pin = "07".repeat(32);
    let create = crate::route_http_request(
        "POST",
        "/api/v0/pods",
        None,
        &format!(
            r#"{{"pod":{{"podId":"pod-oracle-forward","name":"Oracle Forward","capabilities":[0],"privateServicePolicy":{{"enabled":true,"maxMembers":2,"gatewayPeerId":"tester","gatewayCertificateSha256":"{pin}","registeredServices":[],"allowedDestinations":[{{"hostPattern":"service","port":80,"protocol":"tcp","allowPublic":false}}]}}}}}}"#
        ),
        &state,
    )
    .await
    .expect("create gateway pod");
    assert_eq!(create.status, "201 Created", "{}", create.body);
    let mut gateway = test_capability_descriptor(
        "gateway",
        vec![slskr_client::capabilities::FEATURE_MESH_V1.to_owned()],
    );
    gateway.peer_id = "tester".to_owned();
    state.mesh.write().await.capability_records.push(gateway);
    let probe = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("port probe");
    let local_port = probe.local_addr().unwrap().port();
    drop(probe);

    let start = crate::route_http_request(
        "POST",
        "/api/v0/portforwarding/start",
        None,
        &format!(
            r#"{{"localPort":{local_port},"podId":"pod-oracle-forward","destinationHost":"service","destinationPort":80}}"#
        ),
        &state,
    )
    .await
    .expect("start port forwarding via the oracle-spelled route");
    assert_eq!(start.status, "200 OK", "{}", start.body);
    let status = crate::route_http_request(
        "GET",
        &format!("/api/v0/port-forwarding/status/{local_port}"),
        None,
        "",
        &state,
    )
    .await
    .expect("forwarding status");
    assert_eq!(status.status, "200 OK");
    assert!(status.body.contains("pod-oracle-forward"));

    let stop = crate::route_http_request(
        "POST",
        &format!("/api/port-forwarding/stop/{local_port}"),
        None,
        "",
        &state,
    )
    .await
    .expect("stop port forwarding");
    assert_eq!(stop.status, "200 OK");

    let ledger = [serde_json::json!({
        "target": "slskdn",
        "method": "POST",
        "route": "/api/v0/portforwarding/start",
        "case": "mutation-side-effects-and-readback",
        "pass": true,
    })];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("portforwarding_start_readback.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
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
pub(super) async fn controller_api_differential_peer_and_mesh_preview_stream_tickets_are_short_lived(
) {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));

    {
        let mut searches = state.searches.write().await;
        searches
            .create(None, "preview".to_owned(), "global", None, Vec::new(), 300)
            .expect("create preview search");
        searches
            .records
            .last_mut()
            .unwrap()
            .results
            .push(crate::SearchResultEntry {
                peer_username: Some("peer".to_owned()),
                filename: "Remote/Other.flac".to_owned(),
                size: 42,
                extension: "flac".to_owned(),
                bit_rate: None,
                sample_rate: None,
                bit_depth: None,
                length_seconds: None,
                locked: false,
                slot_free: Some(true),
                average_speed: Some(1),
                queue_length: Some(0),
            });
    }

    let unmatched = crate::route_http_request(
        "POST",
        "/api/v0/peer-streams/tickets",
        None,
        r#"{"filename":"Remote/Requested.flac","username":"peer"}"#,
        &state,
    )
    .await
    .expect("unmatched peer ticket");
    assert_eq!(unmatched.status, "200 OK", "{}", unmatched.body);
    assert_eq!(unmatched.content_type, "application/json");
    let unmatched_json = serde_json::from_str::<serde_json::Value>(&unmatched.body).unwrap();
    assert_eq!(unmatched_json["filename"], "Remote/Requested.flac");

    let peer_ticket = crate::route_http_request(
        "POST",
        "/api/v0/peer-streams/tickets",
        None,
        r#"{"filename":"Virtual/Test.flac","username":"peer"}"#,
        &state,
    )
    .await
    .expect("peer ticket");
    assert_eq!(peer_ticket.status, "200 OK");
    assert_eq!(peer_ticket.content_type, "application/json");
    let peer_json = serde_json::from_str::<serde_json::Value>(&peer_ticket.body).unwrap();
    assert_eq!(peer_json["expiresInSeconds"], 120);
    assert_eq!(peer_json["source"], "local-share");
    assert_eq!(peer_json["streamUrl"], peer_json["stream_url"]);

    let peer_stream = crate::route_http_request(
        "GET",
        peer_json["streamUrl"].as_str().unwrap(),
        None,
        "",
        &state,
    )
    .await
    .expect("peer stream");
    assert_eq!(peer_stream.status, "200 OK");
    assert_eq!(peer_stream.content_type, "application/json");
    let stream_json = serde_json::from_str::<serde_json::Value>(&peer_stream.body).unwrap();
    assert_eq!(stream_json["status"], "available");
    assert_eq!(stream_json["acceptRanges"], "none");
    assert_eq!(stream_json["cacheControl"], "no-store");

    let mesh_ticket = crate::route_http_request(
        "POST",
        "/api/v0/mesh-streams/tickets",
        None,
        r#"{"contentId":"mesh-content","filename":"Virtual/Test.flac","peerId":"mesh-peer"}"#,
        &state,
    )
    .await
    .expect("mesh ticket");
    assert_eq!(mesh_ticket.status, "200 OK");
    assert_eq!(mesh_ticket.content_type, "application/json");
    let mesh_json = serde_json::from_str::<serde_json::Value>(&mesh_ticket.body).unwrap();
    assert!(mesh_json["streamUrl"]
        .as_str()
        .unwrap()
        .starts_with("/api/v0/mesh-streams/"));

    let mesh_stream = crate::route_http_request(
        "GET",
        mesh_json["streamUrl"].as_str().unwrap(),
        None,
        "",
        &state,
    )
    .await
    .expect("mesh stream");
    assert_eq!(mesh_stream.status, "200 OK", "{}", mesh_stream.body);
    assert_eq!(mesh_stream.content_type, "application/json");
    let mesh_stream_json = serde_json::from_str::<serde_json::Value>(&mesh_stream.body).unwrap();
    assert_eq!(mesh_stream_json["status"], "available");
    assert_eq!(mesh_stream_json["cacheControl"], "no-store");

    let missing =
        crate::route_http_request("GET", "/api/v0/peer-streams/not-a-ticket", None, "", &state)
            .await
            .expect("missing ticket");
    assert_eq!(missing.status, "404 Not Found");
    assert_eq!(missing.content_type, "application/json");

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("peer_mesh_stream_tickets.json"),
        serde_json::to_string_pretty(&[
            serde_json::json!({
                "target": "slskdn",
                "method": "POST",
                "route": "/api/v0/peer-streams/tickets",
                "case": "nominal-status-headers-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/peer-streams/{ticket}",
                "case": "populated-dynamic-state",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "POST",
                "route": "/api/v0/mesh-streams/tickets",
                "case": "nominal-status-headers-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/mesh-streams/{ticket}",
                "case": "populated-dynamic-state",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/peer-streams/{ticket}",
                "case": "missing-empty-or-conflict-state",
                "pass": true,
            }),
        ])
        .expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
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
pub(super) async fn controller_api_differential_peer_stream_ticket_validation_and_limits() {
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

    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let nominal = crate::route_http_request(
        "POST",
        "/api/v0/peer-streams/tickets",
        None,
        r#"{"filename":"Track.mp3","username":"peer","size":321}"#,
        &state,
    )
    .await
    .expect("peer ticket nominal response");
    let nominal_json = serde_json::from_str::<serde_json::Value>(&nominal.body)
        .unwrap_or_else(|_| serde_json::json!({}));
    let nominal_ticket = nominal_json["ticket"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    record!(
        "POST",
        "/api/v0/peer-streams/tickets",
        "nominal-status-headers-body",
        nominal.status == "200 OK"
            && nominal.content_type == "application/json"
            && nominal_json["contentType"] == "audio/mpeg"
            && nominal_json["expiresInSeconds"] == 120
            && nominal_json["streamUrl"] == format!("/api/v0/peer-streams/{nominal_ticket}")
            && nominal_json["stream_url"] == nominal_json["streamUrl"]
            && nominal_json["size"] == 321
            && !nominal_ticket.is_empty()
    );

    let stored = state
        .stream_tickets
        .write()
        .await
        .get(&nominal_ticket)
        .is_some_and(|ticket| {
            ticket.family == "peer"
                && ticket.filename == "Track.mp3"
                && ticket.peer_username.as_deref() == Some("peer")
                && ticket.size == 321
        });
    record!(
        "POST",
        "/api/v0/peer-streams/tickets",
        "mutation-side-effects-and-readback",
        stored
    );

    let nominal_get = crate::route_http_request(
        "GET",
        &format!("/api/v0/peer-streams/{nominal_ticket}"),
        None,
        "",
        &state,
    )
    .await
    .expect("peer ticket nominal read");
    let nominal_get_json = serde_json::from_str::<serde_json::Value>(&nominal_get.body)
        .unwrap_or_else(|_| serde_json::json!({}));
    record!(
        "GET",
        "/api/v0/peer-streams/{ticket}",
        "nominal-status-headers-body",
        nominal_get.status == "200 OK"
            && nominal_get.content_type == "application/json"
            && nominal_get_json["status"] == "available"
            && nominal_get_json["filename"] == "Track.mp3"
            && nominal_get_json["peer_username"] == "peer"
            && nominal_get_json["size"] == 321
            && nominal_get_json["cacheControl"] == "no-store"
            && nominal_get_json["acceptRanges"] == "none"
    );
    record!(
        "GET",
        "/api/v0/peer-streams/{ticket}",
        "populated-dynamic-state",
        nominal_get_json["status"] == "available"
            && nominal_get_json["ticket"] == nominal_ticket
            && nominal_get_json["contentType"] == "audio/mpeg"
    );

    let malformed_bodies = [
        r#"{"filename":"../escape.flac","username":"peer"}"#,
        r#"{"filename":"Track.flac","username":"peer","size":-1}"#,
        r#"{"filename":"archive.zip","username":"peer"}"#,
        r#"{"filename":"Track.flac","username":"   "}"#,
    ];
    let mut malformed = true;
    let mut malformed_details = Vec::new();
    for body in malformed_bodies {
        let response =
            crate::route_http_request("POST", "/api/v0/peer-streams/tickets", None, body, &state)
                .await
                .expect("peer ticket malformed response");
        let pass = response.status == "400 Bad Request"
            && response.content_type == "application/json"
            && serde_json::from_str::<serde_json::Value>(&response.body)
                .map(|value| {
                    value["error"]
                        .as_str()
                        .is_some_and(|error| !error.is_empty())
                })
                .unwrap_or(false);
        if !pass {
            malformed_details.push(format!(
                "body={body:?} response={} {} {:?}",
                response.status, response.content_type, response.body
            ));
        }
        malformed &= pass;
    }
    record!(
        "POST",
        "/api/v0/peer-streams/tickets",
        "malformed-path-query-or-body",
        malformed
    );

    let missing =
        crate::route_http_request("GET", "/api/v0/peer-streams/not-a-ticket", None, "", &state)
            .await
            .expect("peer ticket missing response");
    record!(
        "GET",
        "/api/v0/peer-streams/{ticket}",
        "missing-empty-or-conflict-state",
        missing.status == "404 Not Found" && missing.content_type == "application/json"
    );

    let malformed_path = crate::route_http_request(
        "GET",
        "/api/v0/peer-streams/not-a-ticket/extra",
        None,
        "",
        &state,
    )
    .await
    .expect("peer ticket malformed path response");
    record!(
        "GET",
        "/api/v0/peer-streams/{ticket}",
        "malformed-path-query-or-body",
        malformed_path.status == "404 Not Found"
    );

    let (disabled_state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with(
                "SLSKR_ADVANCED_NETWORKING_JSON",
                r#"{"feature":{"streaming":false}}"#,
            ),
    );
    let disabled_post = crate::route_http_request(
        "POST",
        "/api/v0/peer-streams/tickets",
        None,
        r#"{"filename":"Track.flac","username":"peer"}"#,
        &disabled_state,
    )
    .await
    .expect("disabled peer ticket create response");
    record!(
        "POST",
        "/api/v0/peer-streams/tickets",
        "missing-empty-or-conflict-state",
        disabled_post.status == "404 Not Found"
    );

    let (capacity_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    {
        let mut tickets = capacity_state.stream_tickets.write().await;
        for index in 0..crate::MAX_PREVIEW_STREAM_TICKETS {
            assert!(tickets
                .issue(
                    "peer",
                    "peer-unresolved",
                    format!("peer-capacity-{index}"),
                    "Track.flac".to_owned(),
                    Some("peer".to_owned()),
                    0,
                    "audio/flac".to_owned(),
                    120,
                )
                .is_some());
        }
    }
    let capacity = crate::route_http_request(
        "POST",
        "/api/v0/peer-streams/tickets",
        None,
        r#"{"filename":"Capacity.flac","username":"peer"}"#,
        &capacity_state,
    )
    .await
    .expect("peer ticket capacity response");
    record!(
        "POST",
        "/api/v0/peer-streams/tickets",
        "runtime-failure-and-timeout",
        capacity.status == "429 Too Many Requests"
            && capacity.content_type == "text/plain; charset=utf-8"
            && capacity.body == "Peer stream limit reached."
    );

    let failure_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("peer stream runtime-failure database");
    let (failure_state, _failure_receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target),
        crate::SearchStore::new(),
        Some(failure_db.clone()),
    );
    let failure_ticket = crate::route_http_request(
        "POST",
        "/api/v0/peer-streams/tickets",
        None,
        r#"{"filename":"Runtime.flac","username":"peer"}"#,
        &failure_state,
    )
    .await
    .expect("create peer runtime-failure ticket");
    let failure_ticket_json = serde_json::from_str::<serde_json::Value>(&failure_ticket.body)
        .unwrap_or_else(|_| serde_json::json!({}));
    failure_db.close_for_test().await;
    let failure_get = crate::route_http_request(
        "GET",
        &format!(
            "/api/v0/peer-streams/{}",
            failure_ticket_json["ticket"].as_str().unwrap_or_default()
        ),
        None,
        "",
        &failure_state,
    )
    .await
    .expect("peer stream read with closed unrelated database");
    record!(
        "GET",
        "/api/v0/peer-streams/{ticket}",
        "runtime-failure-and-timeout",
        failure_get.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&failure_get.body)
                .map(|value| value["status"] == "available")
                .unwrap_or(false)
    );

    let (restarted_state, _restarted_receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let reset_get = crate::route_http_request(
        "GET",
        &format!("/api/v0/peer-streams/{nominal_ticket}"),
        None,
        "",
        &restarted_state,
    )
    .await
    .expect("peer stream ticket after restart");
    let reset_create = crate::route_http_request(
        "POST",
        "/api/v0/peer-streams/tickets",
        None,
        r#"{"filename":"Restart.flac","username":"peer"}"#,
        &restarted_state,
    )
    .await
    .expect("peer stream ticket create after restart");
    record!(
        "POST",
        "/api/v0/peer-streams/tickets",
        "restart-persistence-or-reset",
        reset_get.status == "404 Not Found" && reset_create.status == "200 OK"
    );

    let (concurrent_state, _concurrent_receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let concurrent = tokio::join!(
        crate::route_http_request(
            "POST",
            "/api/v0/peer-streams/tickets",
            None,
            r#"{"filename":"A.flac","username":"peer"}"#,
            &concurrent_state,
        ),
        crate::route_http_request(
            "POST",
            "/api/v0/peer-streams/tickets",
            None,
            r#"{"filename":"B.flac","username":"peer"}"#,
            &concurrent_state,
        ),
    );
    let concurrent_pass = match concurrent {
        (Ok(left), Ok(right)) => {
            let left_json = serde_json::from_str::<serde_json::Value>(&left.body)
                .unwrap_or_else(|_| serde_json::json!({}));
            let right_json = serde_json::from_str::<serde_json::Value>(&right.body)
                .unwrap_or_else(|_| serde_json::json!({}));
            left.status == "200 OK"
                && right.status == "200 OK"
                && left_json["ticket"].as_str().is_some_and(|ticket| {
                    !ticket.is_empty()
                        && ticket != right_json["ticket"].as_str().unwrap_or_default()
                })
                && concurrent_state.stream_tickets.read().await.records.len() == 2
        }
        _ => false,
    };
    record!(
        "POST",
        "/api/v0/peer-streams/tickets",
        "concurrency-and-idempotency",
        concurrent_pass
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("peer_stream_ticket_validation_and_limits.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        mismatches.is_empty(),
        "{} peer stream ticket mismatches:\n{}\nmalformed details: {:?}",
        mismatches.len(),
        mismatches.join("\n"),
        malformed_details
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
pub(super) async fn controller_api_differential_soulseek_user_interests_route_returns_remote_server_response(
) {
    use slskr_client::protocol::server::{ServerMessage, UserInterests};

    let (state, mut receiver) = test_state();
    state.session.write().await.state = "connected";
    let task_state = Arc::clone(&state);
    let task = tokio::spawn(async move {
        crate::route_http_request(
            "GET",
            "/api/v0/soulseek/users/remote-peer/interests",
            None,
            "",
            &task_state,
        )
        .await
    });
    assert_eq!(
        receiver.recv().await.unwrap(),
        crate::SessionCommand::RequestUserInterests("remote-peer".to_owned())
    );
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
    crate::session_runtime::project_server_message(
        &state,
        &mut session,
        &ServerMessage::UserInterests(UserInterests {
            username: "remote-peer".to_owned(),
            liked: vec!["drum and bass".to_owned()],
            hated: vec!["bad rips".to_owned()],
        }),
    )
    .await;
    let response = task.await.unwrap().unwrap();
    assert_eq!(response.status, "200 OK");
    assert!(response.content_type.contains("application/json"));
    let json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
    assert_eq!(json["username"], "remote-peer");
    assert_eq!(json["liked"], serde_json::json!(["drum and bass"]));
    assert_eq!(json["hated"], serde_json::json!(["bad rips"]));
    let opinions = crate::route_http_request("GET", "/api/v0/opinions", None, "", &state)
        .await
        .unwrap();
    let opinions_json = serde_json::from_str::<serde_json::Value>(&opinions.body).unwrap();
    assert_eq!(opinions_json.as_array().unwrap().len(), 2);
    assert!(opinions.body.contains("soulseek-interest"));
    let replacement = slskr_client::protocol::server::UserInterests {
        username: "remote-peer".to_owned(),
        liked: vec!["new-track".to_owned()],
        hated: Vec::new(),
    };
    crate::session_runtime::project_server_message(
        &state,
        &mut session,
        &ServerMessage::UserInterests(replacement),
    )
    .await;
    let opinions = crate::route_http_request("GET", "/api/v0/opinions", None, "", &state)
        .await
        .unwrap();
    assert!(!opinions.body.contains("drum and bass"));
    assert!(opinions.body.contains("new-track"));

    let ledger = vec![
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/soulseek/users/{username}/interests",
            "case": "nominal-status-headers-body",
            "pass": true,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/soulseek/users/{username}/interests",
            "case": "populated-dynamic-state",
            "pass": true,
        }),
    ];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("soulseek_user_interests_projection.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}

/// Bulk differential proof crediting 6 port-forwarding routes' cases,
/// independently re-derived from `port_forwarding_reads_are_bounded_
/// and_start_requires_an_authorized_pinned_gateway`'s real bounded
/// port-listing and gateway-pinning security checks (start is
/// forbidden without a real, authorized, certificate-pinned gateway
/// pod, and succeeds once one genuinely exists). The registry lists
/// these routes as `/api/v0/portforwarding/...` (no hyphen);
/// `normalize_api_path` maps that to the hyphenated `/api/port-
/// forwarding/...` internal form the source test calls directly --
/// both resolve to the same handler (see the `normalize_api_path`
/// unit test asserting this exact mapping), so either request path
/// exercises the same registered route. slskdN-only (confirmed
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
pub(super) async fn controller_api_differential_port_forwarding() {
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

    let (state, _receiver) = test_state_with_env(MapEnv::default().with(
        "SLSKR_TEST_USER_ENDPOINT_OVERRIDES",
        "gateway=127.0.0.1:2234",
    ));

    let status =
        crate::route_http_request("GET", "/api/v0/portforwarding/status", None, "", &state)
            .await
            .expect("port forwarding status");
    record!(
        "GET",
        "/api/v0/portforwarding/status",
        "nominal-status-headers-body",
        status.status == "200 OK" && status.body == "[]"
    );
    let malformed_status_collection = crate::route_http_request(
        "GET",
        "/api/v0/portforwarding/status/extra/more",
        None,
        "",
        &state,
    )
    .await
    .expect("malformed port forwarding status collection");
    record!(
        "GET",
        "/api/v0/portforwarding/status",
        "malformed-path-query-or-body",
        malformed_status_collection.status == "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/portforwarding/status",
        "missing-empty-or-conflict-state",
        status.status == "200 OK" && status.body == "[]"
    );

    let available = crate::route_http_request(
        "GET",
        "/api/v0/portforwarding/available-ports?startPort=2000&endPort=2010&limit=3",
        None,
        "",
        &state,
    )
    .await
    .expect("available port page");
    let available_json =
        serde_json::from_str::<serde_json::Value>(&available.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/portforwarding/available-ports",
        "nominal-status-headers-body",
        available.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/portforwarding/available-ports",
        "populated-dynamic-state",
        available_json["availablePortCount"] == 11
            && available_json["usedPortCount"] == 0
            && available_json["availablePorts"] == serde_json::json!([2000, 2001, 2002])
    );
    let available_default = crate::route_http_request(
        "GET",
        "/api/v0/portforwarding/available-ports",
        None,
        "",
        &state,
    )
    .await
    .expect("default available ports");
    record!(
        "GET",
        "/api/v0/portforwarding/available-ports",
        "missing-empty-or-conflict-state",
        available_default.status == "200 OK"
    );

    let mut invalid_query_pass = true;
    for path in [
        "/api/v0/portforwarding/available-ports?startPort=0",
        "/api/v0/portforwarding/available-ports?startPort=3000&endPort=2000",
        "/api/v0/portforwarding/available-ports?limit=0",
        "/api/v0/portforwarding/available-ports?limit=1001",
    ] {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        invalid_query_pass &= response.status == "400 Bad Request";
    }
    record!(
        "GET",
        "/api/v0/portforwarding/available-ports",
        "malformed-path-query-or-body",
        invalid_query_pass
    );

    let stats = crate::route_http_request(
        "GET",
        "/api/v0/portforwarding/stream-stats",
        None,
        "",
        &state,
    )
    .await
    .expect("port forwarding stream stats");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/portforwarding/stream-stats",
        "nominal-status-headers-body",
        stats.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/portforwarding/stream-stats",
        "populated-dynamic-state",
        stats_json["totalForwardingRules"] == 0 && stats_json["rules"] == serde_json::json!([])
    );
    let malformed_stats = crate::route_http_request(
        "GET",
        "/api/v0/portforwarding/stream-stats/extra",
        None,
        "",
        &state,
    )
    .await
    .expect("malformed port forwarding stream stats");
    record!(
        "GET",
        "/api/v0/portforwarding/stream-stats",
        "malformed-path-query-or-body",
        malformed_stats.status == "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/portforwarding/stream-stats",
        "missing-empty-or-conflict-state",
        stats.status == "200 OK" && stats_json["totalForwardingRules"] == 0
    );

    let missing = crate::route_http_request(
        "GET",
        "/api/v0/portforwarding/status/2000",
        None,
        "",
        &state,
    )
    .await
    .expect("missing port forwarding status");
    record!(
        "GET",
        "/api/v0/portforwarding/status/{localPort:int}",
        "missing-empty-or-conflict-state",
        missing.status == "404 Not Found"
    );
    let malformed_status = crate::route_http_request(
        "GET",
        "/api/v0/portforwarding/status/not-a-port",
        None,
        "",
        &state,
    )
    .await
    .expect("malformed port forwarding status");
    record!(
        "GET",
        "/api/v0/portforwarding/status/{localPort:int}",
        "malformed-path-query-or-body",
        malformed_status.status == "404 Not Found"
    );

    let malformed_start = crate::route_http_request(
        "POST",
        "/api/v0/portforwarding/start/extra",
        None,
        "{}",
        &state,
    )
    .await
    .expect("malformed port forwarding start");
    record!(
        "POST",
        "/api/v0/portforwarding/start",
        "malformed-path-query-or-body",
        malformed_start.status == "404 Not Found"
    );

    let start_unpinned = crate::route_http_request(
        "POST",
        "/api/v0/portforwarding/start",
        None,
        r#"{"localPort":2000,"podId":"pod-differential-unpinned","destinationHost":"service","destinationPort":80}"#,
        &state,
    )
    .await
    .expect("unauthorized port forwarding start");
    record!(
        "POST",
        "/api/v0/portforwarding/start",
        "missing-empty-or-conflict-state",
        start_unpinned.status == "403 Forbidden"
    );

    let pin = "07".repeat(32);
    let create = crate::route_http_request(
        "POST",
        "/api/v0/pods",
        None,
        &format!(
            r#"{{"pod":{{"podId":"pod-differential-forward","name":"Forward Differential","capabilities":[0],"privateServicePolicy":{{"enabled":true,"maxMembers":2,"gatewayPeerId":"tester","gatewayCertificateSha256":"{pin}","registeredServices":[],"allowedDestinations":[{{"hostPattern":"service","port":80,"protocol":"tcp","allowPublic":false}}]}}}}}}"#
        ),
        &state,
    )
    .await
    .expect("create gateway pod fixture");
    assert_eq!(create.status, "201 Created", "{}", create.body);

    let mut gateway = test_capability_descriptor(
        "gateway",
        vec![slskr_client::capabilities::FEATURE_MESH_V1.to_owned()],
    );
    gateway.peer_id = "tester".to_owned();
    state.mesh.write().await.capability_records.push(gateway);

    let probe = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("port probe");
    let local_port = probe.local_addr().unwrap().port();
    drop(probe);

    let start = crate::route_http_request(
        "POST",
        "/api/v0/portforwarding/start",
        None,
        &format!(
            r#"{{"localPort":{local_port},"podId":"pod-differential-forward","destinationHost":"service","destinationPort":80}}"#
        ),
        &state,
    )
    .await
    .expect("start port forwarding");
    record!(
        "POST",
        "/api/v0/portforwarding/start",
        "nominal-status-headers-body",
        start.status == "200 OK"
    );
    let repeated_start = crate::route_http_request(
        "POST",
        "/api/v0/portforwarding/start",
        None,
        &format!(
            r#"{{"localPort":{local_port},"podId":"pod-differential-forward","destinationHost":"service","destinationPort":80}}"#
        ),
        &state,
    )
    .await
    .expect("repeated port forwarding start");
    record!(
        "POST",
        "/api/v0/portforwarding/start",
        "concurrency-and-idempotency",
        start.status == "200 OK" && repeated_start.status == "409 Conflict"
    );

    let status_route = format!("/api/v0/portforwarding/status/{local_port}");
    let running_status = crate::route_http_request("GET", &status_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{status_route}: {error}"));
    record!(
        "GET",
        "/api/v0/portforwarding/status/{localPort:int}",
        "nominal-status-headers-body",
        running_status.status == "200 OK"
            && running_status.body.contains("pod-differential-forward")
    );
    record!(
        "GET",
        "/api/v0/portforwarding/status/{localPort:int}",
        "populated-dynamic-state",
        running_status.status == "200 OK"
            && running_status.body.contains("pod-differential-forward")
    );
    record!(
        "POST",
        "/api/v0/portforwarding/start",
        "mutation-side-effects-and-readback",
        start.status == "200 OK"
            && running_status.status == "200 OK"
            && running_status.body.contains("pod-differential-forward")
    );

    let populated_status =
        crate::route_http_request("GET", "/api/v0/portforwarding/status", None, "", &state)
            .await
            .expect("populated port forwarding status");
    record!(
        "GET",
        "/api/v0/portforwarding/status",
        "populated-dynamic-state",
        populated_status.status == "200 OK"
            && populated_status.body.contains("pod-differential-forward")
    );

    let stop_route = format!("/api/v0/portforwarding/stop/{local_port}");
    let stop = crate::route_http_request("POST", &stop_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{stop_route}: {error}"));
    record!(
        "POST",
        "/api/v0/portforwarding/stop/{localPort:int}",
        "mutation-side-effects-and-readback",
        stop.status == "200 OK"
    );
    let stop_missing = crate::route_http_request("POST", &stop_route, None, "", &state)
        .await
        .expect("missing port forwarding stop");
    record!(
        "POST",
        "/api/v0/portforwarding/stop/{localPort:int}",
        "missing-empty-or-conflict-state",
        stop_missing.status == "200 OK"
    );
    let malformed_stop = crate::route_http_request(
        "POST",
        "/api/v0/portforwarding/stop/not-a-port",
        None,
        "",
        &state,
    )
    .await
    .expect("malformed port forwarding stop");
    record!(
        "POST",
        "/api/v0/portforwarding/stop/{localPort:int}",
        "malformed-path-query-or-body",
        malformed_stop.status == "404 Not Found"
    );

    let occupied = tokio::net::TcpListener::bind(("127.0.0.1", local_port))
        .await
        .expect("occupy forwarding local port");
    let runtime_start = crate::route_http_request(
        "POST",
        "/api/v0/portforwarding/start",
        None,
        &format!(
            r#"{{"localPort":{local_port},"podId":"pod-differential-forward","destinationHost":"service","destinationPort":80}}"#
        ),
        &state,
    )
    .await
    .expect("runtime port forwarding start");
    drop(occupied);
    record!(
        "POST",
        "/api/v0/portforwarding/start",
        "runtime-failure-and-timeout",
        runtime_start.status == "500 Internal Server Error"
            && runtime_start
                .body
                .contains("Failed to start port forwarding")
    );

    let (restart_state, _restart_receiver) = test_state_with_env(MapEnv::default().with(
        "SLSKR_TEST_USER_ENDPOINT_OVERRIDES",
        "gateway=127.0.0.1:2234",
    ));
    let restart_pod = crate::route_http_request(
        "POST",
        "/api/v0/pods",
        None,
        &format!(
            r#"{{"pod":{{"podId":"pod-differential-forward-restart","name":"Forward Restart","capabilities":[0],"privateServicePolicy":{{"enabled":true,"maxMembers":2,"gatewayPeerId":"tester","gatewayCertificateSha256":"{pin}","registeredServices":[],"allowedDestinations":[{{"hostPattern":"service","port":80,"protocol":"tcp","allowPublic":false}}]}}}}}}"#
        ),
        &restart_state,
    )
    .await
    .expect("create restarted gateway pod");
    assert_eq!(restart_pod.status, "201 Created", "{}", restart_pod.body);
    let mut restart_gateway = test_capability_descriptor(
        "gateway",
        vec![slskr_client::capabilities::FEATURE_MESH_V1.to_owned()],
    );
    restart_gateway.peer_id = "tester".to_owned();
    restart_state
        .mesh
        .write()
        .await
        .capability_records
        .push(restart_gateway);
    let restart_probe = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("restarted port probe");
    let restart_port = restart_probe.local_addr().unwrap().port();
    drop(restart_probe);
    let restart_start = crate::route_http_request(
        "POST",
        "/api/v0/portforwarding/start",
        None,
        &format!(
            r#"{{"localPort":{restart_port},"podId":"pod-differential-forward-restart","destinationHost":"service","destinationPort":80}}"#
        ),
        &restart_state,
    )
    .await
    .expect("restarted port forwarding start");
    record!(
        "POST",
        "/api/v0/portforwarding/start",
        "restart-persistence-or-reset",
        restart_start.status == "200 OK"
    );
    let _ = restart_state.port_forwarding.stop(restart_port).await;

    let (stop_restart_state, _stop_restart_receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let stop_restart = crate::route_http_request(
        "POST",
        "/api/v0/portforwarding/stop/29999",
        None,
        "",
        &stop_restart_state,
    )
    .await
    .expect("restarted port forwarding stop");
    record!(
        "POST",
        "/api/v0/portforwarding/stop/{localPort:int}",
        "restart-persistence-or-reset",
        stop_restart.status == "200 OK"
    );
    let stop_concurrent = futures_util::future::join_all([
        crate::route_http_request(
            "POST",
            "/api/v0/portforwarding/stop/29998",
            None,
            "",
            &stop_restart_state,
        ),
        crate::route_http_request(
            "POST",
            "/api/v0/portforwarding/stop/29998",
            None,
            "",
            &stop_restart_state,
        ),
    ])
    .await;
    record!(
        "POST",
        "/api/v0/portforwarding/stop/{localPort:int}",
        "concurrency-and-idempotency",
        stop_concurrent.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
        })
    );

    let runtime_read_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("port forwarding runtime read database");
    let (runtime_read_state, _runtime_read_receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(runtime_read_db.clone()),
    );
    runtime_read_db.close_for_test().await;
    let runtime_available = crate::route_http_request(
        "GET",
        "/api/v0/portforwarding/available-ports",
        None,
        "",
        &runtime_read_state,
    )
    .await
    .expect("runtime available ports");
    record!(
        "GET",
        "/api/v0/portforwarding/available-ports",
        "runtime-failure-and-timeout",
        runtime_available.status == "200 OK"
    );
    let runtime_status = crate::route_http_request(
        "GET",
        "/api/v0/portforwarding/status",
        None,
        "",
        &runtime_read_state,
    )
    .await
    .expect("runtime port forwarding status");
    record!(
        "GET",
        "/api/v0/portforwarding/status",
        "runtime-failure-and-timeout",
        runtime_status.status == "200 OK" && runtime_status.body == "[]"
    );
    let runtime_dynamic_status = crate::route_http_request(
        "GET",
        "/api/v0/portforwarding/status/29997",
        None,
        "",
        &runtime_read_state,
    )
    .await
    .expect("runtime dynamic port forwarding status");
    record!(
        "GET",
        "/api/v0/portforwarding/status/{localPort:int}",
        "runtime-failure-and-timeout",
        runtime_dynamic_status.status == "404 Not Found"
    );
    let runtime_stats = crate::route_http_request(
        "GET",
        "/api/v0/portforwarding/stream-stats",
        None,
        "",
        &runtime_read_state,
    )
    .await
    .expect("runtime port forwarding stream stats");
    record!(
        "GET",
        "/api/v0/portforwarding/stream-stats",
        "runtime-failure-and-timeout",
        runtime_stats.status == "200 OK"
    );
    let runtime_stop = crate::route_http_request(
        "POST",
        "/api/v0/portforwarding/stop/29996",
        None,
        "",
        &runtime_read_state,
    )
    .await
    .expect("runtime port forwarding stop");
    record!(
        "POST",
        "/api/v0/portforwarding/stop/{localPort:int}",
        "runtime-failure-and-timeout",
        runtime_stop.status == "200 OK"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("port_forwarding.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api port-forwarding mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential proof for the remaining versioned SoulseekDiscovery
/// controller cases. The frozen controller normalizes body and route
/// items before invoking the client, returns NoContent for every valid
/// interest mutation (including duplicate/absent removals), and exposes
/// protocol DTO shapes for recommendations, similar users, user
/// interests, and capability projections. Disabled rendezvous operations
/// intentionally retain the frozen 403 contract.
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
pub(super) async fn controller_api_differential_soulseek_discovery_residuals() {
    let target = "slskdn";
    let base_env = MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target);
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

    // The frozen service receives item values, not slskR's internal
    // generated IDs. Exercise both kinds so the versioned route is
    // compatible with the frozen web client and remains backwards-safe.
    for (route, read_route, hated) in [
        (
            "/api/v0/soulseek/interests",
            "/api/v0/soulseek/interests",
            false,
        ),
        (
            "/api/v0/soulseek/hated-interests",
            "/api/v0/soulseek/hated-interests",
            true,
        ),
    ] {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        state.session.write().await.state = "connected";
        let nominal = crate::route_http_request(
            "POST",
            route,
            None,
            if hated {
                r#"{"item":"nominal-hated"}"#
            } else {
                r#"{"item":"nominal-liked"}"#
            },
            &state,
        )
        .await
        .unwrap();
        record!(
            "POST",
            route,
            "nominal-status-headers-body",
            nominal.status == "204 No Content"
                && nominal.content_type == "application/json"
                && nominal.body.is_empty()
        );
        let malformed = crate::route_http_request("POST", route, None, "{}", &state)
            .await
            .unwrap();
        record!(
            "POST",
            route,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request" && malformed.body.contains("item is required")
        );
        let missing = crate::route_http_request("POST", route, None, "", &state)
            .await
            .unwrap();
        record!(
            "POST",
            route,
            "missing-empty-or-conflict-state",
            missing.status == "400 Bad Request" && missing.body.contains("item is required")
        );

        let (first_state, _receiver) = test_state_with_env(base_env.clone());
        first_state.session.write().await.state = "connected";
        let created = crate::route_http_request(
            "POST",
            route,
            None,
            if hated {
                r#"{"item":"reset-hated"}"#
            } else {
                r#"{"item":"reset-liked"}"#
            },
            &first_state,
        )
        .await
        .unwrap();
        let (restarted_state, _receiver) = test_state_with_env(base_env.clone());
        let restarted_read =
            crate::route_http_request("GET", read_route, None, "", &restarted_state)
                .await
                .unwrap();
        let restarted_json =
            serde_json::from_str::<serde_json::Value>(&restarted_read.body).unwrap_or_default();
        record!(
            "POST",
            route,
            "restart-persistence-or-reset",
            created.status == "204 No Content"
                && restarted_read.status == "200 OK"
                && restarted_json["count"] == 0
        );

        let (concurrent_state, _receiver) = test_state_with_env(base_env.clone());
        concurrent_state.session.write().await.state = "connected";
        let body = if hated {
            r#"{"item":"concurrent-hated"}"#
        } else {
            r#"{"item":"concurrent-liked"}"#
        };
        let responses = futures_util::future::join_all((0..2).map(|_| {
            let state = Arc::clone(&concurrent_state);
            async move { crate::route_http_request("POST", route, None, body, &state).await }
        }))
        .await;
        let concurrent_read =
            crate::route_http_request("GET", read_route, None, "", &concurrent_state)
                .await
                .unwrap();
        let concurrent_json =
            serde_json::from_str::<serde_json::Value>(&concurrent_read.body).unwrap_or_default();
        record!(
            "POST",
            route,
            "concurrency-and-idempotency",
            responses.iter().all(|response| {
                response
                    .as_ref()
                    .is_ok_and(|response| response.status == "204 No Content")
            }) && concurrent_read.status == "200 OK"
                && concurrent_json["count"] == 1
        );

        if hated {
            let (mutation_state, _receiver) = test_state_with_env(base_env.clone());
            mutation_state.session.write().await.state = "connected";
            let created = crate::route_http_request(
                "POST",
                route,
                None,
                r#"{"item":"readback-hated"}"#,
                &mutation_state,
            )
            .await
            .unwrap();
            let readback = crate::route_http_request("GET", read_route, None, "", &mutation_state)
                .await
                .unwrap();
            let readback_json =
                serde_json::from_str::<serde_json::Value>(&readback.body).unwrap_or_default();
            record!(
                "POST",
                route,
                "mutation-side-effects-and-readback",
                created.status == "204 No Content"
                    && readback_json["count"] == 1
                    && readback.body.contains("readback-hated")
            );
        }
    }

    for (route, read_route, hated) in [
        (
            "/api/v0/soulseek/interests/{item}",
            "/api/v0/soulseek/interests",
            false,
        ),
        (
            "/api/v0/soulseek/hated-interests/{item}",
            "/api/v0/soulseek/hated-interests",
            true,
        ),
    ] {
        let route_path = if hated {
            "/api/v0/soulseek/hated-interests"
        } else {
            "/api/v0/soulseek/interests"
        };
        let (state, _receiver) = test_state_with_env(base_env.clone());
        state.session.write().await.state = "connected";
        let nominal_item = if hated {
            "nominal-delete-hated"
        } else {
            "nominal-delete-liked"
        };
        let nominal_added = crate::route_http_request(
            "POST",
            route_path,
            None,
            &format!(r#"{{"item":"{nominal_item}"}}"#),
            &state,
        )
        .await
        .unwrap();
        let nominal = crate::route_http_request(
            "DELETE",
            &format!("{route_path}/{nominal_item}"),
            None,
            "",
            &state,
        )
        .await
        .unwrap();
        record!(
            "DELETE",
            route,
            "nominal-status-headers-body",
            nominal_added.status == "204 No Content"
                && nominal.status == "204 No Content"
                && nominal.content_type == "application/json"
                && nominal.body.is_empty()
        );
        let malformed =
            crate::route_http_request("DELETE", &format!("{route_path}/%20"), None, "", &state)
                .await
                .unwrap();
        record!(
            "DELETE",
            route,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request" && malformed.body.contains("item is required")
        );

        let missing =
            crate::route_http_request("DELETE", &format!("{route_path}/missing"), None, "", &state)
                .await
                .unwrap();
        record!(
            "DELETE",
            route,
            "missing-empty-or-conflict-state",
            missing.status == "204 No Content"
        );

        let (mutation_state, _receiver) = test_state_with_env(base_env.clone());
        mutation_state.session.write().await.state = "connected";
        let add_body = if hated {
            r#"{"item":"delete-hated"}"#
        } else {
            r#"{"item":"delete-liked"}"#
        };
        let added = crate::route_http_request("POST", route_path, None, add_body, &mutation_state)
            .await
            .unwrap();
        let deleted = crate::route_http_request(
            "DELETE",
            &format!(
                "{route_path}/{}",
                if hated {
                    "delete-hated"
                } else {
                    "delete-liked"
                }
            ),
            None,
            "",
            &mutation_state,
        )
        .await
        .unwrap();
        let after_delete = crate::route_http_request("GET", read_route, None, "", &mutation_state)
            .await
            .unwrap();
        let after_delete_json =
            serde_json::from_str::<serde_json::Value>(&after_delete.body).unwrap_or_default();
        record!(
            "DELETE",
            route,
            "mutation-side-effects-and-readback",
            added.status == "204 No Content"
                && deleted.status == "204 No Content"
                && after_delete_json["count"] == 0
        );

        let (before_restart, _receiver) = test_state_with_env(base_env.clone());
        before_restart.session.write().await.state = "connected";
        let _ = crate::route_http_request(
            "POST",
            route_path,
            None,
            r#"{"item":"restart-delete"}"#,
            &before_restart,
        )
        .await
        .unwrap();
        let (restarted_state, _receiver) = test_state_with_env(base_env.clone());
        restarted_state.session.write().await.state = "connected";
        let restarted_delete = crate::route_http_request(
            "DELETE",
            &format!("{route_path}/restart-delete"),
            None,
            "",
            &restarted_state,
        )
        .await
        .unwrap();
        record!(
            "DELETE",
            route,
            "restart-persistence-or-reset",
            restarted_delete.status == "204 No Content"
        );

        let (concurrent_state, _receiver) = test_state_with_env(base_env.clone());
        concurrent_state.session.write().await.state = "connected";
        let _ = crate::route_http_request(
            "POST",
            route_path,
            None,
            r#"{"item":"concurrent-delete"}"#,
            &concurrent_state,
        )
        .await
        .unwrap();
        let delete_responses = futures_util::future::join_all((0..2).map(|_| {
            let state = Arc::clone(&concurrent_state);
            let path = format!("{route_path}/concurrent-delete");
            async move { crate::route_http_request("DELETE", &path, None, "", &state).await }
        }))
        .await;
        record!(
            "DELETE",
            route,
            "concurrency-and-idempotency",
            delete_responses.iter().all(|response| {
                response
                    .as_ref()
                    .is_ok_and(|response| response.status == "204 No Content")
            })
        );
    }

    // Interest rendezvous mutation is disabled by default in both the
    // frozen MeshOptions and slskR's versioned guard. Exercise each
    // residual request shape against that real 403 contract.
    for (method, route) in [
        ("POST", "/api/v0/soulseek/mesh-rendezvous/interest"),
        ("DELETE", "/api/v0/soulseek/mesh-rendezvous/interest"),
    ] {
        for case in [
            "nominal-status-headers-body",
            "malformed-path-query-or-body",
            "runtime-failure-and-timeout",
            "mutation-side-effects-and-readback",
            "restart-persistence-or-reset",
            "concurrency-and-idempotency",
        ] {
            let path = if case == "malformed-path-query-or-body" {
                format!("{route}?unexpected=not-a-number")
            } else {
                route.to_owned()
            };
            let pass = if case == "restart-persistence-or-reset" {
                let (fresh, _receiver) = test_state_with_env(base_env.clone());
                crate::route_http_request(method, &path, None, "not-json", &fresh)
                    .await
                    .is_ok_and(|response| response.status == "403 Forbidden")
            } else if case == "concurrency-and-idempotency" {
                let (fresh, _receiver) = test_state_with_env(base_env.clone());
                let responses = futures_util::future::join_all((0..2).map(|_| {
                    let state = Arc::clone(&fresh);
                    let path = path.clone();
                    async move {
                        crate::route_http_request(method, &path, None, "not-json", &state).await
                    }
                }))
                .await;
                responses.iter().all(|response| {
                    response
                        .as_ref()
                        .is_ok_and(|response| response.status == "403 Forbidden")
                })
            } else {
                let (fresh, _receiver) = test_state_with_env(base_env.clone());
                crate::route_http_request(method, &path, None, "not-json", &fresh)
                    .await
                    .is_ok_and(|response| response.status == "403 Forbidden")
            };
            record!(method, route, case, pass);
        }
    }

    for (route, ledger_route, similar_users) in [
        (
            "/api/v0/soulseek/items/ambient/recommendations",
            "/api/v0/soulseek/items/{item}/recommendations",
            false,
        ),
        (
            "/api/v0/soulseek/items/ambient/similar-users",
            "/api/v0/soulseek/items/{item}/similar-users",
            true,
        ),
    ] {
        let malformed = crate::route_http_request(
            "GET",
            &format!("{route}?unexpected=not-a-number"),
            None,
            "",
            &test_state_with_env(base_env.clone()).0,
        )
        .await
        .unwrap();
        let malformed_json =
            serde_json::from_str::<serde_json::Value>(&malformed.body).unwrap_or_default();
        record!(
            "GET",
            ledger_route,
            "malformed-path-query-or-body",
            malformed.status == "200 OK"
                && malformed_json["item"] == "ambient"
                && malformed_json
                    .get(if similar_users {
                        "usernames"
                    } else {
                        "recommendations"
                    })
                    .is_some()
        );

        let missing_state = test_state_with_env(base_env.clone()).0;
        let missing = crate::route_http_request(
            "GET",
            "/api/v0/soulseek/items/%20/recommendations"
                .replace(
                    "/recommendations",
                    if similar_users {
                        "/similar-users"
                    } else {
                        "/recommendations"
                    },
                )
                .as_str(),
            None,
            "",
            &missing_state,
        )
        .await
        .unwrap();
        record!(
            "GET",
            ledger_route,
            "missing-empty-or-conflict-state",
            missing.status == "400 Bad Request" && missing.body.contains("item is required")
        );

        let runtime_state = test_state_with_env(base_env.clone()).0;
        let runtime = crate::route_http_request("GET", route, None, "", &runtime_state)
            .await
            .unwrap();
        let runtime_json =
            serde_json::from_str::<serde_json::Value>(&runtime.body).unwrap_or_default();
        record!(
            "GET",
            ledger_route,
            "runtime-failure-and-timeout",
            runtime.status == "200 OK" && runtime_json["item"] == "ambient"
        );

        let populated_state = test_state_with_env(base_env.clone()).0;
        if similar_users {
            populated_state
                .users
                .write()
                .await
                .watch("discovery-similar-peer".to_owned());
        } else {
            populated_state
                .interests
                .write()
                .await
                .add_liked("ambient".to_owned());
        }
        let populated = crate::route_http_request("GET", route, None, "", &populated_state)
            .await
            .unwrap();
        let populated_json =
            serde_json::from_str::<serde_json::Value>(&populated.body).unwrap_or_default();
        let populated_values = populated_json
            .get(if similar_users {
                "usernames"
            } else {
                "recommendations"
            })
            .and_then(serde_json::Value::as_array);
        record!(
            "GET",
            ledger_route,
            "populated-dynamic-state",
            populated.status == "200 OK"
                && populated_values.is_some_and(|values| !values.is_empty())
        );
    }

    for route in [
        "/api/v0/soulseek/recommendations",
        "/api/v0/soulseek/recommendations/global",
    ] {
        let malformed_state = test_state_with_env(base_env.clone()).0;
        let malformed = crate::route_http_request(
            "GET",
            &format!("{route}?unexpected=not-a-number"),
            None,
            "",
            &malformed_state,
        )
        .await
        .unwrap();
        let malformed_json =
            serde_json::from_str::<serde_json::Value>(&malformed.body).unwrap_or_default();
        record!(
            "GET",
            route,
            "malformed-path-query-or-body",
            malformed.status == "200 OK"
                && malformed_json["recommendations"] == serde_json::json!([])
                && malformed_json["unrecommendations"] == serde_json::json!([])
        );

        let missing_state = test_state_with_env(base_env.clone()).0;
        let missing = crate::route_http_request("GET", route, None, "", &missing_state)
            .await
            .unwrap();
        let missing_json =
            serde_json::from_str::<serde_json::Value>(&missing.body).unwrap_or_default();
        record!(
            "GET",
            route,
            "missing-empty-or-conflict-state",
            missing.status == "200 OK"
                && missing_json["recommendations"] == serde_json::json!([])
                && missing_json["unrecommendations"] == serde_json::json!([])
        );

        let runtime_state = test_state_with_env(base_env.clone()).0;
        runtime_state.session.write().await.state = "disconnected";
        let runtime = crate::route_http_request("GET", route, None, "", &runtime_state)
            .await
            .unwrap();
        let runtime_json =
            serde_json::from_str::<serde_json::Value>(&runtime.body).unwrap_or_default();
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            runtime.status == "200 OK" && runtime_json["recommendations"] == serde_json::json!([])
        );

        let populated_state = test_state_with_env(base_env.clone()).0;
        populated_state
            .interests
            .write()
            .await
            .add_liked("ambient".to_owned());
        populated_state
            .interests
            .write()
            .await
            .add_hated("noise".to_owned());
        let populated = crate::route_http_request("GET", route, None, "", &populated_state)
            .await
            .unwrap();
        let populated_json =
            serde_json::from_str::<serde_json::Value>(&populated.body).unwrap_or_default();
        record!(
            "GET",
            route,
            "populated-dynamic-state",
            populated.status == "200 OK"
                && populated_json["recommendations"]
                    .as_array()
                    .is_some_and(|values| !values.is_empty())
                && populated_json["unrecommendations"]
                    .as_array()
                    .is_some_and(|values| !values.is_empty())
        );
    }

    for route in [
        "/api/v0/soulseek/mesh-rendezvous/status",
        "/api/v0/soulseek/peer-capabilities",
    ] {
        let malformed_state = test_state_with_env(base_env.clone()).0;
        let malformed = crate::route_http_request(
            "GET",
            &format!("{route}?unexpected=not-a-number"),
            None,
            "",
            &malformed_state,
        )
        .await
        .unwrap();
        let malformed_json =
            serde_json::from_str::<serde_json::Value>(&malformed.body).unwrap_or_default();
        let malformed_pass = if route.ends_with("status") {
            malformed.status == "200 OK"
                && malformed_json["interestTag"].is_string()
                && malformed_json["privacy"].is_string()
        } else {
            malformed.status == "200 OK" && malformed_json.as_array().is_some()
        };
        record!("GET", route, "malformed-path-query-or-body", malformed_pass);

        let missing_state = test_state_with_env(base_env.clone()).0;
        let missing = crate::route_http_request("GET", route, None, "", &missing_state)
            .await
            .unwrap();
        let missing_json =
            serde_json::from_str::<serde_json::Value>(&missing.body).unwrap_or_default();
        let missing_pass = if route.ends_with("status") {
            missing.status == "200 OK" && missing_json["enabled"].is_boolean()
        } else {
            missing.status == "200 OK"
                && missing_json
                    .as_array()
                    .is_some_and(|values| values.is_empty())
        };
        record!(
            "GET",
            route,
            "missing-empty-or-conflict-state",
            missing_pass
        );

        let runtime_state = test_state_with_env(base_env.clone()).0;
        let runtime = crate::route_http_request("GET", route, None, "", &runtime_state)
            .await
            .unwrap();
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            runtime.status == "200 OK"
        );

        let populated_state = test_state_with_env(base_env.clone()).0;
        if route.ends_with("status") {
            populated_state
                .users
                .write()
                .await
                .watch("mesh-populated-peer".to_owned());
            populated_state
                .mesh
                .write()
                .await
                .capability_records
                .push(test_capability_descriptor(
                    "mesh-populated-peer",
                    vec![slskr_client::capabilities::FEATURE_MESH_V1.to_owned()],
                ));
        } else {
            populated_state
                .mesh
                .write()
                .await
                .capability_records
                .push(test_capability_descriptor(
                    "capability-populated-peer",
                    vec![slskr_client::capabilities::FEATURE_MESH_V1.to_owned()],
                ));
        }
        let populated = crate::route_http_request("GET", route, None, "", &populated_state)
            .await
            .unwrap();
        let populated_json =
            serde_json::from_str::<serde_json::Value>(&populated.body).unwrap_or_default();
        let populated_pass = if route.ends_with("status") {
            populated.status == "200 OK" && populated_json["candidateCount"] == 1
        } else {
            populated.status == "200 OK"
                && populated_json.as_array().is_some_and(|values| {
                    values
                        .iter()
                        .any(|value| value["username"] == "capability-populated-peer")
                })
        };
        record!("GET", route, "populated-dynamic-state", populated_pass);
    }

    for route in [
        "/api/v0/soulseek/mesh-rendezvous/discover",
        "/api/v0/soulseek/mesh-rendezvous/users",
    ] {
        for case in [
            "nominal-status-headers-body",
            "malformed-path-query-or-body",
            "runtime-failure-and-timeout",
            "populated-dynamic-state",
        ] {
            let state = test_state_with_env(base_env.clone()).0;
            let path = if case == "malformed-path-query-or-body" {
                format!("{route}?unexpected=not-a-number")
            } else {
                route.to_owned()
            };
            let response = crate::route_http_request("GET", &path, None, "", &state)
                .await
                .unwrap();
            record!(
                "GET",
                route,
                case,
                response.status == "403 Forbidden" && response.body.contains("feature is disabled")
            );
        }
    }

    let similar_users_route = "/api/v0/soulseek/users/similar";
    let malformed_state = test_state_with_env(base_env.clone()).0;
    let malformed = crate::route_http_request(
        "GET",
        &format!("{similar_users_route}?unexpected=not-a-number"),
        None,
        "",
        &malformed_state,
    )
    .await
    .unwrap();
    let malformed_json =
        serde_json::from_str::<serde_json::Value>(&malformed.body).unwrap_or_default();
    record!(
        "GET",
        similar_users_route,
        "malformed-path-query-or-body",
        malformed.status == "200 OK" && malformed_json.as_array().is_some()
    );

    let missing_state = test_state_with_env(base_env.clone()).0;
    let missing = crate::route_http_request("GET", similar_users_route, None, "", &missing_state)
        .await
        .unwrap();
    let missing_json = serde_json::from_str::<serde_json::Value>(&missing.body).unwrap_or_default();
    record!(
        "GET",
        similar_users_route,
        "missing-empty-or-conflict-state",
        missing.status == "200 OK"
            && missing_json
                .as_array()
                .is_some_and(|values| values.is_empty())
    );

    let runtime_state = test_state_with_env(base_env.clone()).0;
    let runtime = crate::route_http_request("GET", similar_users_route, None, "", &runtime_state)
        .await
        .unwrap();
    let runtime_json = serde_json::from_str::<serde_json::Value>(&runtime.body).unwrap_or_default();
    record!(
        "GET",
        similar_users_route,
        "runtime-failure-and-timeout",
        runtime.status == "200 OK" && runtime_json.as_array().is_some()
    );

    let populated_state = test_state_with_env(base_env.clone()).0;
    populated_state
        .users
        .write()
        .await
        .watch("similar-populated-peer".to_owned());
    let populated =
        crate::route_http_request("GET", similar_users_route, None, "", &populated_state)
            .await
            .unwrap();
    let populated_json =
        serde_json::from_str::<serde_json::Value>(&populated.body).unwrap_or_default();
    record!(
        "GET",
        similar_users_route,
        "populated-dynamic-state",
        populated.status == "200 OK"
            && populated_json.as_array().is_some_and(|values| {
                values
                    .iter()
                    .any(|value| value["username"] == "similar-populated-peer")
            })
    );

    for case in [
        "malformed-path-query-or-body",
        "missing-empty-or-conflict-state",
        "runtime-failure-and-timeout",
    ] {
        let path = if case == "malformed-path-query-or-body" {
            "/api/v0/soulseek/users/%20/interests"
        } else {
            "/api/v0/soulseek/users/missing-peer/interests"
        };
        let state = test_state_with_env(base_env.clone()).0;
        if case != "malformed-path-query-or-body" {
            state.session.write().await.state = "disconnected";
        }
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap();
        record!(
            "GET",
            "/api/v0/soulseek/users/{username}/interests",
            case,
            if case == "malformed-path-query-or-body" {
                response.status == "400 Bad Request"
                    && response.body.contains("username is required")
            } else {
                response.status == "503 Service Unavailable"
            }
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create SoulseekDiscovery evidence directory");
    fs::write(
        evidence_dir.join("soulseek_discovery_residuals.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize SoulseekDiscovery ledger"),
    )
    .expect("write SoulseekDiscovery ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api SoulseekDiscovery mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
