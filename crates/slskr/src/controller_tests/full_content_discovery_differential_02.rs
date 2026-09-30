//! Controller full content discovery differential 02 ownership.

use super::*;

/// Differential proof for the remaining versioned HashDb controller
/// cases.  The fixture uses the frozen verification request shape for
/// local stores, the durable HashDb/HashDbState projection for restart
/// checks, and a closed SQLite pool for every runtime-failure case.
/// slskdN-only (confirmed against the frozen controller registry).
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
pub(super) async fn controller_api_differential_hashdb_domain_contracts() {
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

    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("HashDb controller contract database");
    let env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKR_SHARE_FIXTURE", "")
        .with("SLSKR_PERSISTENCE_ENABLED", "true");
    let (state, _receiver) =
        test_state_with_env_parts(env.clone(), crate::SearchStore::new(), Some(db.clone()));

    let filename = "Contracts/Versioned.flac";
    let size = 4_096_u64;
    let byte_hash = "a".repeat(64);
    let hash_key = crate::content_discovery::generate_flac_key(filename, size);

    let malformed_store =
        crate::route_http_request("POST", "/api/v0/hashdb/hash", None, "not-json", &state)
            .await
            .expect("malformed HashDb store");
    record!(
        "POST",
        "/api/v0/hashdb/hash",
        "malformed-path-query-or-body",
        malformed_store.status == "400 Bad Request"
    );
    let missing_store =
        crate::route_http_request("POST", "/api/v0/hashdb/hash", None, "{}", &state)
            .await
            .expect("missing HashDb store fields");
    record!(
        "POST",
        "/api/v0/hashdb/hash",
        "missing-empty-or-conflict-state",
        missing_store.status == "400 Bad Request"
    );
    let store_body = serde_json::json!({
        "filename": filename,
        "size": size,
        "byteHash": byte_hash,
        "sampleRate": 44100,
        "channels": 2,
        "bitDepth": 16,
    })
    .to_string();
    let stored =
        crate::route_http_request("POST", "/api/v0/hashdb/hash", None, &store_body, &state)
            .await
            .expect("store frozen HashDb verification request");
    let stored_json =
        serde_json::from_str::<serde_json::Value>(&stored.body).unwrap_or(serde_json::Value::Null);
    record!(
        "POST",
        "/api/v0/hashdb/hash",
        "nominal-status-headers-body",
        stored.status == "200 OK" && stored_json == serde_json::json!({"stored": true})
    );
    let lookup = crate::route_http_request(
        "GET",
        &format!("/api/v0/hashdb/hash/{hash_key}"),
        None,
        "",
        &state,
    )
    .await
    .expect("read stored HashDb verification entry");
    let lookup_json =
        serde_json::from_str::<serde_json::Value>(&lookup.body).unwrap_or(serde_json::Value::Null);
    record!(
        "GET",
        "/api/v0/hashdb/hash/{flacKey}",
        "nominal-status-headers-body",
        lookup.status == "200 OK"
            && lookup_json["flacKey"] == hash_key
            && lookup_json["byteHash"] == "a".repeat(64)
            && lookup_json["size"] == size
    );
    record!(
        "POST",
        "/api/v0/hashdb/hash",
        "mutation-side-effects-and-readback",
        lookup.status == "200 OK" && lookup_json["flacKey"] == hash_key
    );

    let inventory_key =
        crate::hashdb_flac_inventory_key("contract-peer", "Inventory/Track.flac", size);
    state
        .controller_features
        .write_for_test()
        .await
        .upsert(
            inventory_key,
            crate::hashdb_flac_inventory_record("contract-peer", "Inventory/Track.flac", size),
        )
        .expect("seed HashDb inventory projection");
    let inventory_by_size = crate::route_http_request(
        "GET",
        &format!("/api/v0/hashdb/inventory/by-size/{size}"),
        None,
        "",
        &state,
    )
    .await
    .expect("read HashDb inventory by size");
    let inventory_by_size_json = serde_json::from_str::<serde_json::Value>(&inventory_by_size.body)
        .unwrap_or(serde_json::Value::Null);
    record!(
        "GET",
        "/api/v0/hashdb/inventory/by-size/{size}",
        "populated-dynamic-state",
        inventory_by_size.status == "200 OK"
            && inventory_by_size_json["count"] == 1
            && inventory_by_size_json["entries"][0]["path"] == "Inventory/Track.flac"
    );
    let unhashed =
        crate::route_http_request("GET", "/api/v0/hashdb/inventory/unhashed", None, "", &state)
            .await
            .expect("read unhashed HashDb inventory");
    let unhashed_json = serde_json::from_str::<serde_json::Value>(&unhashed.body)
        .unwrap_or(serde_json::Value::Null);
    record!(
        "GET",
        "/api/v0/hashdb/inventory/unhashed",
        "populated-dynamic-state",
        unhashed.status == "200 OK"
            && unhashed_json["count"] == 1
            && unhashed_json["entries"][0]["path"] == "Inventory/Track.flac"
    );
    let generated_key = crate::route_http_request(
        "GET",
        &format!(
            "/api/v0/hashdb/key?filename={}&size={size}",
            crate::url_encode(filename)
        ),
        None,
        "",
        &state,
    )
    .await
    .expect("generate populated HashDb key");
    let generated_key_json = serde_json::from_str::<serde_json::Value>(&generated_key.body)
        .unwrap_or(serde_json::Value::Null);
    record!(
        "GET",
        "/api/v0/hashdb/key",
        "populated-dynamic-state",
        generated_key.status == "200 OK" && generated_key_json["flacKey"] == hash_key
    );
    let schema = crate::route_http_request("GET", "/api/v0/hashdb/schema", None, "", &state)
        .await
        .expect("read populated HashDb schema");
    let schema_json =
        serde_json::from_str::<serde_json::Value>(&schema.body).unwrap_or(serde_json::Value::Null);
    record!(
        "GET",
        "/api/v0/hashdb/schema",
        "populated-dynamic-state",
        schema.status == "200 OK"
            && schema_json["currentVersion"] == crate::HASHDB_SCHEMA_VERSION
            && schema_json["targetVersion"] == crate::HASHDB_SCHEMA_VERSION
            && schema_json["isUpToDate"] == true
    );

    let sync_body = serde_json::json!({
        "entries": [{
            "flacKey": "contract-sync-key",
            "byteHash": "b".repeat(64),
            "size": 4097,
        }]
    })
    .to_string();
    let malformed_sync = crate::route_http_request(
        "POST",
        "/api/v0/hashdb/sync/merge",
        None,
        "not-json",
        &state,
    )
    .await
    .expect("malformed HashDb sync");
    record!(
        "POST",
        "/api/v0/hashdb/sync/merge",
        "malformed-path-query-or-body",
        malformed_sync.status == "400 Bad Request"
    );
    let missing_sync =
        crate::route_http_request("POST", "/api/v0/hashdb/sync/merge", None, "{}", &state)
            .await
            .expect("missing HashDb sync entries");
    record!(
        "POST",
        "/api/v0/hashdb/sync/merge",
        "missing-empty-or-conflict-state",
        missing_sync.status == "400 Bad Request"
    );
    let merged = crate::route_http_request(
        "POST",
        "/api/v0/hashdb/sync/merge",
        None,
        &sync_body,
        &state,
    )
    .await
    .expect("merge HashDb sync entries");
    let merged_json =
        serde_json::from_str::<serde_json::Value>(&merged.body).unwrap_or(serde_json::Value::Null);
    record!(
        "POST",
        "/api/v0/hashdb/sync/merge",
        "nominal-status-headers-body",
        merged.status == "200 OK" && merged_json["merged"] == 1
    );
    let sync_read = crate::route_http_request(
        "GET",
        "/api/v0/hashdb/hash/contract-sync-key",
        None,
        "",
        &state,
    )
    .await
    .expect("read merged HashDb sync entry");
    record!(
        "POST",
        "/api/v0/hashdb/sync/merge",
        "mutation-side-effects-and-readback",
        sync_read.status == "200 OK" && sync_read.body.contains("contract-sync-key")
    );
    let idempotent = crate::route_http_request(
        "POST",
        "/api/v0/hashdb/sync/merge",
        None,
        &sync_body,
        &state,
    )
    .await
    .expect("idempotent HashDb sync merge");
    let idempotent_json = serde_json::from_str::<serde_json::Value>(&idempotent.body)
        .unwrap_or(serde_json::Value::Null);
    record!(
        "POST",
        "/api/v0/hashdb/sync/merge",
        "concurrency-and-idempotency",
        idempotent.status == "200 OK" && idempotent_json["merged"] == 0
    );

    let persisted_entries = db
        .list_hash_db_entries()
        .await
        .expect("list persisted HashDb contract entries")
        .into_iter()
        .map(crate::hash_db_entry_from_persistence)
        .collect::<Result<Vec<_>, _>>()
        .expect("convert persisted HashDb contract entries");
    let latest_seq = db
        .get_hash_db_state("latest_seq")
        .await
        .expect("read persisted HashDb cursor")
        .and_then(|record| record.value)
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or_default();
    let (restarted_state, _restarted_receiver) =
        test_state_with_env_parts(env.clone(), crate::SearchStore::new(), Some(db.clone()));
    restarted_state
        .content_discovery
        .write()
        .await
        .restore_hash_entries(persisted_entries, latest_seq)
        .expect("rehydrate HashDb contract state");
    let restarted_lookup = crate::route_http_request(
        "GET",
        "/api/v0/hashdb/hash/contract-sync-key",
        None,
        "",
        &restarted_state,
    )
    .await
    .expect("read restarted HashDb sync entry");
    record!(
        "POST",
        "/api/v0/hashdb/sync/merge",
        "restart-persistence-or-reset",
        restarted_lookup.status == "200 OK" && restarted_lookup.body.contains("contract-sync-key")
    );
    let restarted_hash = crate::route_http_request(
        "GET",
        &format!("/api/v0/hashdb/hash/{hash_key}"),
        None,
        "",
        &restarted_state,
    )
    .await
    .expect("read restarted locally stored HashDb entry");
    record!(
        "POST",
        "/api/v0/hashdb/hash",
        "restart-persistence-or-reset",
        restarted_hash.status == "200 OK" && restarted_hash.body.contains(&hash_key)
    );
    let concurrent_results = futures_util::future::join_all((0..4).map(|index| {
        let path = "/api/v0/hashdb/hash";
        let body = serde_json::json!({
            "filename": format!("Concurrent/Track-{index}.flac"),
            "size": 8_000 + index,
            "byteHash": format!("{:064x}", index + 10),
        })
        .to_string();
        let state = state.clone();
        async move { crate::route_http_request("POST", path, None, &body, &state).await }
    }))
    .await;
    record!(
        "POST",
        "/api/v0/hashdb/hash",
        "concurrency-and-idempotency",
        concurrent_results.iter().all(|result| result
            .as_ref()
            .is_ok_and(|response| response.status == "200 OK"))
    );

    let optimize_state = restarted_state.clone();
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
        let response = crate::route_http_request("POST", path, None, "", &optimize_state)
            .await
            .expect("HashDb optimize mutation");
        let value = serde_json::from_str::<serde_json::Value>(&response.body)
            .unwrap_or(serde_json::Value::Null);
        record!(
            "POST",
            path,
            "nominal-status-headers-body",
            response.status == "200 OK" && value["message"] == message
        );
        let repeated = crate::route_http_request("POST", path, None, "", &optimize_state)
            .await
            .expect("repeated HashDb optimize mutation");
        record!(
            "POST",
            path,
            "mutation-side-effects-and-readback",
            repeated.status == "200 OK"
        );
        let empty_db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("empty optimizer database");
        let (empty_state, _empty_receiver) =
            test_state_with_env_parts(env.clone(), crate::SearchStore::new(), Some(empty_db));
        let empty = crate::route_http_request("POST", path, None, "", &empty_state)
            .await
            .expect("empty HashDb optimize mutation");
        record!(
            "POST",
            path,
            "missing-empty-or-conflict-state",
            empty.status == "200 OK"
        );
        let concurrent = futures_util::future::join_all(
            (0..3).map(|_| crate::route_http_request("POST", path, None, "", &optimize_state)),
        )
        .await;
        record!(
            "POST",
            path,
            "concurrency-and-idempotency",
            concurrent.iter().all(|result| result
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK"))
        );
        let restart = crate::route_http_request("POST", path, None, "", &restarted_state)
            .await
            .expect("restarted HashDb optimize mutation");
        record!(
            "POST",
            path,
            "restart-persistence-or-reset",
            restart.status == "200 OK"
        );
    }

    let profile_body = r#"{"query":"SELECT * FROM hash_entries","parameters":{}}"#;
    let profile = crate::route_http_request(
        "POST",
        "/api/v0/hashdb/optimize/profile",
        None,
        profile_body,
        &optimize_state,
    )
    .await
    .expect("profile HashDb query");
    record!(
        "POST",
        "/api/v0/hashdb/optimize/profile",
        "mutation-side-effects-and-readback",
        profile.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&profile.body)
                .unwrap_or(serde_json::Value::Null)["query"]
                == "SELECT * FROM hash_entries"
    );
    let profile_restart = crate::route_http_request(
        "POST",
        "/api/v0/hashdb/optimize/profile",
        None,
        profile_body,
        &restarted_state,
    )
    .await
    .expect("profile restarted HashDb query");
    record!(
        "POST",
        "/api/v0/hashdb/optimize/profile",
        "restart-persistence-or-reset",
        profile_restart.status == "200 OK"
    );
    let profile_concurrent = futures_util::future::join_all((0..3).map(|_| {
        crate::route_http_request(
            "POST",
            "/api/v0/hashdb/optimize/profile",
            None,
            profile_body,
            &optimize_state,
        )
    }))
    .await;
    record!(
        "POST",
        "/api/v0/hashdb/optimize/profile",
        "concurrency-and-idempotency",
        profile_concurrent.iter().all(|result| result
            .as_ref()
            .is_ok_and(|response| response.status == "200 OK"))
    );

    let failure_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("HashDb runtime failure database");
    let (failure_state, _failure_receiver) =
        test_state_with_env_parts(env, crate::SearchStore::new(), Some(failure_db.clone()));
    failure_db.close_for_test().await;
    for (path, expected_status) in [
        (
            "/api/v0/hashdb/backfill/candidates",
            "500 Internal Server Error",
        ),
        ("/api/v0/hashdb/entries", "500 Internal Server Error"),
        (
            "/api/v0/hashdb/hash/runtime-key",
            "500 Internal Server Error",
        ),
        (
            "/api/v0/hashdb/hash/by-size/4096",
            "500 Internal Server Error",
        ),
        (
            "/api/v0/hashdb/inventory/by-size/4096",
            "500 Internal Server Error",
        ),
        (
            "/api/v0/hashdb/inventory/unhashed",
            "500 Internal Server Error",
        ),
        (
            "/api/v0/hashdb/optimize/analyze",
            "500 Internal Server Error",
        ),
        (
            "/api/v0/hashdb/optimize/slow-queries",
            "500 Internal Server Error",
        ),
        ("/api/v0/hashdb/peers", "500 Internal Server Error"),
        ("/api/v0/hashdb/schema", "500 Internal Server Error"),
        ("/api/v0/hashdb/stats", "500 Internal Server Error"),
        ("/api/v0/hashdb/sync/since/0", "500 Internal Server Error"),
    ] {
        let response = crate::route_http_request("GET", path, None, "", &failure_state)
            .await
            .expect("HashDb read failure response");
        record!(
            "GET",
            match path {
                "/api/v0/hashdb/backfill/candidates" => {
                    "/api/v0/hashdb/backfill/candidates"
                }
                "/api/v0/hashdb/entries" => "/api/v0/hashdb/entries",
                "/api/v0/hashdb/hash/runtime-key" => "/api/v0/hashdb/hash/{flacKey}",
                "/api/v0/hashdb/hash/by-size/4096" => "/api/v0/hashdb/hash/by-size/{size}",
                "/api/v0/hashdb/inventory/by-size/4096" => {
                    "/api/v0/hashdb/inventory/by-size/{size}"
                }
                "/api/v0/hashdb/inventory/unhashed" => "/api/v0/hashdb/inventory/unhashed",
                "/api/v0/hashdb/optimize/analyze" => "/api/v0/hashdb/optimize/analyze",
                "/api/v0/hashdb/optimize/slow-queries" => {
                    "/api/v0/hashdb/optimize/slow-queries"
                }
                "/api/v0/hashdb/peers" => "/api/v0/hashdb/peers",
                "/api/v0/hashdb/schema" => "/api/v0/hashdb/schema",
                "/api/v0/hashdb/stats" => "/api/v0/hashdb/stats",
                "/api/v0/hashdb/sync/since/0" => "/api/v0/hashdb/sync/since/{sinceSeq}",
                _ => path,
            },
            "runtime-failure-and-timeout",
            response.status == expected_status
                && response.body.contains("hash database storage unavailable")
        );
    }
    let key_failure = crate::route_http_request(
        "GET",
        "/api/v0/hashdb/key?filename=Runtime.flac&size=4096",
        None,
        "",
        &failure_state,
    )
    .await
    .expect("HashDb key remains available during storage failure");
    record!(
        "GET",
        "/api/v0/hashdb/key",
        "runtime-failure-and-timeout",
        key_failure.status == "200 OK"
    );

    for (path, body, route) in [
        (
            "/api/v0/hashdb/hash",
            store_body.as_str(),
            "/api/v0/hashdb/hash",
        ),
        (
            "/api/v0/hashdb/sync/merge",
            sync_body.as_str(),
            "/api/v0/hashdb/sync/merge",
        ),
        (
            "/api/v0/hashdb/backfill/from-history",
            "",
            "/api/v0/hashdb/backfill/from-history",
        ),
        (
            "/api/v0/hashdb/optimize/indexes",
            "",
            "/api/v0/hashdb/optimize/indexes",
        ),
        (
            "/api/v0/hashdb/optimize/vacuum",
            "",
            "/api/v0/hashdb/optimize/vacuum",
        ),
        (
            "/api/v0/hashdb/optimize/profile",
            profile_body,
            "/api/v0/hashdb/optimize/profile",
        ),
    ] {
        let response = crate::route_http_request("POST", path, None, body, &failure_state)
            .await
            .expect("HashDb write failure response");
        record!(
            "POST",
            route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("hash database storage unavailable")
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create HashDb controller evidence directory");
    fs::write(
        evidence_dir.join("hashdb_domain_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize HashDb controller contract ledger"),
    )
    .expect("write HashDb controller contract ledger");
    assert!(
        mismatches.is_empty(),
        "{} HashDb controller contract mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
