/// Differential lifecycle proof for the frozen slskd Transfers database
/// `Batches` domain.  This deliberately exercises the dedicated SQLite
/// table rather than the generic controller-feature JSON fallback.
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
async fn persistence_lifecycle_differential_controller_batches_domain() {
    let target = "slskd";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!("{target} Batches {}", $case));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "domain": "Batches",
                "case": $case,
                "pass": pass,
            }));
        }};
    }

    let schema_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("batches schema database");
    record!(
        "schema-create-and-migrate",
        schema_db
            .get_transfer_batch("aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa")
            .await
            .is_ok_and(|record| record.is_none())
    );

    let create_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("batches controller database");
    let env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKR_PERSISTENCE_ENABLED", "true")
        .with(
            "SLSKR_TEST_USER_ENDPOINT_OVERRIDES",
            "batches-roundtrip-peer=127.0.0.1:2234",
        );
    let (state, _receiver) =
        test_state_with_env_parts(env, super::SearchStore::new(), Some(create_db.clone()));
    let batch_id = "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb";
    let created = super::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/batches",
        None,
        &format!(
            r#"{{"id":"{batch_id}","username":"batches-roundtrip-peer","searchId":"cccccccc-cccc-4ccc-8ccc-cccccccccccc","files":[{{"filename":"Batches/Track.flac","size":42}}],"options":{{"destination":"Albums"}}}}"#
        ),
        &state,
    )
    .await
    .expect("create durable transfer batch");
    let stored = create_db
        .get_transfer_batch(batch_id)
        .await
        .expect("read durable batch")
        .expect("durable batch row");
    let fetched = super::route_http_request(
        "GET",
        &format!("/api/v0/transfers/downloads/batches/{batch_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("read durable batch through controller");
    let fetched_json = serde_json::from_str::<serde_json::Value>(&fetched.body).ok();
    record!(
        "create-and-read-roundtrip",
        created.status == "201 Created"
            && stored.username == "batches-roundtrip-peer"
            && stored.search_id.as_deref() == Some("cccccccc-cccc-4ccc-8ccc-cccccccccccc")
            && stored.options_json.as_deref() == Some(r#"{"destination":"Albums"}"#)
            && fetched.status == "200 OK"
            && fetched_json.as_ref().is_some_and(|json| {
                json["id"] == batch_id
                    && json["searchId"] == "cccccccc-cccc-4ccc-8ccc-cccccccccccc"
                    && json["options"]["destination"] == "Albums"
                    && json["transfers"]
                        .as_array()
                        .is_some_and(|rows| rows.len() == 1)
            })
    );

    let mut updated = stored.clone();
    updated.search_id = None;
    updated.username = "batches-updated-peer".to_owned();
    updated.options_json = Some(r#"{"destination":"Updated"}"#.to_owned());
    let update_result = create_db.update_transfer_batch(&updated).await;
    let after_update = create_db
        .get_transfer_batch(batch_id)
        .await
        .expect("read updated durable batch");
    let delete_result = create_db
        .delete_transfer_batch(batch_id)
        .await
        .expect("delete durable batch");
    let after_delete = create_db
        .get_transfer_batch(batch_id)
        .await
        .expect("read deleted durable batch");
    record!(
        "update-delete-and-readback",
        update_result.is_ok()
            && after_update.as_ref().is_some_and(|record| {
                record.username == "batches-updated-peer"
                    && record.search_id.is_none()
                    && record.options_json.as_deref() == Some(r#"{"destination":"Updated"}"#)
            })
            && delete_result
            && after_delete.is_none()
    );

    let restart_path = std::env::temp_dir().join(format!(
        "slskr-batches-restart-{}-{}.sqlite",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    let restart_id = "dddddddd-dddd-4ddd-8ddd-dddddddddddd";
    {
        let restart_db = super::persistence::DatabaseManager::new(
            restart_path.to_str().expect("restart database path"),
        )
        .await
        .expect("create file-backed batches database");
        restart_db
            .insert_transfer_batch(&super::persistence::TransferBatchRecord {
                id: restart_id.to_owned(),
                search_id: None,
                username: "batches-restart-peer".to_owned(),
                direction: 0,
                created_at: "2026-08-12T00:00:00Z".to_owned(),
                options_json: Some(r#"{"destination":"Restarted"}"#.to_owned()),
            })
            .await
            .expect("persist restart batch");
    }
    let reopened = super::persistence::DatabaseManager::new(
        restart_path.to_str().expect("reopen database path"),
    )
    .await
    .expect("reopen file-backed batches database");
    let rehydrated = reopened
        .get_transfer_batch(restart_id)
        .await
        .expect("read rehydrated batch");
    record!(
        "restart-rehydration",
        rehydrated.is_some_and(|record| {
            record.username == "batches-restart-peer"
                && record.options_json.as_deref() == Some(r#"{"destination":"Restarted"}"#)
        })
    );
    drop(reopened);
    let _ = fs::remove_file(&restart_path);

    let concurrent_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("batches concurrency database");
    let concurrent_record = super::persistence::TransferBatchRecord {
        id: "eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee".to_owned(),
        search_id: None,
        username: "batches-concurrent-peer".to_owned(),
        direction: 0,
        created_at: "2026-08-12T00:00:00Z".to_owned(),
        options_json: Some("{}".to_owned()),
    };
    let concurrent_results = futures_util::future::join_all(
        (0..2).map(|_| concurrent_db.insert_transfer_batch(&concurrent_record)),
    )
    .await;
    let concurrent_read = concurrent_db
        .get_transfer_batch(&concurrent_record.id)
        .await
        .expect("read concurrent batch");
    record!(
        "transaction-and-concurrency-atomicity",
        concurrent_results
            .iter()
            .filter(|result| result.is_ok())
            .count()
            == 1
            && concurrent_results
                .iter()
                .filter(|result| result.is_err())
                .count()
                == 1
            && concurrent_read.is_some()
    );

    let corrupt_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("batches corrupt-state database");
    corrupt_db
        .execute_raw_for_test(
            "INSERT INTO Batches (Id, SearchId, Username, Direction, CreatedAt, Options) VALUES ('ffffffff-ffff-4fff-8fff-ffffffffffff', NULL, 'corrupt-peer', 0, '2026-08-12T00:00:00Z', 'not-json')",
        )
        .await
        .expect("insert corrupt batch options");
    let (corrupt_state, _receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(corrupt_db),
    );
    let corrupt_response = super::route_http_request(
        "GET",
        "/api/v0/transfers/downloads/batches/ffffffff-ffff-4fff-8fff-ffffffffffff",
        None,
        "",
        &corrupt_state,
    )
    .await
    .expect("corrupt batch response");
    record!(
        "corrupt-state-and-upgrade-failure",
        corrupt_response.status == "500 Internal Server Error"
            && corrupt_response.body.contains("options")
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("persistence-lifecycle");
    fs::create_dir_all(&evidence_dir).expect("create persistence evidence directory");
    fs::write(
        evidence_dir.join("batches_domain_lifecycle.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize batches persistence ledger"),
    )
    .expect("write batches persistence ledger");
    assert!(
        mismatches.is_empty(),
        "{} Batches persistence mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential lifecycle proof for the frozen slskd Shares `files`
/// domain.  slskR keeps the durable share projection in its own
/// `share_files` table; this exercises that production snapshot,
/// rehydration, replacement, restart, transaction, and failure paths
/// without claiming parity for the frozen repository's derived directory,
/// FTS, or scan-history tables.
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
async fn persistence_lifecycle_differential_controller_share_files_domain() {
    let target = "slskd";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!("{target} files {}", $case));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "domain": "files",
                "case": $case,
                "pass": pass,
            }));
        }};
    }

    let schema_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("share files schema database");
    record!(
        "schema-create-and-migrate",
        schema_db
            .list_share_files(10, 0)
            .await
            .is_ok_and(|records| records.is_empty())
    );

    let base = super::persistence::ShareFileRecord {
        filename: "Virtual/Track.flac".to_owned(),
        size: 42,
        extension: "flac".to_owned(),
        root_label: "Virtual".to_owned(),
        local_path: Some("/library/Track.flac".to_owned()),
        attributes_json: "1:42".to_owned(),
        updated_at: 1,
    };
    let stale = super::persistence::ShareFileRecord {
        filename: "Virtual/Stale.flac".to_owned(),
        size: 7,
        extension: "flac".to_owned(),
        root_label: "Virtual".to_owned(),
        local_path: Some("/library/Stale.flac".to_owned()),
        attributes_json: "".to_owned(),
        updated_at: 1,
    };
    let initial = vec![base.clone(), stale.clone()];
    let create_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("share files lifecycle database");
    create_db
        .replace_share_files(&initial)
        .await
        .expect("persist initial share files snapshot");
    let stored = create_db
        .list_share_files(10, 0)
        .await
        .expect("read initial share files snapshot");
    record!(
        "create-and-read-roundtrip",
        stored.len() == 2
            && stored.iter().any(|record| {
                record.filename == base.filename
                    && record.size == base.size
                    && record.local_path == base.local_path
                    && record.attributes_json == base.attributes_json
            })
    );

    let updated = super::persistence::ShareFileRecord {
        size: 84,
        local_path: Some("/library/Track-remastered.flac".to_owned()),
        attributes_json: "1:84,2:320".to_owned(),
        updated_at: 2,
        ..base.clone()
    };
    create_db
        .replace_share_files(std::slice::from_ref(&updated))
        .await
        .expect("replace share files snapshot");
    let after_update = create_db
        .list_share_files(10, 0)
        .await
        .expect("read replaced share files snapshot");
    record!(
        "update-delete-and-readback",
        after_update.len() == 1
            && after_update[0].filename == updated.filename
            && after_update[0].size == 84
            && after_update[0].local_path == updated.local_path
            && after_update[0].attributes_json == updated.attributes_json
            && !after_update
                .iter()
                .any(|record| record.filename == stale.filename)
    );

    let restart_path = std::env::temp_dir().join(format!(
        "slskr-share-files-restart-{}-{}.sqlite",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    {
        let restart_db = super::persistence::DatabaseManager::new(
            restart_path.to_str().expect("share files restart path"),
        )
        .await
        .expect("create file-backed share files database");
        restart_db
            .replace_share_files(std::slice::from_ref(&base))
            .await
            .expect("persist share file for restart");
    }
    let reopened = super::persistence::DatabaseManager::new(
        restart_path
            .to_str()
            .expect("reopen share files restart path"),
    )
    .await
    .expect("reopen file-backed share files database");
    let rehydrated = reopened
        .list_share_files(10, 0)
        .await
        .expect("read rehydrated share file");
    record!(
        "restart-rehydration",
        rehydrated.len() == 1
            && rehydrated[0].filename == base.filename
            && rehydrated[0].size == base.size
            && rehydrated[0].root_label == base.root_label
    );
    drop(reopened);
    let _ = fs::remove_file(&restart_path);

    let concurrent_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("share files concurrency database");
    let concurrent_a = super::persistence::ShareFileRecord {
        filename: "Virtual/Concurrent-A.flac".to_owned(),
        ..base.clone()
    };
    let concurrent_b = super::persistence::ShareFileRecord {
        filename: "Virtual/Concurrent-B.flac".to_owned(),
        ..base.clone()
    };
    let concurrent_results = futures_util::future::join_all(
        vec![
            (concurrent_db.clone(), vec![concurrent_a.clone()]),
            (concurrent_db.clone(), vec![concurrent_b.clone()]),
        ]
        .into_iter()
        .map(|(db, records)| async move { db.replace_share_files(&records).await }),
    )
    .await;
    let concurrent_rows = concurrent_db
        .list_share_files(10, 0)
        .await
        .expect("read concurrent share files snapshot");
    record!(
        "transaction-and-concurrency-atomicity",
        concurrent_results.iter().all(Result::is_ok)
            && concurrent_rows.len() == 1
            && matches!(
                concurrent_rows[0].filename.as_str(),
                "Virtual/Concurrent-A.flac" | "Virtual/Concurrent-B.flac"
            )
    );

    let corrupt_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("share files corrupt-state database");
    corrupt_db
        .replace_share_files(std::slice::from_ref(&base))
        .await
        .expect("persist share file before corruption");
    corrupt_db
        .execute_raw_for_test("UPDATE share_files SET size = 'not-a-number'")
        .await
        .expect("corrupt share file size");
    record!(
        "corrupt-state-and-upgrade-failure",
        corrupt_db.list_share_files(10, 0).await.is_err()
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("persistence-lifecycle");
    fs::create_dir_all(&evidence_dir).expect("create persistence evidence directory");
    fs::write(
        evidence_dir.join("share_files_domain_lifecycle.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize share files persistence ledger"),
    )
    .expect("write share files persistence ledger");
    assert!(
        mismatches.is_empty(),
        "{} files persistence mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential lifecycle proof for the frozen `Transfers` domain.  The
/// transfer controller creates rows through search/download workflows in
/// production, so this proof uses the same durable `TransferRecord` write
/// path and its real read/update/delete/reopen methods instead of inventing
/// a REST creation endpoint that neither frozen registry declares.
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
async fn persistence_lifecycle_differential_transfers_domain_full_lifecycle() {
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($target:expr, $case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!("{} Transfers {}", $target, $case));
            }
            ledger.push(serde_json::json!({
                "target": $target,
                "domain": "Transfers",
                "case": $case,
                "pass": pass,
            }));
        }};
    }

    for target in ["slskd", "slskdn"] {
        let schema_db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("transfer schema database");
        record!(
            target,
            "schema-create-and-migrate",
            schema_db
                .list_transfers(None, 10, 0)
                .await
                .is_ok_and(|records| records.is_empty())
        );

        let base = super::persistence::TransferRecord {
            id: format!("transfer-roundtrip-{target}"),
            direction: "download".to_owned(),
            filename: "Persistence/Transfer.flac".to_owned(),
            peer_username: "transfer-peer".to_owned(),
            filesize: 4096,
            progress: 512,
            status: "queued".to_owned(),
            started_at: 1_700_000_000,
            completed_at: None,
            request_id: Some("request-1".to_owned()),
            wishlist_item_id: None,
            request_name: Some("transfer-proof".to_owned()),
            destination_directory: Some("Albums".to_owned()),
            local_path: None,
            batch_id: None,
            reason: None,
            bit_rate: Some(320),
            sample_rate: Some(44_100),
            bit_depth: Some(16),
            length_seconds: Some(12),
            artist: Some("Parity".to_owned()),
            album: Some("Lifecycle".to_owned()),
            title: Some("Transfer".to_owned()),
            track_number: Some(1),
            year: Some(2026),
            attempts: 1,
            auto_replace_attempts: 0,
            next_attempt_at: None,
            updated_at_ms: 1_700_000_000_000,
        };

        let create_db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("transfer lifecycle database");
        create_db
            .insert_transfer(&base)
            .await
            .expect("insert transfer lifecycle row");
        let stored = create_db
            .get_transfer(&base.id)
            .await
            .expect("read transfer lifecycle row");
        record!(
            target,
            "create-and-read-roundtrip",
            stored.as_ref().is_some_and(|row| {
                row.filename == base.filename
                    && row.progress == 512
                    && row.request_name.as_deref() == Some("transfer-proof")
            })
        );

        create_db
            .update_transfer_progress(&base.id, 2048, base.updated_at_ms as u64 + 1)
            .await
            .expect("update transfer progress");
        let updated = create_db
            .get_transfer(&base.id)
            .await
            .expect("read updated transfer");
        create_db
            .delete_transfer(&base.id)
            .await
            .expect("delete transfer lifecycle row");
        let deleted = create_db
            .get_transfer(&base.id)
            .await
            .expect("read deleted transfer");
        record!(
            target,
            "update-delete-and-readback",
            updated.is_some_and(|row| row.progress == 2048) && deleted.is_none()
        );

        let restart_path = std::env::temp_dir().join(format!(
            "slskr-transfer-lifecycle-{target}-{}-{}.sqlite",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        {
            let restart_db = super::persistence::DatabaseManager::new(
                restart_path.to_str().expect("transfer restart path"),
            )
            .await
            .expect("create transfer restart database");
            restart_db
                .insert_transfer(&base)
                .await
                .expect("persist transfer for restart");
        }
        let reopened = super::persistence::DatabaseManager::new(
            restart_path.to_str().expect("reopen transfer restart path"),
        )
        .await
        .expect("reopen transfer restart database");
        let rehydrated = reopened
            .get_transfer(&base.id)
            .await
            .expect("read rehydrated transfer");
        record!(
            target,
            "restart-rehydration",
            rehydrated.is_some_and(|row| row.peer_username == "transfer-peer")
        );
        drop(reopened);
        let _ = std::fs::remove_file(&restart_path);

        let concurrent_db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("transfer concurrency database");
        let concurrent_writes = futures_util::future::join_all((0..8).map(|index| {
            let mut row = base.clone();
            row.id = format!("transfer-concurrent-{target}-{index}");
            let db = concurrent_db.clone();
            async move { db.insert_transfer(&row).await }
        }))
        .await;
        let concurrent_rows = concurrent_db
            .list_transfers(None, 32, 0)
            .await
            .expect("read concurrent transfers");
        record!(
            target,
            "transaction-and-concurrency-atomicity",
            concurrent_writes.iter().all(Result::is_ok)
                && concurrent_rows.len() == 8
                && concurrent_rows
                    .iter()
                    .all(|row| row.filename == "Persistence/Transfer.flac")
        );

        let corrupt_db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("transfer corrupt-state database");
        corrupt_db
            .insert_transfer(&base)
            .await
            .expect("insert transfer before corruption");
        corrupt_db
            .execute_raw_for_test(&format!(
                "UPDATE transfers SET progress = 'not-a-number' WHERE id = '{}'",
                base.id
            ))
            .await
            .expect("corrupt transfer progress");
        record!(
            target,
            "corrupt-state-and-upgrade-failure",
            corrupt_db.list_transfers(None, 10, 0).await.is_err()
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("persistence-lifecycle");
    fs::create_dir_all(&evidence_dir).expect("create persistence evidence directory");
    fs::write(
        evidence_dir.join("transfers_domain_full_lifecycle.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize transfer lifecycle evidence"),
    )
    .expect("write transfer lifecycle evidence");
    assert!(
        mismatches.is_empty(),
        "{} transfer persistence mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential lifecycle proof for the frozen slskdN HashDb and
/// HashDbState tables.  Controller writes use the same SQLite snapshot
/// transaction that startup rehydrates, while state parsing failures are
/// exercised through the backfill route rather than a raw-only assertion.
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
async fn persistence_lifecycle_differential_native_hashdb_domains() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($domain:expr, $case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!("{target} {} {}", $domain, $case));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "domain": $domain,
                "case": $case,
                "pass": pass,
            }));
        }};
    }

    let schema_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("HashDb schema database");
    record!(
        "HashDb",
        "schema-create-and-migrate",
        schema_db
            .get_hash_db_entry("schema-missing")
            .await
            .is_ok_and(|entry| entry.is_none())
    );
    record!(
        "HashDbState",
        "schema-create-and-migrate",
        schema_db
            .get_hash_db_state("schema-missing")
            .await
            .is_ok_and(|state| state.is_none())
    );

    let create_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("HashDb controller database");
    let env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKR_PERSISTENCE_ENABLED", "true");
    let (state, _receiver) =
        test_state_with_env_parts(env, super::SearchStore::new(), Some(create_db.clone()));
    let filename = "Persistence/HashDb.flac";
    let hash_key = super::content_discovery::generate_flac_key(filename, 4096);
    let byte_hash = "a".repeat(64);
    let stored = super::route_http_request(
        "POST",
        "/api/v0/hashdb/hash",
        None,
        &serde_json::json!({
            "filename": filename,
            "byteHash": byte_hash,
            "size": 4096,
        })
        .to_string(),
        &state,
    )
    .await
    .expect("store HashDb entry through controller");
    let persisted_hash = create_db
        .get_hash_db_entry(&hash_key)
        .await
        .expect("read controller-persisted HashDb entry");
    let persisted_state = create_db
        .get_hash_db_state("latest_seq")
        .await
        .expect("read controller-persisted HashDb state");
    let read_back = super::route_http_request(
        "GET",
        &format!("/api/v0/hashdb/hash/{hash_key}"),
        None,
        "",
        &state,
    )
    .await
    .expect("read HashDb entry through controller");
    record!(
        "HashDb",
        "create-and-read-roundtrip",
        stored.status == "200 OK"
            && persisted_hash.as_ref().is_some_and(|entry| {
                entry.flac_key == hash_key
                    && entry.byte_hash == byte_hash
                    && entry.size == 4096
                    && entry.seq_id == 1
            })
            && read_back.status == "200 OK"
            && read_back.body.contains(&hash_key)
    );
    record!(
        "HashDbState",
        "create-and-read-roundtrip",
        persisted_state.is_some_and(|state| state.value.as_deref() == Some("1"))
    );

    let mut updated_hash = persisted_hash.expect("HashDb row for update");
    updated_hash.byte_hash = "b".repeat(64);
    updated_hash.use_count = 4;
    let hash_update = create_db.upsert_hash_db_entry(&updated_hash).await;
    let hash_after_update = create_db
        .get_hash_db_entry(&hash_key)
        .await
        .expect("read updated HashDb entry");
    let hash_deleted = create_db
        .delete_hash_db_entry(&hash_key)
        .await
        .expect("delete HashDb entry");
    let hash_after_delete = create_db
        .get_hash_db_entry(&hash_key)
        .await
        .expect("read deleted HashDb entry");
    record!(
        "HashDb",
        "update-delete-and-readback",
        hash_update.is_ok()
            && hash_after_update
                .as_ref()
                .is_some_and(|entry| { entry.byte_hash == "b".repeat(64) && entry.use_count == 4 })
            && hash_deleted
            && hash_after_delete.is_none()
    );

    let state_key = "backfill_progress";
    let state_update = create_db
        .upsert_hash_db_state(&super::persistence::HashDbStateRecord {
            key: state_key.to_owned(),
            value: Some(r#"{"lastProcessedAt":12}"#.to_owned()),
        })
        .await;
    let state_after_update = create_db
        .get_hash_db_state(state_key)
        .await
        .expect("read updated HashDb state");
    let state_deleted = create_db
        .delete_hash_db_state(state_key)
        .await
        .expect("delete HashDb state");
    let state_after_delete = create_db
        .get_hash_db_state(state_key)
        .await
        .expect("read deleted HashDb state");
    record!(
        "HashDbState",
        "update-delete-and-readback",
        state_update.is_ok()
            && state_after_update.is_some_and(|state| {
                state.value.as_deref() == Some(r#"{"lastProcessedAt":12}"#)
            })
            && state_deleted
            && state_after_delete.is_none()
    );

    let restart_path = std::env::temp_dir().join(format!(
        "slskr-hashdb-restart-{}-{}.sqlite",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    {
        let restart_db = super::persistence::DatabaseManager::new(
            restart_path.to_str().expect("HashDb restart path"),
        )
        .await
        .expect("create file-backed HashDb");
        restart_db
            .replace_hash_db_snapshot(
                &[super::persistence::HashDbRecord {
                    flac_key: "hashdb-restart-key".to_owned(),
                    byte_hash: "c".repeat(64),
                    size: 8192,
                    first_seen_at: 1,
                    last_updated_at: 2,
                    seq_id: 7,
                    use_count: 2,
                    full_file_hash: String::new(),
                    musicbrainz_id: String::new(),
                    file_sha256: String::new(),
                }],
                7,
            )
            .await
            .expect("persist restart HashDb snapshot");
        restart_db
            .upsert_hash_db_state(&super::persistence::HashDbStateRecord {
                key: "backfill_progress".to_owned(),
                value: Some(r#"{"lastProcessedAt":7}"#.to_owned()),
            })
            .await
            .expect("persist restart HashDb state");
    }
    let reopened = super::persistence::DatabaseManager::new(
        restart_path.to_str().expect("reopen HashDb path"),
    )
    .await
    .expect("reopen file-backed HashDb");
    let restart_hash = reopened
        .get_hash_db_entry("hashdb-restart-key")
        .await
        .expect("read rehydrated HashDb");
    let restart_state = reopened
        .get_hash_db_state("backfill_progress")
        .await
        .expect("read rehydrated HashDb state");
    record!(
        "HashDb",
        "restart-rehydration",
        restart_hash.is_some_and(|entry| entry.seq_id == 7 && entry.size == 8192)
    );
    record!(
        "HashDbState",
        "restart-rehydration",
        restart_state
            .is_some_and(|state| { state.value.as_deref() == Some(r#"{"lastProcessedAt":7}"#) })
    );
    drop(reopened);
    let _ = fs::remove_file(&restart_path);

    let concurrent_hash_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("HashDb concurrency database");
    let snapshot_a = vec![super::persistence::HashDbRecord {
        flac_key: "hashdb-concurrent-a".to_owned(),
        byte_hash: "d".repeat(64),
        size: 100,
        first_seen_at: 1,
        last_updated_at: 1,
        seq_id: 1,
        use_count: 1,
        full_file_hash: String::new(),
        musicbrainz_id: String::new(),
        file_sha256: String::new(),
    }];
    let snapshot_b = vec![super::persistence::HashDbRecord {
        flac_key: "hashdb-concurrent-b".to_owned(),
        byte_hash: "e".repeat(64),
        size: 200,
        first_seen_at: 2,
        last_updated_at: 2,
        seq_id: 2,
        use_count: 1,
        full_file_hash: String::new(),
        musicbrainz_id: String::new(),
        file_sha256: String::new(),
    }];
    let snapshots = [(&snapshot_a, 1_i64), (&snapshot_b, 2_i64)];
    let snapshot_results =
        futures_util::future::join_all(snapshots.iter().map(|(records, latest_seq)| {
            concurrent_hash_db.replace_hash_db_snapshot(records, *latest_seq)
        }))
        .await;
    let final_hash_rows = concurrent_hash_db
        .list_hash_db_entries()
        .await
        .expect("read concurrent HashDb snapshot");
    let final_seq = concurrent_hash_db
        .get_hash_db_state("latest_seq")
        .await
        .expect("read concurrent HashDb cursor")
        .and_then(|state| state.value)
        .and_then(|value| value.parse::<i64>().ok());
    let snapshot_is_atomic = final_hash_rows.len() == 1
        && final_seq == final_hash_rows.first().map(|entry| entry.seq_id)
        && (final_hash_rows[0].flac_key == "hashdb-concurrent-a"
            || final_hash_rows[0].flac_key == "hashdb-concurrent-b");
    record!(
        "HashDb",
        "transaction-and-concurrency-atomicity",
        snapshot_results.iter().all(Result::is_ok) && snapshot_is_atomic
    );

    let concurrent_state_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("HashDb state concurrency database");
    let concurrent_states = (0..4)
        .map(|index| super::persistence::HashDbStateRecord {
            key: format!("concurrent-{index}"),
            value: Some(index.to_string()),
        })
        .collect::<Vec<_>>();
    let state_results = futures_util::future::join_all(
        concurrent_states
            .iter()
            .map(|state| concurrent_state_db.upsert_hash_db_state(state)),
    )
    .await;
    let states_read = futures_util::future::join_all(
        concurrent_states
            .iter()
            .map(|state| concurrent_state_db.get_hash_db_state(&state.key)),
    )
    .await;
    record!(
        "HashDbState",
        "transaction-and-concurrency-atomicity",
        state_results.iter().all(Result::is_ok)
            && states_read
                .iter()
                .all(|result| { result.as_ref().is_ok_and(|state| state.is_some()) })
    );

    let corrupt_hash_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("corrupt HashDb database");
    corrupt_hash_db
        .execute_raw_for_test(
            "INSERT INTO HashDb (flac_key, byte_hash, size, first_seen_at, last_updated_at, seq_id, use_count) VALUES ('corrupt-hash', 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', 'not-a-number', 1, 1, 1, 1)",
        )
        .await
        .expect("insert corrupt HashDb row");
    record!(
        "HashDb",
        "corrupt-state-and-upgrade-failure",
        corrupt_hash_db
            .get_hash_db_entry("corrupt-hash")
            .await
            .is_err()
    );

    let corrupt_state_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("corrupt HashDb state database");
    corrupt_state_db
        .execute_raw_for_test(
            "INSERT INTO HashDbState (key, value) VALUES ('hashdb/backfill/progress', 'not-json')",
        )
        .await
        .expect("insert corrupt HashDb state");
    let (corrupt_state, _receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(corrupt_state_db),
    );
    let corrupt_response = super::route_http_request(
        "POST",
        "/api/v0/hashdb/backfill/from-history",
        None,
        "",
        &corrupt_state,
    )
    .await
    .expect("corrupt HashDb state response");
    record!(
        "HashDbState",
        "corrupt-state-and-upgrade-failure",
        corrupt_response.status == "500 Internal Server Error"
            && corrupt_response.body.contains("HashDbState")
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("persistence-lifecycle");
    fs::create_dir_all(&evidence_dir).expect("create persistence evidence directory");
    fs::write(
        evidence_dir.join("hashdb_domains_lifecycle.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize HashDb persistence ledger"),
    )
    .expect("write HashDb persistence ledger");
    assert!(
        mismatches.is_empty(),
        "{} HashDb persistence mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

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
async fn persistence_lifecycle_differential_native_songid_runs() {
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

    let schema_db = super::persistence::DatabaseManager::in_memory()
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
    let restart_db = super::persistence::DatabaseManager::new(
        restart_path.to_str().expect("SongID restart path"),
    )
    .await
    .expect("create SongID restart database");
    let env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKR_PERSISTENCE_ENABLED", "true");
    let (state, _receiver) =
        test_state_with_env_parts(env, super::SearchStore::new(), Some(restart_db.clone()));
    let create = super::route_http_request(
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
    let read = super::route_http_request(
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

    let rehydrated = super::RuntimeCompatState::from_persisted(&persisted);
    {
        let mut runtime = state.runtime.write().await;
        *runtime = rehydrated;
    }
    let restarted_read = super::route_http_request(
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
            super::RuntimeCompatState::try_from_persisted(record).is_err()
        })
    );
    drop(state);
    drop(restart_db);
    let _ = fs::remove_file(&restart_path);

    let concurrent_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("SongID concurrency database");
    let (concurrent_state, _receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(concurrent_db.clone()),
    );
    let bodies = (0..4)
        .map(|index| format!(r#"{{"source":"parallel-songid-{index}"}}"#))
        .collect::<Vec<_>>();
    let responses = futures_util::future::join_all(bodies.iter().map(|body| {
        super::route_http_request("POST", "/api/v0/songid/runs", None, body, &concurrent_state)
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
async fn persistence_lifecycle_differential_native_traffic_stats_domain() {
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

    let schema_db = super::persistence::DatabaseManager::in_memory()
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

    let create_db = super::persistence::DatabaseManager::in_memory()
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
        let restart_db = super::persistence::DatabaseManager::new(
            restart_path.to_str().expect("TrafficStats restart path"),
        )
        .await
        .expect("create file-backed TrafficStats database");
        restart_db
            .add_traffic(101, 202, 303, 404)
            .await
            .expect("persist TrafficStats restart fixture");
    }
    let reopened = super::persistence::DatabaseManager::new(
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

    let concurrent_db = super::persistence::DatabaseManager::in_memory()
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

    let corrupt_db = super::persistence::DatabaseManager::in_memory()
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
async fn controller_api_differential_hashdb_domain_contracts() {
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

    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("HashDb controller contract database");
    let env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKR_SHARE_FIXTURE", "")
        .with("SLSKR_PERSISTENCE_ENABLED", "true");
    let (state, _receiver) =
        test_state_with_env_parts(env.clone(), super::SearchStore::new(), Some(db.clone()));

    let filename = "Contracts/Versioned.flac";
    let size = 4_096_u64;
    let byte_hash = "a".repeat(64);
    let hash_key = super::content_discovery::generate_flac_key(filename, size);

    let malformed_store =
        super::route_http_request("POST", "/api/v0/hashdb/hash", None, "not-json", &state)
            .await
            .expect("malformed HashDb store");
    record!(
        "POST",
        "/api/v0/hashdb/hash",
        "malformed-path-query-or-body",
        malformed_store.status == "400 Bad Request"
    );
    let missing_store =
        super::route_http_request("POST", "/api/v0/hashdb/hash", None, "{}", &state)
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
        super::route_http_request("POST", "/api/v0/hashdb/hash", None, &store_body, &state)
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
    let lookup = super::route_http_request(
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
        super::hashdb_flac_inventory_key("contract-peer", "Inventory/Track.flac", size);
    state
        .controller_features
        .write_for_test()
        .await
        .upsert(
            inventory_key,
            super::hashdb_flac_inventory_record("contract-peer", "Inventory/Track.flac", size),
        )
        .expect("seed HashDb inventory projection");
    let inventory_by_size = super::route_http_request(
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
        super::route_http_request("GET", "/api/v0/hashdb/inventory/unhashed", None, "", &state)
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
    let generated_key = super::route_http_request(
        "GET",
        &format!(
            "/api/v0/hashdb/key?filename={}&size={size}",
            super::url_encode(filename)
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
    let schema = super::route_http_request("GET", "/api/v0/hashdb/schema", None, "", &state)
        .await
        .expect("read populated HashDb schema");
    let schema_json =
        serde_json::from_str::<serde_json::Value>(&schema.body).unwrap_or(serde_json::Value::Null);
    record!(
        "GET",
        "/api/v0/hashdb/schema",
        "populated-dynamic-state",
        schema.status == "200 OK"
            && schema_json["currentVersion"] == super::HASHDB_SCHEMA_VERSION
            && schema_json["targetVersion"] == super::HASHDB_SCHEMA_VERSION
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
    let malformed_sync = super::route_http_request(
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
        super::route_http_request("POST", "/api/v0/hashdb/sync/merge", None, "{}", &state)
            .await
            .expect("missing HashDb sync entries");
    record!(
        "POST",
        "/api/v0/hashdb/sync/merge",
        "missing-empty-or-conflict-state",
        missing_sync.status == "400 Bad Request"
    );
    let merged = super::route_http_request(
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
    let sync_read = super::route_http_request(
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
    let idempotent = super::route_http_request(
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
        .map(super::hash_db_entry_from_persistence)
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
        test_state_with_env_parts(env.clone(), super::SearchStore::new(), Some(db.clone()));
    restarted_state
        .content_discovery
        .write()
        .await
        .restore_hash_entries(persisted_entries, latest_seq)
        .expect("rehydrate HashDb contract state");
    let restarted_lookup = super::route_http_request(
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
    let restarted_hash = super::route_http_request(
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
        async move { super::route_http_request("POST", path, None, &body, &state).await }
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
        let response = super::route_http_request("POST", path, None, "", &optimize_state)
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
        let repeated = super::route_http_request("POST", path, None, "", &optimize_state)
            .await
            .expect("repeated HashDb optimize mutation");
        record!(
            "POST",
            path,
            "mutation-side-effects-and-readback",
            repeated.status == "200 OK"
        );
        let empty_db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("empty optimizer database");
        let (empty_state, _empty_receiver) =
            test_state_with_env_parts(env.clone(), super::SearchStore::new(), Some(empty_db));
        let empty = super::route_http_request("POST", path, None, "", &empty_state)
            .await
            .expect("empty HashDb optimize mutation");
        record!(
            "POST",
            path,
            "missing-empty-or-conflict-state",
            empty.status == "200 OK"
        );
        let concurrent = futures_util::future::join_all(
            (0..3).map(|_| super::route_http_request("POST", path, None, "", &optimize_state)),
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
        let restart = super::route_http_request("POST", path, None, "", &restarted_state)
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
    let profile = super::route_http_request(
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
    let profile_restart = super::route_http_request(
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
        super::route_http_request(
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

    let failure_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("HashDb runtime failure database");
    let (failure_state, _failure_receiver) =
        test_state_with_env_parts(env, super::SearchStore::new(), Some(failure_db.clone()));
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
        let response = super::route_http_request("GET", path, None, "", &failure_state)
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
    let key_failure = super::route_http_request(
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
        let response = super::route_http_request("POST", path, None, body, &failure_state)
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

/// Differential proof for the residual PodsController surface.  Each
/// row is backed by a live route call plus durable readback, a confined
/// state-path failure, a fresh PodStore/PodChannelStore load, or a
/// concurrent mutation.  The route/case keys intentionally match the
/// frozen slskdN controller manifest exactly.
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
async fn controller_api_differential_pods_controller_residuals() {
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

    macro_rules! request {
        ($state:expr, $method:expr, $path:expr, $body:expr) => {{
            super::route_http_request($method, $path, None, $body, $state)
                .await
                .unwrap_or_else(|error| panic!("{} {}: {}", $method, $path, error))
        }};
    }

    macro_rules! seed_pod {
        ($state:expr, $pod_id:expr) => {{
            $state
                .pods
                .write()
                .await
                .create(
                    serde_json::from_value::<super::pods::PodRecord>(serde_json::json!({
                        "podId": $pod_id,
                        "name": format!("Pods residual {}", $pod_id),
                        "isPublic": true,
                        "maxMembers": 16,
                        "channels": [{
                            "channelId": "general",
                            "kind": 0,
                            "name": "General"
                        }]
                    }))
                    .expect("deserialize PodsController fixture pod"),
                    "tester".to_owned(),
                )
                .expect("persist PodsController fixture pod");
        }};
    }

    macro_rules! add_member {
        ($state:expr, $pod_id:expr, $peer_id:expr) => {{
            $state
                .pods
                .write()
                .await
                .join($pod_id, $peer_id.to_owned())
                .expect("join PodsController fixture member")
                .expect("PodsController fixture member must join");
        }};
    }

    let block_file = |path: PathBuf| {
        if path.exists() {
            fs::remove_file(&path).expect("remove state file before runtime failure");
        }
        fs::create_dir(&path).expect("block state file with directory");
    };
    let create_body = |pod_id: &str| {
        serde_json::json!({
            "pod": {
                "podId": pod_id,
                "name": format!("Created {pod_id}"),
                "isPublic": true,
                "maxMembers": 16,
                "channels": [{
                    "channelId": "general",
                    "kind": 0,
                    "name": "General"
                }]
            },
            "requestingPeerId": "ignored-by-auth"
        })
        .to_string()
    };
    let update_body = |pod_id: &str, name: &str| {
        serde_json::json!({
            "pod": {
                "podId": pod_id,
                "name": name,
                "isPublic": true,
                "maxMembers": 16,
                "channels": [{
                    "channelId": "general",
                    "kind": 0,
                    "name": "General"
                }]
            }
        })
        .to_string()
    };

    let delete_route = "/api/v0/pods/{podId}";
    let list_route = "/api/v0/pods";
    let detail_route = "/api/v0/pods/{podId}";
    let messages_route = "/api/v0/pods/{podId}/channels/{channelId}/messages";
    let members_route = "/api/v0/pods/{podId}/members";
    let create_route = "/api/v0/pods";
    let ban_route = "/api/v0/pods/{podId}/ban";
    let bind_route = "/api/v0/pods/{podId}/channels/{channelId}/bind";
    let send_route = "/api/v0/pods/{podId}/channels/{channelId}/messages";
    let unbind_route = "/api/v0/pods/{podId}/channels/{channelId}/unbind";
    let join_route = "/api/v0/pods/{podId}/join";
    let leave_route = "/api/v0/pods/{podId}/leave";
    let update_route = "/api/v0/pods/{podId}";

    // DELETE /pods/{podId}
    {
        let (state, _receiver) = test_state();
        seed_pod!(&state, "pods-delete-nominal");
        let deleted = request!(&state, "DELETE", "/api/v0/pods/pods-delete-nominal", "");
        record!(
            "DELETE",
            delete_route,
            "nominal-status-headers-body",
            deleted.status == "204 No Content"
        );
    }
    {
        let (state, _receiver) = test_state();
        let malformed = request!(&state, "DELETE", "/api/v0/pods/%20", "");
        record!(
            "DELETE",
            delete_route,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request" && malformed.body.contains("PodId is required")
        );
    }
    {
        let (state, _receiver) = test_state();
        seed_pod!(&state, "pods-delete-runtime");
        block_file(state.config.state_dir.join("pods.json"));
        let runtime = request!(&state, "DELETE", "/api/v0/pods/pods-delete-runtime", "");
        record!(
            "DELETE",
            delete_route,
            "runtime-failure-and-timeout",
            runtime.status == "500 Internal Server Error"
                && runtime.body.contains("Failed to delete pod")
        );
    }
    {
        let (state, _receiver) = test_state();
        seed_pod!(&state, "pods-delete-mutation");
        let deleted = request!(&state, "DELETE", "/api/v0/pods/pods-delete-mutation", "");
        let missing = request!(&state, "GET", "/api/v0/pods/pods-delete-mutation", "");
        record!(
            "DELETE",
            delete_route,
            "mutation-side-effects-and-readback",
            deleted.status == "204 No Content" && missing.status == "404 Not Found"
        );
    }
    {
        let (state, _receiver) = test_state();
        seed_pod!(&state, "pods-delete-restart");
        let deleted = request!(&state, "DELETE", "/api/v0/pods/pods-delete-restart", "");
        let loaded =
            super::pods::PodStore::load(&state.config.state_dir).expect("reload deleted pod state");
        record!(
            "DELETE",
            delete_route,
            "restart-persistence-or-reset",
            deleted.status == "204 No Content" && loaded.get("pods-delete-restart").is_none()
        );
    }
    {
        let (state, _receiver) = test_state();
        seed_pod!(&state, "pods-delete-concurrent");
        let responses = futures_util::future::join_all((0..2).map(|_| {
            let state = Arc::clone(&state);
            async move { request!(&state, "DELETE", "/api/v0/pods/pods-delete-concurrent", "") }
        }))
        .await;
        let success = responses
            .iter()
            .filter(|response| response.status == "204 No Content")
            .count();
        let missing = responses
            .iter()
            .filter(|response| response.status == "404 Not Found")
            .count();
        record!(
            "DELETE",
            delete_route,
            "concurrency-and-idempotency",
            success == 1 && missing == 1
        );
    }

    // GET /pods
    {
        let (state, _receiver) = test_state();
        let response = request!(&state, "GET", "/api/v0/pods?unexpected=1", "");
        record!(
            "GET",
            list_route,
            "malformed-path-query-or-body",
            response.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&response.body)
                    .unwrap_or_default()
                    .as_array()
                    .is_some_and(Vec::is_empty)
        );
    }
    {
        let (state, _receiver) = test_state();
        let response = request!(&state, "GET", "/api/v0/pods", "");
        record!(
            "GET",
            list_route,
            "missing-empty-or-conflict-state",
            response.status == "200 OK" && response.body == "[]"
        );
    }
    {
        let (state, _receiver) = test_state();
        seed_pod!(&state, "pods-list-runtime");
        block_file(state.config.state_dir.join("pods.json"));
        let response = request!(&state, "GET", "/api/v0/pods", "");
        record!(
            "GET",
            list_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("Failed to list pods")
        );
    }
    {
        let (state, _receiver) = test_state();
        seed_pod!(&state, "pods-list-one");
        seed_pod!(&state, "pods-list-two");
        let response = request!(&state, "GET", "/api/v0/pods", "");
        let json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "GET",
            list_route,
            "populated-dynamic-state",
            response.status == "200 OK" && json.as_array().is_some_and(|pods| pods.len() == 2)
        );
    }

    // GET /pods/{podId}
    {
        let (state, _receiver) = test_state();
        let response = request!(&state, "GET", "/api/v0/pods/%20", "");
        record!(
            "GET",
            detail_route,
            "malformed-path-query-or-body",
            response.status == "400 Bad Request" && response.body.contains("PodId is required")
        );
    }
    {
        let (state, _receiver) = test_state();
        let response = request!(&state, "GET", "/api/v0/pods/pods-detail-missing", "");
        record!(
            "GET",
            detail_route,
            "missing-empty-or-conflict-state",
            response.status == "404 Not Found"
        );
    }
    {
        let (state, _receiver) = test_state();
        seed_pod!(&state, "pods-detail-runtime");
        block_file(state.config.state_dir.join("pods.json"));
        let response = request!(&state, "GET", "/api/v0/pods/pods-detail-runtime", "");
        record!(
            "GET",
            detail_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("Failed to get pod")
        );
    }

    // GET /pods/{podId}/channels/{channelId}/messages
    {
        let (state, _receiver) = test_state();
        seed_pod!(&state, "pods-messages-runtime");
        add_member!(&state, "pods-messages-runtime", "message-peer");
        state
            .pod_channels
            .write()
            .await
            .append(
                "pods-messages-runtime".to_owned(),
                "general".to_owned(),
                "tester".to_owned(),
                "runtime fixture".to_owned(),
                String::new(),
                super::unix_timestamp_millis(),
            )
            .expect("seed pod channel message");
        block_file(state.config.state_dir.join("pod-channel-messages.json"));
        let response = request!(
            &state,
            "GET",
            "/api/v0/pods/pods-messages-runtime/channels/general/messages",
            ""
        );
        record!(
            "GET",
            messages_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("Failed to get messages")
        );
    }

    // GET /pods/{podId}/members
    {
        let (state, _receiver) = test_state();
        let response = request!(&state, "GET", "/api/v0/pods/%20/members", "");
        record!(
            "GET",
            members_route,
            "malformed-path-query-or-body",
            response.status == "400 Bad Request" && response.body.contains("PodId is required")
        );
    }
    {
        let (state, _receiver) = test_state();
        let response = request!(
            &state,
            "GET",
            "/api/v0/pods/pods-members-missing/members",
            ""
        );
        record!(
            "GET",
            members_route,
            "missing-empty-or-conflict-state",
            response.status == "404 Not Found"
        );
    }
    {
        let (state, _receiver) = test_state();
        seed_pod!(&state, "pods-members-runtime");
        block_file(state.config.state_dir.join("pods.json"));
        let response = request!(
            &state,
            "GET",
            "/api/v0/pods/pods-members-runtime/members",
            ""
        );
        record!(
            "GET",
            members_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("Failed to get pod members")
        );
    }
    {
        let (state, _receiver) = test_state();
        seed_pod!(&state, "pods-members-populated");
        add_member!(&state, "pods-members-populated", "member-peer");
        let response = request!(
            &state,
            "GET",
            "/api/v0/pods/pods-members-populated/members",
            ""
        );
        let json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "GET",
            members_route,
            "populated-dynamic-state",
            response.status == "200 OK"
                && json.as_array().is_some_and(|members| members.len() == 2)
        );
    }

    // POST /pods
    {
        let (state, _receiver) = test_state();
        let response = request!(&state, "POST", "/api/v0/pods", r#"{"pod":null}"#);
        record!(
            "POST",
            create_route,
            "malformed-path-query-or-body",
            response.status == "400 Bad Request" && response.body.contains("Pod data is required")
        );
    }
    {
        let (state, _receiver) = test_state();
        block_file(state.config.state_dir.join("pods.json"));
        let response = request!(
            &state,
            "POST",
            "/api/v0/pods",
            &create_body("pods-create-runtime")
        );
        record!(
            "POST",
            create_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("Failed to create pod")
        );
    }
    {
        let (state, _receiver) = test_state();
        let response = request!(
            &state,
            "POST",
            "/api/v0/pods",
            &create_body("pods-create-restart")
        );
        let loaded =
            super::pods::PodStore::load(&state.config.state_dir).expect("reload created pod state");
        record!(
            "POST",
            create_route,
            "restart-persistence-or-reset",
            response.status == "201 Created" && loaded.get("pods-create-restart").is_some()
        );
    }
    {
        let (state, _receiver) = test_state();
        let ids = (0..4)
            .map(|index| format!("pods-create-concurrent-{index}"))
            .collect::<Vec<_>>();
        let responses = futures_util::future::join_all(ids.iter().map(|pod_id| {
            let state = Arc::clone(&state);
            let body = create_body(pod_id);
            async move { request!(&state, "POST", "/api/v0/pods", &body) }
        }))
        .await;
        let loaded = super::pods::PodStore::load(&state.config.state_dir)
            .expect("reload concurrent created pods");
        record!(
            "POST",
            create_route,
            "concurrency-and-idempotency",
            responses
                .iter()
                .all(|response| response.status == "201 Created")
                && ids.iter().all(|pod_id| loaded.get(pod_id).is_some())
        );
    }

    // POST /pods/{podId}/ban
    {
        let (state, _receiver) = test_state();
        seed_pod!(&state, "pods-ban-nominal");
        add_member!(&state, "pods-ban-nominal", "ban-peer");
        let response = request!(
            &state,
            "POST",
            "/api/v0/pods/pods-ban-nominal/ban",
            r#"{"peerId":"ban-peer"}"#
        );
        record!(
            "POST",
            ban_route,
            "nominal-status-headers-body",
            response.status == "200 OK" && response.body.contains("\"banned\":true")
        );
    }
    {
        let (state, _receiver) = test_state();
        seed_pod!(&state, "pods-ban-malformed");
        let response = request!(&state, "POST", "/api/v0/pods/pods-ban-malformed/ban", "{}");
        record!(
            "POST",
            ban_route,
            "malformed-path-query-or-body",
            response.status == "400 Bad Request" && response.body.contains("PeerId is required")
        );
    }
    {
        let (state, _receiver) = test_state();
        let response = request!(
            &state,
            "POST",
            "/api/v0/pods/pods-ban-missing/ban",
            r#"{"peerId":"ban-peer"}"#
        );
        record!(
            "POST",
            ban_route,
            "missing-empty-or-conflict-state",
            response.status == "403 Forbidden"
        );
    }
    {
        let (state, _receiver) = test_state();
        seed_pod!(&state, "pods-ban-runtime");
        add_member!(&state, "pods-ban-runtime", "ban-peer");
        block_file(state.config.state_dir.join("pods.json"));
        let response = request!(
            &state,
            "POST",
            "/api/v0/pods/pods-ban-runtime/ban",
            r#"{"peerId":"ban-peer"}"#
        );
        record!(
            "POST",
            ban_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("Failed to ban member")
        );
    }
    {
        let (state, _receiver) = test_state();
        seed_pod!(&state, "pods-ban-restart");
        add_member!(&state, "pods-ban-restart", "ban-peer");
        let response = request!(
            &state,
            "POST",
            "/api/v0/pods/pods-ban-restart/ban",
            r#"{"peerId":"ban-peer"}"#
        );
        let loaded =
            super::pods::PodStore::load(&state.config.state_dir).expect("reload banned pod state");
        let members = loaded.members("pods-ban-restart").unwrap_or_default();
        record!(
            "POST",
            ban_route,
            "restart-persistence-or-reset",
            response.status == "200 OK"
                && members.iter().all(|member| member.peer_id != "ban-peer")
        );
    }
    {
        let (state, _receiver) = test_state();
        seed_pod!(&state, "pods-ban-concurrent");
        add_member!(&state, "pods-ban-concurrent", "ban-peer");
        let responses = futures_util::future::join_all((0..2).map(|_| {
            let state = Arc::clone(&state);
            async move {
                request!(
                    &state,
                    "POST",
                    "/api/v0/pods/pods-ban-concurrent/ban",
                    r#"{"peerId":"ban-peer"}"#
                )
            }
        }))
        .await;
        let loaded = super::pods::PodStore::load(&state.config.state_dir)
            .expect("reload concurrently banned pod");
        let members = loaded.members("pods-ban-concurrent").unwrap_or_default();
        record!(
            "POST",
            ban_route,
            "concurrency-and-idempotency",
            responses.iter().all(|response| response.status == "200 OK")
                && members.iter().all(|member| member.peer_id != "ban-peer")
        );
    }

    // POST /pods/{podId}/channels/{channelId}/bind
    {
        let (state, _receiver) = test_state();
        seed_pod!(&state, "pods-bind-nominal");
        let response = request!(
            &state,
            "POST",
            "/api/v0/pods/pods-bind-nominal/channels/general/bind",
            r#"{"roomName":"ambient","mode":"mirror"}"#
        );
        let detail = request!(&state, "GET", "/api/v0/pods/pods-bind-nominal", "");
        record!(
            "POST",
            bind_route,
            "nominal-status-headers-body",
            response.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&detail.body).unwrap_or_default()
                    ["channels"][0]["bindingInfo"]
                    == "soulseek-room:ambient"
        );
    }
    {
        let (state, _receiver) = test_state();
        let response = request!(
            &state,
            "POST",
            "/api/v0/pods/pods-bind-missing/channels/general/bind",
            r#"{"roomName":"ambient"}"#
        );
        record!(
            "POST",
            bind_route,
            "missing-empty-or-conflict-state",
            response.status == "403 Forbidden"
        );
    }
    {
        let (state, _receiver) = test_state();
        seed_pod!(&state, "pods-bind-runtime");
        block_file(state.config.state_dir.join("pods.json"));
        let response = request!(
            &state,
            "POST",
            "/api/v0/pods/pods-bind-runtime/channels/general/bind",
            r#"{"roomName":"ambient"}"#
        );
        record!(
            "POST",
            bind_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("Failed to bind room")
        );
    }
    {
        let (state, _receiver) = test_state();
        seed_pod!(&state, "pods-bind-restart");
        let response = request!(
            &state,
            "POST",
            "/api/v0/pods/pods-bind-restart/channels/general/bind",
            r#"{"roomName":"ambient","mode":"readonly"}"#
        );
        let loaded =
            super::pods::PodStore::load(&state.config.state_dir).expect("reload bound pod state");
        let binding = loaded.get("pods-bind-restart").and_then(|pod| {
            pod.channels
                .first()
                .and_then(|channel| channel.binding_info.clone())
        });
        record!(
            "POST",
            bind_route,
            "restart-persistence-or-reset",
            response.status == "200 OK" && binding.as_deref() == Some("soulseek-room:ambient")
        );
    }
    {
        let (state, _receiver) = test_state();
        seed_pod!(&state, "pods-bind-concurrent");
        let responses = futures_util::future::join_all((0..2).map(|_| {
            let state = Arc::clone(&state);
            async move {
                request!(
                    &state,
                    "POST",
                    "/api/v0/pods/pods-bind-concurrent/channels/general/bind",
                    r#"{"roomName":"ambient","mode":"mirror"}"#
                )
            }
        }))
        .await;
        let loaded = super::pods::PodStore::load(&state.config.state_dir)
            .expect("reload concurrently bound pod");
        let binding = loaded.get("pods-bind-concurrent").and_then(|pod| {
            pod.channels
                .first()
                .and_then(|channel| channel.binding_info.clone())
        });
        record!(
            "POST",
            bind_route,
            "concurrency-and-idempotency",
            responses.iter().all(|response| response.status == "200 OK")
                && binding.as_deref() == Some("soulseek-room:ambient")
        );
    }

    // POST /pods/{podId}/channels/{channelId}/messages
    {
        let (state, _receiver) = test_state();
        seed_pod!(&state, "pods-send-runtime");
        block_file(state.config.state_dir.join("pod-channel-messages.json"));
        let response = request!(
            &state,
            "POST",
            "/api/v0/pods/pods-send-runtime/channels/general/messages",
            r#"{"body":"runtime","senderPeerId":"tester"}"#
        );
        record!(
            "POST",
            send_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("Failed to send message")
        );
    }
    {
        let (state, _receiver) = test_state();
        seed_pod!(&state, "pods-send-restart");
        let response = request!(
            &state,
            "POST",
            "/api/v0/pods/pods-send-restart/channels/general/messages",
            r#"{"body":"restart","senderPeerId":"tester"}"#
        );
        let loaded = super::pod_channels::PodChannelStore::load(&state.config.state_dir)
            .expect("reload sent pod channel message");
        let persisted = loaded
            .list("pods-send-restart", "general", None)
            .iter()
            .any(|message| message.body == "restart");
        record!(
            "POST",
            send_route,
            "restart-persistence-or-reset",
            response.status == "200 OK" && persisted
        );
    }
    {
        let (state, _receiver) = test_state();
        seed_pod!(&state, "pods-send-concurrent");
        let responses = futures_util::future::join_all((0..4).map(|index| {
            let state = Arc::clone(&state);
            let body = format!(r#"{{"body":"concurrent-{index}","senderPeerId":"tester"}}"#);
            async move {
                request!(
                    &state,
                    "POST",
                    "/api/v0/pods/pods-send-concurrent/channels/general/messages",
                    &body
                )
            }
        }))
        .await;
        let loaded = super::pod_channels::PodChannelStore::load(&state.config.state_dir)
            .expect("reload concurrently sent pod messages");
        let messages = loaded.list("pods-send-concurrent", "general", None);
        record!(
            "POST",
            send_route,
            "concurrency-and-idempotency",
            responses.iter().all(|response| response.status == "200 OK") && messages.len() == 4
        );
    }

    // POST /pods/{podId}/channels/{channelId}/unbind
    {
        let (state, _receiver) = test_state();
        seed_pod!(&state, "pods-unbind-nominal");
        let bound = request!(
            &state,
            "POST",
            "/api/v0/pods/pods-unbind-nominal/channels/general/bind",
            r#"{"roomName":"ambient"}"#
        );
        let response = request!(
            &state,
            "POST",
            "/api/v0/pods/pods-unbind-nominal/channels/general/unbind",
            ""
        );
        record!(
            "POST",
            unbind_route,
            "nominal-status-headers-body",
            bound.status == "200 OK" && response.status == "200 OK"
        );
    }
    {
        let (state, _receiver) = test_state();
        seed_pod!(&state, "pods-unbind-malformed");
        let response = request!(
            &state,
            "POST",
            "/api/v0/pods/pods-unbind-malformed/channels/%20/unbind",
            ""
        );
        record!(
            "POST",
            unbind_route,
            "malformed-path-query-or-body",
            response.status == "400 Bad Request"
                && response.body.contains("PodId and ChannelId are required")
        );
    }
    {
        let (state, _receiver) = test_state();
        seed_pod!(&state, "pods-unbind-runtime");
        let bound = request!(
            &state,
            "POST",
            "/api/v0/pods/pods-unbind-runtime/channels/general/bind",
            r#"{"roomName":"ambient"}"#
        );
        block_file(state.config.state_dir.join("pods.json"));
        let response = request!(
            &state,
            "POST",
            "/api/v0/pods/pods-unbind-runtime/channels/general/unbind",
            ""
        );
        record!(
            "POST",
            unbind_route,
            "runtime-failure-and-timeout",
            bound.status == "200 OK"
                && response.status == "500 Internal Server Error"
                && response.body.contains("Failed to unbind room")
        );
    }
    {
        let (state, _receiver) = test_state();
        seed_pod!(&state, "pods-unbind-mutation");
        let bound = request!(
            &state,
            "POST",
            "/api/v0/pods/pods-unbind-mutation/channels/general/bind",
            r#"{"roomName":"ambient"}"#
        );
        let response = request!(
            &state,
            "POST",
            "/api/v0/pods/pods-unbind-mutation/channels/general/unbind",
            ""
        );
        let detail = request!(&state, "GET", "/api/v0/pods/pods-unbind-mutation", "");
        let json = serde_json::from_str::<serde_json::Value>(&detail.body).unwrap_or_default();
        record!(
            "POST",
            unbind_route,
            "mutation-side-effects-and-readback",
            bound.status == "200 OK"
                && response.status == "200 OK"
                && json["channels"][0]["bindingInfo"].is_null()
        );
    }
    {
        let (state, _receiver) = test_state();
        seed_pod!(&state, "pods-unbind-restart");
        let bound = request!(
            &state,
            "POST",
            "/api/v0/pods/pods-unbind-restart/channels/general/bind",
            r#"{"roomName":"ambient"}"#
        );
        let response = request!(
            &state,
            "POST",
            "/api/v0/pods/pods-unbind-restart/channels/general/unbind",
            ""
        );
        let loaded =
            super::pods::PodStore::load(&state.config.state_dir).expect("reload unbound pod state");
        let binding = loaded.get("pods-unbind-restart").and_then(|pod| {
            pod.channels
                .first()
                .and_then(|channel| channel.binding_info.clone())
        });
        record!(
            "POST",
            unbind_route,
            "restart-persistence-or-reset",
            bound.status == "200 OK" && response.status == "200 OK" && binding.is_none()
        );
    }
    {
        let (state, _receiver) = test_state();
        seed_pod!(&state, "pods-unbind-concurrent");
        let bound = request!(
            &state,
            "POST",
            "/api/v0/pods/pods-unbind-concurrent/channels/general/bind",
            r#"{"roomName":"ambient"}"#
        );
        let responses = futures_util::future::join_all((0..2).map(|_| {
            let state = Arc::clone(&state);
            async move {
                request!(
                    &state,
                    "POST",
                    "/api/v0/pods/pods-unbind-concurrent/channels/general/unbind",
                    ""
                )
            }
        }))
        .await;
        let success = responses
            .iter()
            .filter(|response| response.status == "200 OK")
            .count();
        let missing = responses
            .iter()
            .filter(|response| response.status == "404 Not Found")
            .count();
        record!(
            "POST",
            unbind_route,
            "concurrency-and-idempotency",
            bound.status == "200 OK" && success == 1 && missing == 1
        );
    }

    // POST /pods/{podId}/join
    {
        let (state, _receiver) = test_state();
        seed_pod!(&state, "pods-join-nominal");
        *state.runtime_credentials.write().await = Some(super::LoginCredentials::default_client(
            "join-peer",
            "secret",
        ));
        let response = request!(
            &state,
            "POST",
            "/api/v0/pods/pods-join-nominal/join",
            r#"{"peerId":"ignored-by-auth"}"#
        );
        record!(
            "POST",
            join_route,
            "nominal-status-headers-body",
            response.status == "200 OK" && response.body.contains("\"joined\":true")
        );
    }
    {
        let (state, _receiver) = test_state();
        let response = request!(&state, "POST", "/api/v0/pods/%20/join", "{}");
        record!(
            "POST",
            join_route,
            "malformed-path-query-or-body",
            response.status == "400 Bad Request" && response.body.contains("PodId is required")
        );
    }
    {
        let (state, _receiver) = test_state();
        let response = request!(&state, "POST", "/api/v0/pods/pods-join-missing/join", "{}");
        record!(
            "POST",
            join_route,
            "missing-empty-or-conflict-state",
            response.status == "404 Not Found"
        );
    }
    {
        let (state, _receiver) = test_state();
        seed_pod!(&state, "pods-join-runtime");
        *state.runtime_credentials.write().await = Some(super::LoginCredentials::default_client(
            "join-runtime-peer",
            "secret",
        ));
        block_file(state.config.state_dir.join("pods.json"));
        let response = request!(&state, "POST", "/api/v0/pods/pods-join-runtime/join", "{}");
        record!(
            "POST",
            join_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("Failed to join pod")
        );
    }
    {
        let (state, _receiver) = test_state();
        seed_pod!(&state, "pods-join-restart");
        *state.runtime_credentials.write().await = Some(super::LoginCredentials::default_client(
            "join-restart-peer",
            "secret",
        ));
        let response = request!(&state, "POST", "/api/v0/pods/pods-join-restart/join", "{}");
        let loaded =
            super::pods::PodStore::load(&state.config.state_dir).expect("reload joined pod state");
        let joined = loaded
            .members("pods-join-restart")
            .unwrap_or_default()
            .iter()
            .any(|member| member.peer_id == "join-restart-peer");
        record!(
            "POST",
            join_route,
            "restart-persistence-or-reset",
            response.status == "200 OK" && joined
        );
    }
    {
        let (state, _receiver) = test_state();
        seed_pod!(&state, "pods-join-concurrent");
        *state.runtime_credentials.write().await = Some(super::LoginCredentials::default_client(
            "join-concurrent-peer",
            "secret",
        ));
        let responses = futures_util::future::join_all((0..2).map(|_| {
            let state = Arc::clone(&state);
            async move {
                request!(
                    &state,
                    "POST",
                    "/api/v0/pods/pods-join-concurrent/join",
                    "{}"
                )
            }
        }))
        .await;
        let success = responses
            .iter()
            .filter(|response| response.status == "200 OK")
            .count();
        let duplicate = responses
            .iter()
            .filter(|response| response.status == "400 Bad Request")
            .count();
        record!(
            "POST",
            join_route,
            "concurrency-and-idempotency",
            success == 1 && duplicate == 1
        );
    }

    // POST /pods/{podId}/leave
    {
        let (state, _receiver) = test_state();
        seed_pod!(&state, "pods-leave-nominal");
        add_member!(&state, "pods-leave-nominal", "leave-peer");
        *state.runtime_credentials.write().await = Some(super::LoginCredentials::default_client(
            "leave-peer",
            "secret",
        ));
        let response = request!(
            &state,
            "POST",
            "/api/v0/pods/pods-leave-nominal/leave",
            r#"{"peerId":"ignored-by-auth"}"#
        );
        record!(
            "POST",
            leave_route,
            "nominal-status-headers-body",
            response.status == "200 OK" && response.body.contains("\"left\":true")
        );
    }
    {
        let (state, _receiver) = test_state();
        let response = request!(&state, "POST", "/api/v0/pods/%20/leave", "{}");
        record!(
            "POST",
            leave_route,
            "malformed-path-query-or-body",
            response.status == "400 Bad Request" && response.body.contains("PodId is required")
        );
    }
    {
        let (state, _receiver) = test_state();
        let response = request!(
            &state,
            "POST",
            "/api/v0/pods/pods-leave-missing/leave",
            "{}"
        );
        record!(
            "POST",
            leave_route,
            "missing-empty-or-conflict-state",
            response.status == "404 Not Found"
        );
    }
    {
        let (state, _receiver) = test_state();
        seed_pod!(&state, "pods-leave-runtime");
        add_member!(&state, "pods-leave-runtime", "leave-runtime-peer");
        *state.runtime_credentials.write().await = Some(super::LoginCredentials::default_client(
            "leave-runtime-peer",
            "secret",
        ));
        block_file(state.config.state_dir.join("pods.json"));
        let response = request!(
            &state,
            "POST",
            "/api/v0/pods/pods-leave-runtime/leave",
            "{}"
        );
        record!(
            "POST",
            leave_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("Failed to leave pod")
        );
    }
    {
        let (state, _receiver) = test_state();
        seed_pod!(&state, "pods-leave-mutation");
        add_member!(&state, "pods-leave-mutation", "leave-mutation-peer");
        *state.runtime_credentials.write().await = Some(super::LoginCredentials::default_client(
            "leave-mutation-peer",
            "secret",
        ));
        let response = request!(
            &state,
            "POST",
            "/api/v0/pods/pods-leave-mutation/leave",
            "{}"
        );
        let members = request!(
            &state,
            "GET",
            "/api/v0/pods/pods-leave-mutation/members",
            ""
        );
        record!(
            "POST",
            leave_route,
            "mutation-side-effects-and-readback",
            response.status == "200 OK" && !members.body.contains("leave-mutation-peer")
        );
    }
    {
        let (state, _receiver) = test_state();
        seed_pod!(&state, "pods-leave-restart");
        add_member!(&state, "pods-leave-restart", "leave-restart-peer");
        *state.runtime_credentials.write().await = Some(super::LoginCredentials::default_client(
            "leave-restart-peer",
            "secret",
        ));
        let response = request!(
            &state,
            "POST",
            "/api/v0/pods/pods-leave-restart/leave",
            "{}"
        );
        let loaded =
            super::pods::PodStore::load(&state.config.state_dir).expect("reload left pod state");
        let left = loaded
            .members("pods-leave-restart")
            .unwrap_or_default()
            .iter()
            .all(|member| member.peer_id != "leave-restart-peer");
        record!(
            "POST",
            leave_route,
            "restart-persistence-or-reset",
            response.status == "200 OK" && left
        );
    }
    {
        let (state, _receiver) = test_state();
        seed_pod!(&state, "pods-leave-concurrent");
        add_member!(&state, "pods-leave-concurrent", "leave-concurrent-peer");
        *state.runtime_credentials.write().await = Some(super::LoginCredentials::default_client(
            "leave-concurrent-peer",
            "secret",
        ));
        let responses = futures_util::future::join_all((0..2).map(|_| {
            let state = Arc::clone(&state);
            async move {
                request!(
                    &state,
                    "POST",
                    "/api/v0/pods/pods-leave-concurrent/leave",
                    "{}"
                )
            }
        }))
        .await;
        let success = responses
            .iter()
            .filter(|response| response.status == "200 OK")
            .count();
        let missing = responses
            .iter()
            .filter(|response| response.status == "404 Not Found")
            .count();
        record!(
            "POST",
            leave_route,
            "concurrency-and-idempotency",
            success == 1 && missing == 1
        );
    }

    // PUT /pods/{podId}
    {
        let (state, _receiver) = test_state();
        let response = request!(
            &state,
            "PUT",
            "/api/v0/pods/pods-update-malformed",
            &update_body("different-pod-id", "Mismatch")
        );
        record!(
            "PUT",
            update_route,
            "malformed-path-query-or-body",
            response.status == "400 Bad Request"
                && response
                    .body
                    .contains("PodId in URL must match PodId in body")
        );
    }
    {
        let (state, _receiver) = test_state();
        let response = request!(
            &state,
            "PUT",
            "/api/v0/pods/pods-update-missing",
            &update_body("pods-update-missing", "Missing")
        );
        record!(
            "PUT",
            update_route,
            "missing-empty-or-conflict-state",
            response.status == "404 Not Found"
        );
    }
    {
        let (state, _receiver) = test_state();
        seed_pod!(&state, "pods-update-runtime");
        block_file(state.config.state_dir.join("pods.json"));
        let response = request!(
            &state,
            "PUT",
            "/api/v0/pods/pods-update-runtime",
            &update_body("pods-update-runtime", "Runtime")
        );
        record!(
            "PUT",
            update_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("Failed to update pod")
        );
    }
    {
        let (state, _receiver) = test_state();
        seed_pod!(&state, "pods-update-restart");
        let response = request!(
            &state,
            "PUT",
            "/api/v0/pods/pods-update-restart",
            &update_body("pods-update-restart", "Restarted")
        );
        let loaded =
            super::pods::PodStore::load(&state.config.state_dir).expect("reload updated pod state");
        let updated = loaded
            .get("pods-update-restart")
            .is_some_and(|pod| pod.name == "Restarted");
        record!(
            "PUT",
            update_route,
            "restart-persistence-or-reset",
            response.status == "200 OK" && updated
        );
    }
    {
        let (state, _receiver) = test_state();
        seed_pod!(&state, "pods-update-concurrent");
        let responses = futures_util::future::join_all((0..2).map(|index| {
            let state = Arc::clone(&state);
            let body = update_body("pods-update-concurrent", &format!("Concurrent-{index}"));
            async move { request!(&state, "PUT", "/api/v0/pods/pods-update-concurrent", &body) }
        }))
        .await;
        let loaded = super::pods::PodStore::load(&state.config.state_dir)
            .expect("reload concurrently updated pod");
        let updated = loaded
            .get("pods-update-concurrent")
            .is_some_and(|pod| pod.name.starts_with("Concurrent-"));
        record!(
            "PUT",
            update_route,
            "concurrency-and-idempotency",
            responses.iter().all(|response| response.status == "200 OK") && updated
        );
    }

    assert_eq!(ledger.len(), 60, "PodsController residual ledger size");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create PodsController evidence directory");
    fs::write(
        evidence_dir.join("pods_controller_residuals.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize PodsController ledger"),
    )
    .expect("write PodsController ledger");
    assert!(
        mismatches.is_empty(),
        "{} PodsController residual mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential proof for the remaining frozen WishlistController cases.
/// The v0 controller binds Guid route values, returns 200 for an executed
/// manual search, and lets ordinary service/storage failures surface as
/// 500 responses.  Each row below is backed by a live route call and, for
/// stateful cases, durable SQLite readback or a fresh in-memory projection.
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
async fn controller_api_differential_wishlist_controller_residuals() {
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

    macro_rules! request {
        ($state:expr, $method:expr, $path:expr, $body:expr) => {{
            super::route_http_request($method, $path, None, $body, $state)
                .await
                .unwrap_or_else(|error| panic!("{} {}: {}", $method, $path, error))
        }};
    }

    macro_rules! seed_item {
        ($state:expr, $id:expr) => {{
            $state
                .wishlist
                .write()
                .await
                .add_item_with_contract(
                    Some($id.to_owned()),
                    "Artist".to_owned(),
                    "Track".to_owned(),
                    "Audio".to_owned(),
                    String::new(),
                    true,
                    false,
                    100,
                    None,
                )
                .expect("seed WishlistController item");
        }};
    }

    macro_rules! seed_ignored {
        ($state:expr, $id:expr) => {{
            $state
                .wishlist
                .write()
                .await
                .ignore_result($id, "Peer", "Remote/Album", false)
                .expect("seed WishlistController ignored result")
                .0
        }};
    }

    macro_rules! db_state {
        () => {{
            let db = super::persistence::DatabaseManager::in_memory()
                .await
                .expect("WishlistController in-memory database");
            let (state, receiver) = test_state_with_env_parts(
                MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
                super::SearchStore::new(),
                Some(db.clone()),
            );
            (state, db, receiver)
        }};
    }

    let create_body = |search_text: &str| {
        serde_json::json!({
            "searchText": search_text,
            "filter": "lossless",
            "enabled": true,
            "autoDownload": false,
            "maxResults": 10,
            "maxDownloads": 2,
        })
        .to_string()
    };
    let update_body = |search_text: &str| {
        serde_json::json!({
            "searchText": search_text,
            "filter": "updated",
            "enabled": false,
            "autoDownload": true,
            "maxResults": 5,
            "maxDownloads": null,
        })
        .to_string()
    };
    let ignored_body = r#"{"username":"Peer","directory":"Remote\\Album/"}"#;
    let import_body = r#"{"csvText":"Artist,Title,Album\nImported Artist,Imported Track,Imported Album","filter":"imported","maxResults":5,"includeAlbum":true}"#;

    let delete_route = "/api/v0/wishlist/{id}";
    let ignored_delete_route = "/api/v0/wishlist/{id}/ignored-results/{ignoredResultId}";
    let list_route = "/api/v0/wishlist";
    let detail_route = "/api/v0/wishlist/{id}";
    let ignored_list_route = "/api/v0/wishlist/{id}/ignored-results";
    let searches_route = "/api/v0/wishlist/{id}/searches";
    let create_route = "/api/v0/wishlist";
    let ignored_create_route = "/api/v0/wishlist/{id}/ignored-results";
    let viewed_route = "/api/v0/wishlist/{id}/mark-viewed";
    let search_route = "/api/v0/wishlist/{id}/search";
    let import_route = "/api/v0/wishlist/import/csv";
    let mark_all_route = "/api/v0/wishlist/mark-all-viewed";
    let update_route = "/api/v0/wishlist/{id}";

    // DELETE /wishlist/{id}: malformed, missing, restart, concurrency.
    {
        let (state, _receiver) = test_state();
        let malformed = request!(&state, "DELETE", "/api/v0/wishlist/%20", "");
        record!(
            "DELETE",
            delete_route,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
    }
    {
        let (state, _receiver) = test_state();
        let missing_id = uuid::Uuid::new_v4().to_string();
        let missing = request!(
            &state,
            "DELETE",
            &format!("/api/v0/wishlist/{missing_id}"),
            ""
        );
        record!(
            "DELETE",
            delete_route,
            "missing-empty-or-conflict-state",
            missing.status == "204 No Content"
        );
    }
    {
        let (state, db, _receiver) = db_state!();
        let created = request!(
            &state,
            "POST",
            "/api/v0/wishlist",
            &create_body("delete-restart")
        );
        let item_id = serde_json::from_str::<serde_json::Value>(&created.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("delete restart item id");
        let deleted = request!(&state, "DELETE", &format!("/api/v0/wishlist/{item_id}"), "");
        let persisted = db.list_wishlist_items(10, 0).await.unwrap_or_default();
        record!(
            "DELETE",
            delete_route,
            "restart-persistence-or-reset",
            created.status == "201 Created"
                && deleted.status == "204 No Content"
                && persisted.is_empty()
        );
    }
    {
        let (state, _receiver) = test_state();
        let item_id = uuid::Uuid::new_v4().to_string();
        seed_item!(&state, &item_id);
        let responses = futures_util::future::join_all((0..2).map(|_| {
            let state = Arc::clone(&state);
            let path = format!("/api/v0/wishlist/{item_id}");
            async move { request!(&state, "DELETE", &path, "") }
        }))
        .await;
        let readback = request!(&state, "GET", &format!("/api/v0/wishlist/{item_id}"), "");
        record!(
            "DELETE",
            delete_route,
            "concurrency-and-idempotency",
            responses
                .iter()
                .all(|response| response.status == "204 No Content")
                && readback.status == "404 Not Found"
        );
    }

    // DELETE /wishlist/{id}/ignored-results/{ignoredResultId}.
    {
        let (state, _receiver) = test_state();
        let malformed_first = request!(
            &state,
            "DELETE",
            "/api/v0/wishlist/not-a-guid/ignored-results/not-a-guid",
            ""
        );
        let valid_id = uuid::Uuid::new_v4();
        let malformed_second = request!(
            &state,
            "DELETE",
            &format!("/api/v0/wishlist/{valid_id}/ignored-results/not-a-guid"),
            ""
        );
        record!(
            "DELETE",
            ignored_delete_route,
            "malformed-path-query-or-body",
            malformed_first.status == "400 Bad Request"
                && malformed_second.status == "400 Bad Request"
        );
    }
    {
        let (state, _receiver) = test_state();
        let item_id = uuid::Uuid::new_v4();
        let rule_id = uuid::Uuid::new_v4();
        let missing = request!(
            &state,
            "DELETE",
            &format!("/api/v0/wishlist/{item_id}/ignored-results/{rule_id}"),
            ""
        );
        record!(
            "DELETE",
            ignored_delete_route,
            "missing-empty-or-conflict-state",
            missing.status == "404 Not Found"
        );
    }
    {
        let (state, db, _receiver) = db_state!();
        let item_id = uuid::Uuid::new_v4().to_string();
        seed_item!(&state, &item_id);
        let rule = seed_ignored!(&state, &item_id);
        db.close_for_test().await;
        let response = request!(
            &state,
            "DELETE",
            &format!("/api/v0/wishlist/{item_id}/ignored-results/{}", rule.id),
            ""
        );
        record!(
            "DELETE",
            ignored_delete_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
        );
    }
    {
        let (state, db, _receiver) = db_state!();
        let created = request!(
            &state,
            "POST",
            "/api/v0/wishlist",
            &create_body("ignored-delete-restart")
        );
        let item_id = serde_json::from_str::<serde_json::Value>(&created.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("ignored delete item id");
        let ignored = request!(
            &state,
            "POST",
            &format!("/api/v0/wishlist/{item_id}/ignored-results"),
            ignored_body
        );
        let rule_id = serde_json::from_str::<serde_json::Value>(&ignored.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("ignored delete rule id");
        let deleted = request!(
            &state,
            "DELETE",
            &format!("/api/v0/wishlist/{item_id}/ignored-results/{rule_id}"),
            ""
        );
        let persisted = db
            .list_wishlist_ignored_results(&item_id)
            .await
            .unwrap_or_default();
        record!(
            "DELETE",
            ignored_delete_route,
            "restart-persistence-or-reset",
            ignored.status == "201 Created"
                && deleted.status == "204 No Content"
                && persisted.is_empty()
        );
    }
    {
        let (state, _receiver) = test_state();
        let item_id = uuid::Uuid::new_v4().to_string();
        seed_item!(&state, &item_id);
        let rule = seed_ignored!(&state, &item_id);
        let path = format!("/api/v0/wishlist/{item_id}/ignored-results/{}", rule.id);
        let responses = futures_util::future::join_all((0..2).map(|_| {
            let state = Arc::clone(&state);
            let path = path.clone();
            async move { request!(&state, "DELETE", &path, "") }
        }))
        .await;
        let success = responses
            .iter()
            .filter(|response| response.status == "204 No Content")
            .count();
        let missing = responses
            .iter()
            .filter(|response| response.status == "404 Not Found")
            .count();
        record!(
            "DELETE",
            ignored_delete_route,
            "concurrency-and-idempotency",
            success == 1 && missing == 1
        );
    }

    // GET /wishlist.
    {
        let (state, _receiver) = test_state();
        let response = request!(&state, "GET", "/api/v0/wishlist", "");
        record!(
            "GET",
            list_route,
            "nominal-status-headers-body",
            response.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&response.body)
                    .unwrap_or_default()
                    .is_array()
        );
    }
    {
        let (state, _receiver) = test_state();
        let response = request!(&state, "GET", "/api/v0/wishlist?unexpected=1", "");
        record!(
            "GET",
            list_route,
            "malformed-path-query-or-body",
            response.status == "200 OK"
        );
    }
    {
        let (state, _receiver) = test_state();
        let response = request!(&state, "GET", "/api/v0/wishlist", "");
        record!(
            "GET",
            list_route,
            "missing-empty-or-conflict-state",
            response.status == "200 OK" && response.body == "[]"
        );
    }
    {
        let (state, db, _receiver) = db_state!();
        let item_id = uuid::Uuid::new_v4().to_string();
        seed_item!(&state, &item_id);
        db.close_for_test().await;
        let response = request!(&state, "GET", "/api/v0/wishlist", "");
        record!(
            "GET",
            list_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
        );
    }
    {
        let (state, _receiver) = test_state();
        let first_id = uuid::Uuid::new_v4().to_string();
        let second_id = uuid::Uuid::new_v4().to_string();
        seed_item!(&state, &first_id);
        seed_item!(&state, &second_id);
        let response = request!(&state, "GET", "/api/v0/wishlist", "");
        let json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "GET",
            list_route,
            "populated-dynamic-state",
            response.status == "200 OK" && json.as_array().is_some_and(|items| items.len() == 2)
        );
    }

    // GET /wishlist/{id}.
    {
        let (state, _receiver) = test_state();
        let item_id = uuid::Uuid::new_v4();
        let response = request!(&state, "GET", &format!("/api/v0/wishlist/{item_id}"), "");
        record!(
            "GET",
            detail_route,
            "missing-empty-or-conflict-state",
            response.status == "404 Not Found"
        );
    }
    {
        let (state, db, _receiver) = db_state!();
        let item_id = uuid::Uuid::new_v4().to_string();
        seed_item!(&state, &item_id);
        db.close_for_test().await;
        let response = request!(&state, "GET", &format!("/api/v0/wishlist/{item_id}"), "");
        record!(
            "GET",
            detail_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
        );
    }

    // GET /wishlist/{id}/ignored-results.
    {
        let (state, _receiver) = test_state();
        let item_id = uuid::Uuid::new_v4();
        let response = request!(
            &state,
            "GET",
            &format!("/api/v0/wishlist/{item_id}/ignored-results"),
            ""
        );
        record!(
            "GET",
            ignored_list_route,
            "missing-empty-or-conflict-state",
            response.status == "404 Not Found"
        );
    }
    {
        let (state, db, _receiver) = db_state!();
        let item_id = uuid::Uuid::new_v4().to_string();
        seed_item!(&state, &item_id);
        db.close_for_test().await;
        let response = request!(
            &state,
            "GET",
            &format!("/api/v0/wishlist/{item_id}/ignored-results"),
            ""
        );
        record!(
            "GET",
            ignored_list_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
        );
    }

    // GET /wishlist/{id}/searches.
    {
        let (state, _receiver) = test_state();
        let created = request!(
            &state,
            "POST",
            "/api/v0/wishlist",
            &create_body("searches-nominal")
        );
        let item_id = serde_json::from_str::<serde_json::Value>(&created.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("searches nominal item id");
        let response = request!(
            &state,
            "GET",
            &format!("/api/v0/wishlist/{item_id}/searches"),
            ""
        );
        record!(
            "GET",
            searches_route,
            "nominal-status-headers-body",
            response.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&response.body)
                    .unwrap_or_default()
                    .is_array()
        );
    }
    {
        let (state, _receiver) = test_state();
        let item_id = uuid::Uuid::new_v4();
        let response = request!(
            &state,
            "GET",
            &format!("/api/v0/wishlist/{item_id}/searches"),
            ""
        );
        record!(
            "GET",
            searches_route,
            "missing-empty-or-conflict-state",
            response.status == "404 Not Found"
        );
    }
    {
        let (state, db, _receiver) = db_state!();
        let item_id = uuid::Uuid::new_v4().to_string();
        seed_item!(&state, &item_id);
        db.close_for_test().await;
        let response = request!(
            &state,
            "GET",
            &format!("/api/v0/wishlist/{item_id}/searches"),
            ""
        );
        record!(
            "GET",
            searches_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
        );
    }
    {
        let (state, mut receiver) = test_state();
        let created = request!(
            &state,
            "POST",
            "/api/v0/wishlist",
            &create_body("searches-populated")
        );
        let item_id = serde_json::from_str::<serde_json::Value>(&created.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("searches populated item id");
        let started = request!(
            &state,
            "POST",
            &format!("/api/v0/wishlist/{item_id}/search"),
            ""
        );
        let _ = receiver.try_recv();
        let response = request!(
            &state,
            "GET",
            &format!("/api/v0/wishlist/{item_id}/searches"),
            ""
        );
        let json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "GET",
            searches_route,
            "populated-dynamic-state",
            started.status == "200 OK"
                && response.status == "200 OK"
                && json.as_array().is_some_and(|searches| !searches.is_empty())
        );
    }

    // POST /wishlist: malformed, missing, restart, concurrency.
    {
        let (state, _receiver) = test_state();
        let malformed = request!(&state, "POST", "/api/v0/wishlist", "{}");
        record!(
            "POST",
            create_route,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
                && malformed.body.contains("SearchText is required")
        );
    }
    {
        let (state, _receiver) = test_state();
        let response = request!(
            &state,
            "POST",
            "/api/v0/wishlist",
            &create_body("empty-state-create")
        );
        record!(
            "POST",
            create_route,
            "missing-empty-or-conflict-state",
            response.status == "201 Created"
        );
    }
    {
        let (state, db, _receiver) = db_state!();
        let response = request!(
            &state,
            "POST",
            "/api/v0/wishlist",
            &create_body("create-restart")
        );
        let persisted = db.list_wishlist_items(10, 0).await.unwrap_or_default();
        let restored =
            super::WishlistStore::from_persisted_with_ignored(persisted.clone(), Vec::new());
        record!(
            "POST",
            create_route,
            "restart-persistence-or-reset",
            response.status == "201 Created"
                && persisted.len() == 1
                && restored.records[0].items.len() == 1
        );
    }
    {
        let (state, _receiver) = test_state();
        let responses = futures_util::future::join_all((0..4).map(|index| {
            let state = Arc::clone(&state);
            let body = create_body(&format!("create-concurrent-{index}"));
            async move { request!(&state, "POST", "/api/v0/wishlist", &body) }
        }))
        .await;
        let count = state
            .wishlist
            .read()
            .await
            .records
            .iter()
            .flat_map(|record| record.items.iter())
            .count();
        record!(
            "POST",
            create_route,
            "concurrency-and-idempotency",
            responses
                .iter()
                .all(|response| response.status == "201 Created")
                && count == 4
        );
    }

    // POST /wishlist/{id}/ignored-results.
    {
        let (state, _receiver) = test_state();
        let item_id = uuid::Uuid::new_v4();
        let malformed = request!(
            &state,
            "POST",
            &format!("/api/v0/wishlist/{item_id}/ignored-results"),
            "{}"
        );
        record!(
            "POST",
            ignored_create_route,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
                && malformed
                    .body
                    .contains("Username and Directory are required")
        );
    }
    {
        let (state, _receiver) = test_state();
        let item_id = uuid::Uuid::new_v4();
        let missing = request!(
            &state,
            "POST",
            &format!("/api/v0/wishlist/{item_id}/ignored-results"),
            ignored_body
        );
        record!(
            "POST",
            ignored_create_route,
            "missing-empty-or-conflict-state",
            missing.status == "404 Not Found"
        );
    }
    {
        let (state, db, _receiver) = db_state!();
        let item_id = uuid::Uuid::new_v4().to_string();
        seed_item!(&state, &item_id);
        db.close_for_test().await;
        let response = request!(
            &state,
            "POST",
            &format!("/api/v0/wishlist/{item_id}/ignored-results"),
            ignored_body
        );
        record!(
            "POST",
            ignored_create_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
        );
    }
    {
        let (state, db, _receiver) = db_state!();
        let created = request!(
            &state,
            "POST",
            "/api/v0/wishlist",
            &create_body("ignored-create-restart")
        );
        let item_id = serde_json::from_str::<serde_json::Value>(&created.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("ignored create item id");
        let response = request!(
            &state,
            "POST",
            &format!("/api/v0/wishlist/{item_id}/ignored-results"),
            ignored_body
        );
        let persisted = db
            .list_wishlist_ignored_results(&item_id)
            .await
            .unwrap_or_default();
        record!(
            "POST",
            ignored_create_route,
            "restart-persistence-or-reset",
            response.status == "201 Created" && persisted.len() == 1
        );
    }
    {
        let (state, _receiver) = test_state();
        let item_id = uuid::Uuid::new_v4().to_string();
        seed_item!(&state, &item_id);
        let path = format!("/api/v0/wishlist/{item_id}/ignored-results");
        let responses = futures_util::future::join_all((0..2).map(|_| {
            let state = Arc::clone(&state);
            let path = path.clone();
            async move { request!(&state, "POST", &path, ignored_body) }
        }))
        .await;
        let rules = state.wishlist.read().await.list_ignored_results(&item_id);
        record!(
            "POST",
            ignored_create_route,
            "concurrency-and-idempotency",
            responses
                .iter()
                .all(|response| response.status == "201 Created")
                && rules.as_ref().is_some_and(|rules| rules.len() == 1)
        );
    }

    // POST /wishlist/{id}/mark-viewed.
    {
        let (state, _receiver) = test_state();
        let malformed = request!(
            &state,
            "POST",
            "/api/v0/wishlist/not-a-guid/mark-viewed",
            ""
        );
        record!(
            "POST",
            viewed_route,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
    }
    {
        let (state, _receiver) = test_state();
        let item_id = uuid::Uuid::new_v4();
        let missing = request!(
            &state,
            "POST",
            &format!("/api/v0/wishlist/{item_id}/mark-viewed"),
            ""
        );
        record!(
            "POST",
            viewed_route,
            "missing-empty-or-conflict-state",
            missing.status == "404 Not Found"
        );
    }
    {
        let (state, db, _receiver) = db_state!();
        let item_id = uuid::Uuid::new_v4().to_string();
        seed_item!(&state, &item_id);
        db.close_for_test().await;
        let response = request!(
            &state,
            "POST",
            &format!("/api/v0/wishlist/{item_id}/mark-viewed"),
            ""
        );
        record!(
            "POST",
            viewed_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
        );
    }
    {
        let (state, db, _receiver) = db_state!();
        let created = request!(
            &state,
            "POST",
            "/api/v0/wishlist",
            &create_body("viewed-restart")
        );
        let item_id = serde_json::from_str::<serde_json::Value>(&created.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("viewed restart item id");
        let response = request!(
            &state,
            "POST",
            &format!("/api/v0/wishlist/{item_id}/mark-viewed"),
            ""
        );
        let persisted = db.list_wishlist_items(10, 0).await.unwrap_or_default();
        record!(
            "POST",
            viewed_route,
            "restart-persistence-or-reset",
            response.status == "204 No Content"
                && persisted
                    .first()
                    .and_then(|item| item.last_viewed_at)
                    .is_some()
        );
    }
    {
        let (state, _receiver) = test_state();
        let item_id = uuid::Uuid::new_v4().to_string();
        seed_item!(&state, &item_id);
        let path = format!("/api/v0/wishlist/{item_id}/mark-viewed");
        let responses = futures_util::future::join_all((0..2).map(|_| {
            let state = Arc::clone(&state);
            let path = path.clone();
            async move { request!(&state, "POST", &path, "") }
        }))
        .await;
        let viewed = state
            .wishlist
            .read()
            .await
            .get_item(&item_id)
            .and_then(|item| item.last_viewed_at)
            .is_some();
        record!(
            "POST",
            viewed_route,
            "concurrency-and-idempotency",
            responses
                .iter()
                .all(|response| response.status == "204 No Content")
                && viewed
        );
    }

    // POST /wishlist/{id}/search.
    {
        let (state, _receiver) = test_state();
        let created = request!(
            &state,
            "POST",
            "/api/v0/wishlist",
            &create_body("manual-search-nominal")
        );
        let item_id = serde_json::from_str::<serde_json::Value>(&created.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("manual search nominal item id");
        let response = request!(
            &state,
            "POST",
            &format!("/api/v0/wishlist/{item_id}/search"),
            ""
        );
        record!(
            "POST",
            search_route,
            "nominal-status-headers-body",
            response.status == "200 OK" && response.body.contains("\"search_started\":true")
        );
    }
    {
        let (state, _receiver) = test_state();
        let malformed = request!(&state, "POST", "/api/v0/wishlist/not-a-guid/search", "");
        record!(
            "POST",
            search_route,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
    }
    {
        let (state, _receiver) = test_state();
        let item_id = uuid::Uuid::new_v4();
        let missing = request!(
            &state,
            "POST",
            &format!("/api/v0/wishlist/{item_id}/search"),
            ""
        );
        record!(
            "POST",
            search_route,
            "missing-empty-or-conflict-state",
            missing.status == "404 Not Found"
        );
    }
    {
        let (state, db, _receiver) = db_state!();
        let item_id = uuid::Uuid::new_v4().to_string();
        seed_item!(&state, &item_id);
        db.close_for_test().await;
        let response = request!(
            &state,
            "POST",
            &format!("/api/v0/wishlist/{item_id}/search"),
            ""
        );
        record!(
            "POST",
            search_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
        );
    }
    {
        let (state, _receiver) = test_state();
        let created = request!(
            &state,
            "POST",
            "/api/v0/wishlist",
            &create_body("manual-search-mutation")
        );
        let item_id = serde_json::from_str::<serde_json::Value>(&created.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("manual search mutation item id");
        let response = request!(
            &state,
            "POST",
            &format!("/api/v0/wishlist/{item_id}/search"),
            ""
        );
        let history = request!(
            &state,
            "GET",
            &format!("/api/v0/wishlist/{item_id}/searches"),
            ""
        );
        record!(
            "POST",
            search_route,
            "mutation-side-effects-and-readback",
            response.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&history.body)
                    .unwrap_or_default()
                    .as_array()
                    .is_some_and(|searches| !searches.is_empty())
        );
    }
    {
        let (state, db, _receiver) = db_state!();
        let created = request!(
            &state,
            "POST",
            "/api/v0/wishlist",
            &create_body("manual-search-restart")
        );
        let item_id = serde_json::from_str::<serde_json::Value>(&created.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("manual search restart item id");
        let response = request!(
            &state,
            "POST",
            &format!("/api/v0/wishlist/{item_id}/search"),
            ""
        );
        let persisted = db.list_searches(10, 0).await.unwrap_or_default();
        record!(
            "POST",
            search_route,
            "restart-persistence-or-reset",
            response.status == "200 OK" && !persisted.is_empty()
        );
    }
    {
        let (state, _receiver) = test_state();
        let item_id = uuid::Uuid::new_v4().to_string();
        seed_item!(&state, &item_id);
        let path = format!("/api/v0/wishlist/{item_id}/search");
        let responses = futures_util::future::join_all((0..2).map(|_| {
            let state = Arc::clone(&state);
            let path = path.clone();
            async move { request!(&state, "POST", &path, "") }
        }))
        .await;
        let search_count = state.searches.read().await.records.len();
        record!(
            "POST",
            search_route,
            "concurrency-and-idempotency",
            responses.iter().all(|response| response.status == "200 OK") && search_count == 2
        );
    }

    // POST /wishlist/import/csv.
    {
        let (state, _receiver) = test_state();
        let malformed = request!(&state, "POST", import_route, "not-json");
        record!(
            "POST",
            import_route,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
    }
    {
        let (state, _receiver) = test_state();
        let missing = request!(&state, "POST", import_route, r#"{"csvText":""}"#);
        record!(
            "POST",
            import_route,
            "missing-empty-or-conflict-state",
            missing.status == "400 Bad Request" && missing.body.contains("CsvText is required")
        );
    }
    {
        let (state, db, _receiver) = db_state!();
        let response = request!(&state, "POST", import_route, import_body);
        let persisted = db.list_wishlist_items(10, 0).await.unwrap_or_default();
        let restored =
            super::WishlistStore::from_persisted_with_ignored(persisted.clone(), Vec::new());
        record!(
            "POST",
            import_route,
            "restart-persistence-or-reset",
            response.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default()
                    ["createdCount"]
                    == 1
                && restored.records[0].items.len() == 1
        );
    }

    // POST /wishlist/mark-all-viewed.
    {
        let (state, _receiver) = test_state();
        let response = request!(&state, "POST", mark_all_route, "");
        record!(
            "POST",
            mark_all_route,
            "nominal-status-headers-body",
            response.status == "204 No Content"
        );
    }
    {
        let (state, _receiver) = test_state();
        let response = request!(&state, "POST", mark_all_route, "not-json");
        record!(
            "POST",
            mark_all_route,
            "malformed-path-query-or-body",
            response.status == "204 No Content"
        );
    }
    {
        let (state, _receiver) = test_state();
        let response = request!(&state, "POST", mark_all_route, "");
        record!(
            "POST",
            mark_all_route,
            "missing-empty-or-conflict-state",
            response.status == "204 No Content"
        );
    }
    {
        let (state, db, _receiver) = db_state!();
        let item_id = uuid::Uuid::new_v4().to_string();
        seed_item!(&state, &item_id);
        db.close_for_test().await;
        let response = request!(&state, "POST", mark_all_route, "");
        record!(
            "POST",
            mark_all_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
        );
    }
    {
        let (state, _receiver) = test_state();
        let first_id = uuid::Uuid::new_v4().to_string();
        let second_id = uuid::Uuid::new_v4().to_string();
        seed_item!(&state, &first_id);
        seed_item!(&state, &second_id);
        let response = request!(&state, "POST", mark_all_route, "");
        let viewed = state
            .wishlist
            .read()
            .await
            .records
            .iter()
            .flat_map(|record| record.items.iter())
            .all(|item| item.last_viewed_at.is_some());
        record!(
            "POST",
            mark_all_route,
            "mutation-side-effects-and-readback",
            response.status == "204 No Content" && viewed
        );
    }
    {
        let (state, db, _receiver) = db_state!();
        for text in ["mark-all-restart-one", "mark-all-restart-two"] {
            let response = request!(&state, "POST", "/api/v0/wishlist", &create_body(text));
            assert_eq!(response.status, "201 Created", "{text}");
        }
        let response = request!(&state, "POST", mark_all_route, "");
        let persisted = db.list_wishlist_items(10, 0).await.unwrap_or_default();
        record!(
            "POST",
            mark_all_route,
            "restart-persistence-or-reset",
            response.status == "204 No Content"
                && persisted.len() == 2
                && persisted.iter().all(|item| item.last_viewed_at.is_some())
        );
    }
    {
        let (state, _receiver) = test_state();
        let item_id = uuid::Uuid::new_v4().to_string();
        seed_item!(&state, &item_id);
        let responses = futures_util::future::join_all((0..2).map(|_| {
            let state = Arc::clone(&state);
            async move { request!(&state, "POST", mark_all_route, "") }
        }))
        .await;
        let viewed = state
            .wishlist
            .read()
            .await
            .get_item(&item_id)
            .and_then(|item| item.last_viewed_at)
            .is_some();
        record!(
            "POST",
            mark_all_route,
            "concurrency-and-idempotency",
            responses
                .iter()
                .all(|response| response.status == "204 No Content")
                && viewed
        );
    }

    // PUT /wishlist/{id}.
    {
        let (state, _receiver) = test_state();
        let created = request!(
            &state,
            "POST",
            "/api/v0/wishlist",
            &create_body("update-nominal")
        );
        let item_id = serde_json::from_str::<serde_json::Value>(&created.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("update nominal item id");
        let response = request!(
            &state,
            "PUT",
            &format!("/api/v0/wishlist/{item_id}"),
            &update_body("updated-nominal")
        );
        let json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "PUT",
            update_route,
            "nominal-status-headers-body",
            response.status == "200 OK" && json["searchText"] == "updated-nominal"
        );
    }
    {
        let (state, _receiver) = test_state();
        let malformed = request!(
            &state,
            "PUT",
            "/api/v0/wishlist/not-a-guid",
            &update_body("malformed")
        );
        record!(
            "PUT",
            update_route,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
    }
    {
        let (state, _receiver) = test_state();
        let item_id = uuid::Uuid::new_v4();
        let missing = request!(
            &state,
            "PUT",
            &format!("/api/v0/wishlist/{item_id}"),
            &update_body("missing")
        );
        record!(
            "PUT",
            update_route,
            "missing-empty-or-conflict-state",
            missing.status == "404 Not Found"
        );
    }
    {
        let (state, db, _receiver) = db_state!();
        let created = request!(
            &state,
            "POST",
            "/api/v0/wishlist",
            &create_body("update-restart")
        );
        let item_id = serde_json::from_str::<serde_json::Value>(&created.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("update restart item id");
        let response = request!(
            &state,
            "PUT",
            &format!("/api/v0/wishlist/{item_id}"),
            &update_body("updated-restart")
        );
        let persisted = db.list_wishlist_items(10, 0).await.unwrap_or_default();
        record!(
            "PUT",
            update_route,
            "restart-persistence-or-reset",
            response.status == "200 OK"
                && persisted
                    .first()
                    .is_some_and(|item| item.artist == "updated-restart")
        );
    }
    {
        let (state, _receiver) = test_state();
        let item_id = uuid::Uuid::new_v4().to_string();
        seed_item!(&state, &item_id);
        let path = format!("/api/v0/wishlist/{item_id}");
        let responses = futures_util::future::join_all((0..2).map(|index| {
            let state = Arc::clone(&state);
            let path = path.clone();
            let body = update_body(&format!("updated-concurrent-{index}"));
            async move { request!(&state, "PUT", &path, &body) }
        }))
        .await;
        let updated = state
            .wishlist
            .read()
            .await
            .get_item(&item_id)
            .is_some_and(|item| item.search_text().starts_with("updated-concurrent-"));
        record!(
            "PUT",
            update_route,
            "concurrency-and-idempotency",
            responses.iter().all(|response| response.status == "200 OK") && updated
        );
    }

    assert_eq!(ledger.len(), 58, "WishlistController residual ledger size");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create WishlistController evidence directory");
    fs::write(
        evidence_dir.join("wishlist_controller_residuals.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize WishlistController ledger"),
    )
    .expect("write WishlistController ledger");
    assert!(
        mismatches.is_empty(),
        "{} WishlistController residual mismatches:\n{}",
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
    feature = "bounded-controller-api-tests-4"
))]
async fn controller_api_differential_virtual_soulfind_legacy_residuals() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {
            if !$pass {
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
                "pass": $pass,
            }));
        };
    }

    macro_rules! request {
        ($state:expr, $method:expr, $path:expr, $body:expr) => {{
            super::route_http_request($method, $path, None, $body, $state)
                .await
                .unwrap_or_else(|error| panic!("{path}: {error}", path = $path))
        }};
    }

    fn json_body(response: &super::routing::HttpResponse) -> serde_json::Value {
        serde_json::from_str(&response.body).unwrap_or_default()
    }

    let (empty_state, _receiver) = test_state();
    let (runtime_db, runtime_state, _runtime_receiver) = {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("VirtualSoulfind runtime database");
        let (state, receiver) = test_state_with_env_parts(
            MapEnv::default(),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        db.close_for_test().await;
        (db, state, receiver)
    };
    let _runtime_db = runtime_db;

    let (populated_state, _populated_receiver) = test_state();
    let populated_mbid = "00000000-0000-0000-0000-000000000111";
    let variant_hash = "11".repeat(32);
    {
        let mut discovery = populated_state.content_discovery.write().await;
        discovery
            .merge_hash_entries(vec![super::content_discovery::HashDbEntry {
                flac_key: super::content_discovery::generate_flac_key("Legacy/Variant.flac", 123),
                byte_hash: variant_hash.clone(),
                size: 123,
                full_file_hash: variant_hash.clone(),
                music_brainz_id: populated_mbid.to_owned(),
                file_sha256: variant_hash,
                ..Default::default()
            }])
            .expect("seed legacy VirtualSoulfind variant");
        discovery
            .merge_shadow_records(vec![super::content_discovery::ShadowIndexRecord {
                recording_id: populated_mbid.to_owned(),
                peer_ids: vec!["peer-legacy".to_owned()],
                updated_at: 1,
            }])
            .expect("seed legacy VirtualSoulfind shadow record");
    }

    let canonical_routes = [
        (
            "/api/v0/virtualsoulfind/canonical/{mbid}",
            "/api/v0/virtualsoulfind/canonical/00000000-0000-0000-0000-000000000101",
        ),
        (
            "/api/virtualsoulfind/canonical/{mbid}",
            "/api/virtualsoulfind/canonical/00000000-0000-0000-0000-000000000102",
        ),
    ];
    for (route, nominal_path) in canonical_routes {
        let nominal = request!(&empty_state, "GET", nominal_path, "");
        let nominal_json = json_body(&nominal);
        record!(
            "GET",
            route,
            "nominal-status-headers-body",
            nominal.status == "200 OK"
                && nominal.content_type == "application/json"
                && nominal_json["canonical_variant"].is_null()
                && nominal_json["available_variants"] == 0
                && nominal_json["selection_reason"] == "No variants found in shadow index"
        );

        let malformed_path = if route.starts_with("/api/v0/") {
            "/api/v0/virtualsoulfind/canonical/%20"
        } else {
            "/api/virtualsoulfind/canonical/%20"
        };
        let malformed = request!(&empty_state, "GET", malformed_path, "invalid-json");
        record!(
            "GET",
            route,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
                && malformed.body == r#"{"error":"MBID is required"}"#
        );

        let missing = request!(&empty_state, "GET", nominal_path, "");
        let missing_json = json_body(&missing);
        record!(
            "GET",
            route,
            "missing-empty-or-conflict-state",
            missing.status == "200 OK"
                && missing_json["canonical_variant"].is_null()
                && missing_json["available_variants"] == 0
        );

        let runtime_path = if route.starts_with("/api/v0/") {
            "/api/v0/virtualsoulfind/canonical/runtime-failure"
        } else {
            "/api/virtualsoulfind/canonical/runtime-failure"
        };
        let runtime = request!(&runtime_state, "GET", runtime_path, "");
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            runtime.status == "500 Internal Server Error"
                && runtime.body == r#"{"error":"Failed to select canonical variant"}"#
        );

        let populated_path = if route.starts_with("/api/v0/") {
            format!("/api/v0/virtualsoulfind/canonical/{populated_mbid}")
        } else {
            format!("/api/virtualsoulfind/canonical/{populated_mbid}")
        };
        let populated = request!(&populated_state, "GET", &populated_path, "");
        let populated_json = json_body(&populated);
        record!(
            "GET",
            route,
            "populated-dynamic-state",
            populated.status == "200 OK"
                && populated_json["canonical_variant"]["codec"] == "FLAC"
                && populated_json["canonical_variant"]["bitrate"] == 0
                && populated_json["canonical_variant"]["fileSize"] == 123
                && populated_json["canonical_variant"]["qualityScore"] == 1.0
                && populated_json["available_variants"] == 1
                && populated_json["selection_reason"]
                    == "Selected highest quality variant from shadow index"
        );
    }

    let shadow_routes = [
        (
            "/api/v0/virtualsoulfind/shadow-index/{mbid}",
            "/api/v0/virtualsoulfind/shadow-index/00000000-0000-0000-0000-000000000201",
        ),
        (
            "/api/virtualsoulfind/shadow-index/{mbid}",
            "/api/virtualsoulfind/shadow-index/00000000-0000-0000-0000-000000000202",
        ),
    ];
    for (route, nominal_path) in shadow_routes {
        let nominal = request!(&empty_state, "GET", nominal_path, "");
        let nominal_json = json_body(&nominal);
        record!(
            "GET",
            route,
            "nominal-status-headers-body",
            nominal.status == "200 OK"
                && nominal.content_type == "application/json"
                && nominal_json["variants"]
                    .as_array()
                    .is_some_and(Vec::is_empty)
        );

        let malformed_path = if route.starts_with("/api/v0/") {
            "/api/v0/virtualsoulfind/shadow-index/%20"
        } else {
            "/api/virtualsoulfind/shadow-index/%20"
        };
        let malformed = request!(&empty_state, "GET", malformed_path, "invalid-json");
        record!(
            "GET",
            route,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
                && malformed.body == r#"{"error":"MBID is required"}"#
        );

        let missing = request!(&empty_state, "GET", nominal_path, "");
        let missing_json = json_body(&missing);
        record!(
            "GET",
            route,
            "missing-empty-or-conflict-state",
            missing.status == "200 OK"
                && missing_json["variants"]
                    .as_array()
                    .is_some_and(Vec::is_empty)
        );

        let runtime_path = if route.starts_with("/api/v0/") {
            "/api/v0/virtualsoulfind/shadow-index/runtime-failure"
        } else {
            "/api/virtualsoulfind/shadow-index/runtime-failure"
        };
        let runtime = request!(&runtime_state, "GET", runtime_path, "");
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            runtime.status == "500 Internal Server Error"
                && runtime.body == r#"{"error":"Failed to query shadow index"}"#
        );

        let populated_path = if route.starts_with("/api/v0/") {
            format!("/api/v0/virtualsoulfind/shadow-index/{populated_mbid}")
        } else {
            format!("/api/virtualsoulfind/shadow-index/{populated_mbid}")
        };
        let populated = request!(&populated_state, "GET", &populated_path, "");
        let populated_json = json_body(&populated);
        record!(
            "GET",
            route,
            "populated-dynamic-state",
            populated.status == "200 OK"
                && populated_json["variants"]
                    .as_array()
                    .is_some_and(|variants| {
                        variants.len() == 1
                            && variants[0]["codec"] == "FLAC"
                            && variants[0]["bitrate"] == 0
                            && variants[0]["fileSize"] == 123
                            && variants[0]["qualityScore"] == 1.0
                    })
        );
    }

    let disaster_routes = [
        (
            "/api/v0/virtualsoulfind/disaster-mode/status",
            "/api/v0/virtualsoulfind/disaster-mode/status",
        ),
        (
            "/api/virtualsoulfind/disaster-mode/status",
            "/api/virtualsoulfind/disaster-mode/status",
        ),
    ];
    let (forced_state, _forced_receiver) = test_state_with_env(MapEnv::default().with(
        "SLSKR_ADVANCED_NETWORKING_JSON",
        r#"{"virtualSoulfind":{"disasterMode":{"force":true}}}"#,
    ));
    for (route, path) in disaster_routes {
        let malformed = request!(
            &empty_state,
            "GET",
            &format!("{path}?unexpected=%7B"),
            "invalid-json"
        );
        let malformed_json = json_body(&malformed);
        record!(
            "GET",
            route,
            "malformed-path-query-or-body",
            malformed.status == "200 OK"
                && malformed_json["level"] == 0
                && malformed_json["mode_family"] == "legacy_fallback"
        );

        let missing = request!(&empty_state, "GET", path, "");
        let missing_json = json_body(&missing);
        record!(
            "GET",
            route,
            "missing-empty-or-conflict-state",
            missing.status == "200 OK"
                && missing_json["level_name"] == "Normal"
                && missing_json["is_active"] == false
        );

        let runtime = request!(&runtime_state, "GET", path, "");
        let runtime_json = json_body(&runtime);
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            runtime.status == "200 OK"
                && runtime_json["mode_family"] == "legacy_fallback"
                && runtime_json["networks"].is_object()
        );

        let populated = request!(&forced_state, "GET", path, "");
        let populated_json = json_body(&populated);
        record!(
            "GET",
            route,
            "populated-dynamic-state",
            populated.status == "200 OK"
                && populated_json["level"] == 0
                && populated_json["level_name"] == "Normal"
                && populated_json["is_active"] == false
                && populated_json["networks"]["full_fallback"] == false
        );
    }

    assert_eq!(
        ledger.len(),
        28,
        "legacy VirtualSoulfind residual ledger size"
    );
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create legacy VirtualSoulfind evidence directory");
    fs::write(
        evidence_dir.join("virtualsoulfind_legacy_residuals.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize legacy VirtualSoulfind ledger"),
    )
    .expect("write legacy VirtualSoulfind ledger");
    assert!(
        mismatches.is_empty(),
        "{} legacy VirtualSoulfind residual mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
