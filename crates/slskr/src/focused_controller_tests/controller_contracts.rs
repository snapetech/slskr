use super::fixtures::*;

#[tokio::test]
async fn controller_api_differential_controller_file_transfer_room_residuals() {
    let target = "slskd";
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

    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_REMOTE_FILE_MANAGEMENT", "true")
            .with("SLSKR_TEST_USER_ENDPOINT_OVERRIDES", "peer=127.0.0.1:2234"),
    );
    let downloads = state.config.downloads_dir.clone();
    let incomplete = state.config.incomplete_dir.clone();
    fs::create_dir_all(downloads.join("Artist/Album")).expect("downloads fixture");
    fs::write(downloads.join("Artist/Album/Track.flac"), b"track").expect("download fixture");
    fs::create_dir_all(incomplete.join("Partial")).expect("incomplete fixture");
    fs::write(incomplete.join("Partial/Track.part"), b"partial").expect("partial fixture");

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
            .expect("root storage list");
        let json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        let pass = response.status == "200 OK"
            && json["directories"].is_array()
            && json["files"].is_array();
        record!("GET", route, "nominal-status-headers-body", pass);
        record!("GET", route, "populated-dynamic-state", pass);
    }

    for (path, route, expected_name) in [
        (
            "/api/v0/files/downloads/directories/QXJ0aXN0L0FsYnVt",
            "/api/v0/files/downloads/directories/{base64SubdirectoryName}",
            "Track.flac",
        ),
        (
            "/api/v0/files/incomplete/directories/UGFydGlhbA==",
            "/api/v0/files/incomplete/directories/{base64SubdirectoryName}",
            "Track.part",
        ),
    ] {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .expect("nested storage list");
        let json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        let pass = response.status == "200 OK"
            && json["files"]
                .as_array()
                .is_some_and(|files| files.iter().any(|file| file["name"] == expected_name));
        record!("GET", route, "nominal-status-headers-body", pass);
        record!("GET", route, "populated-dynamic-state", pass);
    }

    let missing_nested = crate::route_http_request(
        "GET",
        "/api/v0/files/incomplete/directories/TWlzc2luZw==",
        None,
        "",
        &state,
    )
    .await
    .expect("missing nested storage list");
    record!(
        "GET",
        "/api/v0/files/incomplete/directories/{base64SubdirectoryName}",
        "missing-empty-or-conflict-state",
        missing_nested.status == "404 Not Found"
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
        let root = if storage == "downloads" {
            &downloads
        } else {
            &incomplete
        };
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("delete fixture parent"))
            .expect("delete fixture directory");
        if resource == "directories" {
            fs::create_dir_all(&path).expect("delete fixture directory path");
            fs::write(path.join("delete-me.bin"), b"delete me")
                .expect("delete fixture directory file");
        } else {
            fs::write(&path, b"delete me").expect("delete fixture file");
        }
        let encoded = base64::engine::general_purpose::STANDARD.encode(relative);
        let request_path = format!("/api/v0/files/{storage}/{resource}/{encoded}");
        let deleted = crate::route_http_request("DELETE", &request_path, None, "", &state)
            .await
            .expect("delete storage path");
        let nominal = deleted.status == "204 No Content" && !path.exists();
        record!("DELETE", route, "nominal-status-headers-body", nominal);
        record!(
            "DELETE",
            route,
            "mutation-side-effects-and-readback",
            nominal
        );
        let repeated = crate::route_http_request("DELETE", &request_path, None, "", &state)
            .await
            .expect("repeat delete storage path");
        let expected_repeated_status = if resource == "files" {
            "204 No Content"
        } else {
            "404 Not Found"
        };
        record!(
            "DELETE",
            route,
            "concurrency-and-idempotency",
            repeated.status == expected_repeated_status
        );
    }

    for (storage, resource, encoded, route) in [
        (
            "downloads",
            "files",
            base64::engine::general_purpose::STANDARD.encode("Missing.mp3"),
            "/api/v0/files/downloads/files/{base64FileName}",
        ),
        (
            "incomplete",
            "files",
            base64::engine::general_purpose::STANDARD.encode("Missing.part"),
            "/api/v0/files/incomplete/files/{base64FileName}",
        ),
    ] {
        let response = crate::route_http_request(
            "DELETE",
            &format!("/api/v0/files/{storage}/{resource}/{encoded}"),
            None,
            "",
            &state,
        )
        .await
        .expect("missing storage delete");
        record!(
            "DELETE",
            route,
            "missing-empty-or-conflict-state",
            response.status == "204 No Content"
        );
    }
    let traversal = crate::route_http_request(
        "DELETE",
        &format!(
            "/api/v0/files/downloads/files/{}",
            base64::engine::general_purpose::STANDARD.encode("../secret")
        ),
        None,
        "",
        &state,
    )
    .await
    .expect("traversal storage delete");
    record!(
        "DELETE",
        "/api/v0/files/downloads/files/{base64FileName}",
        "malformed-path-query-or-body",
        traversal.status == "400 Bad Request"
    );

    let reset_dir = state.config.state_dir.display().to_string();
    let (reset_state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_REMOTE_FILE_MANAGEMENT", "true")
            .with("SLSKR_STATE_DIR", &reset_dir),
    );
    for (storage, resource, relative, route) in [
        (
            "downloads",
            "files",
            "Remote/Reset.mp3",
            "/api/v0/files/downloads/files/{base64FileName}",
        ),
        (
            "downloads",
            "directories",
            "ResetDownload",
            "/api/v0/files/downloads/directories/{base64SubdirectoryName}",
        ),
        (
            "incomplete",
            "files",
            "Reset.part",
            "/api/v0/files/incomplete/files/{base64FileName}",
        ),
        (
            "incomplete",
            "directories",
            "ResetIncomplete",
            "/api/v0/files/incomplete/directories/{base64SubdirectoryName}",
        ),
    ] {
        let response = crate::route_http_request(
            "DELETE",
            &format!(
                "/api/v0/files/{storage}/{resource}/{}",
                base64::engine::general_purpose::STANDARD.encode(relative)
            ),
            None,
            "",
            &reset_state,
        )
        .await
        .expect("reset storage delete");
        record!(
            "DELETE",
            route,
            "restart-persistence-or-reset",
            response.status
                == if resource == "files" {
                    "204 No Content"
                } else {
                    "404 Not Found"
                }
        );
    }

    let downloads_conflict = std::env::temp_dir().join(format!(
        "slskr-focused-download-conflict-{}",
        uuid::Uuid::new_v4().simple()
    ));
    let incomplete_conflict = std::env::temp_dir().join(format!(
        "slskr-focused-incomplete-conflict-{}",
        uuid::Uuid::new_v4().simple()
    ));
    fs::write(&downloads_conflict, b"not a directory").expect("downloads conflict");
    fs::write(&incomplete_conflict, b"not a directory").expect("incomplete conflict");
    let (failure_state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_REMOTE_FILE_MANAGEMENT", "true"),
    );
    *failure_state
        .downloads_dir
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = downloads_conflict.clone();
    *failure_state
        .incomplete_dir
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = incomplete_conflict.clone();
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
                base64::engine::general_purpose::STANDARD.encode(relative)
            ),
            None,
            "",
            &failure_state,
        )
        .await
        .expect("storage runtime failure");
        record!(
            "DELETE",
            route,
            "runtime-failure-and-timeout",
            response.status == "503 Service Unavailable"
        );
    }
    #[cfg(unix)]
    {
        for storage in ["downloads", "incomplete"] {
            let target_dir = std::env::temp_dir().join(format!(
                "slskr-focused-list-target-{}-{}",
                std::process::id(),
                uuid::Uuid::new_v4().simple()
            ));
            let link = std::env::temp_dir().join(format!(
                "slskr-focused-list-link-{}-{}",
                std::process::id(),
                uuid::Uuid::new_v4().simple()
            ));
            fs::create_dir_all(&target_dir).expect("create storage list target");
            std::os::unix::fs::symlink(&target_dir, &link).expect("create storage list symlink");
            let state_root = if storage == "downloads" {
                &failure_state.downloads_dir
            } else {
                &failure_state.incomplete_dir
            };
            *state_root
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = link.clone();
            let response = crate::route_http_request(
                "GET",
                &format!("/api/v0/files/{storage}/directories"),
                None,
                "",
                &failure_state,
            )
            .await
            .expect("storage root runtime failure");
            record!(
                "GET",
                if storage == "downloads" {
                    "/api/v0/files/downloads/directories"
                } else {
                    "/api/v0/files/incomplete/directories"
                },
                "runtime-failure-and-timeout",
                response.status == "503 Service Unavailable"
            );
            let _ = fs::remove_file(link);
            let _ = fs::remove_dir_all(target_dir);
        }
        *failure_state
            .downloads_dir
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = downloads_conflict.clone();
        *failure_state
            .incomplete_dir
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = incomplete_conflict.clone();
    }
    for (storage, route) in [
        (
            "downloads",
            "/api/v0/files/downloads/directories/{base64SubdirectoryName}",
        ),
        (
            "incomplete",
            "/api/v0/files/incomplete/directories/{base64SubdirectoryName}",
        ),
    ] {
        let response = crate::route_http_request(
            "GET",
            &format!(
                "/api/v0/files/{storage}/directories/{}",
                base64::engine::general_purpose::STANDARD.encode("RuntimeFailureDirectory")
            ),
            None,
            "",
            &failure_state,
        )
        .await
        .expect("nested storage runtime failure");
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            response.status == "503 Service Unavailable"
        );
    }
    let _ = fs::remove_file(downloads_conflict);
    let _ = fs::remove_file(incomplete_conflict);

    let enqueue = crate::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/peer",
        None,
        r#"{"files":[{"filename":"Remote/Queued.flac","size":99}]}"#,
        &state,
    )
    .await
    .expect("enqueue focused transfer");
    let enqueue_json = serde_json::from_str::<serde_json::Value>(&enqueue.body).unwrap_or_default();
    let transfer_id = enqueue_json["transfers"][0]["id"]
        .as_str()
        .and_then(|value| value.parse::<u64>().ok())
        .or_else(|| enqueue_json["transfers"][0]["id"].as_u64())
        .expect("focused transfer id");
    let enqueue_pass = enqueue.status == "200 OK"
        && enqueue_json["queued"] == 1
        && enqueue_json["transfers"][0]["username"] == "peer";
    record!(
        "POST",
        "/api/v0/transfers/downloads/{username}",
        "nominal-status-headers-body",
        enqueue_pass
    );
    record!(
        "POST",
        "/api/v0/transfers/downloads/{username}",
        "mutation-side-effects-and-readback",
        enqueue_pass
    );

    let list = crate::route_http_request("GET", "/api/v0/transfers/downloads", None, "", &state)
        .await
        .expect("focused transfer list");
    let list_json = serde_json::from_str::<serde_json::Value>(&list.body).unwrap_or_default();
    let list_pass = list.status == "200 OK"
        && list_json
            .as_array()
            .is_some_and(|rows| rows.iter().any(|row| row["username"] == "peer"));
    record!(
        "GET",
        "/api/v0/transfers/downloads",
        "nominal-status-headers-body",
        list_pass
    );
    record!(
        "GET",
        "/api/v0/transfers/downloads",
        "populated-dynamic-state",
        list_pass
    );

    let detail = crate::route_http_request(
        "GET",
        &format!("/api/v0/transfers/downloads/peer/{transfer_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("focused transfer detail");
    let detail_json = serde_json::from_str::<serde_json::Value>(&detail.body).unwrap_or_default();
    let detail_pass = detail.status == "200 OK"
        && (detail_json["id"] == serde_json::json!(transfer_id)
            || detail_json["id"] == serde_json::json!(transfer_id.to_string()));
    record!(
        "GET",
        "/api/v0/transfers/downloads/{username}/{id}",
        "nominal-status-headers-body",
        detail_pass
    );
    record!(
        "GET",
        "/api/v0/transfers/downloads/{username}/{id}",
        "populated-dynamic-state",
        detail_pass
    );
    let missing_detail = crate::route_http_request(
        "GET",
        "/api/v0/transfers/downloads/other/999999",
        None,
        "",
        &state,
    )
    .await
    .expect("missing focused transfer detail");
    record!(
        "GET",
        "/api/v0/transfers/downloads/{username}/{id}",
        "missing-empty-or-conflict-state",
        missing_detail.status == "404 Not Found"
    );
    let cancelled = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/transfers/downloads/peer/{transfer_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("cancel focused transfer");
    let cancel_pass = cancelled.status == "204 No Content";
    record!(
        "DELETE",
        "/api/v0/transfers/downloads/{username}/{id}",
        "nominal-status-headers-body",
        cancel_pass
    );
    record!(
        "DELETE",
        "/api/v0/transfers/downloads/{username}/{id}",
        "mutation-side-effects-and-readback",
        cancel_pass
    );

    let batch_id = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
    let batch_body = format!(
        r#"{{"id":"{batch_id}","username":"peer","files":[{{"filename":"Music/A.flac","size":42}}]}}"#
    );
    let batch = crate::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/batches",
        None,
        &batch_body,
        &state,
    )
    .await
    .expect("focused transfer batch");
    let batch_json = serde_json::from_str::<serde_json::Value>(&batch.body).unwrap_or_default();
    let batch_pass = batch.status == "201 Created" && batch_json["batch"]["id"] == batch_id;
    record!(
        "POST",
        "/api/v0/transfers/downloads/batches",
        "nominal-status-headers-body",
        batch_pass
    );
    record!(
        "POST",
        "/api/v0/transfers/downloads/batches",
        "mutation-side-effects-and-readback",
        batch_pass
    );
    let duplicate = crate::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/batches",
        None,
        &batch_body,
        &state,
    )
    .await
    .expect("duplicate focused transfer batch");
    record!(
        "POST",
        "/api/v0/transfers/downloads/batches",
        "missing-empty-or-conflict-state",
        duplicate.status == "409 Conflict"
    );

    state.session.write().await.state = "connected";
    let available = crate::route_http_request("GET", "/api/v0/rooms/available", None, "", &state)
        .await
        .expect("focused available rooms");
    let available_json =
        serde_json::from_str::<serde_json::Value>(&available.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/rooms/available",
        "nominal-status-headers-body",
        available.status == "200 OK" && available_json.is_array()
    );
    for (path, route) in [
        (
            "/api/v0/rooms/joined/missing/messages",
            "/api/v0/rooms/joined/{roomName}/messages",
        ),
        (
            "/api/v0/rooms/joined/missing/users",
            "/api/v0/rooms/joined/{roomName}/users",
        ),
    ] {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .expect("missing focused room subresource");
        record!(
            "GET",
            route,
            "missing-empty-or-conflict-state",
            response.status == "404 Not Found"
        );
    }

    assert!(
        mismatches.is_empty(),
        "{} focused slskd controller mismatches: {}",
        mismatches.len(),
        mismatches.join("; ")
    );
    write_ledger(
        "controller_focused_file_transfer_room_residuals.json",
        &ledger,
    );
}

