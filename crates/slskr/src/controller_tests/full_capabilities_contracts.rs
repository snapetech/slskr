//! Controller full capabilities contracts ownership.

use super::*;

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn capabilities_negotiate_returns_intersection() {
    let (state, _receiver) = test_state();

    let response = crate::route_http_request(
        "POST",
        "/api/v0/capabilities/negotiate",
        None,
        "{\"capabilities\":[\"shares\",\"telemetry\",\"bogus\"]}",
        &state,
    )
    .await
    .expect("capability negotiation response");

    assert_eq!(response.status, "200 OK");
    assert_eq!(response.content_type, "application/json");
    assert!(response
        .body
        .contains("\"accepted\":[\"shares\",\"telemetry\"]"));
    assert!(response.body.contains("\"unsupported\":[\"bogus\"]"));
    assert!(response.body.contains("\"server_capabilities\":["));
    assert!(!response.body.contains("secret"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn capabilities_parse_uses_real_tag_and_version_grammar() {
    let (state, _receiver) = test_state();

    let tagged = crate::route_http_request(
        "POST",
        "/api/capabilities/parse",
        None,
        r#"{"description":"Client slskr_caps:v7;dht=1;mesh=1;swarm=1;hashx=1;flacdb=1;partial=1"}"#,
        &state,
    )
    .await
    .expect("tag capability parse response");
    assert_eq!(tagged.status, "200 OK");
    let tagged_json = serde_json::from_str::<serde_json::Value>(&tagged.body).unwrap();
    assert_eq!(tagged_json["isSlskdn"], true);
    assert_eq!(tagged_json["flags"], "SupportsDHT, SupportsHashExchange, SupportsPartialDownload, SupportsMeshSync, SupportsFlacHashDb, SupportsSwarm");
    assert_eq!(tagged_json["flagsValue"], 63);
    assert_eq!(tagged_json["protocolVersion"], 7);
    assert_eq!(tagged_json["clientVersion"], "");
    assert_eq!(tagged_json["canSwarm"], true);
    assert_eq!(tagged_json["canMeshSync"], true);

    let version = crate::route_http_request(
        "POST",
        "/api/capabilities/parse",
        None,
        r#"{"description":"ordinary client","versionString":"slskdn/2.4.1+dht+mesh+swarm"}"#,
        &state,
    )
    .await
    .expect("version capability parse response");
    let version_json = serde_json::from_str::<serde_json::Value>(&version.body).unwrap();
    assert_eq!(
        version_json["flags"],
        "SupportsDHT, SupportsMeshSync, SupportsSwarm"
    );
    assert_eq!(version_json["flagsValue"], 41);
    assert_eq!(version_json["protocolVersion"], 1);
    assert_eq!(version_json["clientVersion"], "2.4.1");

    let tag_takes_precedence = crate::route_http_request(
        "POST",
        "/api/capabilities/parse",
        None,
        r#"{"description":"slskr_caps:v3;mesh=1","versionString":"slskdn/9.9+swarm"}"#,
        &state,
    )
    .await
    .expect("tag precedence response");
    let precedence_json =
        serde_json::from_str::<serde_json::Value>(&tag_takes_precedence.body).unwrap();
    assert_eq!(precedence_json["flagsValue"], 8);
    assert_eq!(precedence_json["protocolVersion"], 3);
    assert_eq!(precedence_json["clientVersion"], "");

    let false_positive = crate::route_http_request(
        "POST",
        "/api/capabilities/parse",
        None,
        r#"{"description":"ordinary slskdn client"}"#,
        &state,
    )
    .await
    .expect("non-capability response");
    assert_eq!(false_positive.body, r#"{"isSlskdn":false}"#);

    let duplicate_flags = crate::route_http_request(
        "POST",
        "/api/capabilities/parse",
        None,
        r#"{"description":"slskr_caps:v1;dht=1;dht=1"}"#,
        &state,
    )
    .await
    .expect("duplicate capability parse response");
    let duplicate_json = serde_json::from_str::<serde_json::Value>(&duplicate_flags.body).unwrap();
    assert_eq!(duplicate_json["flags"], "SupportsDHT");
    assert_eq!(duplicate_json["flagsValue"], 1);

    let no_flags = crate::route_http_request(
        "POST",
        "/api/capabilities/parse",
        None,
        r#"{"versionString":"slskdn/2.4.1"}"#,
        &state,
    )
    .await
    .expect("empty capability parse response");
    let no_flags_json = serde_json::from_str::<serde_json::Value>(&no_flags.body).unwrap();
    assert_eq!(no_flags_json["flags"], "None");
    assert_eq!(no_flags_json["flagsValue"], 0);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn capabilities_peers_routes_use_the_real_capability_store() {
    // Matches the oracle's real GetPeers/GetMeshPeers: known peer
    // *capability* records (mesh-capable subset for mesh-peers), not
    // the generic connected-Soulseek-user list. Previously both
    // "/api/capabilities/peers" and its "/api/v0/capabilities/
    // mesh-peers" alias collapsed into the same wrong handler.
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));
    {
        let mut mesh = state.mesh.write().await;
        mesh.capability_records.push(test_capability_descriptor(
            "mesh-capable-peer",
            vec![slskr_client::capabilities::FEATURE_MESH_V1.to_owned()],
        ));
        mesh.capability_records
            .push(test_capability_descriptor("plain-peer", vec![]));
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&[42; 32]);
        let persisted_descriptor = slskr_client::capabilities::PeerCapabilityDescriptor::unsigned(
            "persisted-peer",
            vec![slskr_client::capabilities::FEATURE_MESH_V1.to_owned()],
            Vec::new(),
            std::time::Duration::from_secs(300),
            &signing_key,
            std::time::SystemTime::now(),
        )
        .unwrap()
        .sign(&signing_key)
        .unwrap();
        let projection = vec![serde_json::json!({
            "peerId": persisted_descriptor.peer_id,
            "username": persisted_descriptor.username,
            "features": persisted_descriptor.features,
            "endpoints": persisted_descriptor.endpoints,
            "overlayPort": persisted_descriptor.overlay_port,
            "maxPayloadLength": persisted_descriptor.max_payload_length,
            "issuedAtUnix": persisted_descriptor.issued_at_unix,
            "expiresAtUnix": persisted_descriptor.expires_at_unix,
            "publicKey": base64::engine::general_purpose::STANDARD.encode(persisted_descriptor.public_key),
            "signature": persisted_descriptor.signature.map(|value| base64::engine::general_purpose::STANDARD.encode(value)),
        })];
        state
            .controller_features
            .write_for_test()
            .await
            .upsert(
                "hashdb/peers".to_owned(),
                serde_json::json!({"peers": projection}),
            )
            .unwrap();
    }
    // A watched Soulseek user with no capability record at all must
    // not appear in either capability listing.
    state.users.write().await.apply_status(&crate::UserStatus {
        username: "unrelated-soulseek-user".to_owned(),
        status: 2,
        privileged: false,
    });

    let peers = crate::route_http_request("GET", "/api/capabilities/peers", None, "", &state)
        .await
        .expect("list all known peer capability records");
    assert_eq!(peers.status, "200 OK");
    let peers_json = serde_json::from_str::<serde_json::Value>(&peers.body).unwrap();
    let usernames = peers_json["peers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|record| record["username"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(peers_json["count"], 2);
    assert!(usernames.contains(&"mesh-capable-peer"));
    assert!(usernames.contains(&"plain-peer"));
    assert!(!usernames.contains(&"unrelated-soulseek-user"));
    assert_eq!(peers_json["peers"][0]["protocolVersion"], 1);
    assert_eq!(
        peers_json["peers"][0]["clientVersion"],
        "slskdn/runtime-capability-v1"
    );
    assert_eq!(peers_json["peers"][0]["flagsValue"], 8);
    assert_eq!(peers_json["peers"][0]["flags"], "SupportsMeshSync");
    assert_eq!(peers_json["peers"][0]["canMeshSync"], true);
    assert!(peers_json["peers"][0]["lastSeen"].is_string());
    assert_eq!(peers_json["peers"][0]["meshSeqId"], 0);
    assert!(peers_json["peers"][0].get("peerId").is_none());

    let mesh_peers =
        crate::route_http_request("GET", "/api/v0/capabilities/mesh-peers", None, "", &state)
            .await
            .expect("list only mesh-capable peer capability records");
    assert_eq!(mesh_peers.status, "200 OK");
    let mesh_peers_json = serde_json::from_str::<serde_json::Value>(&mesh_peers.body).unwrap();
    assert_eq!(mesh_peers_json["count"], 1);
    assert_eq!(mesh_peers_json["peers"][0]["username"], "mesh-capable-peer");
    assert!(mesh_peers_json["peers"][0].get("flags").is_none());

    let peer = crate::route_http_request(
        "GET",
        "/api/capabilities/peers/mesh-capable-peer",
        None,
        "",
        &state,
    )
    .await
    .expect("get one capability peer");
    let peer_json = serde_json::from_str::<serde_json::Value>(&peer.body).unwrap();
    assert_eq!(peer.status, "200 OK");
    assert_eq!(peer_json["username"], "mesh-capable-peer");
    assert_eq!(peer_json["flagsValue"], 8);

    let hash_peers = crate::route_http_request("GET", "/api/v0/hashdb/peers", None, "", &state)
        .await
        .expect("list hashdb capability peers");
    assert_eq!(hash_peers.status, "200 OK", "{}", hash_peers.body);
    let hash_peers_json = serde_json::from_str::<serde_json::Value>(&hash_peers.body).unwrap();
    assert_eq!(hash_peers_json["count"], 1);
    assert_eq!(hash_peers_json["peers"][0]["peerId"], "mesh-capable-peer");
    assert_eq!(hash_peers_json["peers"][0]["caps"], 8);
    assert_eq!(hash_peers_json["peers"][0]["capsFlags"], "SupportsMeshSync");
    assert_eq!(
        hash_peers_json["peers"][0]["clientVersion"],
        "slskdn/runtime-capability-v1"
    );
    assert!(hash_peers_json["peers"][0]["lastSeen"].is_string());
    assert_eq!(hash_peers_json["peers"][0]["backfillsToday"], 0);

    state
        .backfill
        .write()
        .await
        .record_peer_success("mesh-capable-peer", crate::unix_timestamp());
    let hash_peers = crate::route_http_request("GET", "/api/v0/hashdb/peers", None, "", &state)
        .await
        .expect("list hashdb peers after a backfill");
    let hash_peers_json = serde_json::from_str::<serde_json::Value>(&hash_peers.body).unwrap();
    assert_eq!(hash_peers_json["peers"][0]["backfillsToday"], 1);

    let persisted = state
        .controller_features
        .read()
        .await
        .get("hashdb/peers")
        .cloned()
        .unwrap();
    let mut restored = crate::MeshState::new();
    restored.restore_persisted_capabilities(persisted["peers"].as_array().unwrap());
    assert_eq!(restored.capability_records.len(), 1);
    assert!(restored
        .capability_records
        .iter()
        .any(|record| record.username == "persisted-peer"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn deterministic_openapi_mutations_match_native_status_and_dto_contracts() {
    let (state, _receiver) = test_state();

    for (path, enabled) in [
        ("/api/v0/autoreplace/enable", true),
        ("/api/v0/autoreplace/disable", false),
    ] {
        let response = crate::route_http_request("PUT", path, None, "", &state)
            .await
            .unwrap();
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
        assert_eq!(value["enabled"], enabled);
        for key in [
            "lastRunAt",
            "lastRunProcessedCount",
            "lastRunReplacedCount",
            "intervalSeconds",
        ] {
            assert!(value.get(key).is_some(), "{path}: missing {key}");
        }
    }

    let destination = crate::route_http_request(
        "POST",
        "/api/v0/destinations/validate",
        None,
        r#"{"path":"/tmp/slskdn-route-audit"}"#,
        &state,
    )
    .await
    .unwrap();
    let destination = serde_json::from_str::<serde_json::Value>(&destination.body).unwrap();
    assert_eq!(destination["path"], "/tmp/slskdn-route-audit");
    assert!(destination.get("exists").is_some());
    assert!(destination.get("writable").is_some());

    let announce = crate::route_http_request("POST", "/api/v0/dht/announce", None, "", &state)
        .await
        .unwrap();
    assert_eq!(announce.status, "400 Bad Request");
    assert_eq!(announce.body, r#"{"error":"Not beacon capable"}"#);
    let discover = crate::route_http_request("POST", "/api/v0/dht/discover", None, "", &state)
        .await
        .unwrap();
    let discover = serde_json::from_str::<serde_json::Value>(&discover.body).unwrap();
    assert!(discover.get("newConnectionsMade").is_some());
    assert!(discover.get("totalMeshConnections").is_some());

    for (path, message) in [
        (
            "/api/v0/hashdb/optimize/indexes",
            "Index optimization completed",
        ),
        (
            "/api/v0/hashdb/optimize/vacuum",
            "VACUUM and ANALYZE completed",
        ),
    ] {
        let response = crate::route_http_request("POST", path, None, "", &state)
            .await
            .unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&response.body).unwrap()["message"],
            message
        );
    }
    let profile = crate::route_http_request(
        "POST",
        "/api/v0/hashdb/optimize/profile",
        None,
        r#"{"query":"route-audit","parameters":{}}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(profile.status, "400 Bad Request");

    for (method, path) in [
        ("DELETE", "/api/v0/nowplaying"),
        ("DELETE", "/api/v0/integrations/spotify"),
        ("DELETE", "/api/v0/transfers/downloads/all/completed"),
        ("DELETE", "/api/v0/transfers/uploads/all/completed"),
        (
            "PATCH",
            "/api/v0/library/health/issues/00000000-0000-4000-8000-000000000001",
        ),
    ] {
        let response =
            crate::route_http_request(method, path, None, r#"{"status":"Resolved"}"#, &state)
                .await
                .unwrap();
        assert_eq!(response.status, "204 No Content", "{method} {path}");
        assert!(response.body.is_empty(), "{method} {path}");
    }

    let blocked = crate::route_http_request(
        "POST",
        "/api/v0/overlay/blocklist/username",
        None,
        r#"{"username":"route-audit-peer"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(blocked.status, "200 OK");
    let unblocked = crate::route_http_request(
        "DELETE",
        "/api/v0/overlay/blocklist/username/route-audit-peer",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(unblocked.body, r#"{"message":"Blocklist entry removed"}"#);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn capabilities_negotiation_escapes_unsupported_values() {
    let response =
        crate::capabilities_negotiate_response(r#"{"capabilities":["shares","a\",\"x\":\"y"]}"#);
    let parsed = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
    assert_eq!(parsed["accepted"], serde_json::json!(["shares"]));
    assert_eq!(parsed["unsupported"], serde_json::json!(["a\",\"x\":\"y"]));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn capability_identity_is_stable_across_reloads() {
    let root = std::env::temp_dir().join(format!(
        "slskr-capability-identity-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let first = crate::load_or_create_capability_signing_key(&root).unwrap();
    let second = crate::load_or_create_capability_signing_key(&root).unwrap();
    assert_eq!(
        first.verifying_key().to_bytes(),
        second.verifying_key().to_bytes()
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn capability_identity_rejects_symlinked_key() {
    use std::os::unix::fs::symlink;

    let root = std::env::temp_dir().join(format!(
        "slskr-capability-symlink-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let target = root.join("target-key.bin");
    std::fs::write(&target, [7_u8; 32]).unwrap();
    symlink(&target, root.join("peer-capability-key.bin")).unwrap();
    let error = crate::load_or_create_capability_signing_key(&root).unwrap_err();
    assert!(error.contains("must be a regular file"), "{error}");
    std::fs::remove_dir_all(root).unwrap();
}
