//! Controller full content discovery contracts ownership.

use super::*;

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn hashdb_shadow_index_uses_operator_trusted_frozen_peer_for_mesh_swarm() {
    use sha2::{Digest, Sha256};

    let content = Arc::new(b"verified mesh source discovery".to_vec());
    let expected_hash = hex::encode(Sha256::digest(content.as_slice()));
    let (source_a, task_a) = spawn_mesh_range_source(Arc::clone(&content)).await;
    let (source_b, task_b) = spawn_mesh_range_source(Arc::clone(&content)).await;
    let trusted_peers = serde_json::json!([{
        "peerId": "peer-a",
        "username": "source-a",
        "overlayEndpoint": "127.0.0.1:50305",
        "certificateSha256": "11".repeat(32),
        "rangeEndpoint": format!("http://{source_a}/content")
    }]);
    let (state, _receiver) = test_state_with_env(
        MapEnv::default().with("SLSKR_TRUSTED_MESH_PEERS", &trusted_peers.to_string()),
    );

    let hash_merge = crate::route_http_request(
        "POST",
        "/api/v0/hashdb/sync/merge",
        None,
        &format!(
            r#"{{"entries":[{{"flacKey":"track-key","size":{},"fileSha256":"{}","musicBrainzId":"recording-1"}}]}}"#,
            content.len(),
            expected_hash.to_ascii_uppercase()
        ),
        &state,
    )
    .await
    .expect("merge hash metadata");
    assert_eq!(hash_merge.status, "200 OK");

    let shadow_merge = crate::route_http_request(
        "POST",
        "/api/v0/virtualsoulfind/shadow-index/sync/merge",
        None,
        r#"{"records":[{"recordingId":"recording-1","peerIds":["peer-a","peer-b"]}]}"#,
        &state,
    )
    .await
    .expect("merge shadow index");
    assert_eq!(shadow_merge.status, "200 OK");

    {
        let mut mesh = state.mesh.write().await;
        let mut descriptor = test_capability_descriptor(
            "source-b",
            vec![slskr_client::capabilities::FEATURE_MESH_V1.to_owned()],
        );
        descriptor.peer_id = "peer-b".to_owned();
        descriptor.endpoints = vec![format!("http://{source_b}/content")];
        mesh.capability_records.push(descriptor);
    }

    let by_size = crate::route_http_request(
        "GET",
        &format!("/api/v0/hashdb/hash/by-size/{}", content.len()),
        None,
        "",
        &state,
    )
    .await
    .expect("lookup hashes by size");
    let by_size_json = serde_json::from_str::<serde_json::Value>(&by_size.body).unwrap();
    assert_eq!(by_size_json["count"], 1);
    assert_eq!(by_size_json["entries"][0]["musicBrainzId"], "recording-1");

    let output_path = format!("mesh-discovery/{}.flac", uuid::Uuid::new_v4());
    let swarm = crate::route_http_request(
        "POST",
        "/api/v0/multisource/swarm",
        None,
        &format!(
            r#"{{"filename":"Track.flac","fileSize":{},"expectedHash":"{}","outputPath":"{}","sources":[]}}"#,
            content.len(),
            expected_hash.to_ascii_uppercase(),
            output_path
        ),
        &state,
    )
    .await
    .expect("execute discovered mesh swarm");
    let swarm_json = serde_json::from_str::<serde_json::Value>(&swarm.body).unwrap();
    assert_eq!(swarm.status, "200 OK", "{}", swarm.body);
    assert_eq!(swarm_json["success"], true, "{}", swarm.body);
    assert_eq!(
        std::fs::read(state.config.downloads_dir.join(&output_path))
            .expect("read verified mesh output"),
        content.as_slice()
    );

    task_a.abort();
    task_b.abort();
    std::fs::remove_dir_all(&state.config.state_dir).expect("remove test state directory");
}
#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn hashdb_key_matches_frozen_dto_and_positive_size_validation() {
    let (state, _receiver) = test_state();
    let response = crate::route_http_request(
        "GET",
        "/api/v0/hashdb/key?filename=Track.flac&size=123",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(response.status, "200 OK");
    let json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
    assert!(json["flacKey"].is_string(), "{json}");
    assert!(json.get("key").is_none());
    assert!(json.get("filename").is_none());
    assert!(json.get("size").is_none());

    let zero = crate::route_http_request(
        "GET",
        "/api/v0/hashdb/key?filename=Track.flac&size=0",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(zero.status, "400 Bad Request");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn hashdb_history_backfill_batches_persists_inventory_and_progress() {
    let (state, _receiver) = test_state();
    {
        let mut searches = state.searches.write().await;
        for index in 1..=11_u64 {
            searches.records.push(crate::SearchRecord {
                id: format!("history-{index}"),
                token: u32::try_from(index).unwrap(),
                query: format!("history {index}"),
                target: "global",
                target_name: None,
                status: "completed",
                results: vec![crate::SearchResultEntry {
                    peer_username: Some(format!("peer-{index}")),
                    filename: format!("Library/Track-{index}.flac"),
                    size: 32_768 + index,
                    extension: "flac".to_owned(),
                    bit_rate: None,
                    sample_rate: None,
                    bit_depth: None,
                    length_seconds: None,
                    locked: false,
                    slot_free: Some(true),
                    average_speed: Some(1_000),
                    queue_length: Some(0),
                }],
                raw_response_count: 1,
                filtered_out_count: 0,
                ignored_result_count: 0,
                hidden_locked_count: 0,
                fallback_attempts: 0,
                ttl_seconds: crate::DEFAULT_SEARCH_TTL_SECONDS,
                expires_at: 0,
                created_at: index,
                updated_at: index,
            });
        }
    }

    let first = crate::route_http_request(
        "POST",
        "/api/v0/hashdb/backfill/from-history?batchSize=10",
        None,
        "",
        &state,
    )
    .await
    .expect("first history backfill batch");
    assert_eq!(first.status, "200 OK");
    let first_json = serde_json::from_str::<serde_json::Value>(&first.body).unwrap();
    assert_eq!(first_json["searchesProcessed"], 10);
    assert_eq!(first_json["flacsDiscovered"], 10);
    assert_eq!(first_json["totalSearches"], 11);
    assert_eq!(first_json["remainingSearches"], 1);
    assert_eq!(first_json["complete"], false);

    let candidates = crate::route_http_request(
        "GET",
        "/api/v0/hashdb/backfill/candidates?limit=20",
        None,
        "",
        &state,
    )
    .await
    .expect("hashdb backfill candidates");
    let candidates_json = serde_json::from_str::<serde_json::Value>(&candidates.body).unwrap();
    assert_eq!(candidates_json["count"], 10, "{candidates_json}");
    assert_eq!(
        candidates_json["entries"].as_array().unwrap().len(),
        10,
        "{candidates_json}"
    );

    let second = crate::route_http_request(
        "POST",
        "/api/v0/hashdb/backfill/from-history?batchSize=10",
        None,
        "",
        &state,
    )
    .await
    .expect("second history backfill batch");
    let second_json = serde_json::from_str::<serde_json::Value>(&second.body).unwrap();
    assert_eq!(second_json["searchesProcessed"], 1);
    assert_eq!(second_json["flacsDiscovered"], 1);
    assert_eq!(second_json["remainingSearches"], 0);
    assert_eq!(second_json["complete"], true);

    let complete = crate::route_http_request(
        "POST",
        "/api/v0/hashdb/backfill/from-history",
        None,
        "",
        &state,
    )
    .await
    .expect("completed history backfill");
    let complete_json = serde_json::from_str::<serde_json::Value>(&complete.body).unwrap();
    assert_eq!(complete_json["searchesProcessed"], 0);
    assert_eq!(complete_json["flacsDiscovered"], 0);
    assert_eq!(complete_json["complete"], true);

    let reset = crate::route_http_request(
        "POST",
        "/api/v0/hashdb/backfill/from-history?reset=true",
        None,
        "",
        &state,
    )
    .await
    .expect("reset history backfill");
    let reset_json = serde_json::from_str::<serde_json::Value>(&reset.body).unwrap();
    assert_eq!(reset_json["searchesProcessed"], 11);
    assert_eq!(reset_json["flacsDiscovered"], 11);
    assert_eq!(reset_json["complete"], true);

    let stats = crate::route_http_request("GET", "/api/v0/hashdb/stats", None, "", &state)
        .await
        .expect("hashdb stats after backfill");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap();
    assert_eq!(stats_json["totalFlacEntries"], 11);
    assert_eq!(stats_json["hashedFlacEntries"], 0);

    let analysis =
        crate::route_http_request("GET", "/api/v0/hashdb/optimize/analyze", None, "", &state)
            .await
            .expect("hashdb optimize analysis after backfill");
    let analysis_json = serde_json::from_str::<serde_json::Value>(&analysis.body).unwrap();
    assert_eq!(analysis_json["flacInventoryEntryCount"], 11);
    assert_eq!(analysis_json["peerCount"], 11);

    let peers = crate::route_http_request("GET", "/api/v0/hashdb/peers", None, "", &state)
        .await
        .expect("hashdb peers after backfill");
    let peers_json = serde_json::from_str::<serde_json::Value>(&peers.body).unwrap();
    assert_eq!(peers_json["count"], 0);

    let compatibility_peers =
        crate::route_http_request("GET", "/api/hashdb/peers", None, "", &state)
            .await
            .expect("compatibility hashdb peers after backfill");
    let compatibility_peers_json =
        serde_json::from_str::<serde_json::Value>(&compatibility_peers.body).unwrap();
    assert_eq!(compatibility_peers_json["count"], 11);
}