#[tokio::test]
async fn file_lifecycle_differential_controller_file_service_existing_missing_overwrite() {
    let target = "slskd";
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_REMOTE_FILE_MANAGEMENT", "true"),
    );
    let managed_file = state
        .config
        .downloads_dir
        .join("FileService")
        .join("managed.bin");
    fs::create_dir_all(managed_file.parent().expect("managed file parent"))
        .expect("create managed file parent");
    fs::write(&managed_file, b"managed-file").expect("write managed file");
    let encoded = base64::engine::general_purpose::STANDARD.encode("FileService/managed.bin");
    let deleted = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/files/downloads/files/{encoded}"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete managed file");
    let missing = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/files/downloads/files/{encoded}"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete missing managed file");
    let pass = deleted.status == "204 No Content"
        && missing.status == "204 No Content"
        && !managed_file.exists();
    assert!(
        pass,
        "slskd FileService delete contract: first={}, second={}, exists={}",
        deleted.status,
        missing.status,
        managed_file.exists()
    );
    write_file_lifecycle_ledger(
        "controller_focused_file_service_existing_missing_overwrite.json",
        &[serde_json::json!({
            "target": target,
            "subject": "Files/FileService",
            "case": "existing-missing-and-overwrite",
            "pass": pass,
        })],
    );
}

