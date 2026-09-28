use std::{collections::BTreeSet, fs, sync::Arc};

use base64::Engine;

use super::{
    add_test_share, test_capability_descriptor, test_state, test_state_with_env,
    test_state_with_env_parts, MapEnv,
};

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
pub(super) async fn controller_api_differential_mesh_stats_reflect_real_merge_activity_not_hardcoded_zeros(
) {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));

    let baseline = crate::route_http_request("GET", "/api/v0/mesh/stats", None, "", &state)
        .await
        .expect("baseline mesh stats");
    assert_eq!(baseline.status, "200 OK", "{}", baseline.body);
    assert_eq!(baseline.content_type, "application/json");
    let baseline_json = serde_json::from_str::<serde_json::Value>(&baseline.body).unwrap();
    assert_eq!(baseline_json["totalSyncs"], 0, "{baseline_json}");
    assert_eq!(baseline_json["totalEntriesSent"], 0, "{baseline_json}");
    assert_eq!(baseline_json["currentSeqId"], 0, "{baseline_json}");

    let valid_entry = serde_json::json!({
        "flacKey": "mesh-stats-audit-key",
        "size": 1234,
        "byteHash": "a".repeat(64),
    });
    let merged = crate::route_http_request(
        "POST",
        "/api/v0/mesh/merge?fromUser=mesh-peer",
        None,
        &serde_json::json!({"entries": [valid_entry]}).to_string(),
        &state,
    )
    .await
    .expect("merge real entry");
    assert_eq!(merged.status, "200 OK", "{}", merged.body);
    assert_eq!(merged.content_type, "application/json");
    let merged_json = serde_json::from_str::<serde_json::Value>(&merged.body).unwrap();
    assert_eq!(merged_json["merged"], 1, "{merged_json}");
    let real_seq_id = merged_json["latestSeqId"].as_u64().unwrap();
    assert!(real_seq_id > 0, "{merged_json}");

    let delta = crate::route_http_request(
        "GET",
        "/api/v0/mesh/delta?sinceSeq=0&maxEntries=1",
        None,
        "",
        &state,
    )
    .await
    .expect("generate a real mesh delta");
    assert_eq!(delta.status, "200 OK", "{}", delta.body);
    assert_eq!(delta.content_type, "application/json");
    let delta_json = serde_json::from_str::<serde_json::Value>(&delta.body).unwrap();
    assert_eq!(delta_json["type"], 3, "{delta_json}");
    assert_eq!(
        delta_json["entries"].as_array().unwrap().len(),
        1,
        "{delta_json}"
    );
    assert_eq!(
        delta_json["entries"][0]["seq_id"], real_seq_id,
        "{delta_json}"
    );
    assert!(
        delta_json["entries"][0].get("flacKey").is_none(),
        "{delta_json}"
    );
    assert_eq!(delta_json["has_more"], false, "{delta_json}");
    assert_eq!(delta_json["latest_seq_id"], real_seq_id, "{delta_json}");

    // A structurally usable but malformed hash is retained through the
    // controller's request normalization and then skipped individually,
    // matching MeshSyncService.MergeEntriesAsync.
    let invalid_entry = serde_json::json!({
        "flacKey": "mesh-stats-audit-invalid",
        "byteHash": "not-a-sha256",
        "size": 1234
    });
    let failed = crate::route_http_request(
        "POST",
        "/api/v0/mesh/merge?fromUser=mesh-peer",
        None,
        &serde_json::json!({"entries": [invalid_entry]}).to_string(),
        &state,
    )
    .await
    .expect("merge invalid entry");
    assert_eq!(failed.status, "200 OK", "{}", failed.body);
    assert_eq!(failed.content_type, "application/json");
    let failed_json = serde_json::from_str::<serde_json::Value>(&failed.body).unwrap();
    assert_eq!(failed_json["merged"], 0, "{failed_json}");
    assert_eq!(failed_json["skipped"], 1, "{failed_json}");

    let stats = crate::route_http_request("GET", "/api/v0/mesh/stats", None, "", &state)
        .await
        .expect("mesh stats after activity");
    assert_eq!(stats.status, "200 OK", "{}", stats.body);
    assert_eq!(stats.content_type, "application/json");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap();
    assert_eq!(stats_json["totalSyncs"], 2, "{stats_json}");
    assert_eq!(stats_json["successfulSyncs"], 2, "{stats_json}");
    assert_eq!(stats_json["failedSyncs"], 0, "{stats_json}");
    assert_eq!(stats_json["totalEntriesReceived"], 2, "{stats_json}");
    assert_eq!(stats_json["totalEntriesSent"], 1, "{stats_json}");
    assert_eq!(stats_json["skippedEntries"], 1, "{stats_json}");
    assert_eq!(stats_json["totalEntriesMerged"], 1, "{stats_json}");
    assert_eq!(stats_json["currentSeqId"], real_seq_id, "{stats_json}");

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("mesh_stats_and_merge.json"),
        serde_json::to_string_pretty(&[
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/mesh/stats",
                "case": "nominal-status-headers-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "POST",
                "route": "/api/v0/mesh/merge",
                "case": "nominal-status-headers-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "POST",
                "route": "/api/v0/mesh/merge",
                "case": "mutation-side-effects-and-readback",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/mesh/delta",
                "case": "populated-dynamic-state",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/mesh/stats",
                "case": "populated-dynamic-state",
                "pass": true,
            }),
        ])
        .expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn mesh_hello_matches_frozen_message_dto() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKD_SLSK_USERNAME", "mesh-user"));

    let hello = crate::route_http_request("GET", "/api/v0/mesh/hello", None, "", &state)
        .await
        .expect("mesh hello");
    assert_eq!(hello.status, "200 OK", "{}", hello.body);
    let json = serde_json::from_str::<serde_json::Value>(&hello.body).unwrap();
    assert_eq!(json["type"], 1, "{json}");
    assert_eq!(json["proto_version"], 1, "{json}");
    assert_eq!(json["client_id"], "tester", "{json}");
    assert_eq!(json["client_version"], crate::APP_VERSION, "{json}");
    assert_eq!(json["latest_seq_id"], 0, "{json}");
    assert_eq!(json["hash_count"], 0, "{json}");
    assert_eq!(json["public_key"], "", "{json}");
    assert_eq!(json["signature"], "", "{json}");
    assert_eq!(json["timestamp_ms"], 0, "{json}");
    assert!(json.get("peerId").is_none(), "{json}");
    assert!(json.get("capabilities").is_none(), "{json}");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn versioned_mesh_message_matches_typed_controller_validation() {
    let (state, _receiver) = test_state();

    let missing_peer = crate::route_http_request(
        "POST",
        "/api/v0/mesh/message",
        None,
        r#"{"type":1}"#,
        &state,
    )
    .await
    .expect("missing fromUser response");
    assert_eq!(missing_peer.status, "400 Bad Request");
    assert!(missing_peer.body.contains("fromUser"));

    let missing_type = crate::route_http_request(
        "POST",
        "/api/v0/mesh/message?fromUser=mesh-peer",
        None,
        "{}",
        &state,
    )
    .await
    .expect("missing type response");
    assert_eq!(missing_type.status, "400 Bad Request");
    assert!(missing_type.body.contains("type"));

    let unsupported = crate::route_http_request(
        "POST",
        "/api/v0/mesh/message?fromUser=mesh-peer",
        None,
        r#"{"type":6}"#,
        &state,
    )
    .await
    .expect("unsupported type response");
    assert_eq!(unsupported.status, "400 Bad Request");

    let unsigned = crate::route_http_request(
        "POST",
        "/api/v0/mesh/message?fromUser=mesh-peer",
        None,
        r#"{"type":1}"#,
        &state,
    )
    .await
    .expect("unsigned envelope response");
    assert_eq!(unsigned.status, "200 OK", "{}", unsigned.body);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&unsigned.body).unwrap(),
        serde_json::json!({"handled": true, "response": null})
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
pub(super) async fn controller_api_differential_mesh_message_runtime() {
    use slskr_client::mesh_sync::{
        MeshHashEntry, MeshHelloMessage, MeshMessageType, MeshPushDeltaMessage,
        MeshReqChunkMessage, MeshReqDeltaMessage, MeshReqKeyMessage, MeshSyncBase, MeshSyncMessage,
    };

    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));
    let root = std::env::temp_dir().join(format!(
        "slskr-controller-mesh-message-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&root).expect("create mesh controller fixture directory");
    let local_path = root.join("fixture.flac");
    let local_bytes = b"mesh-controller-message-fixture";
    fs::write(&local_path, local_bytes).expect("write mesh controller fixture");
    let filename = "Virtual/mesh-controller.flac";
    add_test_share(&state, filename, &local_path, local_bytes.len() as u64).await;
    let local_key = crate::content_discovery::generate_flac_key(filename, local_bytes.len() as u64);
    state
        .content_discovery
        .write()
        .await
        .merge_hash_entries(vec![crate::content_discovery::HashDbEntry {
            flac_key: local_key.clone(),
            byte_hash: "a".repeat(64),
            size: local_bytes.len() as u64,
            ..crate::content_discovery::HashDbEntry::default()
        }])
        .expect("seed mesh controller hash database");

    let remote_key = ed25519_dalek::SigningKey::from_bytes(&[61; 32]);
    let sign_request = |mut message: MeshSyncMessage| -> String {
        message
            .sign_at(&remote_key, crate::unix_timestamp_millis() as i64)
            .expect("sign mesh controller request");
        String::from_utf8(
            message
                .encode_json()
                .expect("encode mesh controller request"),
        )
        .expect("mesh controller request is UTF-8")
    };
    let decode_response = |response: &crate::HttpResponse| {
        assert_eq!(response.status, "200 OK", "{}", response.body);
        assert_eq!(response.content_type, "application/json");
        let outer = serde_json::from_str::<serde_json::Value>(&response.body)
            .expect("decode mesh controller response envelope");
        assert_eq!(outer["handled"], true, "{outer}");
        let encoded = serde_json::to_vec(&outer["response"])
            .expect("encode mesh controller response payload");
        let message =
            MeshSyncMessage::decode_json(&encoded).expect("decode typed mesh controller response");
        message
            .verify_signature()
            .expect("verify typed mesh controller response");
        (outer, message)
    };
    let route = "/api/v0/mesh/message?fromUser=mesh-peer";

    let hello_body = sign_request(MeshSyncMessage::Hello(MeshHelloMessage {
        message_type: MeshMessageType::Hello,
        base: MeshSyncBase::default(),
        client_id: "mesh-peer".to_owned(),
        client_version: "fixture".to_owned(),
        latest_sequence_id: 0,
        hash_count: 0,
    }));
    let hello_response = crate::route_http_request("POST", route, None, &hello_body, &state)
        .await
        .expect("mesh controller hello");
    let (hello_outer, hello) = decode_response(&hello_response);
    assert_eq!(hello_outer["response"]["type"], 1, "{hello_outer}");
    assert!(matches!(hello, MeshSyncMessage::Hello(_)));

    let delta_body = sign_request(MeshSyncMessage::ReqDelta(MeshReqDeltaMessage {
        message_type: MeshMessageType::ReqDelta,
        base: MeshSyncBase::default(),
        since_sequence_id: 0,
        max_entries: 10,
    }));
    let delta_response = crate::route_http_request("POST", route, None, &delta_body, &state)
        .await
        .expect("mesh controller delta");
    let (delta_outer, delta) = decode_response(&delta_response);
    assert_eq!(delta_outer["response"]["type"], 3, "{delta_outer}");
    match delta {
        MeshSyncMessage::PushDelta(message) => assert_eq!(message.entries.len(), 1),
        other => panic!("expected PushDelta response, got {other:?}"),
    }

    let key_body = sign_request(MeshSyncMessage::ReqKey(MeshReqKeyMessage {
        message_type: MeshMessageType::ReqKey,
        base: MeshSyncBase::default(),
        flac_key: local_key.clone(),
    }));
    let key_response = crate::route_http_request("POST", route, None, &key_body, &state)
        .await
        .expect("mesh controller key lookup");
    let (key_outer, key) = decode_response(&key_response);
    assert_eq!(key_outer["response"]["type"], 5, "{key_outer}");
    match key {
        MeshSyncMessage::RespKey(message) => assert!(message.found),
        other => panic!("expected RespKey response, got {other:?}"),
    }

    let chunk_body = sign_request(MeshSyncMessage::ReqChunk(MeshReqChunkMessage {
        message_type: MeshMessageType::ReqChunk,
        base: MeshSyncBase::default(),
        flac_key: local_key.clone(),
        offset: 5,
        length: 7,
    }));
    let chunk_response = crate::route_http_request("POST", route, None, &chunk_body, &state)
        .await
        .expect("mesh controller chunk request");
    let (chunk_outer, chunk) = decode_response(&chunk_response);
    assert_eq!(chunk_outer["response"]["type"], 8, "{chunk_outer}");
    match chunk {
        MeshSyncMessage::RespChunk(message) => {
            assert!(message.success);
            assert_eq!(
                base64::engine::general_purpose::STANDARD
                    .decode(message.data_base64)
                    .expect("decode mesh chunk"),
                &local_bytes[5..12]
            );
        }
        other => panic!("expected RespChunk response, got {other:?}"),
    }

    let pushed_key = "1122334455667788".to_owned();
    let push_body = sign_request(MeshSyncMessage::PushDelta(MeshPushDeltaMessage {
        message_type: MeshMessageType::PushDelta,
        base: MeshSyncBase::default(),
        entries: vec![MeshHashEntry {
            sequence_id: 0,
            flac_key: pushed_key.clone(),
            byte_hash: "b".repeat(64),
            size: 17,
            metadata_flags: None,
            signer_public_key: None,
            signature: None,
        }],
        latest_sequence_id: 0,
        has_more: false,
    }));
    let push_response = crate::route_http_request("POST", route, None, &push_body, &state)
        .await
        .expect("mesh controller delta merge");
    let (push_outer, push) = decode_response(&push_response);
    assert_eq!(push_outer["response"]["type"], 6, "{push_outer}");
    match push {
        MeshSyncMessage::Ack(message) => assert_eq!(message.merged_count, 1),
        other => panic!("expected Ack response, got {other:?}"),
    }

    let pushed_key_body = sign_request(MeshSyncMessage::ReqKey(MeshReqKeyMessage {
        message_type: MeshMessageType::ReqKey,
        base: MeshSyncBase::default(),
        flac_key: pushed_key,
    }));
    let pushed_key_response =
        crate::route_http_request("POST", route, None, &pushed_key_body, &state)
            .await
            .expect("mesh controller merged key readback");
    let (_, pushed_key) = decode_response(&pushed_key_response);
    assert!(matches!(
        pushed_key,
        MeshSyncMessage::RespKey(message) if message.found
    ));

    let unsupported = crate::route_http_request("POST", route, None, r#"{"type":6}"#, &state)
        .await
        .expect("mesh controller unsupported type");
    assert_eq!(unsupported.status, "400 Bad Request");
    let unsigned = crate::route_http_request("POST", route, None, r#"{"type":1}"#, &state)
        .await
        .expect("mesh controller unsigned message");
    assert_eq!(unsigned.status, "200 OK", "{}", unsigned.body);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&unsigned.body).unwrap(),
        serde_json::json!({"handled": true, "response": null})
    );

    let missing_from_user = crate::route_http_request(
        "POST",
        "/api/v0/mesh/message",
        None,
        r#"{"type":1}"#,
        &state,
    )
    .await
    .expect("mesh controller missing fromUser");
    let missing_type = crate::route_http_request("POST", route, None, "{}", &state)
        .await
        .expect("mesh controller missing type");
    let missing_case = missing_from_user.status == "400 Bad Request"
        && missing_from_user.content_type == "application/json"
        && missing_from_user.body.contains("fromUser")
        && missing_type.status == "400 Bad Request"
        && missing_type.content_type == "application/json"
        && missing_type.body.contains("type");

    let missing_chunk_body = sign_request(MeshSyncMessage::ReqChunk(MeshReqChunkMessage {
        message_type: MeshMessageType::ReqChunk,
        base: MeshSyncBase::default(),
        flac_key: "deadbeefdeadbeef".to_owned(),
        offset: 0,
        length: 4,
    }));
    let missing_chunk_response =
        crate::route_http_request("POST", route, None, &missing_chunk_body, &state)
            .await
            .expect("mesh controller missing chunk response");
    let (missing_chunk_outer, missing_chunk) = decode_response(&missing_chunk_response);
    let runtime_failure_case = missing_chunk_outer["response"]["type"] == 8
        && matches!(
            missing_chunk,
            MeshSyncMessage::RespChunk(message) if !message.success
        );

    let concurrent_bodies = (0..6)
        .map(|index| {
            sign_request(MeshSyncMessage::Hello(MeshHelloMessage {
                message_type: MeshMessageType::Hello,
                base: MeshSyncBase::default(),
                client_id: format!("mesh-concurrent-message-{index}"),
                client_version: "fixture".to_owned(),
                latest_sequence_id: 0,
                hash_count: 0,
            }))
        })
        .collect::<Vec<_>>();
    let concurrent_responses =
        futures_util::future::join_all(concurrent_bodies.iter().map(|body| {
            let state = Arc::clone(&state);
            async move { crate::route_http_request("POST", route, None, body, &state).await }
        }))
        .await;
    let concurrency_case = concurrent_responses.iter().all(|response| {
        response.as_ref().is_ok_and(|response| {
            if response.status != "200 OK" || response.content_type != "application/json" {
                return false;
            }
            let value =
                serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
            value["handled"] == true && value["response"]["type"] == 1
        })
    });

    let state_dir = state.config.state_dir.clone();
    let persistent_store = crate::content_discovery::ContentDiscoveryStore::load(&state_dir)
        .expect("create path-backed mesh message store");
    *state.content_discovery.write().await = persistent_store;
    let restart_key = "cafebabecafebabe".to_owned();
    let restart_body = sign_request(MeshSyncMessage::PushDelta(MeshPushDeltaMessage {
        message_type: MeshMessageType::PushDelta,
        base: MeshSyncBase::default(),
        entries: vec![MeshHashEntry {
            sequence_id: 0,
            flac_key: restart_key.clone(),
            byte_hash: "c".repeat(64),
            size: 8001,
            metadata_flags: None,
            signer_public_key: None,
            signature: None,
        }],
        latest_sequence_id: 0,
        has_more: false,
    }));
    let restart_response = crate::route_http_request("POST", route, None, &restart_body, &state)
        .await
        .expect("persist mesh message delta");
    let (restart_outer, restart_message) = decode_response(&restart_response);
    let reloaded = crate::content_discovery::ContentDiscoveryStore::load(&state_dir)
        .expect("reload path-backed mesh message store");
    let restart_case = restart_outer["response"]["type"] == 6
        && matches!(
            restart_message,
            MeshSyncMessage::Ack(message) if message.merged_count == 1
        )
        && reloaded.lookup_hash(&restart_key).is_some();

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create mesh controller evidence directory");
    fs::write(
        evidence_dir.join("mesh_message_runtime.json"),
        serde_json::to_string_pretty(&[
            serde_json::json!({
                "target": "slskdn",
                "method": "POST",
                "route": "/api/v0/mesh/message",
                "case": "nominal-status-headers-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "POST",
                "route": "/api/v0/mesh/message",
                "case": "malformed-path-query-or-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "POST",
                "route": "/api/v0/mesh/message",
                "case": "mutation-side-effects-and-readback",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "POST",
                "route": "/api/v0/mesh/message",
                "case": "missing-empty-or-conflict-state",
                "pass": missing_case,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "POST",
                "route": "/api/v0/mesh/message",
                "case": "runtime-failure-and-timeout",
                "pass": runtime_failure_case,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "POST",
                "route": "/api/v0/mesh/message",
                "case": "restart-persistence-or-reset",
                "pass": restart_case,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "POST",
                "route": "/api/v0/mesh/message",
                "case": "concurrency-and-idempotency",
                "pass": concurrency_case,
            }),
        ])
        .expect("serialize mesh controller evidence"),
    )
    .expect("write mesh controller evidence");
    assert!(
        missing_case && runtime_failure_case && restart_case && concurrency_case,
        "mesh message differential cases failed: missing={missing_case}, runtime={runtime_failure_case}, restart={restart_case}, concurrency={concurrency_case}"
    );
    fs::remove_dir_all(state_dir).expect("remove mesh message test state directory");
    fs::remove_dir_all(root).expect("remove mesh controller fixture directory");
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
pub(super) async fn controller_api_differential_mesh_controller_edge_cases() {
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

    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_SHARE_FIXTURE", ""),
    );

    // MeshController's [ApiController] binding rejects malformed long/int
    // query values instead of silently substituting the defaults.
    let malformed_delta = crate::route_http_request(
        "GET",
        "/api/v0/mesh/delta?sinceSeq=not-an-integer",
        None,
        "",
        &state,
    )
    .await
    .expect("malformed mesh delta response");
    let malformed_max_entries = crate::route_http_request(
        "GET",
        "/api/v0/mesh/delta?maxEntries=not-an-integer",
        None,
        "",
        &state,
    )
    .await
    .expect("malformed mesh delta maxEntries response");
    record!(
        "GET",
        "/api/v0/mesh/delta",
        "malformed-path-query-or-body",
        malformed_delta.status == "400 Bad Request"
            && malformed_max_entries.status == "400 Bad Request"
    );

    for (request_path, ledger_route) in [
        ("/api/v0/mesh/health/extra", "/api/v0/mesh/health"),
        ("/api/v0/mesh/hello/extra", "/api/v0/mesh/hello"),
        ("/api/v0/mesh/peers/extra", "/api/v0/mesh/peers"),
        ("/api/v0/mesh/stats/extra", "/api/v0/mesh/stats"),
        ("/api/v0/mesh/transport/extra", "/api/v0/mesh/transport"),
    ] {
        let response = crate::route_http_request("GET", request_path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("malformed {request_path}: {error}"));
        record!(
            "GET",
            ledger_route,
            "malformed-path-query-or-body",
            response.status == "404 Not Found"
        );
    }

    let malformed_nat_detect =
        crate::route_http_request("POST", "/api/v0/mesh/nat/detect/extra", None, "", &state)
            .await
            .expect("malformed versioned mesh NAT detection path response");
    record!(
        "POST",
        "/api/v0/mesh/nat/detect",
        "malformed-path-query-or-body",
        malformed_nat_detect.status == "404 Not Found"
    );

    // The frozen controller always returns a two-field JSON result for
    // a completed best-effort STUN probe. A no-response environment is
    // a normal unknown result, not an HTTP failure or an exception
    // shape; successful probes use one of the same four lower-case NAT
    // labels and set detected accordingly.
    let nat_detect = crate::route_http_request("POST", "/api/v0/mesh/nat/detect", None, "", &state)
        .await
        .expect("versioned mesh NAT detection response");
    let nat_detect_json =
        serde_json::from_str::<serde_json::Value>(&nat_detect.body).unwrap_or_default();
    let nat_detect_keys = nat_detect_json
        .as_object()
        .map(|object| object.keys().cloned().collect::<BTreeSet<_>>())
        .unwrap_or_default();
    let nat_type = nat_detect_json["type"].as_str().unwrap_or_default();
    let nat_detected = nat_detect_json["detected"].as_bool();
    record!(
        "POST",
        "/api/v0/mesh/nat/detect",
        "nominal-status-headers-body",
        nat_detect.status == "200 OK"
            && nat_detect.content_type.starts_with("application/json")
            && nat_detect_keys == BTreeSet::from(["detected".to_owned(), "type".to_owned()])
            && matches!(nat_type, "unknown" | "direct" | "symmetric" | "restricted")
            && nat_detected.is_some_and(|detected| detected == (nat_type != "unknown"))
    );

    let missing_lookup = crate::route_http_request(
        "GET",
        "/api/v0/mesh/lookup/edge-case-missing",
        None,
        "",
        &state,
    )
    .await
    .expect("missing mesh lookup response");
    let missing_lookup_json =
        serde_json::from_str::<serde_json::Value>(&missing_lookup.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/mesh/lookup/{flacKey}",
        "missing-empty-or-conflict-state",
        missing_lookup.status == "404 Not Found" && missing_lookup_json["found"] == false
    );

    let malformed_lookup =
        crate::route_http_request("GET", "/api/v0/mesh/lookup/", None, "", &state)
            .await
            .expect("malformed mesh lookup path response");
    record!(
        "GET",
        "/api/v0/mesh/lookup/{flacKey}",
        "malformed-path-query-or-body",
        malformed_lookup.status == "404 Not Found"
    );

    let empty_health = crate::route_http_request("GET", "/api/v0/mesh/health", None, "", &state)
        .await
        .expect("empty mesh health response");
    let empty_health_json =
        serde_json::from_str::<serde_json::Value>(&empty_health.body).unwrap_or_default();
    let empty_health_keys = empty_health_json
        .as_object()
        .map(|object| object.keys().cloned().collect::<BTreeSet<_>>())
        .unwrap_or_default();
    record!(
        "GET",
        "/api/v0/mesh/health",
        "missing-empty-or-conflict-state",
        empty_health.status == "200 OK"
            && empty_health.content_type == "application/json"
            && empty_health_keys
                == BTreeSet::from([
                    "contentPeerHints".to_owned(),
                    "generatedAt".to_owned(),
                    "routingNodes".to_owned(),
                    "storedKeys".to_owned(),
                ])
            && empty_health_json["routingNodes"] == 0
            && empty_health_json["storedKeys"] == 0
            && empty_health_json["contentPeerHints"] == 0
            && empty_health_json["generatedAt"].is_string()
    );

    let published_key = "edge-case-published";
    let published = crate::route_http_request(
        "POST",
        "/api/v0/mesh/publish",
        None,
        &serde_json::json!({
            "flacKey": published_key,
            "byteHash": "b".repeat(64),
            "size": 8192,
        })
        .to_string(),
        &state,
    )
    .await
    .expect("publish versioned mesh hash");
    let published_json =
        serde_json::from_str::<serde_json::Value>(&published.body).unwrap_or_default();
    let publish_pass = published.status == "200 OK" && published_json["published"] == true;
    record!(
        "POST",
        "/api/v0/mesh/publish",
        "nominal-status-headers-body",
        publish_pass
    );
    record!(
        "POST",
        "/api/v0/mesh/publish",
        "mutation-side-effects-and-readback",
        publish_pass
    );

    let lookup = crate::route_http_request(
        "GET",
        &format!("/api/v0/mesh/lookup/{published_key}"),
        None,
        "",
        &state,
    )
    .await
    .expect("read back versioned mesh hash");
    let lookup_json =
        serde_json::from_str::<serde_json::Value>(&lookup.body).unwrap_or(serde_json::Value::Null);
    let lookup_pass = lookup.status == "200 OK"
        && lookup_json["found"] == true
        && lookup_json["entry"]["flacKey"] == published_key
        && lookup_json["entry"]["size"] == 8192;
    record!(
        "GET",
        "/api/v0/mesh/lookup/{flacKey}",
        "nominal-status-headers-body",
        lookup_pass
    );
    record!(
        "GET",
        "/api/v0/mesh/lookup/{flacKey}",
        "populated-dynamic-state",
        lookup_pass
    );

    let health = crate::route_http_request("GET", "/api/v0/mesh/health", None, "", &state)
        .await
        .expect("populated versioned mesh health response");
    let health_json = serde_json::from_str::<serde_json::Value>(&health.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/mesh/health",
        "populated-dynamic-state",
        health.status == "200 OK"
            && health_json["storedKeys"]
                .as_u64()
                .is_some_and(|count| count >= 1)
    );

    state
        .mesh
        .write()
        .await
        .capability_records
        .push(test_capability_descriptor(
            "edge-case-peer",
            vec![slskr_client::capabilities::FEATURE_MESH_V1.to_owned()],
        ));
    let peers = crate::route_http_request("GET", "/api/v0/mesh/peers", None, "", &state)
        .await
        .expect("populated versioned mesh peers response");
    let peers_json = serde_json::from_str::<serde_json::Value>(&peers.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/mesh/peers",
        "populated-dynamic-state",
        peers.status == "200 OK"
            && peers_json["peers"]
                .as_array()
                .is_some_and(|rows| { rows.iter().any(|row| row["username"] == "edge-case-peer") })
    );

    let malformed_publish =
        crate::route_http_request("POST", "/api/v0/mesh/publish", None, "not-json", &state)
            .await
            .expect("malformed versioned mesh publish response");
    record!(
        "POST",
        "/api/v0/mesh/publish",
        "malformed-path-query-or-body",
        malformed_publish.status == "400 Bad Request"
    );

    let missing_publish =
        crate::route_http_request("POST", "/api/v0/mesh/publish", None, "{}", &state)
            .await
            .expect("missing versioned mesh publish fields response");
    record!(
        "POST",
        "/api/v0/mesh/publish",
        "missing-empty-or-conflict-state",
        missing_publish.status == "400 Bad Request" && missing_publish.body.contains("flacKey")
    );

    let valid_merge = serde_json::json!({
        "entries": [{
            "flacKey": "edge-case-merge",
            "byteHash": "c".repeat(64),
            "size": 4097,
        }]
    })
    .to_string();
    let malformed_merge = crate::route_http_request(
        "POST",
        "/api/v0/mesh/merge?fromUser=edge-case-peer",
        None,
        "not-json",
        &state,
    )
    .await
    .expect("malformed versioned mesh merge response");
    record!(
        "POST",
        "/api/v0/mesh/merge",
        "malformed-path-query-or-body",
        malformed_merge.status == "400 Bad Request"
    );

    let missing_merge =
        crate::route_http_request("POST", "/api/v0/mesh/merge", None, &valid_merge, &state)
            .await
            .expect("missing versioned mesh merge peer response");
    record!(
        "POST",
        "/api/v0/mesh/merge",
        "missing-empty-or-conflict-state",
        missing_merge.status == "400 Bad Request" && missing_merge.body.contains("fromUser")
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create mesh edge-case evidence directory");
    fs::write(
        evidence_dir.join("mesh_controller_edge_cases.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize mesh edge-case ledger"),
    )
    .expect("write mesh edge-case ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api mesh edge-case mismatches:\n{}",
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
    feature = "bounded-controller-api-tests-1"
))]
pub(super) async fn controller_api_differential_mesh_runtime_and_nat_lifecycle() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!("{target} {} {} [{}]", $method, $route, $case));
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

    let runtime_env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKR_PERSISTENCE_ENABLED", "true");
    let runtime_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("mesh runtime database");
    let (runtime_state, _runtime_receiver) = test_state_with_env_parts(
        runtime_env.clone(),
        crate::SearchStore::new(),
        Some(runtime_db.clone()),
    );
    runtime_db.close_for_test().await;

    for (path, route) in [
        ("/api/v0/mesh/delta", "/api/v0/mesh/delta"),
        ("/api/v0/mesh/health", "/api/v0/mesh/health"),
        ("/api/v0/mesh/hello", "/api/v0/mesh/hello"),
        (
            "/api/v0/mesh/lookup/mesh-runtime-missing",
            "/api/v0/mesh/lookup/{flacKey}",
        ),
        ("/api/v0/mesh/peers", "/api/v0/mesh/peers"),
        ("/api/v0/mesh/stats", "/api/v0/mesh/stats"),
        ("/api/v0/mesh/transport", "/api/v0/mesh/transport"),
    ] {
        let response = crate::route_http_request("GET", path, None, "", &runtime_state)
            .await
            .unwrap_or_else(|error| panic!("runtime mesh GET {path}: {error}"));
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            response.status == "200 OK"
                || (route == "/api/v0/mesh/lookup/{flacKey}"
                    && response.status == "404 Not Found"
                    && response.body.contains("\"found\":false"))
        );
    }

    let gateway_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("mesh gateway runtime database");
    let gateway_env = runtime_env
        .clone()
        .with("SLSKD_MESH_GATEWAY_ENABLED", "true")
        .with("SLSKD_MESH_GATEWAY_ALLOWED_SERVICES", "pods");
    let (gateway_state, _gateway_receiver) = test_state_with_env_parts(
        gateway_env,
        crate::SearchStore::new(),
        Some(gateway_db.clone()),
    );
    gateway_db.close_for_test().await;
    let runtime_services =
        crate::route_http_request("GET", "/mesh/http/services", None, "", &gateway_state)
            .await
            .expect("runtime mesh gateway services");
    let runtime_services_json =
        serde_json::from_str::<serde_json::Value>(&runtime_services.body).unwrap_or_default();
    record!(
        "GET",
        "/mesh/http/services",
        "runtime-failure-and-timeout",
        runtime_services.status == "200 OK" && runtime_services_json["gateway"]["enabled"] == true
    );

    let merge_body = serde_json::json!({
        "entries": [{
            "flacKey": "mesh-runtime-merge",
            "byteHash": "1".repeat(64),
            "size": 4096,
        }]
    })
    .to_string();
    let runtime_merge = crate::route_http_request(
        "POST",
        "/api/v0/mesh/merge?fromUser=mesh-runtime-peer",
        None,
        &merge_body,
        &runtime_state,
    )
    .await
    .expect("runtime mesh merge");
    record!(
        "POST",
        "/api/v0/mesh/merge",
        "runtime-failure-and-timeout",
        runtime_merge.status == "503 Service Unavailable"
            && runtime_state
                .content_discovery
                .read()
                .await
                .lookup_hash("mesh-runtime-merge")
                .is_none()
    );

    let runtime_publish = crate::route_http_request(
        "POST",
        "/api/v0/mesh/publish",
        None,
        &serde_json::json!({
            "flacKey": "mesh-runtime-publish",
            "byteHash": "2".repeat(64),
            "size": 4097,
        })
        .to_string(),
        &runtime_state,
    )
    .await
    .expect("runtime mesh publish");
    record!(
        "POST",
        "/api/v0/mesh/publish",
        "runtime-failure-and-timeout",
        runtime_publish.status == "503 Service Unavailable"
            && runtime_state
                .content_discovery
                .read()
                .await
                .lookup_hash("mesh-runtime-publish")
                .is_none()
    );

    let runtime_nat =
        crate::route_http_request("POST", "/api/v0/mesh/nat/detect", None, "", &runtime_state)
            .await
            .expect("runtime mesh NAT detection");
    let nat_shape = |response: &crate::HttpResponse| {
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        response.status == "200 OK"
            && value["type"].as_str().is_some_and(|value| {
                matches!(value, "unknown" | "direct" | "symmetric" | "restricted")
            })
            && value["detected"].is_boolean()
    };
    record!(
        "POST",
        "/api/v0/mesh/nat/detect",
        "runtime-failure-and-timeout",
        nat_shape(&runtime_nat)
    );

    let (nat_state, _nat_receiver) = test_state_with_env(runtime_env.clone());
    let nat_mutation =
        crate::route_http_request("POST", "/api/v0/mesh/nat/detect", None, "", &nat_state)
            .await
            .expect("mesh NAT mutation");
    record!(
        "POST",
        "/api/v0/mesh/nat/detect",
        "mutation-side-effects-and-readback",
        nat_shape(&nat_mutation)
    );

    let (nat_restart_state, _nat_restart_receiver) = test_state_with_env(runtime_env.clone());
    let nat_restart = crate::route_http_request(
        "POST",
        "/api/v0/mesh/nat/detect",
        None,
        "",
        &nat_restart_state,
    )
    .await
    .expect("mesh NAT restart");
    record!(
        "POST",
        "/api/v0/mesh/nat/detect",
        "restart-persistence-or-reset",
        nat_shape(&nat_restart)
    );

    let nat_concurrent = futures_util::future::join_all([
        crate::route_http_request("POST", "/api/v0/mesh/nat/detect", None, "", &nat_state),
        crate::route_http_request("POST", "/api/v0/mesh/nat/detect", None, "", &nat_state),
    ])
    .await;
    record!(
        "POST",
        "/api/v0/mesh/nat/detect",
        "concurrency-and-idempotency",
        nat_concurrent
            .iter()
            .all(|response| response.as_ref().is_ok_and(nat_shape))
    );

    let sync_shape = |response: &crate::HttpResponse| {
        response.status == "400 Bad Request"
            && response.body == r#"{"error":"Failed to sync with peer"}"#
    };
    let sync_mutation = crate::route_http_request(
        "POST",
        "/api/v0/mesh/sync/mesh-runtime-peer",
        None,
        "{}",
        &runtime_state,
    )
    .await
    .expect("runtime mesh sync mutation");
    record!(
        "POST",
        "/api/v0/mesh/sync/{username}",
        "mutation-side-effects-and-readback",
        sync_shape(&sync_mutation)
    );
    let (sync_restart_state, _sync_restart_receiver) = test_state_with_env(runtime_env);
    let sync_restart = crate::route_http_request(
        "POST",
        "/api/v0/mesh/sync/mesh-runtime-peer",
        None,
        "{}",
        &sync_restart_state,
    )
    .await
    .expect("runtime mesh sync restart");
    record!(
        "POST",
        "/api/v0/mesh/sync/{username}",
        "restart-persistence-or-reset",
        sync_shape(&sync_restart)
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create mesh runtime evidence directory");
    fs::write(
        evidence_dir.join("mesh_runtime_and_nat_lifecycle.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize mesh runtime evidence"),
    )
    .expect("write mesh runtime evidence");
    assert!(
        mismatches.is_empty(),
        "{} controller-api mesh runtime mismatches:\n{}",
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
    feature = "bounded-controller-api-tests-1"
))]
pub(super) async fn controller_api_differential_mesh_merge_publish_restart_and_concurrency() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {
            let pass = $pass;
            if !pass {
                mismatches.push(format!("{target} {} {} [{}]", $method, $route, $case));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": $method,
                "route": $route,
                "case": $case,
                "pass": pass,
            }));
        };
    }

    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));

    let idempotent_body = serde_json::json!({
        "entries": [{
            "flacKey": "mesh-idempotent-entry",
            "byteHash": "d".repeat(64),
            "size": 4096,
        }]
    })
    .to_string();
    let first_merge = crate::route_http_request(
        "POST",
        "/api/v0/mesh/merge?fromUser=mesh-idempotency-peer",
        None,
        &idempotent_body,
        &state,
    )
    .await
    .expect("first idempotent mesh merge");
    let second_merge = crate::route_http_request(
        "POST",
        "/api/v0/mesh/merge?fromUser=mesh-idempotency-peer",
        None,
        &idempotent_body,
        &state,
    )
    .await
    .expect("second idempotent mesh merge");
    let first_merge_json =
        serde_json::from_str::<serde_json::Value>(&first_merge.body).unwrap_or_default();
    let second_merge_json =
        serde_json::from_str::<serde_json::Value>(&second_merge.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/mesh/merge",
        "concurrency-and-idempotency",
        first_merge.status == "200 OK"
            && second_merge.status == "200 OK"
            && first_merge_json["merged"] == 1
            && second_merge_json["merged"] == 0
    );

    let merge_bodies = (0..6)
        .map(|index| {
            serde_json::json!({
                "entries": [{
                    "flacKey": format!("mesh-concurrent-entry-{index}"),
                    "byteHash": format!("{:064x}", index + 1),
                    "size": 5000 + index,
                }]
            })
            .to_string()
        })
        .collect::<Vec<_>>();
    let merge_responses = futures_util::future::join_all(merge_bodies.iter().map(|body| {
        crate::route_http_request(
            "POST",
            "/api/v0/mesh/merge?fromUser=mesh-concurrency-peer",
            None,
            body,
            &state,
        )
    }))
    .await;
    let concurrent_merges_ok = merge_responses.iter().all(|response| {
        response.as_ref().is_ok_and(|response| {
            response.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&response.body)
                    .is_ok_and(|value| value["merged"] == 1)
        })
    });
    let concurrent_lookups_ok = futures_util::future::join_all((0..6).map(|index| {
        let state = Arc::clone(&state);
        async move {
            let response = crate::route_http_request(
                "GET",
                &format!("/api/v0/mesh/lookup/mesh-concurrent-entry-{index}"),
                None,
                "",
                &state,
            )
            .await
            .expect("read concurrent mesh merge");
            let value =
                serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
            response.status == "200 OK" && value["found"] == true
        }
    }))
    .await
    .into_iter()
    .all(|pass| pass);
    record!(
        "POST",
        "/api/v0/mesh/merge",
        "concurrency-and-idempotency",
        concurrent_merges_ok && concurrent_lookups_ok
    );

    let publish_bodies = (0..6)
        .map(|index| {
            serde_json::json!({
                "flacKey": format!("mesh-publish-concurrent-{index}"),
                "byteHash": format!("{:064x}", index + 101),
                "size": 6000 + index,
            })
            .to_string()
        })
        .collect::<Vec<_>>();
    let publish_responses =
        futures_util::future::join_all(publish_bodies.iter().map(|body| {
            crate::route_http_request("POST", "/api/v0/mesh/publish", None, body, &state)
        }))
        .await;
    let publish_ok = publish_responses.iter().all(|response| {
        response.as_ref().is_ok_and(|response| {
            response.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&response.body)
                    .is_ok_and(|value| value["published"] == true)
        })
    });
    let published_lookups_ok = futures_util::future::join_all((0..6).map(|index| {
        let state = Arc::clone(&state);
        async move {
            let response = crate::route_http_request(
                "GET",
                &format!("/api/v0/mesh/lookup/mesh-publish-concurrent-{index}"),
                None,
                "",
                &state,
            )
            .await
            .expect("read concurrent mesh publish");
            let value =
                serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
            response.status == "200 OK" && value["found"] == true
        }
    }))
    .await
    .into_iter()
    .all(|pass| pass);
    record!(
        "POST",
        "/api/v0/mesh/publish",
        "concurrency-and-idempotency",
        publish_ok && published_lookups_ok
    );

    // Replace the test-only in-memory store with the same path-backed
    // store production startup loads. The route mutation must then
    // survive a fresh store load, not merely remain in the live lock.
    let state_dir = state.config.state_dir.clone();
    let persistent_store = crate::content_discovery::ContentDiscoveryStore::load(&state_dir)
        .expect("create path-backed mesh discovery store");
    *state.content_discovery.write().await = persistent_store;
    let restart_merge = crate::route_http_request(
        "POST",
        "/api/v0/mesh/merge?fromUser=mesh-restart-peer",
        None,
        r#"{"entries":[{"flacKey":"mesh-restart-merge","byteHash":"eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee","size":7001}]} "#,
        &state,
    )
    .await
    .expect("persist mesh merge");
    let restart_publish = crate::route_http_request(
        "POST",
        "/api/v0/mesh/publish",
        None,
        r#"{"flacKey":"mesh-restart-publish","byteHash":"ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff","size":7002}"#,
        &state,
    )
    .await
    .expect("persist mesh publish");
    let reloaded = crate::content_discovery::ContentDiscoveryStore::load(&state_dir)
        .expect("reload path-backed mesh discovery store");
    let restart_merge_json =
        serde_json::from_str::<serde_json::Value>(&restart_merge.body).unwrap_or_default();
    let restart_publish_json =
        serde_json::from_str::<serde_json::Value>(&restart_publish.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/mesh/merge",
        "restart-persistence-or-reset",
        restart_merge.status == "200 OK"
            && restart_merge_json["merged"] == 1
            && reloaded.lookup_hash("mesh-restart-merge").is_some()
    );
    record!(
        "POST",
        "/api/v0/mesh/publish",
        "restart-persistence-or-reset",
        restart_publish.status == "200 OK"
            && restart_publish_json["published"] == true
            && reloaded.lookup_hash("mesh-restart-publish").is_some()
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create mesh merge/publish evidence directory");
    fs::write(
        evidence_dir.join("mesh_merge_publish_restart_concurrency.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize mesh merge/publish evidence"),
    )
    .expect("write mesh merge/publish evidence");
    assert!(
        mismatches.is_empty(),
        "{} controller-api mesh merge/publish mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
    fs::remove_dir_all(state_dir).expect("remove mesh merge/publish test state directory");
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
pub(super) async fn controller_api_differential_mesh_sync_failure_and_concurrency_contracts() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($case:expr, $pass:expr) => {
            let pass = $pass;
            if !pass {
                mismatches.push(format!(
                    "{target} POST /api/v0/mesh/sync/{{username}} [{}]",
                    $case
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "POST",
                "route": "/api/v0/mesh/sync/{username}",
                "case": $case,
                "pass": pass,
            }));
        };
    }

    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let route = "/api/v0/mesh/sync/mesh-unsupported-peer";
    let response = crate::route_http_request("POST", route, None, "{}", &state)
        .await
        .expect("unsupported mesh sync response");
    let expected_body = r#"{"error":"Failed to sync with peer"}"#;
    let failure_shape = response.status == "400 Bad Request"
        && response.content_type == "application/json"
        && response.body == expected_body;
    record!("nominal-status-headers-body", failure_shape);
    record!("missing-empty-or-conflict-state", failure_shape);

    // A capability-present peer reaches the frozen service's next
    // failure boundary: the pinned build has no usable mesh transport
    // and still returns the same generic controller error.
    state
        .mesh
        .write()
        .await
        .capability_records
        .push(test_capability_descriptor(
            "mesh-transport-unavailable-peer",
            vec![slskr_client::capabilities::FEATURE_MESH_V1.to_owned()],
        ));
    let transport_failure = crate::route_http_request(
        "POST",
        "/api/v0/mesh/sync/mesh-transport-unavailable-peer",
        None,
        "{}",
        &state,
    )
    .await
    .expect("mesh transport failure response");
    record!(
        "runtime-failure-and-timeout",
        transport_failure.status == "400 Bad Request"
            && transport_failure.content_type == "application/json"
            && transport_failure.body == expected_body
    );

    let concurrent = futures_util::future::join_all((0..6).map(|_| {
        let state = Arc::clone(&state);
        async move {
            crate::route_http_request(
                "POST",
                "/api/v0/mesh/sync/mesh-unsupported-peer",
                None,
                "{}",
                &state,
            )
            .await
        }
    }))
    .await;
    record!(
        "concurrency-and-idempotency",
        concurrent.iter().all(|response| {
            response.as_ref().is_ok_and(|response| {
                response.status == "400 Bad Request"
                    && response.content_type == "application/json"
                    && response.body == expected_body
            })
        })
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create mesh sync evidence directory");
    fs::write(
        evidence_dir.join("mesh_sync_failure_and_concurrency.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize mesh sync evidence"),
    )
    .expect("write mesh sync evidence");
    assert!(
        mismatches.is_empty(),
        "{} controller-api mesh sync mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
