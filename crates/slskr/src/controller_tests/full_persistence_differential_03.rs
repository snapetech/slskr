//! Controller full persistence differential 03 ownership.

use super::*;

/// Differential lifecycle proof for the frozen slskdN SongID run store.
/// slskR keeps the bounded run payloads in the durable runtime-compat
/// record so the public SongID routes retain their state across a restart.
/// The frozen store exposes upsert/read/list operations only; it has no
/// delete or corruption-repair contract.
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
pub(super) async fn persistence_lifecycle_differential_native_songid_runs() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!("{target} songid_runs {}", $case));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "domain": "songid_runs",
                "case": $case,
                "pass": pass,
            }));
        }};
    }

    let schema_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("SongID schema database");
    record!(
        "schema-create-and-migrate",
        schema_db.get_runtime_compat_state().await.is_ok()
    );

    let restart_path = std::env::temp_dir().join(format!(
        "slskr-songid-runs-restart-{}-{}.sqlite",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    let restart_db = crate::persistence::DatabaseManager::new(
        restart_path.to_str().expect("SongID restart path"),
    )
    .await
    .expect("create SongID restart database");
    let env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKR_PERSISTENCE_ENABLED", "true");
    let (state, _receiver) =
        test_state_with_env_parts(env, crate::SearchStore::new(), Some(restart_db.clone()));
    let create = crate::route_http_request(
        "POST",
        "/api/v0/songid/runs",
        None,
        r#"{"source":"persistence-songid-query"}"#,
        &state,
    )
    .await
    .expect("create SongID run");
    let run = serde_json::from_str::<serde_json::Value>(&create.body)
        .expect("SongID create response JSON");
    let run_id = run["id"].as_str().unwrap_or_default().to_owned();
    let persisted = restart_db
        .get_runtime_compat_state()
        .await
        .expect("read persisted SongID runtime state")
        .expect("persisted SongID runtime state");
    let persisted_runs =
        serde_json::from_str::<Vec<serde_json::Value>>(&persisted.songid_run_records_json)
            .expect("persisted SongID run payload");
    let read = crate::route_http_request(
        "GET",
        &format!("/api/v0/songid/runs/{run_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("read SongID run");
    record!(
        "create-and-read-roundtrip",
        create.status == "202 Accepted"
            && !run_id.is_empty()
            && persisted.songid_runs == 1
            && persisted_runs.len() == 1
            && read.status == "200 OK"
            && read.body.contains(&run_id)
            && read.body.contains("persistence-songid-query")
    );

    let rehydrated = crate::RuntimeCompatState::from_persisted(&persisted);
    {
        let mut runtime = state.runtime.write().await;
        *runtime = rehydrated;
    }
    let restarted_read = crate::route_http_request(
        "GET",
        &format!("/api/v0/songid/runs/{run_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("read rehydrated SongID run");
    record!(
        "restart-rehydration",
        restarted_read.status == "200 OK"
            && restarted_read.body.contains(&run_id)
            && restarted_read.body.contains("persistence-songid-query")
    );
    restart_db
        .execute_raw_for_test(
            "UPDATE runtime_compat_state SET songid_run_records_json = 'not-json'",
        )
        .await
        .expect("corrupt persisted SongID state");
    let corrupt_record = restart_db
        .get_runtime_compat_state()
        .await
        .expect("read corrupt SongID runtime state");
    record!(
        "corrupt-state-and-upgrade-failure",
        corrupt_record.as_ref().is_some_and(|record| {
            crate::RuntimeCompatState::try_from_persisted(record).is_err()
        })
    );
    drop(state);
    drop(restart_db);
    let _ = fs::remove_file(&restart_path);

    let concurrent_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("SongID concurrency database");
    let (concurrent_state, _receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(concurrent_db.clone()),
    );
    let bodies = (0..4)
        .map(|index| format!(r#"{{"source":"parallel-songid-{index}"}}"#))
        .collect::<Vec<_>>();
    let responses = futures_util::future::join_all(bodies.iter().map(|body| {
        crate::route_http_request("POST", "/api/v0/songid/runs", None, body, &concurrent_state)
    }))
    .await;
    let concurrent_persisted = concurrent_db
        .get_runtime_compat_state()
        .await
        .expect("read concurrent SongID state")
        .expect("concurrent SongID state");
    let concurrent_runs = serde_json::from_str::<Vec<serde_json::Value>>(
        &concurrent_persisted.songid_run_records_json,
    )
    .expect("concurrent SongID run payload");
    let accepted_concurrent = responses
        .iter()
        .filter(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "202 Accepted")
        })
        .count();
    record!(
        "transaction-and-concurrency-atomicity",
        responses.iter().all(|response| {
            response.as_ref().is_ok_and(|response| {
                response.status == "202 Accepted" || response.status == "503 Service Unavailable"
            })
        }) && accepted_concurrent > 0
            && concurrent_persisted.songid_runs as usize == accepted_concurrent
            && concurrent_runs.len() == accepted_concurrent
            && concurrent_runs.iter().all(|run| {
                run["status"] == "completed"
                    && run["source"]
                        .as_str()
                        .is_some_and(|source| source.starts_with("parallel-songid-"))
            })
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("persistence-lifecycle");
    fs::create_dir_all(&evidence_dir).expect("create SongID persistence evidence directory");
    fs::write(
        evidence_dir.join("songid_runs.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize SongID persistence ledger"),
    )
    .expect("write SongID persistence ledger");
    assert!(
        mismatches.is_empty(),
        "{} SongID persistence mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential lifecycle proof for slskdN's exact TrafficStats table
/// and additive accounting contract.  The frozen service exposes reads
/// and additive writes, but no delete operation; update-delete remains
/// intentionally open rather than inventing a non-frozen API.
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
pub(super) async fn persistence_lifecycle_differential_native_traffic_stats_domain() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!("{target} TrafficStats {}", $case));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "domain": "TrafficStats",
                "case": $case,
                "pass": pass,
            }));
        }};
    }

    let schema_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("TrafficStats schema database");
    record!(
        "schema-create-and-migrate",
        schema_db
            .get_traffic_totals()
            .await
            .is_ok_and(|totals| totals.overlay_upload_bytes == 0
                && totals.overlay_download_bytes == 0
                && totals.soulseek_upload_bytes == 0
                && totals.soulseek_download_bytes == 0)
    );

    let create_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("TrafficStats create database");
    let created = create_db.add_traffic(11, 22, 33, 44).await;
    let created_totals = create_db
        .get_traffic_totals()
        .await
        .expect("read created TrafficStats row");
    record!(
        "create-and-read-roundtrip",
        created.is_ok()
            && created_totals.overlay_upload_bytes == 11
            && created_totals.overlay_download_bytes == 22
            && created_totals.soulseek_upload_bytes == 33
            && created_totals.soulseek_download_bytes == 44
    );

    let restart_path = std::env::temp_dir().join(format!(
        "slskr-traffic-stats-restart-{}-{}.sqlite",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    {
        let restart_db = crate::persistence::DatabaseManager::new(
            restart_path.to_str().expect("TrafficStats restart path"),
        )
        .await
        .expect("create file-backed TrafficStats database");
        restart_db
            .add_traffic(101, 202, 303, 404)
            .await
            .expect("persist TrafficStats restart fixture");
    }
    let reopened = crate::persistence::DatabaseManager::new(
        restart_path.to_str().expect("reopen TrafficStats path"),
    )
    .await
    .expect("reopen file-backed TrafficStats database");
    let restarted_totals = reopened
        .get_traffic_totals()
        .await
        .expect("read rehydrated TrafficStats row");
    record!(
        "restart-rehydration",
        restarted_totals.overlay_upload_bytes == 101
            && restarted_totals.overlay_download_bytes == 202
            && restarted_totals.soulseek_upload_bytes == 303
            && restarted_totals.soulseek_download_bytes == 404
    );
    drop(reopened);
    let _ = fs::remove_file(&restart_path);

    let concurrent_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("TrafficStats concurrency database");
    let concurrent_results =
        futures_util::future::join_all((0..6).map(|_| concurrent_db.add_traffic(1, 2, 3, 4))).await;
    let concurrent_totals = concurrent_db
        .get_traffic_totals()
        .await
        .expect("read concurrent TrafficStats row");
    record!(
        "transaction-and-concurrency-atomicity",
        concurrent_results.iter().all(Result::is_ok)
            && concurrent_totals.overlay_upload_bytes == 6
            && concurrent_totals.overlay_download_bytes == 12
            && concurrent_totals.soulseek_upload_bytes == 18
            && concurrent_totals.soulseek_download_bytes == 24
    );

    let corrupt_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("corrupt TrafficStats database");
    corrupt_db
        .execute_raw_for_test(
            "INSERT INTO TrafficStats (key, overlay_upload_bytes, overlay_download_bytes, soulseek_upload_bytes, soulseek_download_bytes, updated_at) VALUES ('global', 'not-a-number', 0, 0, 0, 1)",
        )
        .await
        .expect("insert corrupt TrafficStats row");
    record!(
        "corrupt-state-and-upgrade-failure",
        corrupt_db.get_traffic_totals().await.is_err()
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("persistence-lifecycle");
    fs::create_dir_all(&evidence_dir).expect("create TrafficStats evidence directory");
    fs::write(
        evidence_dir.join("traffic_stats_domain_lifecycle.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize TrafficStats ledger"),
    )
    .expect("write TrafficStats ledger");
    assert!(
        mismatches.is_empty(),
        "TrafficStats persistence mismatches: {}",
        mismatches.join(", ")
    );
}
