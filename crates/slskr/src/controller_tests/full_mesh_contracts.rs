//! Controller full mesh contracts ownership.

use super::*;

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn mesh_and_signal_routes_honor_configured_runtime_switches() {
    let advanced = serde_json::json!({
        "mesh": {
            "enabled": true,
            "enableOverlay": false,
            "enableDht": false,
            "enableStun": false
        },
        "SignalSystem": {
            "enabled": false,
            "deduplicationCacheSize": 2048,
            "defaultTtl": "00:07:30",
            "meshChannel": {
                "enabled": false,
                "priority": 3,
                "requireActiveSession": true
            },
            "btExtensionChannel": {
                "enabled": true,
                "priority": 4,
                "requireActiveSession": false
            }
        }
    });
    let (state, _receiver) = test_state_with_env(
        MapEnv::default().with("SLSKR_ADVANCED_NETWORKING_JSON", &advanced.to_string()),
    );

    let config = crate::route_http_request("GET", "/api/signals/config", None, "", &state)
        .await
        .expect("signal config response");
    let config_json = serde_json::from_str::<serde_json::Value>(&config.body).unwrap();
    assert!(!config_json["enabled"].as_bool().unwrap());
    assert_eq!(config_json["deduplicationCacheSize"], 2_048);
    assert_eq!(config_json["defaultTtlSeconds"], 450);
    assert!(!config_json["meshChannel"]["enabled"].as_bool().unwrap());
    assert_eq!(config_json["btExtensionChannel"]["priority"], 4);

    let status = crate::route_http_request("GET", "/api/signals/status", None, "", &state)
        .await
        .expect("signal status response");
    let status_json = serde_json::from_str::<serde_json::Value>(&status.body).unwrap();
    assert_eq!(
        status_json["activeChannels"],
        serde_json::json!(["btExtension"])
    );

    for (method, path) in [
        ("GET", "/api/mesh/stats"),
        ("GET", "/api/dht/peers"),
        ("POST", "/api/mesh/nat/detect"),
    ] {
        let response = crate::route_http_request(method, path, None, "", &state)
            .await
            .expect("disabled mesh route response");
        assert_eq!(response.status, "404 Not Found", "{method} {path}");
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn swarm_analytics_routes_share_a_bounded_snapshot() {
    let (state, _receiver) = test_state();
    let now = crate::unix_timestamp();
    state
        .multisource
        .write()
        .await
        .insert(crate::multisource::SwarmJob {
            id: "swarm-analytics-fixture".to_owned(),
            status: "completed".to_owned(),
            filename: "album.flac".to_owned(),
            output_path: "album.flac".to_owned(),
            file_size: 1_024,
            chunk_size: 512,
            sources: vec!["alice".to_owned(), "bob".to_owned()],
            completed_chunks: 2,
            total_chunks: 2,
            bytes_downloaded: 1_024,
            created_at: now,
            updated_at: now,
            result: Some(crate::multisource::SwarmResult {
                id: "swarm-analytics-fixture".to_owned(),
                success: true,
                filename: "album.flac".to_owned(),
                output_path: "album.flac".to_owned(),
                bytes_downloaded: 1_024,
                total_time_ms: 100,
                sources_used: 2,
                final_hash: "00".repeat(32),
                chunks: vec![
                    crate::multisource::ChunkResult {
                        index: 0,
                        username: "alice".to_owned(),
                        start_offset: 0,
                        end_offset: 511,
                        bytes_downloaded: 512,
                        time_ms: 40,
                    },
                    crate::multisource::ChunkResult {
                        index: 1,
                        username: "bob".to_owned(),
                        start_offset: 512,
                        end_offset: 1_023,
                        bytes_downloaded: 512,
                        time_ms: 60,
                    },
                ],
                error: None,
            }),
        });

    let dashboard = crate::route_http_request(
        "GET",
        "/api/v0/swarm/analytics/dashboard?timeWindowHours=24&rankingLimit=1",
        None,
        "",
        &state,
    )
    .await
    .expect("swarm analytics dashboard");
    assert_eq!(dashboard.status, "200 OK");
    let dashboard = serde_json::from_str::<serde_json::Value>(&dashboard.body).unwrap();
    assert_eq!(dashboard["performanceMetrics"]["totalDownloads"], 1);
    assert_eq!(dashboard["performanceMetrics"]["successRate"], 1.0);
    assert_eq!(dashboard["performanceMetrics"]["timeWindow"], "1.00:00:00");
    assert_eq!(dashboard["peerRankings"].as_array().unwrap().len(), 1);
    assert_eq!(dashboard["peerRankings"][0]["rank"], 1);
    assert!(dashboard["efficiencyMetrics"].is_object());
    assert!(dashboard["recommendations"].is_array());

    for (path, expected_kind) in [
        ("/api/swarm/analytics/performance", "object"),
        ("/api/swarm/analytics/peers/rankings?limit=2", "array"),
        ("/api/swarm/analytics/efficiency", "object"),
        ("/api/swarm/analytics/recommendations", "array"),
    ] {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .expect("swarm analytics projection");
        assert_eq!(response.status, "200 OK", "{path}");
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
        assert_eq!(
            if value.is_array() { "array" } else { "object" },
            expected_kind,
            "{path}"
        );
    }

    let trends = crate::route_http_request(
        "GET",
        "/api/swarm/analytics/trends?timeWindowHours=48&dataPoints=12",
        None,
        "",
        &state,
    )
    .await
    .expect("swarm analytics trends");
    let trends = serde_json::from_str::<serde_json::Value>(&trends.body).unwrap();
    assert_eq!(trends["timePoints"], serde_json::json!([]));

    for path in [
        "/api/swarm/analytics/dashboard?timeWindowHours=0",
        "/api/swarm/analytics/dashboard?rankingLimit=101",
        "/api/swarm/analytics/peers/rankings?limit=0",
        "/api/swarm/analytics/trends?dataPoints=1",
    ] {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .expect("invalid swarm analytics query");
        assert_eq!(response.status, "400 Bad Request", "{path}");
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn mesh_rendezvous_api_discovers_users_and_mesh_capabilities() {
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
        "/api/soulseek/mesh-rendezvous/status",
        None,
        "",
        &state,
    )
    .await
    .expect("mesh status");
    assert_eq!(status.status, "200 OK");
    let status_json = serde_json::from_str::<serde_json::Value>(&status.body).unwrap();
    assert_eq!(status_json["enabled"], true);
    assert_eq!(status_json["activeProbe"], true);
    assert_eq!(status_json["interestTag"], "slskdn-mesh-v1");
    assert_eq!(status_json["candidateCount"], 3);

    let discover = crate::route_http_request(
        "GET",
        "/api/soulseek/mesh-rendezvous/discover",
        None,
        "",
        &state,
    )
    .await
    .expect("mesh discover");
    assert_eq!(discover.status, "200 OK");
    let discover_json = serde_json::from_str::<serde_json::Value>(&discover.body).unwrap();
    let users = discover_json["users"].as_array().unwrap();
    assert_eq!(users[0]["username"], "alice");
    assert_eq!(users[1]["username"], "Bob");
    assert_eq!(users[2]["username"], "carol");
    assert_eq!(discover_json["capabilityRecordCount"], 3);

    let capabilities =
        crate::route_http_request("GET", "/api/soulseek/peer-capabilities", None, "", &state)
            .await
            .expect("peer capabilities");
    assert_eq!(capabilities.status, "200 OK");
    let capabilities_json = serde_json::from_str::<serde_json::Value>(&capabilities.body).unwrap();
    assert_eq!(capabilities_json.as_array().unwrap().len(), 3);
    assert_eq!(capabilities_json[0]["meshCapable"], true);
    assert_eq!(capabilities_json[2]["meshCapable"], false);

    let peers = crate::route_http_request("GET", "/api/mesh/peers", None, "", &state)
        .await
        .expect("mesh peers");
    assert_eq!(peers.status, "200 OK");
    assert!(peers.body.contains("\"peers\""));
    assert!(peers.body.contains("\"carol\""));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn mesh_sync_and_realm_conflict_routes_require_single_encoded_segments() {
    let (state, _receiver) = test_state();
    {
        let mut users = state.users.write().await;
        users.watch("mesh peer".to_owned());
    }

    let mesh_sync =
        crate::route_http_request("POST", "/api/mesh/sync/mesh%20peer", None, "{}", &state)
            .await
            .expect("encoded mesh sync");
    assert_eq!(mesh_sync.status, "202 Accepted");
    let mesh_sync_json = serde_json::from_str::<serde_json::Value>(&mesh_sync.body).unwrap();
    assert_eq!(mesh_sync_json["username"], "mesh peer");
    assert_eq!(mesh_sync_json["queued"], true);

    let aliased_mesh_sync = crate::route_http_request(
        "POST",
        "/api/mesh/sync/mesh%20peer/untrusted",
        None,
        "{}",
        &state,
    )
    .await
    .expect("reject aliased mesh sync");
    assert_eq!(aliased_mesh_sync.status, "404 Not Found");

    let realm_conflicts = crate::route_http_request(
        "GET",
        "/api/realm-subject-indexes/local%20realm/conflicts",
        None,
        "",
        &state,
    )
    .await
    .expect("encoded realm conflicts");
    assert_eq!(realm_conflicts.status, "200 OK");
    assert!(realm_conflicts.body.contains("local realm"));

    let aliased_realm_conflicts = crate::route_http_request(
        "GET",
        "/api/realm-subject-indexes/local/extra/conflicts",
        None,
        "",
        &state,
    )
    .await
    .expect("reject aliased realm conflicts");
    assert_eq!(aliased_realm_conflicts.status, "404 Not Found");
}

#[cfg(unix)]
#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn mesh_sync_chunk_reads_remain_confined_to_share_roots() {
    use slskr_client::mesh_sync::{
        MeshMessageType, MeshReqChunkMessage, MeshSyncBase, MeshSyncMessage,
    };
    use std::os::unix::fs::symlink;

    let unique = uuid::Uuid::new_v4().simple().to_string();
    let root = std::env::temp_dir().join(format!(
        "slskr-mesh-sync-share-root-{}-{unique}",
        std::process::id()
    ));
    let outside = std::env::temp_dir().join(format!(
        "slskr-mesh-sync-share-outside-{}-{unique}",
        std::process::id()
    ));
    let album = root.join("album");
    fs::create_dir_all(&album).expect("create mesh-sync share root");
    fs::create_dir_all(&outside).expect("create mesh-sync outside directory");
    let local_path = album.join("secret.flac");
    let inside_bytes = b"inside-mesh-sync";
    fs::write(&local_path, inside_bytes).expect("write mesh-sync shared file");
    fs::write(outside.join("secret.flac"), b"outside-mesh-sync")
        .expect("write mesh-sync outside file");

    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_SHARE_FIXTURE", "")
            .with("SLSKR_SHARE_DIRS", &root.display().to_string()),
    );
    let (filename, size) = {
        let shares = state.shares.read().await;
        shares
            .entries
            .iter()
            .find_map(|entry| {
                shares
                    .local_paths
                    .get(&entry.filename)
                    .filter(|path| path.as_path() == local_path.as_path())
                    .map(|_| (entry.filename.clone(), entry.size))
            })
            .expect("mesh-sync fixture must be indexed")
    };
    let flac_key = crate::content_discovery::generate_flac_key(&filename, size);
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&[67; 32]);
    let mut request = MeshSyncMessage::ReqChunk(MeshReqChunkMessage {
        message_type: MeshMessageType::ReqChunk,
        base: MeshSyncBase::default(),
        flac_key: flac_key.clone(),
        offset: 0,
        length: 6,
    });
    request
        .sign_at(&signing_key, crate::unix_timestamp_millis() as i64)
        .expect("sign mesh-sync confined read request");
    let response = crate::mesh_sync::handle_signed_message(&state, "mesh-peer", request)
        .await
        .expect("mesh-sync response before path swap");
    match response {
        MeshSyncMessage::RespChunk(message) => assert!(message.success),
        other => panic!("unexpected mesh-sync response before path swap: {other:?}"),
    }

    fs::rename(&album, root.join("album-real")).expect("move original mesh-sync directory");
    symlink(&outside, &album).expect("swap mesh-sync parent with symlink");
    let mut swapped_request = MeshSyncMessage::ReqChunk(MeshReqChunkMessage {
        message_type: MeshMessageType::ReqChunk,
        base: MeshSyncBase::default(),
        flac_key,
        offset: 0,
        length: 6,
    });
    swapped_request
        .sign_at(&signing_key, crate::unix_timestamp_millis() as i64)
        .expect("sign swapped mesh-sync read request");
    let response = crate::mesh_sync::handle_signed_message(&state, "mesh-peer", swapped_request)
        .await
        .expect("mesh-sync response after path swap");
    match response {
        MeshSyncMessage::RespChunk(message) => {
            assert!(!message.success, "swapped share parent must fail closed");
            assert!(message.data_base64.is_empty());
        }
        other => panic!("unexpected mesh-sync response after path swap: {other:?}"),
    }

    let _ = fs::remove_dir_all(root);
    let _ = fs::remove_dir_all(outside);
    let _ = fs::remove_dir_all(state.config.state_dir.clone());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn overlay_pin_rotation_uses_the_real_thumbprint_field_and_no_content_response() {
    // Matches the oracle's real RotateCertificatePin
    // (DhtRendezvousController.cs): the wire field is "thumbprint",
    // not "certificateSha256"/"pin", the pin is stored
    // upper-invariant, and a successful rotation is a real 204 No
    // Content, not a 200 with a JSON body.
    let (state, _receiver) = test_state();

    let missing_field = crate::route_http_request(
        "PUT",
        "/api/overlay/pins/peer1",
        None,
        r#"{"certificateSha256":"07070707070707070707070707070707070707070707070707070707070707"}"#,
        &state,
    )
    .await
    .expect("reject the wrong field name");
    assert_eq!(missing_field.status, "400 Bad Request");

    let too_short = crate::route_http_request(
        "PUT",
        "/api/overlay/pins/peer1",
        None,
        r#"{"thumbprint":"abc123"}"#,
        &state,
    )
    .await
    .expect("reject a too-short thumbprint");
    assert_eq!(too_short.status, "400 Bad Request");

    let rotated = crate::route_http_request(
        "PUT",
        "/api/overlay/pins/peer1",
        None,
        r#"{"thumbprint":"0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a"}"#,
        &state,
    )
    .await
    .expect("rotate the pin");
    assert_eq!(rotated.status, "204 No Content", "{}", rotated.body);
    assert!(rotated.body.is_empty());

    let stored = state
        .controller_features
        .read()
        .await
        .get("overlay/pin/peer1")
        .cloned()
        .expect("pin was really stored");
    assert_eq!(
        stored["certificateSha256"],
        "0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A"
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn overlay_blocklist_honors_real_reason_duration_and_permanent_fields() {
    // Matches the oracle's real BlockIp/BlockUsername/GetBlocklist
    // (DhtRendezvousController.cs): reason/durationMinutes/permanent
    // are real inputs, and the listing reflects the oracle's real
    // BlockedEntryResponse shape (target/type/reason/blockedAt/
    // expiresAt/isPermanent) -- previously these fields were
    // silently discarded in favor of a fixed 1-hour "Manual ban"
    // default, and the listing used an unrelated shape.
    let (state, _receiver) = test_state();

    let blocked = crate::route_http_request(
        "POST",
        "/api/overlay/blocklist/username",
        None,
        r#"{"value":"forever-banned","reason":"repeated abuse","permanent":true}"#,
        &state,
    )
    .await
    .expect("block a username permanently");
    assert_eq!(blocked.status, "200 OK", "{}", blocked.body);

    let listed = crate::route_http_request("GET", "/api/overlay/blocklist", None, "", &state)
        .await
        .expect("list the blocklist");
    assert_eq!(listed.status, "200 OK");
    let listed_json = serde_json::from_str::<serde_json::Value>(&listed.body).unwrap();
    let entries = listed_json["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 1, "{entries:?}");
    assert_eq!(entries[0]["target"], "forever-banned");
    assert_eq!(entries[0]["type"], "username");
    assert_eq!(entries[0]["reason"], "repeated abuse");
    assert_eq!(entries[0]["isPermanent"], true);
    assert!(entries[0]["blockedAt"].as_str().is_some());
    assert!(entries[0]["expiresAt"].as_str().is_some());

    let timed = crate::route_http_request(
        "POST",
        "/api/overlay/blocklist/ip",
        None,
        r#"{"value":"203.0.113.5","durationMinutes":5}"#,
        &state,
    )
    .await
    .expect("block an ip for a real, non-default duration");
    assert_eq!(timed.status, "200 OK");
    let ban = state
        .security
        .read()
        .await
        .bans
        .iter()
        .find(|record| record.kind == "ip")
        .cloned()
        .expect("ip ban recorded");
    assert!(!ban.is_permanent);
    assert_eq!(ban.expires_at - ban.created_at, 5 * 60);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn mesh_publish_and_lookup_round_trip_through_the_real_hash_database() {
    // Matches the oracle's real PublishHashAsync/LookupHashAsync,
    // which both operate on the exact same hash database. Previously
    // POST /api/mesh/publish wrote into an unrelated
    // "mesh/published/{key}" feature-state key that
    // GET /api/mesh/lookup/{flacKey} never read, so a publish then
    // immediate lookup always reported "not found".
    let (state, _receiver) = test_state();

    let missing =
        crate::route_http_request("GET", "/api/mesh/lookup/round-trip-key", None, "", &state)
            .await
            .expect("lookup before publish");
    assert_eq!(missing.status, "404 Not Found");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&missing.body).unwrap()["found"],
        false
    );

    let byte_hash = "a".repeat(64);
    let published = crate::route_http_request(
        "POST",
        "/api/mesh/publish",
        None,
        &format!(r#"{{"flacKey":"round-trip-key","byteHash":"{byte_hash}","size":4096}}"#),
        &state,
    )
    .await
    .expect("publish a real hash");
    assert_eq!(published.status, "200 OK", "{}", published.body);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&published.body).unwrap()["published"],
        true
    );

    let found =
        crate::route_http_request("GET", "/api/mesh/lookup/round-trip-key", None, "", &state)
            .await
            .expect("lookup after publish");
    assert_eq!(found.status, "200 OK", "{}", found.body);
    let found_json = serde_json::from_str::<serde_json::Value>(&found.body).unwrap();
    assert_eq!(found_json["found"], true);
    assert_eq!(found_json["entry"]["flacKey"], "round-trip-key");
    assert_eq!(found_json["entry"]["byteHash"], byte_hash);
    assert_eq!(found_json["entry"]["size"], 4096);
    assert!(found_json.get("flacKey").is_none());
    assert!(found_json.get("peers").is_none());

    // The "key"/"contentId" aliases the previous implementation
    // accepted (and used to bypass byteHash/size validation) are no
    // longer accepted at all -- only the oracle's real "flacKey".
    let alias_rejected = crate::route_http_request(
        "POST",
        "/api/mesh/publish",
        None,
        r#"{"key":"alias-key"}"#,
        &state,
    )
    .await
    .expect("reject the key/contentId alias with no real validation");
    assert_eq!(alias_rejected.status, "400 Bad Request");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn mesh_preview_ticket_fetches_verifies_streams_and_removes_staging_file() {
    run_controller_future_on_large_stack("mesh-preview-ticket-stream", || {
        mesh_preview_ticket_fetches_verifies_streams_and_removes_staging_file_impl()
    });
}
#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn mesh_gateway_disabled_post_short_circuits_compatibility_fallback() {
    let (state, _receiver) = test_state();
    let response = crate::route_http_request(
        "POST",
        "/mesh/http/route-audit-id/route-audit-id",
        None,
        "{}",
        &state,
    )
    .await
    .expect("mesh gateway response");
    assert_eq!(response.status, "404 Not Found");
    assert_eq!(response.body, r#"{"error":"gateway_disabled"}"#);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn mesh_gateway_enabled_enforces_allowlist_and_provider_discovery() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKD_MESH_GATEWAY_ENABLED", "true")
            .with("SLSKD_MESH_GATEWAY_ALLOWED_SERVICES", "pods"),
    );

    let rejected =
        crate::route_http_request("POST", "/mesh/http/route-audit-id/List", None, "{}", &state)
            .await
            .expect("allowlist response");
    assert_eq!(rejected.status, "403 Forbidden");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&rejected.body).unwrap(),
        serde_json::json!({
            "error": "service_not_allowed",
            "message": "Requested service is not allowed",
        })
    );

    let unavailable = crate::route_http_request("POST", "/mesh/http/pods/List", None, "{}", &state)
        .await
        .expect("provider response");
    assert_eq!(unavailable.status, "503 Service Unavailable");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&unavailable.body).unwrap(),
        serde_json::json!({
            "error": "service_unavailable",
            "message": "No providers found for the requested service",
        })
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn mesh_gateway_enabled_enforces_target_auth_and_origin_contract() {
    run_controller_future_on_large_stack("mesh-gateway-auth-origin", || {
        mesh_gateway_enabled_enforces_target_auth_and_origin_contract_impl()
    });
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn mesh_gateway_enabled_dispatches_to_real_local_service_handlers() {
    let root = std::env::temp_dir().join(format!(
        "slskr-mesh-http-gateway-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).expect("mesh gateway state directory");
    let (mut state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKD_MESH_GATEWAY_ENABLED", "true")
            .with("SLSKD_MESH_GATEWAY_ALLOWED_SERVICES", "pods"),
    );
    let gateway = Arc::new(
        crate::private_gateway::Gateway::load_or_create_with_quic(
            "127.0.0.1:0".parse().unwrap(),
            &root,
            None,
        )
        .await
        .expect("mesh gateway"),
    );
    Arc::get_mut(&mut state)
        .expect("unshared test state")
        .private_gateway = Some(gateway);

    let response = crate::route_http_request("POST", "/mesh/http/pods/List", None, "{}", &state)
        .await
        .expect("local service response");
    assert_eq!(response.status, "200 OK", "{}", response.body);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&response.body).unwrap(),
        serde_json::json!([])
    );
    std::fs::remove_dir_all(root).expect("remove mesh gateway state directory");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn overlay_bind_rejects_zero_port() {
    let error = crate::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default().with("SLSKR_OVERLAY_BIND", "127.0.0.1:0"),
    )
    .unwrap_err();
    assert_eq!(error, "SLSKR_OVERLAY_BIND port must be non-zero");
}
