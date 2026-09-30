//! Controller full controller api differential 02 ownership.

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
    feature = "bounded-controller-api-tests-4"
))]
pub(super) async fn controller_api_differential_controller_file_transfer_and_room_contracts() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKR_REMOTE_FILE_MANAGEMENT", "true")
            .with("SLSKR_TEST_USER_ENDPOINT_OVERRIDES", "peer=127.0.0.1:2234"),
    );
    let mut ledger = Vec::new();
    macro_rules! record_evidence {
        ($method:expr, $route:expr, $case:expr) => {
            ledger.push(serde_json::json!({
                "target": "slskd",
                "method": $method,
                "route": $route,
                "case": $case,
                "pass": true,
            }));
        };
    }

    let downloads_root = state.config.downloads_dir.clone();
    let incomplete_root = state.config.incomplete_dir.clone();
    std::fs::create_dir_all(downloads_root.join("Artist/Album")).unwrap();
    std::fs::write(downloads_root.join("Artist/Album/Track.flac"), b"track").unwrap();
    std::fs::create_dir_all(incomplete_root.join("Partial")).unwrap();
    std::fs::write(incomplete_root.join("Partial/Track.part"), b"partial").unwrap();

    for (path, route) in [
        (
            "/api/v0/files/downloads/directories?recursive=true",
            "/api/v0/files/downloads/directories",
        ),
        (
            "/api/v0/files/incomplete/directories?recursive=true",
            "/api/v0/files/incomplete/directories",
        ),
    ] {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        assert_eq!(response.status, "200 OK", "GET {path}");
        let json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
        assert!(json["directories"].is_array());
        assert!(json["files"].is_array());
        record_evidence!("GET", route, "nominal-status-headers-body");
        record_evidence!("GET", route, "populated-dynamic-state");
    }

    let nested = crate::route_http_request(
        "GET",
        "/api/v0/files/downloads/directories/QXJ0aXN0L0FsYnVt",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd nested directory listing");
    assert_eq!(nested.status, "200 OK");
    let nested_json = serde_json::from_str::<serde_json::Value>(&nested.body).unwrap();
    assert_eq!(nested_json["files"][0]["name"], "Track.flac");
    assert_eq!(nested_json["files"][0]["length"], 5);
    record_evidence!(
        "GET",
        "/api/v0/files/downloads/directories/{base64SubdirectoryName}",
        "nominal-status-headers-body"
    );
    record_evidence!(
        "GET",
        "/api/v0/files/downloads/directories/{base64SubdirectoryName}",
        "populated-dynamic-state"
    );

    let missing_directory = crate::route_http_request(
        "GET",
        "/api/v0/files/incomplete/directories/TWlzc2luZw==",
        None,
        "",
        &state,
    )
    .await
    .expect("missing slskd directory");
    assert_eq!(missing_directory.status, "404 Not Found");
    record_evidence!(
        "GET",
        "/api/v0/files/incomplete/directories/{base64SubdirectoryName}",
        "missing-empty-or-conflict-state"
    );

    let file_cases = [
        (
            "downloads",
            "files",
            "Remote/Delete.mp3",
            downloads_root.join("Remote/Delete.mp3"),
            "/api/v0/files/downloads/files/{base64FileName}",
        ),
        (
            "downloads",
            "directories",
            "RemoveDownload",
            downloads_root.join("RemoveDownload/file.bin"),
            "/api/v0/files/downloads/directories/{base64SubdirectoryName}",
        ),
        (
            "incomplete",
            "files",
            "Partial/Remove.part",
            incomplete_root.join("Partial/Remove.part"),
            "/api/v0/files/incomplete/files/{base64FileName}",
        ),
        (
            "incomplete",
            "directories",
            "RemovePartial",
            incomplete_root.join("RemovePartial/file.part"),
            "/api/v0/files/incomplete/directories/{base64SubdirectoryName}",
        ),
    ];
    for (storage, resource, relative, file_path, route) in file_cases {
        std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        std::fs::write(&file_path, b"delete me").unwrap();
        let response = crate::route_http_request(
            "DELETE",
            &format!(
                "/api/v0/files/{storage}/{resource}/{}",
                crate::STANDARD.encode(relative)
            ),
            None,
            "",
            &state,
        )
        .await
        .unwrap_or_else(|error| panic!("DELETE {storage}/{resource}/{relative}: {error}"));
        assert_eq!(response.status, "204 No Content");
        assert!(
            !file_path.exists(),
            "deleted path remains: {}",
            file_path.display()
        );
        record_evidence!("DELETE", route, "nominal-status-headers-body");
        record_evidence!("DELETE", route, "mutation-side-effects-and-readback");
        let repeated = crate::route_http_request(
            "DELETE",
            &format!(
                "/api/v0/files/{storage}/{resource}/{}",
                crate::STANDARD.encode(relative)
            ),
            None,
            "",
            &state,
        )
        .await
        .unwrap_or_else(|error| panic!("repeat DELETE {storage}/{resource}/{relative}: {error}"));
        record_evidence!("DELETE", route, "concurrency-and-idempotency");
        assert_eq!(
            repeated.status,
            if resource == "files" {
                "204 No Content"
            } else {
                "404 Not Found"
            }
        );
    }

    let missing_file = crate::route_http_request(
        "DELETE",
        &format!(
            "/api/v0/files/downloads/files/{}",
            crate::STANDARD.encode("Missing.mp3")
        ),
        None,
        "",
        &state,
    )
    .await
    .expect("missing slskd file");
    assert_eq!(missing_file.status, "204 No Content");
    record_evidence!(
        "DELETE",
        "/api/v0/files/downloads/files/{base64FileName}",
        "missing-empty-or-conflict-state"
    );

    let traversal = crate::route_http_request(
        "DELETE",
        &format!(
            "/api/v0/files/downloads/files/{}",
            crate::STANDARD.encode("../secret")
        ),
        None,
        "",
        &state,
    )
    .await
    .expect("slskd traversal file");
    assert_eq!(traversal.status, "400 Bad Request");
    record_evidence!(
        "DELETE",
        "/api/v0/files/downloads/files/{base64FileName}",
        "malformed-path-query-or-body"
    );

    let reset_state_dir = state.config.state_dir.display().to_string();
    let (reset_state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKR_REMOTE_FILE_MANAGEMENT", "true")
            .with("SLSKR_STATE_DIR", &reset_state_dir),
    );
    for (storage, resource, relative, route) in [
        (
            "downloads",
            "files",
            "Remote/Delete.mp3",
            "/api/v0/files/downloads/files/{base64FileName}",
        ),
        (
            "downloads",
            "directories",
            "RemoveDownload",
            "/api/v0/files/downloads/directories/{base64SubdirectoryName}",
        ),
        (
            "incomplete",
            "files",
            "Partial/Remove.part",
            "/api/v0/files/incomplete/files/{base64FileName}",
        ),
        (
            "incomplete",
            "directories",
            "RemovePartial",
            "/api/v0/files/incomplete/directories/{base64SubdirectoryName}",
        ),
    ] {
        let response = crate::route_http_request(
            "DELETE",
            &format!(
                "/api/v0/files/{storage}/{resource}/{}",
                crate::STANDARD.encode(relative)
            ),
            None,
            "",
            &reset_state,
        )
        .await
        .unwrap_or_else(|error| panic!("reset DELETE {storage}/{resource}/{relative}: {error}"));
        assert_eq!(
            response.status,
            if resource == "files" {
                "204 No Content"
            } else {
                "404 Not Found"
            }
        );
        record_evidence!("DELETE", route, "restart-persistence-or-reset");
    }

    let downloads_conflict_root = std::env::temp_dir().join(format!(
        "slskr-download-delete-conflict-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    let incomplete_conflict_root = std::env::temp_dir().join(format!(
        "slskr-incomplete-delete-conflict-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    fs::write(&downloads_conflict_root, b"downloads root is a file")
        .expect("create downloads delete conflict");
    fs::write(&incomplete_conflict_root, b"incomplete root is a file")
        .expect("create incomplete delete conflict");
    let (failure_state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKR_REMOTE_FILE_MANAGEMENT", "true"),
    );
    *failure_state
        .downloads_dir
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = downloads_conflict_root.clone();
    *failure_state
        .incomplete_dir
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = incomplete_conflict_root.clone();
    for (storage, resource, relative, route) in [
        (
            "downloads",
            "files",
            "RuntimeFailure.mp3",
            "/api/v0/files/downloads/files/{base64FileName}",
        ),
        (
            "downloads",
            "directories",
            "RuntimeFailureDirectory",
            "/api/v0/files/downloads/directories/{base64SubdirectoryName}",
        ),
        (
            "incomplete",
            "files",
            "RuntimeFailure.part",
            "/api/v0/files/incomplete/files/{base64FileName}",
        ),
        (
            "incomplete",
            "directories",
            "RuntimeFailureDirectory",
            "/api/v0/files/incomplete/directories/{base64SubdirectoryName}",
        ),
    ] {
        let response = crate::route_http_request(
            "DELETE",
            &format!(
                "/api/v0/files/{storage}/{resource}/{}",
                crate::STANDARD.encode(relative)
            ),
            None,
            "",
            &failure_state,
        )
        .await
        .unwrap_or_else(|error| panic!("runtime DELETE {storage}/{resource}/{relative}: {error}"));
        assert_eq!(response.status, "503 Service Unavailable");
        assert_eq!(response.body, r#"{"error":"file storage unavailable"}"#);
        assert!(!response.body.contains("root is a file"));
        record_evidence!("DELETE", route, "runtime-failure-and-timeout");
    }
    for (storage, relative, route) in [
        (
            "downloads",
            "RuntimeFailureDirectory",
            "/api/v0/files/downloads/directories/{base64SubdirectoryName}",
        ),
        (
            "incomplete",
            "RuntimeFailureDirectory",
            "/api/v0/files/incomplete/directories/{base64SubdirectoryName}",
        ),
    ] {
        let response = crate::route_http_request(
            "GET",
            &format!(
                "/api/v0/files/{storage}/directories/{}",
                crate::STANDARD.encode(relative)
            ),
            None,
            "",
            &failure_state,
        )
        .await
        .unwrap_or_else(|error| panic!("runtime GET {storage}/{relative}: {error}"));
        assert_eq!(response.status, "503 Service Unavailable");
        assert_eq!(response.body, r#"{"error":"file storage unavailable"}"#);
        assert!(!response.body.contains("root is a file"));
        record_evidence!("GET", route, "runtime-failure-and-timeout");
    }
    #[cfg(unix)]
    {
        let downloads_target = std::env::temp_dir().join(format!(
            "slskr-download-list-target-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        let incomplete_target = std::env::temp_dir().join(format!(
            "slskr-incomplete-list-target-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        let downloads_link = std::env::temp_dir().join(format!(
            "slskr-download-list-link-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        let incomplete_link = std::env::temp_dir().join(format!(
            "slskr-incomplete-list-link-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        fs::create_dir_all(&downloads_target).expect("create downloads list target");
        fs::create_dir_all(&incomplete_target).expect("create incomplete list target");
        std::os::unix::fs::symlink(&downloads_target, &downloads_link)
            .expect("create downloads list symlink");
        std::os::unix::fs::symlink(&incomplete_target, &incomplete_link)
            .expect("create incomplete list symlink");
        let (symlink_state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_CONTROLLER_PROFILE", "legacy")
                .with("SLSKR_REMOTE_FILE_MANAGEMENT", "true"),
        );
        *symlink_state
            .downloads_dir
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = downloads_link.clone();
        *symlink_state
            .incomplete_dir
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = incomplete_link.clone();
        for (storage, route) in [
            ("downloads", "/api/v0/files/downloads/directories"),
            ("incomplete", "/api/v0/files/incomplete/directories"),
        ] {
            let response = crate::route_http_request(
                "GET",
                &format!("/api/v0/files/{storage}/directories"),
                None,
                "",
                &symlink_state,
            )
            .await
            .unwrap_or_else(|error| panic!("runtime root GET {storage}: {error}"));
            assert_eq!(response.status, "503 Service Unavailable");
            assert_eq!(response.body, r#"{"error":"file storage unavailable"}"#);
            record_evidence!("GET", route, "runtime-failure-and-timeout");
        }
        let _ = fs::remove_file(downloads_link);
        let _ = fs::remove_file(incomplete_link);
        let _ = fs::remove_dir_all(downloads_target);
        let _ = fs::remove_dir_all(incomplete_target);
    }
    let _ = fs::remove_file(downloads_conflict_root);
    let _ = fs::remove_file(incomplete_conflict_root);

    let enqueue = crate::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/peer",
        None,
        r#"{"files":[{"filename":"Remote/Queued.flac","size":99}]}"#,
        &state,
    )
    .await
    .expect("slskd transfer enqueue");
    assert_eq!(enqueue.status, "200 OK");
    let enqueue_json = serde_json::from_str::<serde_json::Value>(&enqueue.body).unwrap();
    assert_eq!(enqueue_json["queued"], 1);
    assert_eq!(enqueue_json["transfers"][0]["username"], "peer");
    let transfer_id = enqueue_json["transfers"][0]["id"]
        .as_u64()
        .or_else(|| {
            enqueue_json["transfers"][0]["id"]
                .as_str()
                .and_then(|value| value.parse::<u64>().ok())
        })
        .unwrap_or_else(|| panic!("slskd transfer id missing from {}", enqueue.body));
    record_evidence!(
        "POST",
        "/api/v0/transfers/downloads/{username}",
        "nominal-status-headers-body"
    );
    record_evidence!(
        "POST",
        "/api/v0/transfers/downloads/{username}",
        "mutation-side-effects-and-readback"
    );

    let transfer_list =
        crate::route_http_request("GET", "/api/v0/transfers/downloads", None, "", &state)
            .await
            .expect("slskd transfer list");
    assert_eq!(transfer_list.status, "200 OK");
    let transfer_list_json =
        serde_json::from_str::<serde_json::Value>(&transfer_list.body).unwrap();
    assert_eq!(transfer_list_json[0]["username"], "peer");
    assert_eq!(
        transfer_list_json[0]["directories"][0]["directory"],
        "Remote"
    );
    record_evidence!(
        "GET",
        "/api/v0/transfers/downloads",
        "nominal-status-headers-body"
    );
    record_evidence!(
        "GET",
        "/api/v0/transfers/downloads",
        "populated-dynamic-state"
    );

    let transfer_detail = crate::route_http_request(
        "GET",
        &format!("/api/v0/transfers/downloads/peer/{transfer_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("slskd transfer detail");
    assert_eq!(transfer_detail.status, "200 OK");
    let transfer_detail_json =
        serde_json::from_str::<serde_json::Value>(&transfer_detail.body).unwrap();
    assert!(
        transfer_detail_json["id"] == serde_json::json!(transfer_id)
            || transfer_detail_json["id"] == serde_json::json!(transfer_id.to_string()),
        "unexpected slskd transfer id: {}",
        transfer_detail.body
    );
    record_evidence!(
        "GET",
        "/api/v0/transfers/downloads/{username}/{id}",
        "nominal-status-headers-body"
    );
    record_evidence!(
        "GET",
        "/api/v0/transfers/downloads/{username}/{id}",
        "populated-dynamic-state"
    );

    let missing_transfer = crate::route_http_request(
        "GET",
        &format!("/api/v0/transfers/downloads/other/{transfer_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("missing slskd transfer");
    assert_eq!(missing_transfer.status, "404 Not Found");
    record_evidence!(
        "GET",
        "/api/v0/transfers/downloads/{username}/{id}",
        "missing-empty-or-conflict-state"
    );

    let cancelled = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/transfers/downloads/peer/{transfer_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("cancel slskd transfer");
    assert_eq!(cancelled.status, "204 No Content");
    record_evidence!(
        "DELETE",
        "/api/v0/transfers/downloads/{username}/{id}",
        "nominal-status-headers-body"
    );
    record_evidence!(
        "DELETE",
        "/api/v0/transfers/downloads/{username}/{id}",
        "mutation-side-effects-and-readback"
    );

    let batch_id = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
    let batch_request = format!(
        r#"{{"id":"{batch_id}","username":"peer","files":[{{"filename":"Music/A.flac","size":42}}]}}"#
    );
    let batch = crate::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/batches",
        None,
        &batch_request,
        &state,
    )
    .await
    .expect("slskd transfer batch");
    assert_eq!(batch.status, "201 Created");
    let batch_json = serde_json::from_str::<serde_json::Value>(&batch.body).unwrap();
    assert_eq!(batch_json["batch"]["id"], batch_id);
    record_evidence!(
        "POST",
        "/api/v0/transfers/downloads/batches",
        "nominal-status-headers-body"
    );
    record_evidence!(
        "POST",
        "/api/v0/transfers/downloads/batches",
        "mutation-side-effects-and-readback"
    );

    let fetched_batch = crate::route_http_request(
        "GET",
        &format!("/api/v0/transfers/downloads/batches/{batch_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("slskd fetched transfer batch");
    assert_eq!(fetched_batch.status, "200 OK");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&fetched_batch.body).unwrap()["id"],
        batch_id
    );
    record_evidence!(
        "GET",
        "/api/v0/transfers/downloads/batches/{id}",
        "nominal-status-headers-body"
    );
    record_evidence!(
        "GET",
        "/api/v0/transfers/downloads/batches/{id}",
        "populated-dynamic-state"
    );

    let duplicate_batch = crate::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/batches",
        None,
        &batch_request,
        &state,
    )
    .await
    .expect("duplicate slskd transfer batch");
    assert_eq!(duplicate_batch.status, "409 Conflict");
    record_evidence!(
        "POST",
        "/api/v0/transfers/downloads/batches",
        "missing-empty-or-conflict-state"
    );

    state.session.write().await.state = "connected";
    for (method, path, route) in [
        ("GET", "/api/v0/rooms/joined", "/api/v0/rooms/joined"),
        ("GET", "/api/v0/rooms/available", "/api/v0/rooms/available"),
    ] {
        let response = crate::route_http_request(method, path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
        assert_eq!(response.status, "200 OK");
        assert!(serde_json::from_str::<serde_json::Value>(&response.body)
            .unwrap()
            .is_array());
        record_evidence!(method, route, "nominal-status-headers-body");
    }

    for (method, path, route) in [
        (
            "GET",
            "/api/v0/rooms/joined/missing",
            "/api/v0/rooms/joined/{roomName}",
        ),
        (
            "GET",
            "/api/v0/rooms/joined/missing/users",
            "/api/v0/rooms/joined/{roomName}/users",
        ),
        (
            "GET",
            "/api/v0/rooms/joined/missing/messages",
            "/api/v0/rooms/joined/{roomName}/messages",
        ),
        (
            "DELETE",
            "/api/v0/rooms/joined/missing",
            "/api/v0/rooms/joined/{roomName}",
        ),
        (
            "POST",
            "/api/v0/rooms/joined/missing/messages",
            "/api/v0/rooms/joined/{roomName}/messages",
        ),
        (
            "POST",
            "/api/v0/rooms/joined/missing/ticker",
            "/api/v0/rooms/joined/{roomName}/ticker",
        ),
        (
            "POST",
            "/api/v0/rooms/joined/missing/members",
            "/api/v0/rooms/joined/{roomName}/members",
        ),
    ] {
        let response = crate::route_http_request(method, path, None, r#""value""#, &state)
            .await
            .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
        assert_eq!(response.status, "404 Not Found", "{method} {path}");
        record_evidence!(method, route, "missing-empty-or-conflict-state");
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("controller_file_transfer_room_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskd controller ledger"),
    )
    .expect("write slskd controller ledger");
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
pub(super) async fn controller_api_differential_controller_file_application_and_roster_edges() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKR_REMOTE_FILE_MANAGEMENT", "true"),
    );
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!(
                    "slskd {} {} [{}]",
                    $method, $route, $case
                ));
            }
            ledger.push(serde_json::json!({
                "target": "slskd",
                "method": $method,
                "route": $route,
                "case": $case,
                "pass": pass,
            }));
        }};
    }

    let incomplete_root = state.config.incomplete_dir.clone();
    std::fs::create_dir_all(incomplete_root.join("Nested")).unwrap();
    std::fs::write(incomplete_root.join("Nested/Track.part"), b"partial").unwrap();
    let nested = crate::route_http_request(
        "GET",
        "/api/v0/files/incomplete/directories/TmVzdGVk",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd populated incomplete directory");
    let nested_json = serde_json::from_str::<serde_json::Value>(&nested.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/files/incomplete/directories/{base64SubdirectoryName}",
        "nominal-status-headers-body",
        nested.status == "200 OK"
            && nested_json["files"]
                .as_array()
                .is_some_and(|files| { files.iter().any(|file| file["name"] == "Track.part") })
    );
    record!(
        "GET",
        "/api/v0/files/incomplete/directories/{base64SubdirectoryName}",
        "populated-dynamic-state",
        nested.status == "200 OK"
            && nested_json["files"]
                .as_array()
                .is_some_and(|files| !files.is_empty())
    );

    for (path, route) in [
        (
            "/api/v0/files/downloads/directories?recursive=not-a-boolean",
            "/api/v0/files/downloads/directories",
        ),
        (
            "/api/v0/files/downloads/directories/TmVzdGVk?recursive=not-a-boolean",
            "/api/v0/files/downloads/directories/{base64SubdirectoryName}",
        ),
        (
            "/api/v0/files/incomplete/directories?recursive=not-a-boolean",
            "/api/v0/files/incomplete/directories",
        ),
        (
            "/api/v0/files/incomplete/directories/TmVzdGVk?recursive=not-a-boolean",
            "/api/v0/files/incomplete/directories/{base64SubdirectoryName}",
        ),
    ] {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        record!(
            "GET",
            route,
            "malformed-path-query-or-body",
            response.status == "400 Bad Request"
        );
    }

    let missing_download_directory = crate::route_http_request(
        "GET",
        "/api/v0/files/downloads/directories/Tm9TdWNoRGlyZWN0b3J5",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd missing downloads directory");
    record!(
        "GET",
        "/api/v0/files/downloads/directories/{base64SubdirectoryName}",
        "missing-empty-or-conflict-state",
        missing_download_directory.status == "404 Not Found"
    );

    for (resource, encoded, route) in [
        (
            "directories",
            "Tm9TdWNoRGlyZWN0b3J5",
            "/api/v0/files/downloads/directories/{base64SubdirectoryName}",
        ),
        (
            "directories",
            "Tm9TdWNoRGlyZWN0b3J5",
            "/api/v0/files/incomplete/directories/{base64SubdirectoryName}",
        ),
        (
            "files",
            "Tm9TdWNoRmlsZS5wYXJ0",
            "/api/v0/files/incomplete/files/{base64FileName}",
        ),
    ] {
        let storage = if route.contains("/downloads/") {
            "downloads"
        } else {
            "incomplete"
        };
        let response = crate::route_http_request(
            "DELETE",
            &format!("/api/v0/files/{storage}/{resource}/{encoded}"),
            None,
            "",
            &state,
        )
        .await
        .unwrap_or_else(|error| panic!("DELETE {storage}/{resource}: {error}"));
        record!(
            "DELETE",
            route,
            "missing-empty-or-conflict-state",
            response.status
                == if resource == "files" {
                    "204 No Content"
                } else {
                    "404 Not Found"
                }
        );
    }

    let version = crate::route_http_request("GET", "/api/v0/application/version", None, "", &state)
        .await
        .expect("slskd populated application version");
    record!(
        "GET",
        "/api/v0/application/version",
        "populated-dynamic-state",
        version.status == "200 OK" && !version.body.is_empty()
    );

    crate::record_daemon_log(
        &state,
        crate::logging::LogLevel::Info,
        "slskd.edge",
        "populated log entry",
    )
    .await;
    let logs = crate::route_http_request("GET", "/api/v0/logs", None, "", &state)
        .await
        .expect("slskd populated logs");
    let logs_json = serde_json::from_str::<serde_json::Value>(&logs.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/logs",
        "populated-dynamic-state",
        logs.status == "200 OK"
            && logs_json.as_array().is_some_and(|entries| {
                entries
                    .iter()
                    .any(|entry| entry["category"] == "slskd.edge")
            })
    );

    {
        let mut session = state.session.write().await;
        session.state = "connected";
        session.username = Some("edge-user".to_owned());
    }
    let session = crate::route_http_request("GET", "/api/v0/session", None, "", &state)
        .await
        .expect("slskd populated session");
    let session_json = serde_json::from_str::<serde_json::Value>(&session.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/session",
        "populated-dynamic-state",
        session.status == "200 OK" && session_json["state"] == "connected"
    );

    let (roster_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));
    roster_state.session.write().await.state = "connected";
    roster_state.session.write().await.username = Some("edge-self".to_owned());
    {
        let mut rooms = roster_state.rooms.write().await;
        let room = rooms.join("roster-edge".to_owned()).expect("room fixture");
        let stored = rooms
            .records
            .iter_mut()
            .find(|candidate| candidate.name == room.name)
            .expect("stored room fixture");
        stored.members = vec!["edge-self".to_owned(), "edge-peer".to_owned()];
        stored.roster = vec![
            crate::RoomRosterEntry {
                username: "edge-self".to_owned(),
                status: 2,
                average_speed: 1_000,
                upload_count: 5,
                file_count: 42,
                directory_count: 3,
                slots_free: 1,
                country_code: "CA".to_owned(),
            },
            crate::RoomRosterEntry {
                username: "edge-peer".to_owned(),
                status: 1,
                average_speed: 0,
                upload_count: 0,
                file_count: 0,
                directory_count: 0,
                slots_free: 0,
                country_code: String::new(),
            },
        ];
    }
    let roster = crate::route_http_request(
        "GET",
        "/api/v0/rooms/joined/roster-edge/users",
        None,
        "",
        &roster_state,
    )
    .await
    .expect("slskd populated room roster");
    let roster_json = serde_json::from_str::<serde_json::Value>(&roster.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/rooms/joined/{roomName}/users",
        "nominal-status-headers-body",
        roster.status == "200 OK" && roster_json.as_array().is_some_and(|users| users.len() == 2)
    );
    record!(
        "GET",
        "/api/v0/rooms/joined/{roomName}/users",
        "populated-dynamic-state",
        roster.status == "200 OK" && roster_json[0]["username"] == "edge-self"
    );
    let available =
        crate::route_http_request("GET", "/api/v0/rooms/available", None, "", &roster_state)
            .await
            .expect("slskd populated available rooms");
    let available_json =
        serde_json::from_str::<serde_json::Value>(&available.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/rooms/available",
        "populated-dynamic-state",
        available.status == "200 OK"
            && available_json
                .as_array()
                .is_some_and(|rooms| { rooms.iter().any(|room| room["name"] == "roster-edge") })
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("controller_file_application_roster_edges.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskd edge ledger"),
    )
    .expect("write slskd edge ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskd edge mismatches: {}",
        mismatches.len(),
        mismatches.join("; ")
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
pub(super) async fn controller_api_differential_controller_share_and_relay_lifecycle() {
    let target = "slskd";
    let relay_env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKD_RELAY", "true")
        .with("SLSKD_RELAY_MODE", "agent")
        .with("SLSKD_CONTROLLER_ADDRESS", "http://127.0.0.1:9")
        .with("SLSKD_CONTROLLER_API_KEY", "relay-api-key-123456")
        .with("SLSKD_CONTROLLER_SECRET", "relay-secret-123456");
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
        .expect("relay lifecycle database");
    let (state, _receiver) = test_state_with_env_parts(
        relay_env.clone(),
        crate::SearchStore::new(),
        Some(db.clone()),
    );
    let started = crate::route_http_request("PUT", "/api/v0/relay/agent", None, "", &state)
        .await
        .expect("slskd relay-agent start");
    let started_row = db
        .get_runtime_compat_state()
        .await
        .expect("read relay-agent start")
        .expect("relay-agent runtime row");
    record!(
        "PUT",
        "/api/v0/relay/agent",
        "nominal-status-headers-body",
        started.status == "200 OK" && started.body.is_empty()
    );
    record!(
        "PUT",
        "/api/v0/relay/agent",
        "mutation-side-effects-and-readback",
        state.runtime.read().await.relay_agent_enabled
            && started_row.relay_agent_enabled
            && started_row.relay_enabled
    );

    let repeated_start = crate::route_http_request("PUT", "/api/v0/relay/agent", None, "", &state)
        .await
        .expect("repeat slskd relay-agent start");
    record!(
        "PUT",
        "/api/v0/relay/agent",
        "concurrency-and-idempotency",
        repeated_start.status == "200 OK" && repeated_start.body.is_empty()
    );

    let stopped = crate::route_http_request("DELETE", "/api/v0/relay/agent", None, "", &state)
        .await
        .expect("slskd relay-agent stop");
    let stopped_row = db
        .get_runtime_compat_state()
        .await
        .expect("read relay-agent stop")
        .expect("relay-agent stopped runtime row");
    record!(
        "DELETE",
        "/api/v0/relay/agent",
        "nominal-status-headers-body",
        stopped.status == "204 No Content" && stopped.body.is_empty()
    );
    record!(
        "DELETE",
        "/api/v0/relay/agent",
        "mutation-side-effects-and-readback",
        !state.runtime.read().await.relay_agent_enabled && !stopped_row.relay_agent_enabled
    );

    let repeated_stop =
        crate::route_http_request("DELETE", "/api/v0/relay/agent", None, "", &state)
            .await
            .expect("repeat slskd relay-agent stop");
    record!(
        "DELETE",
        "/api/v0/relay/agent",
        "concurrency-and-idempotency",
        repeated_stop.status == "204 No Content" && repeated_stop.body.is_empty()
    );

    let (restarted_relay_state, _receiver) = test_state_with_env(relay_env.clone());
    let restarted_start = crate::route_http_request(
        "PUT",
        "/api/v0/relay/agent",
        None,
        "",
        &restarted_relay_state,
    )
    .await
    .expect("restarted slskd relay-agent start");
    record!(
        "PUT",
        "/api/v0/relay/agent",
        "restart-persistence-or-reset",
        restarted_start.status == "200 OK"
            && restarted_start.body.is_empty()
            && restarted_relay_state
                .runtime
                .read()
                .await
                .relay_agent_enabled
    );
    let restarted_stop = crate::route_http_request(
        "DELETE",
        "/api/v0/relay/agent",
        None,
        "",
        &restarted_relay_state,
    )
    .await
    .expect("restarted slskd relay-agent stop");
    record!(
        "DELETE",
        "/api/v0/relay/agent",
        "restart-persistence-or-reset",
        restarted_stop.status == "204 No Content"
            && restarted_stop.body.is_empty()
            && !restarted_relay_state
                .runtime
                .read()
                .await
                .relay_agent_enabled
    );

    let failure_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("relay failure database");
    let (failure_state, _receiver) = test_state_with_env_parts(
        relay_env,
        crate::SearchStore::new(),
        Some(failure_db.clone()),
    );
    failure_db.close_for_test().await;
    let failed_start =
        crate::route_http_request("PUT", "/api/v0/relay/agent", None, "", &failure_state)
            .await
            .expect("failed relay-agent start");
    record!(
        "PUT",
        "/api/v0/relay/agent",
        "runtime-failure-and-timeout",
        failed_start.status == "503 Service Unavailable"
            && !failure_state.runtime.read().await.relay_agent_enabled
    );
    let failed_stop =
        crate::route_http_request("DELETE", "/api/v0/relay/agent", None, "", &failure_state)
            .await
            .expect("failed relay-agent stop");
    record!(
        "DELETE",
        "/api/v0/relay/agent",
        "runtime-failure-and-timeout",
        failed_stop.status == "503 Service Unavailable"
            && !failure_state.runtime.read().await.relay_agent_enabled
    );

    let (disabled_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    for method in ["PUT", "DELETE"] {
        let response =
            crate::route_http_request(method, "/api/v0/relay/agent", None, "", &disabled_state)
                .await
                .unwrap_or_else(|error| panic!("disabled relay-agent {method}: {error}"));
        record!(
            method,
            "/api/v0/relay/agent",
            "missing-empty-or-conflict-state",
            response.status == "403 Forbidden"
        );
    }

    let (share_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let scan_permit = share_state
        .share_scans
        .clone()
        .acquire_owned()
        .await
        .expect("occupy share scan permit");
    let cancelled = crate::route_http_request("DELETE", "/api/v0/shares", None, "", &share_state)
        .await
        .expect("slskd share scan cancellation");
    record!(
        "DELETE",
        "/api/v0/shares",
        "nominal-status-headers-body",
        cancelled.status == "204 No Content" && cancelled.body.is_empty()
    );
    record!(
        "DELETE",
        "/api/v0/shares",
        "mutation-side-effects-and-readback",
        cancelled.status == "204 No Content" && share_state.share_scans.available_permits() == 1
    );
    let repeated_cancel =
        crate::route_http_request("DELETE", "/api/v0/shares", None, "", &share_state)
            .await
            .expect("repeat slskd share scan cancellation");
    record!(
        "DELETE",
        "/api/v0/shares",
        "concurrency-and-idempotency",
        repeated_cancel.status == "404 Not Found"
    );
    drop(scan_permit);

    let (restarted_share_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let reset =
        crate::route_http_request("DELETE", "/api/v0/shares", None, "", &restarted_share_state)
            .await
            .expect("slskd share scan restart state");
    record!(
        "DELETE",
        "/api/v0/shares",
        "restart-persistence-or-reset",
        reset.status == "404 Not Found"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("controller_share_relay_lifecycle.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskd share/relay ledger"),
    )
    .expect("write slskd share/relay ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskd share/relay controller mismatches:\n{}",
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
pub(super) async fn controller_api_differential_controller_fixed_route_malformed_paths() {
    let target = "slskd";
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($method:expr, $route:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!(
                    "{target} {} {} [malformed-path-query-or-body]",
                    $method, $route
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": $method,
                "route": $route,
                "case": "malformed-path-query-or-body",
                "pass": pass,
            }));
        }};
    }

    for (method, route) in [
        ("DELETE", "/api/v0/application"),
        ("DELETE", "/api/v0/relay/agent"),
        ("DELETE", "/api/v0/server"),
        ("DELETE", "/api/v0/shares"),
        ("GET", "/api/v0/application"),
        ("GET", "/api/v0/application/dump"),
        ("GET", "/api/v0/application/version"),
        ("GET", "/api/v0/application/version/latest"),
        ("GET", "/api/v0/logs"),
        ("GET", "/api/v0/rooms/available"),
        ("GET", "/api/v0/server"),
        ("GET", "/api/v0/shares/contents"),
        ("GET", "/api/v0/telemetry/metrics"),
        ("GET", "/api/v0/telemetry/metrics/kpis"),
        ("GET", "/api/v0/telemetry/reports/transfers/summary"),
        ("POST", "/api/v0/application/gc"),
        ("POST", "/api/v0/application/loopback"),
        ("POST", "/api/v0/rooms/joined"),
        ("PUT", "/api/v0/application"),
        ("PUT", "/api/v0/options"),
        ("PUT", "/api/v0/server"),
        ("PUT", "/api/v0/shares"),
    ] {
        let malformed_path = format!("{route}/malformed");
        let response = crate::route_http_request(method, &malformed_path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{method} {malformed_path}: {error}"));
        record!(method, route, response.status == "404 Not Found");
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("controller_fixed_route_malformed_paths.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskd malformed-path ledger"),
    )
    .expect("write slskd malformed-path ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskd malformed-path mismatches:\n{}",
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
pub(super) async fn controller_api_differential_controller_parameterized_malformed_paths() {
    let target = "slskd";
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_REMOTE_FILE_MANAGEMENT", "true"),
    );
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($method:expr, $route:expr, $path:expr, $expected:expr) => {{
            let response = crate::route_http_request($method, $path, None, "", &state)
                .await
                .unwrap_or_else(|error| panic!("{} {}: {error}", $method, $path));
            let pass = response.status == $expected;
            if !pass {
                mismatches.push(format!(
                    "{target} {} {} expected {}, got {}",
                    $method, $path, $expected, response.status
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": $method,
                "route": $route,
                "case": "malformed-path-query-or-body",
                "pass": pass,
            }));
        }};
    }

    record!(
        "DELETE",
        "/api/v0/conversations/{username}",
        "/api/v0/conversations/peer/extra",
        "404 Not Found"
    );
    record!(
        "DELETE",
        "/api/v0/files/downloads/directories/{base64SubdirectoryName}",
        "/api/v0/files/downloads/directories/TmVzdGVk/extra",
        "404 Not Found"
    );
    record!(
        "DELETE",
        "/api/v0/files/incomplete/directories/{base64SubdirectoryName}",
        "/api/v0/files/incomplete/directories/TmVzdGVk/extra",
        "404 Not Found"
    );
    record!(
        "DELETE",
        "/api/v0/files/incomplete/files/{base64FileName}",
        "/api/v0/files/incomplete/files/VHJhY2sucGFydA==/extra",
        "404 Not Found"
    );
    record!(
        "DELETE",
        "/api/v0/rooms/joined/{roomName}",
        "/api/v0/rooms/joined/music/extra",
        "404 Not Found"
    );
    record!(
        "DELETE",
        "/api/v0/searches/{id}",
        "/api/v0/searches/not-a-guid/extra",
        "404 Not Found"
    );
    record!(
        "DELETE",
        "/api/v0/transfers/downloads/all/completed",
        "/api/v0/transfers/downloads/all/completed/extra",
        "404 Not Found"
    );
    record!(
        "DELETE",
        "/api/v0/transfers/uploads/all/completed",
        "/api/v0/transfers/uploads/all/completed/extra",
        "404 Not Found"
    );
    record!(
        "DELETE",
        "/api/v0/transfers/uploads/{username}/{id}",
        "/api/v0/transfers/uploads/peer/1/extra",
        "404 Not Found"
    );

    record!(
        "GET",
        "/api/v0/conversations",
        "/api/v0/conversations?includeInactive=not-a-boolean",
        "400 Bad Request"
    );
    record!(
        "GET",
        "/api/v0/events",
        "/api/v0/events/malformed",
        "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/options",
        "/api/v0/options/malformed",
        "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/relay/controller/downloads/{token}",
        "/api/v0/relay/controller/downloads/not-a-guid/extra",
        "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/rooms/joined",
        "/api/v0/rooms/joined/malformed/extra",
        "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/rooms/joined/{roomName}",
        "/api/v0/rooms/joined/music/extra",
        "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/rooms/joined/{roomName}/messages",
        "/api/v0/rooms/joined/music/messages/extra",
        "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/rooms/joined/{roomName}/users",
        "/api/v0/rooms/joined/music/users/extra",
        "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/searches",
        "/api/v0/searches/not-a-guid/extra",
        "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/searches/{id}",
        "/api/v0/searches/not-a-guid/extra",
        "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/searches/{id}/responses",
        "/api/v0/searches/not-a-guid/extra/responses",
        "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/session",
        "/api/v0/session/malformed",
        "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/shares",
        "/api/v0/shares/extra/path",
        "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/shares/{id}",
        "/api/v0/shares/extra/path",
        "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/shares/{id}/contents",
        "/api/v0/shares/extra/contents/extra",
        "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/shares/contents",
        "/api/v0/shares/contents/extra",
        "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/telemetry/reports/transfers/directories",
        "/api/v0/telemetry/reports/transfers/directories/extra",
        "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/telemetry/reports/transfers/users/{username}",
        "/api/v0/telemetry/reports/transfers/users/peer/extra",
        "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/transfers/downloads",
        "/api/v0/transfers/downloads/extra/path",
        "400 Bad Request"
    );
    record!(
        "GET",
        "/api/v0/transfers/downloads/{username}",
        "/api/v0/transfers/downloads/peer/extra/path",
        "400 Bad Request"
    );
    record!(
        "GET",
        "/api/v0/transfers/uploads",
        "/api/v0/transfers/uploads/extra/path",
        "400 Bad Request"
    );
    record!(
        "GET",
        "/api/v0/transfers/uploads/{username}",
        "/api/v0/transfers/uploads/peer/extra/path",
        "400 Bad Request"
    );
    record!(
        "GET",
        "/api/v0/users/{username}/browse",
        "/api/v0/users/peer/browse/extra",
        "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/users/{username}/browse/status",
        "/api/v0/users/peer/browse/status/extra",
        "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/users/{username}/endpoint",
        "/api/v0/users/peer/endpoint/extra",
        "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/users/{username}/info",
        "/api/v0/users/peer/info/extra",
        "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/users/{username}/status",
        "/api/v0/users/peer/status/extra",
        "404 Not Found"
    );

    record!(
        "POST",
        "/api/v0/options",
        "/api/v0/options/malformed",
        "404 Not Found"
    );
    record!(
        "POST",
        "/api/v0/relay/controller/files/{token}",
        "/api/v0/relay/controller/files/not-a-guid/extra",
        "404 Not Found"
    );
    record!(
        "POST",
        "/api/v0/relay/controller/shares/{token}",
        "/api/v0/relay/controller/shares/not-a-guid/extra",
        "404 Not Found"
    );
    record!(
        "POST",
        "/api/v0/rooms/joined/{roomName}/members",
        "/api/v0/rooms/joined/music/members/extra",
        "404 Not Found"
    );
    record!(
        "POST",
        "/api/v0/rooms/joined/{roomName}/messages",
        "/api/v0/rooms/joined/music/messages/extra",
        "404 Not Found"
    );
    record!(
        "POST",
        "/api/v0/rooms/joined/{roomName}/ticker",
        "/api/v0/rooms/joined/music/ticker/extra",
        "404 Not Found"
    );

    record!(
        "PUT",
        "/api/v0/conversations/{username}",
        "/api/v0/conversations/peer/extra/extra",
        "404 Not Found"
    );
    record!(
        "PUT",
        "/api/v0/conversations/{username}/{id}",
        "/api/v0/conversations/peer/1/extra",
        "404 Not Found"
    );
    record!(
        "PUT",
        "/api/v0/relay/agent",
        "/api/v0/relay/agent/malformed",
        "404 Not Found"
    );
    record!(
        "PUT",
        "/api/v0/searches/{id}",
        "/api/v0/searches/not-a-guid/extra",
        "404 Not Found"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("controller_parameterized_malformed_paths.json"),
        serde_json::to_string_pretty(&ledger)
            .expect("serialize slskd parameterized malformed ledger"),
    )
    .expect("write slskd parameterized malformed ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskd parameterized malformed-path mismatches:\n{}",
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
pub(super) async fn controller_api_differential_controller_empty_and_missing_state() {
    let target = "slskd";
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let (relay_state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKD_RELAY", "true")
            .with("SLSKD_RELAY_MODE", "controller")
            .with("SLSKD_CONTROLLER_ADDRESS", "http://127.0.0.1:9")
            .with("SLSKD_CONTROLLER_API_KEY", "relay-api-key-123456")
            .with("SLSKD_CONTROLLER_SECRET", "relay-secret-123456"),
    );
    let (file_state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_REMOTE_FILE_MANAGEMENT", "true"),
    );
    let _ = std::fs::remove_dir_all(&file_state.config.downloads_dir);
    let _ = std::fs::remove_dir_all(&file_state.config.incomplete_dir);
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($state:expr, $method:expr, $route:expr, $path:expr, $body:expr, $expected:expr) => {{
            let response = crate::route_http_request($method, $path, None, $body, $state)
                .await
                .unwrap_or_else(|error| panic!("{} {}: {error}", $method, $path));
            let pass = response.status == $expected;
            if !pass {
                mismatches.push(format!(
                    "{target} {} {} expected {}, got {}",
                    $method, $path, $expected, response.status
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": $method,
                "route": $route,
                "case": "missing-empty-or-conflict-state",
                "pass": pass,
            }));
        }};
    }

    record!(
        &state,
        "GET",
        "/api/v0/application/version",
        "/api/v0/application/version",
        "",
        "200 OK"
    );
    record!(
        &state,
        "GET",
        "/api/v0/application/version/latest",
        "/api/v0/application/version/latest",
        "",
        "200 OK"
    );
    record!(&state, "GET", "/api/v0/logs", "/api/v0/logs", "", "200 OK");
    record!(
        &state,
        "GET",
        "/api/v0/options",
        "/api/v0/options",
        "",
        "200 OK"
    );
    state.session.write().await.state = "connected";
    record!(
        &state,
        "GET",
        "/api/v0/rooms/available",
        "/api/v0/rooms/available",
        "",
        "200 OK"
    );
    record!(
        &state,
        "GET",
        "/api/v0/rooms/joined",
        "/api/v0/rooms/joined",
        "",
        "200 OK"
    );
    record!(
        &state,
        "GET",
        "/api/v0/searches",
        "/api/v0/searches",
        "",
        "200 OK"
    );
    record!(
        &state,
        "GET",
        "/api/v0/conversations",
        "/api/v0/conversations",
        "",
        "200 OK"
    );
    record!(
        &state,
        "GET",
        "/api/v0/conversations/{username}/messages",
        "/api/v0/conversations/missing/messages",
        "",
        "404 Not Found"
    );
    record!(
        &file_state,
        "GET",
        "/api/v0/files/downloads/directories",
        "/api/v0/files/downloads/directories",
        "",
        "404 Not Found"
    );
    record!(
        &file_state,
        "GET",
        "/api/v0/files/incomplete/directories",
        "/api/v0/files/incomplete/directories",
        "",
        "404 Not Found"
    );
    record!(
        &state,
        "GET",
        "/api/v0/server",
        "/api/v0/server",
        "",
        "200 OK"
    );
    record!(
        &state,
        "GET",
        "/api/v0/session",
        "/api/v0/session",
        "",
        "200 OK"
    );
    record!(
        &state,
        "GET",
        "/api/v0/shares",
        "/api/v0/shares",
        "",
        "200 OK"
    );
    record!(
        &state,
        "GET",
        "/api/v0/shares/contents",
        "/api/v0/shares/contents",
        "",
        "200 OK"
    );
    record!(
        &state,
        "GET",
        "/api/v0/telemetry/reports/transfers/users/{username}",
        "/api/v0/telemetry/reports/transfers/users/missing",
        "",
        "200 OK"
    );
    record!(
        &relay_state,
        "GET",
        "/api/v0/relay/controller/downloads/{token}",
        "/api/v0/relay/controller/downloads/11111111-1111-4111-8111-111111111111",
        "",
        "401 Unauthorized"
    );

    record!(
        &state,
        "POST",
        "/api/v0/application/gc",
        "/api/v0/application/gc",
        "",
        "200 OK"
    );
    record!(
        &state,
        "POST",
        "/api/v0/application/loopback",
        "/api/v0/application/loopback",
        "",
        "400 Bad Request"
    );
    state.session.write().await.state = "connected";
    record!(
        &state,
        "POST",
        "/api/v0/conversations/{username}",
        "/api/v0/conversations/missing",
        "",
        "400 Bad Request"
    );
    record!(
        &state,
        "POST",
        "/api/v0/transfers/downloads/{username}",
        "/api/v0/transfers/downloads/missing",
        "",
        "400 Bad Request"
    );
    record!(
        &state,
        "POST",
        "/api/v0/users/{username}/directory",
        "/api/v0/users/missing/directory",
        "",
        "400 Bad Request"
    );

    record!(
        &state,
        "PUT",
        "/api/v0/application",
        "/api/v0/application",
        "{}",
        "204 No Content"
    );
    record!(
        &state,
        "PUT",
        "/api/v0/conversations/{username}",
        "/api/v0/conversations/missing",
        "",
        "404 Not Found"
    );
    record!(
        &state,
        "PUT",
        "/api/v0/conversations/{username}/{id}",
        "/api/v0/conversations/missing/999",
        "",
        "404 Not Found"
    );
    record!(
        &state,
        "PUT",
        "/api/v0/searches/{id}",
        "/api/v0/searches/ffffffff-ffff-4fff-8fff-ffffffffffff",
        "",
        "404 Not Found"
    );
    record!(
        &state,
        "PUT",
        "/api/v0/server",
        "/api/v0/server",
        "",
        "205 Reset Content"
    );
    record!(
        &state,
        "PUT",
        "/api/v0/shares",
        "/api/v0/shares",
        "",
        "200 OK"
    );
    record!(
        &state,
        "DELETE",
        "/api/v0/application",
        "/api/v0/application",
        "",
        "204 No Content"
    );
    record!(
        &state,
        "DELETE",
        "/api/v0/server",
        "/api/v0/server",
        "",
        "204 No Content"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("controller_empty_and_missing_state.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskd empty/missing state ledger"),
    )
    .expect("write slskd empty/missing state ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskd empty/missing state mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