#[test]
fn security_authorization_matrix_matches_declared_policy_for_every_frozen_route() {
    #[derive(serde::Deserialize)]
    struct AuthPolicyRow {
        method: String,
        route: String,
        access: String,
        scheme: String,
        scopes: Vec<String>,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Outcome {
        Allowed,
        Unauthorized,
        Forbidden,
    }

    #[derive(Clone, Copy)]
    struct Profile {
        name: &'static str,
        header: Option<&'static str>,
        credential: Option<(u8, &'static str, bool)>,
    }

    const PROFILES: [Profile; 10] = [
        Profile {
            name: "anonymous",
            header: None,
            credential: None,
        },
        Profile {
            name: "basic-readonly",
            header: Some("ApiKey read-token"),
            credential: Some((0, "api_key", false)),
        },
        Profile {
            name: "basic-readwrite",
            header: Some("ApiKey write-token"),
            credential: Some((1, "api_key", false)),
        },
        Profile {
            name: "basic-administrator",
            header: Some("ApiKey admin-token"),
            credential: Some((2, "api_key", false)),
        },
        Profile {
            name: "bearer-readonly",
            header: Some("Bearer read-token"),
            credential: Some((0, "jwt", false)),
        },
        Profile {
            name: "bearer-readwrite",
            header: Some("Bearer write-token"),
            credential: Some((1, "jwt", false)),
        },
        Profile {
            name: "bearer-administrator",
            header: Some("Bearer admin-token"),
            credential: Some((2, "jwt", false)),
        },
        Profile {
            name: "invalid-or-expired-credential",
            header: Some("Bearer not-a-real-differential-token"),
            credential: None,
        },
        Profile {
            name: "missing-required-scope",
            header: Some("ApiKey nowplaying-token"),
            credential: Some((1, "api_key", true)),
        },
        Profile {
            name: "wrong-authentication-scheme",
            header: None,
            credential: None,
        },
    ];

    fn required_access_rank(access: &str) -> Option<u8> {
        match access {
            "anonymous" | "delegated" => None,
            "administrator" => Some(2),
            "read_write" => Some(1),
            _ => Some(0),
        }
    }

    fn expected_outcome(rule: &AuthPolicyRow, profile: Profile) -> Outcome {
        let Some(required) = required_access_rank(&rule.access) else {
            return Outcome::Allowed;
        };
        let Some((credential_rank, credential_scheme, nowplaying_only)) = profile.credential else {
            return Outcome::Unauthorized;
        };
        if credential_rank < required {
            return Outcome::Forbidden;
        }
        if rule.scheme != "any" && credential_scheme != rule.scheme {
            return Outcome::Forbidden;
        }
        let requires_nowplaying = rule.scopes.iter().any(|scope| scope == "nowplaying");
        if nowplaying_only && !requires_nowplaying {
            return Outcome::Forbidden;
        }
        Outcome::Allowed
    }

    fn placeholder_path(route: &str) -> String {
        let mut segments: Vec<String> = route
            .trim_matches('/')
            .split('/')
            .map(|segment| {
                if segment.starts_with('{') && segment.ends_with('}') {
                    "differential-fixture-value".to_owned()
                } else {
                    segment.to_owned()
                }
            })
            .collect();
        if route.contains("{*") {
            segments.push("differential-fixture-tail".to_owned());
        }
        format!("/{}", segments.join("/"))
    }

    let headers = crate::RequestSecurityHeaders::default();
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    for (target, source) in [
        (
            "slskd",
            include_str!("../../data/legacy-controller-auth-policy.json"),
        ),
        (
            "slskdn",
            include_str!("../../data/native-controller-auth-policy.json"),
        ),
    ] {
        let rules: Vec<AuthPolicyRow> =
            serde_json::from_str(source).expect("checked profile auth policy registry");
        let state_dir = std::env::temp_dir().join(format!(
            "slskr-focused-security-auth-{target}-{}",
            uuid::Uuid::new_v4()
        ));
        let config = crate::AppConfig::from_layers(
            None,
            FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_STATE_DIR", state_dir.to_str().expect("state path"))
                .with("SLSKR_AUTH_DISABLED", "false")
                .with("SLSKR_CONTROLLER_PROFILE", target)
                .with("SLSKR_API_TOKEN", "admin-token")
                .with("SLSKR_API_READ_WRITE_TOKEN", "write-token")
                .with("SLSKR_API_READ_ONLY_TOKEN", "read-token")
                .with("SLSKR_API_NOWPLAYING_TOKEN", "nowplaying-token"),
        )
        .expect("hermetic auth-policy differential config");

        for rule in &rules {
            let path = placeholder_path(&rule.route);
            for profile in PROFILES {
                let (header, expected) = if profile.name == "wrong-authentication-scheme" {
                    if rule.scheme == "jwt" {
                        (Some("ApiKey admin-token"), Outcome::Forbidden)
                    } else if rule.scheme == "api_key" {
                        (Some("Bearer admin-token"), Outcome::Forbidden)
                    } else {
                        let any_scheme_profile = Profile {
                            header: Some("Bearer admin-token"),
                            credential: Some((2, "jwt", false)),
                            ..profile
                        };
                        (
                            any_scheme_profile.header,
                            expected_outcome(rule, any_scheme_profile),
                        )
                    }
                } else {
                    (profile.header, expected_outcome(rule, profile))
                };
                let actual = match crate::routing::check_route_auth(
                    &config,
                    &rule.method,
                    &path,
                    header,
                    &headers,
                ) {
                    Ok(()) => Outcome::Allowed,
                    Err("unauthorized") => Outcome::Unauthorized,
                    Err("forbidden") => Outcome::Forbidden,
                    Err(other) => panic!(
                        "unexpected auth-gate outcome {other:?} for {target} {} {}",
                        rule.method, rule.route
                    ),
                };
                let pass = actual == expected;
                if !pass {
                    mismatches.push(format!(
                        "{target} {} {} [{}]: expected {expected:?}, got {actual:?}",
                        rule.method, rule.route, profile.name
                    ));
                }
                ledger.push(serde_json::json!({
                    "target": target,
                    "method": rule.method,
                    "route": rule.route,
                    "case": profile.name,
                    "pass": pass,
                    "expected": format!("{expected:?}"),
                    "actual": format!("{actual:?}"),
                }));
            }
        }
        let _ = fs::remove_dir_all(&state_dir);
    }

    let evidence_dir = std::env::temp_dir().join("slskr-parity-evidence");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("security-authorization.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize security-authorization ledger"),
    )
    .expect("write security-authorization ledger");

    assert!(
        mismatches.is_empty(),
        "{} security-authorization mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
