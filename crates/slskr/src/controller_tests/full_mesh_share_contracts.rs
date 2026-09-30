//! Controller share admission, backfill, and stream ticket contracts.

use super::*;

#[cfg(feature = "full-controller-tests")]
#[tokio::test]
pub(super) async fn controller_api_differential_share_stream_ticket_admission_enforces_grant_limits(
) {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    {
        let mut grants = state.share_grants.write().await;
        grants.records.extend([
            crate::share_grant_store::ShareGrantRecord {
                id: "grant-limited".to_owned(),
                collection_id: "collection-limited".to_owned(),
                username: "recipient".to_owned(),
                shared_at: crate::unix_timestamp(),
                permissions: "stream".to_owned(),
                max_concurrent_streams: Some(1),
            },
            crate::share_grant_store::ShareGrantRecord {
                id: "grant-independent".to_owned(),
                collection_id: "collection-independent".to_owned(),
                username: "recipient".to_owned(),
                shared_at: crate::unix_timestamp(),
                permissions: "stream".to_owned(),
                max_concurrent_streams: Some(1),
            },
        ]);
    }
    let mut tickets = state.stream_tickets.write().await;
    let limited_first = tickets
        .issue(
            "share",
            "share:grant-limited",
            "content-one".to_owned(),
            "one.flac".to_owned(),
            None,
            0,
            "audio/flac".to_owned(),
            60,
        )
        .expect("first limited stream ticket")
        .0;
    let limited_second = tickets
        .issue(
            "share",
            "share:grant-limited",
            "content-two".to_owned(),
            "two.flac".to_owned(),
            None,
            0,
            "audio/flac".to_owned(),
            60,
        )
        .expect("second limited stream ticket")
        .0;
    let independent = tickets
        .issue(
            "share",
            "share:grant-independent",
            "content-other".to_owned(),
            "other.flac".to_owned(),
            None,
            0,
            "audio/flac".to_owned(),
            60,
        )
        .expect("independent grant stream ticket")
        .0;
    drop(tickets);

    let first = crate::share_stream_limits::acquire_ticket_stream(
        &state,
        "content-one",
        Some(&format!("ticket={limited_first}")),
    )
    .await
    .expect("first stream admission")
    .expect("ticketed share stream lease");
    assert!(matches!(
        crate::share_stream_limits::acquire_ticket_stream(
            &state,
            "content-two",
            Some(&format!("ticket={limited_second}")),
        )
        .await,
        Err(crate::share_stream_limits::AdmissionError::Busy)
    ));
    let independent = crate::share_stream_limits::acquire_ticket_stream(
        &state,
        "content-other",
        Some(&format!("ticket={independent}")),
    )
    .await
    .expect("other-grant admission")
    .expect("other grants have independent capacity");
    drop(first);
    assert!(crate::share_stream_limits::acquire_ticket_stream(
        &state,
        "content-two",
        Some(&format!("ticket={limited_second}")),
    )
    .await
    .expect("released grant capacity")
    .is_some());
    drop(independent);
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
pub(super) async fn controller_api_differential_share_backfill_uses_pinned_mesh_and_publishes_verified_file(
) {
    use sha2::Digest as _;
    use slskr_client::overlay::FEATURE_MESH_SERVICE;

    let root = std::env::temp_dir().join(format!(
        "slskr-share-backfill-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    let share_root = root.join("share");
    let downloads_root = root.join("downloads");
    let identity_root = root.join("identity");
    std::fs::create_dir_all(&share_root).expect("share root");
    std::fs::create_dir_all(&downloads_root).expect("downloads root");
    let content = b"verified share backfill through the pinned mesh transport\n";
    let source_path = share_root.join("treasure.txt");
    std::fs::write(&source_path, content).expect("share file");
    let advanced = serde_json::json!({
        "mesh": {
            "enabled": true,
            "enableOverlay": true,
            "enableDht": false,
        },
        "feature": {
            "mesh": true,
            "pods": true,
            "virtualSoulfind": true,
        }
    });
    let single_bind = "0.0.0.0:50341";
    let (mut state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKR_PARITY_PROFILE", "current")
            .with("SLSKR_ADVANCED_NETWORKING_JSON", &advanced.to_string())
            .with("SLSKR_LISTENER_BIND", single_bind)
            .with("SLSKR_OVERLAY_BIND", single_bind)
            .with("SLSKD_SHARED_DIR", &share_root.display().to_string())
            .with("SLSKD_DOWNLOADS_DIR", &downloads_root.display().to_string()),
    );
    let local_username = crate::pod_request_peer_id(&state)
        .await
        .expect("local mesh username");
    let recipient_username = "backfill-recipient".to_owned();
    let gateway = Arc::new(
        crate::private_gateway::Gateway::load_or_create_with_quic(
            "127.0.0.1:0".parse().unwrap(),
            &identity_root,
            None,
        )
        .await
        .expect("backfill mesh gateway"),
    );
    let endpoint = gateway.bind();
    let certificate_pin = gateway.certificate_sha256();
    {
        let state = Arc::get_mut(&mut state).expect("unshared test state");
        state.config.test_user_endpoint_overrides.insert(
            local_username.clone(),
            SocketAddr::new(
                std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
                endpoint.port(),
            ),
        );
        state
            .config
            .trusted_mesh_peers
            .push(crate::TrustedMeshPeer {
                peer_id: local_username.clone(),
                username: local_username.clone(),
                overlay_endpoint: endpoint,
                certificate_sha256: certificate_pin,
                range_endpoint: None,
            });
        state.private_gateway = Some(gateway.clone());
    }
    let descriptor = slskr_client::capabilities::PeerCapabilityDescriptor::unsigned(
        &local_username,
        vec![FEATURE_MESH_SERVICE.to_owned()],
        vec![format!("tcp:{}:{}", endpoint.ip(), endpoint.port())],
        Duration::from_secs(300),
        &state.capability_signing_key,
        SystemTime::now(),
    )
    .expect("local capability descriptor")
    .with_overlay_port(Some(endpoint.port()))
    .sign(&state.capability_signing_key)
    .expect("sign local capability descriptor");
    state
        .mesh
        .write()
        .await
        .update_capability(descriptor)
        .expect("register local capability descriptor");
    let recipient_key = ed25519_dalek::SigningKey::from_bytes(&[43; 32]);
    let recipient_descriptor = slskr_client::capabilities::PeerCapabilityDescriptor::unsigned(
        &recipient_username,
        vec![FEATURE_MESH_SERVICE.to_owned()],
        Vec::new(),
        Duration::from_secs(300),
        &recipient_key,
        SystemTime::now(),
    )
    .and_then(|descriptor| descriptor.sign(&recipient_key))
    .expect("recipient capability descriptor");
    state
        .mesh
        .write()
        .await
        .update_capability(recipient_descriptor)
        .expect("register recipient capability descriptor");
    crate::remember_peer_endpoint(
        &state,
        crate::PeerAddress {
            username: recipient_username.clone(),
            ip: std::net::Ipv4Addr::LOCALHOST,
            port: 1,
            obfuscation_type: 0,
            obfuscated_port: 0,
        },
    )
    .await;
    Arc::get_mut(&mut state)
        .expect("unique test state before starting gateway")
        .capability_signing_key = recipient_key;
    add_test_share(
        &state,
        "Virtual/treasure.txt",
        &source_path,
        content.len() as u64,
    )
    .await;
    let content_sha256 = hex::encode(sha2::Sha256::digest(content));
    let grant_id = "grant-recipient-backfill".to_owned();
    {
        let mut collections = state.collections.write().await;
        collections
            .create_with_contract(
                "collection-recipient-backfill".to_owned(),
                local_username.clone(),
                "Backfill fixture".to_owned(),
                String::new(),
                "ShareList".to_owned(),
            )
            .expect("create backfill collection");
        collections
            .add_item_with_contract(
                "collection-recipient-backfill",
                Some("item-recipient-backfill".to_owned()),
                "Virtual/treasure.txt".to_owned(),
                String::new(),
                "treasure.txt".to_owned(),
                "text".to_owned(),
                "Virtual/treasure.txt".to_owned(),
                String::new(),
                content_sha256.clone(),
            )
            .expect("add backfill item")
            .expect("backfill item exists");
    }
    let share_token = {
        let mut grants = state.share_grants.write().await;
        grants
            .create_with_contract_and_permissions(
                Some(grant_id.clone()),
                "collection-recipient-backfill".to_owned(),
                recipient_username.clone(),
                "download",
            )
            .expect("create recipient grant");
        drop(grants);
        state
            .share_access_tokens
            .write()
            .await
            .issue(grant_id.clone(), 600)
            .expect("issue recipient grant token")
            .0
    };
    state
        .incoming_shares
        .write()
        .await
        .upsert(crate::IncomingShareRecord {
            id: grant_id.clone(),
            owner_endpoint: "https://ignored.invalid".to_owned(),
            owner_user_id: local_username.clone(),
            recipient_user_id: recipient_username.clone(),
            collection_id: "collection-recipient-backfill".to_owned(),
            collection_title: "Backfill fixture".to_owned(),
            collection_description: String::new(),
            collection_type: "ShareList".to_owned(),
            permissions: "download".to_owned(),
            token: share_token,
            expiry_utc: String::new(),
            max_bitrate_kbps: None,
            max_concurrent_streams: 0,
            items: Vec::new(),
            received_at: crate::unix_timestamp(),
        });

    let gateway_server = tokio::spawn(gateway.run(Arc::clone(&state)));
    let result = crate::share_backfill_controller::backfill_incoming_share(
        &state,
        &grant_id,
        &recipient_username,
    )
    .await;
    gateway_server.abort();
    let _ = gateway_server.await;
    let receipts = result.expect("recipient backfill succeeds");
    assert_eq!(receipts.len(), 1);
    let downloaded =
        std::fs::read(downloads_root.join(&receipts[0].filename)).expect("read downloaded file");
    let _ = std::fs::remove_dir_all(&root);
    assert_eq!(receipts[0].size, content.len() as u64);
    assert_eq!(receipts[0].sha256, content_sha256);
    assert_eq!(downloaded, content);
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
pub(super) async fn controller_api_differential_mesh_stream_ticket_validation_and_limits() {
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

    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_SHARE_FIXTURE", ""),
    );
    let nominal = crate::route_http_request(
        "POST",
        "/api/v0/mesh-streams/tickets",
        None,
        &format!(
            r#"{{"contentId":"mesh-ticket-contract","filename":"Track.aac","peerId":"mesh-peer","expectedSize":321,"expectedHash":"{}"}}"#,
            "A".repeat(64)
        ),
        &state,
    )
    .await
    .expect("mesh ticket nominal response");
    let nominal_json = serde_json::from_str::<serde_json::Value>(&nominal.body)
        .unwrap_or_else(|_| serde_json::json!({}));
    let nominal_keys = nominal_json
        .as_object()
        .map(|object| object.keys().cloned().collect::<BTreeSet<_>>())
        .unwrap_or_default();
    let nominal_ticket = nominal_json["ticket"].as_str().unwrap_or_default();
    record!(
        "POST",
        "/api/v0/mesh-streams/tickets",
        "nominal-status-headers-body",
        nominal.status == "200 OK"
            && nominal.content_type == "application/json"
            && nominal_keys
                == BTreeSet::from([
                    "contentType".to_owned(),
                    "expiresInSeconds".to_owned(),
                    "source".to_owned(),
                    "streamUrl".to_owned(),
                    "ticket".to_owned(),
                ])
            && nominal_json["contentType"] == "audio/aac"
            && nominal_json["expiresInSeconds"] == 120
            && nominal_json["source"] == "mesh"
            && nominal_json["streamUrl"] == format!("/api/v0/mesh-streams/{nominal_ticket}")
            && !nominal_ticket.is_empty()
    );
    let stored = state
        .stream_tickets
        .write()
        .await
        .get(nominal_ticket)
        .is_some();
    record!(
        "POST",
        "/api/v0/mesh-streams/tickets",
        "mutation-side-effects-and-readback",
        stored
    );

    let malformed_bodies = [
        (
            r#"{"contentId":"mesh-ticket-contract","filename":"../escape.flac","peerId":"mesh-peer"}"#,
            "Filename is required.",
        ),
        (
            r#"{"contentId":"mesh-ticket-contract","filename":"Track.flac","peerId":"mesh-peer","expectedSize":-1}"#,
            "Expected size must be greater than or equal to zero.",
        ),
        (
            r#"{"contentId":"mesh-ticket-contract","filename":"Track.flac","peerId":"mesh-peer","expectedHash":"abc"}"#,
            "Expected hash must be a SHA-256 hex digest.",
        ),
        (
            r#"{"contentId":"mesh-ticket-contract","filename":"archive.zip","peerId":"mesh-peer"}"#,
            "Only audio files can be preview streamed from mesh peers.",
        ),
        (
            r#"{"contentId":"mesh-ticket-contract","filename":"Track.oga","peerId":"mesh-peer"}"#,
            "Only audio files can be preview streamed from mesh peers.",
        ),
    ];
    let mut malformed = true;
    for (body, expected_error) in malformed_bodies {
        let response =
            crate::route_http_request("POST", "/api/v0/mesh-streams/tickets", None, body, &state)
                .await
                .expect("mesh ticket malformed response");
        malformed &= response.status == "400 Bad Request"
            && response.body == format!(r#"{{"error":"{expected_error}"}}"#);
    }
    record!(
        "POST",
        "/api/v0/mesh-streams/tickets",
        "malformed-path-query-or-body",
        malformed
    );

    let blank_peer = crate::route_http_request(
        "POST",
        "/api/v0/mesh-streams/tickets",
        None,
        r#"{"contentId":"mesh-blank-peer","filename":"Track.flac","peerId":"   "}"#,
        &state,
    )
    .await
    .expect("mesh ticket blank peer response");
    let blank_peer_json = serde_json::from_str::<serde_json::Value>(&blank_peer.body)
        .unwrap_or_else(|_| serde_json::json!({}));
    record!(
        "POST",
        "/api/v0/mesh-streams/tickets",
        "nominal-status-headers-body",
        blank_peer.status == "200 OK"
            && blank_peer_json["source"] == "mesh"
            && blank_peer_json["contentType"] == "audio/flac"
    );

    let missing =
        crate::route_http_request("GET", "/api/v0/mesh-streams/not-a-ticket", None, "", &state)
            .await
            .expect("mesh ticket missing response");
    record!(
        "GET",
        "/api/v0/mesh-streams/{ticket}",
        "missing-empty-or-conflict-state",
        missing.status == "404 Not Found"
    );

    let malformed_path = crate::route_http_request(
        "GET",
        "/api/v0/mesh-streams/not-a-ticket/extra",
        None,
        "",
        &state,
    )
    .await
    .expect("mesh ticket malformed path response");
    record!(
        "GET",
        "/api/v0/mesh-streams/{ticket}",
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
        "/api/v0/mesh-streams/tickets",
        None,
        r#"{"contentId":"mesh-disabled","filename":"Track.flac"}"#,
        &disabled_state,
    )
    .await
    .expect("disabled mesh ticket create response");
    let disabled_get = crate::route_http_request(
        "GET",
        "/api/v0/mesh-streams/not-a-ticket",
        None,
        "",
        &disabled_state,
    )
    .await
    .expect("disabled mesh ticket get response");
    record!(
        "POST",
        "/api/v0/mesh-streams/tickets",
        "missing-empty-or-conflict-state",
        disabled_post.status == "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/mesh-streams/{ticket}",
        "missing-empty-or-conflict-state",
        disabled_get.status == "404 Not Found"
    );

    let (capacity_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    {
        let mut tickets = capacity_state.stream_tickets.write().await;
        for index in 0..crate::MAX_PREVIEW_STREAM_TICKETS {
            assert!(tickets
                .issue(
                    "mesh",
                    "mesh-unresolved",
                    format!("capacity-{index}"),
                    "Track.flac".to_owned(),
                    None,
                    0,
                    "audio/flac".to_owned(),
                    120,
                )
                .is_some());
        }
    }
    let capacity = crate::route_http_request(
        "POST",
        "/api/v0/mesh-streams/tickets",
        None,
        r#"{"contentId":"mesh-capacity","filename":"Track.flac","peerId":"mesh-peer"}"#,
        &capacity_state,
    )
    .await
    .expect("mesh ticket capacity response");
    record!(
        "POST",
        "/api/v0/mesh-streams/tickets",
        "runtime-failure-and-timeout",
        capacity.status == "429 Too Many Requests"
            && capacity.content_type == "text/plain; charset=utf-8"
            && capacity.body == "Mesh stream limit reached."
    );

    let nominal_get = crate::route_http_request(
        "GET",
        &format!("/api/v0/mesh-streams/{nominal_ticket}"),
        None,
        "",
        &state,
    )
    .await
    .expect("nominal mesh stream read");
    let nominal_get_json = serde_json::from_str::<serde_json::Value>(&nominal_get.body)
        .unwrap_or_else(|_| serde_json::json!({}));
    record!(
        "GET",
        "/api/v0/mesh-streams/{ticket}",
        "nominal-status-headers-body",
        nominal_get.status == "200 OK"
            && nominal_get.content_type == "application/json"
            && nominal_get_json["status"] == "available"
            && nominal_get_json["cacheControl"] == "no-store"
    );

    let failure_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("mesh stream runtime-failure database");
    let (failure_state, _failure_receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target),
        crate::SearchStore::new(),
        Some(failure_db.clone()),
    );
    let failure_ticket = crate::route_http_request(
        "POST",
        "/api/v0/mesh-streams/tickets",
        None,
        r#"{"contentId":"mesh-runtime","filename":"Runtime.flac","peerId":"mesh-peer"}"#,
        &failure_state,
    )
    .await
    .expect("create mesh runtime-failure ticket");
    let failure_ticket_json = serde_json::from_str::<serde_json::Value>(&failure_ticket.body)
        .unwrap_or_else(|_| serde_json::json!({}));
    failure_db.close_for_test().await;
    let failure_get = crate::route_http_request(
        "GET",
        &format!(
            "/api/v0/mesh-streams/{}",
            failure_ticket_json["ticket"].as_str().unwrap_or_default()
        ),
        None,
        "",
        &failure_state,
    )
    .await
    .expect("mesh stream read with closed unrelated database");
    record!(
        "GET",
        "/api/v0/mesh-streams/{ticket}",
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
        &format!("/api/v0/mesh-streams/{nominal_ticket}"),
        None,
        "",
        &restarted_state,
    )
    .await
    .expect("mesh stream ticket after restart");
    let reset_create = crate::route_http_request(
        "POST",
        "/api/v0/mesh-streams/tickets",
        None,
        r#"{"contentId":"mesh-restarted","filename":"Restart.flac","peerId":"mesh-peer"}"#,
        &restarted_state,
    )
    .await
    .expect("mesh stream ticket create after restart");
    record!(
        "POST",
        "/api/v0/mesh-streams/tickets",
        "restart-persistence-or-reset",
        reset_get.status == "404 Not Found" && reset_create.status == "200 OK"
    );

    let (concurrent_state, _concurrent_receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let concurrent = tokio::join!(
        crate::route_http_request(
            "POST",
            "/api/v0/mesh-streams/tickets",
            None,
            r#"{"contentId":"mesh-concurrent-a","filename":"A.flac","peerId":"mesh-peer"}"#,
            &concurrent_state,
        ),
        crate::route_http_request(
            "POST",
            "/api/v0/mesh-streams/tickets",
            None,
            r#"{"contentId":"mesh-concurrent-b","filename":"B.flac","peerId":"mesh-peer"}"#,
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
        "/api/v0/mesh-streams/tickets",
        "concurrency-and-idempotency",
        concurrent_pass
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("mesh_stream_ticket_validation_and_limits.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        mismatches.is_empty(),
        "{} mesh stream ticket mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
