//! Controller full integrations differential ownership.

use super::*;

/// Bulk differential proof crediting 3 musicbrainz-overlay routes'
/// cases, independently re-derived from `musicbrainz_overlay_export_
/// review_and_approval_match_real_oracle_gating`'s real exportable-
/// type gate, unsafe-approvedBy rejection, and idempotent-approval
/// (a repeat approval never overwrites the original decision)
/// checks. slskdN-only (confirmed against the frozen registry).
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
    feature = "bounded-controller-api-tests-2"
))]
pub(super) async fn controller_api_differential_musicbrainz_overlay_export() {
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

    let (state, _receiver) = test_state();
    state.library.write().await.create(
        "Differential Overlay Artist".to_owned(),
        "Differential Overlay Release".to_owned(),
        "Audio".to_owned(),
    );
    let release_graph = crate::route_http_request(
        "GET",
        "/api/v0/musicbrainz/overlays/artist/Differential%20Overlay%20Artist/release-graph",
        None,
        "",
        &state,
    )
    .await
    .expect("read overlay release graph");
    let release_graph_json =
        serde_json::from_str::<serde_json::Value>(&release_graph.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/musicbrainz/overlays/artist/{artistId}/release-graph",
        "nominal-status-headers-body",
        release_graph.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/musicbrainz/overlays/artist/{artistId}/release-graph",
        "populated-dynamic-state",
        release_graph_json["artistId"] == "Differential Overlay Artist"
            && release_graph_json["releases"]
                .as_array()
                .is_some_and(|releases| {
                    releases
                        .iter()
                        .any(|release| release["title"] == "Differential Overlay Release")
                })
            && release_graph_json["edges"].is_array()
    );

    let non_exportable = crate::route_http_request(
        "POST",
        "/api/v0/musicbrainz/overlays/edits",
        None,
        r#"{"type":"Other","targetType":"Recording","targetId":"rec-differential-1","field":"title","value":"New Title","evidence":[{"type":"WorkRef","reference":"opaque-ref-differential"}]}"#,
        &state,
    )
    .await
    .expect("create non-exportable edit");
    let non_exportable_id = serde_json::from_str::<serde_json::Value>(&non_exportable.body)
        .unwrap_or_default()["edit"]["editId"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    record!(
        "POST",
        "/api/v0/musicbrainz/overlays/edits",
        "nominal-status-headers-body",
        non_exportable.status == "200 OK" && !non_exportable_id.is_empty()
    );

    let route_route = format!("/api/v0/musicbrainz/overlays/edits/{non_exportable_id}/routes");
    let empty_route = crate::route_http_request("POST", &route_route, None, "{}", &state)
        .await
        .unwrap_or_else(|error| panic!("{route_route}: {error}"));
    let empty_route_json =
        serde_json::from_str::<serde_json::Value>(&empty_route.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/musicbrainz/overlays/edits/{editId}/routes",
        "malformed-path-query-or-body",
        empty_route.status == "400 Bad Request"
            && empty_route_json["editId"] == non_exportable_id
            && empty_route_json["success"] == false
            && empty_route_json["errorMessage"] == "At least one target peer is required."
            && empty_route_json["targetPeerIds"] == serde_json::json!([])
    );

    let unavailable_route = crate::route_http_request(
        "POST",
        &route_route,
        None,
        r#"{"targetPeerIds":["actor:differential-route-peer"]}"#,
        &state,
    )
    .await
    .unwrap_or_else(|error| panic!("{route_route}: {error}"));
    let unavailable_route_json =
        serde_json::from_str::<serde_json::Value>(&unavailable_route.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/musicbrainz/overlays/edits/{editId}/routes",
        "runtime-failure-and-timeout",
        unavailable_route.status == "400 Bad Request"
            && unavailable_route_json["editId"] == non_exportable_id
            && unavailable_route_json["success"] == false
            && unavailable_route_json["errorMessage"] == "Routing backend is not available."
            && unavailable_route_json["targetPeerIds"]
                == serde_json::json!(["actor:differential-route-peer"])
    );

    let route_attempts = crate::route_http_request("GET", &route_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{route_route}: {error}"));
    let route_attempts_json =
        serde_json::from_str::<serde_json::Value>(&route_attempts.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/musicbrainz/overlays/edits/{editId}/routes",
        "nominal-status-headers-body",
        route_attempts.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v0/musicbrainz/overlays/edits/{editId}/routes",
        "mutation-side-effects-and-readback",
        route_attempts_json.as_array().is_some_and(|attempts| {
            attempts.iter().any(|attempt| {
                attempt["id"] == unavailable_route_json["id"]
                    && attempt["editId"] == non_exportable_id
                    && attempt["errorMessage"] == "Routing backend is not available."
            })
        })
    );
    record!(
        "GET",
        "/api/v0/musicbrainz/overlays/edits/{editId}/routes",
        "populated-dynamic-state",
        route_attempts_json.as_array().is_some_and(|attempts| {
            attempts.iter().any(|attempt| {
                attempt["id"] == unavailable_route_json["id"]
                    && attempt["editId"] == non_exportable_id
                    && attempt["success"] == false
            })
        })
    );

    let review_route =
        format!("/api/v0/musicbrainz/overlays/edits/{non_exportable_id}/export-review");
    let review = crate::route_http_request("GET", &review_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{review_route}: {error}"));
    let review_json = serde_json::from_str::<serde_json::Value>(&review.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/musicbrainz/overlays/edits/{editId}/export-review",
        "nominal-status-headers-body",
        review.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/musicbrainz/overlays/edits/{editId}/export-review",
        "populated-dynamic-state",
        review_json["upstreamTarget"] == "Recording:rec-differential-1"
            && review_json["proposedChange"] == "title => New Title"
            && review_json["canApproveExport"] == false
            && review_json["reviewReason"] == "Overlay edit type is not exportable."
            && review_json["decision"].is_null()
    );

    let rejected_route =
        format!("/api/v0/musicbrainz/overlays/edits/{non_exportable_id}/approve-export");
    let rejected_approval = crate::route_http_request("POST", &rejected_route, None, "{}", &state)
        .await
        .unwrap_or_else(|error| panic!("{rejected_route}: {error}"));
    let rejected_json =
        serde_json::from_str::<serde_json::Value>(&rejected_approval.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/musicbrainz/overlays/edits/{editId}/approve-export",
        "malformed-path-query-or-body",
        rejected_approval.status == "400 Bad Request"
            && rejected_json["errors"]
                .as_array()
                .is_some_and(|errors| errors
                    .iter()
                    .any(|error| error == "Overlay edit type is not exportable."))
            && rejected_json["decision"].is_null()
    );

    let unsafe_edit = crate::route_http_request(
        "POST",
        "/api/v0/musicbrainz/overlays/edits",
        None,
        r#"{"type":"TitleCorrection","targetType":"Recording","targetId":"rec-differential-2","field":"title","value":"Corrected","evidence":[{"type":"WorkRef","reference":"opaque-ref-differential"}]}"#,
        &state,
    )
    .await
    .expect("create exportable edit for unsafe-approver check");
    let unsafe_edit_id = serde_json::from_str::<serde_json::Value>(&unsafe_edit.body)
        .unwrap_or_default()["edit"]["editId"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let unsafe_route =
        format!("/api/v0/musicbrainz/overlays/edits/{unsafe_edit_id}/approve-export");
    let unsafe_approval = crate::route_http_request(
        "POST",
        &unsafe_route,
        None,
        r#"{"approvedBy":"/etc/passwd"}"#,
        &state,
    )
    .await
    .unwrap_or_else(|error| panic!("{unsafe_route}: {error}"));
    record!(
        "POST",
        "/api/v0/musicbrainz/overlays/edits/{editId}/approve-export",
        "missing-empty-or-conflict-state",
        unsafe_approval.status == "400 Bad Request"
            && serde_json::from_str::<serde_json::Value>(&unsafe_approval.body).unwrap_or_default()
                ["errors"]
                .as_array()
                .is_some_and(|errors| errors
                    .iter()
                    .any(|error| error == "Approved-by identifier must be opaque and safe."))
    );

    let exportable = crate::route_http_request(
        "POST",
        "/api/v0/musicbrainz/overlays/edits",
        None,
        r#"{"type":"TitleCorrection","targetType":"Recording","targetId":"rec-differential-3","field":"title","value":"Corrected Title","evidence":[{"type":"WorkRef","reference":"opaque-ref-differential"}]}"#,
        &state,
    )
    .await
    .expect("create exportable edit");
    let exportable_id = serde_json::from_str::<serde_json::Value>(&exportable.body)
        .unwrap_or_default()["edit"]["editId"]
        .as_str()
        .unwrap_or_default()
        .to_owned();

    let approval_route =
        format!("/api/v0/musicbrainz/overlays/edits/{exportable_id}/approve-export");
    let approval = crate::route_http_request(
        "POST",
        &approval_route,
        None,
        r#"{"approvedBy":"differential-reviewer","note":"looks good"}"#,
        &state,
    )
    .await
    .unwrap_or_else(|error| panic!("{approval_route}: {error}"));
    let approval_json =
        serde_json::from_str::<serde_json::Value>(&approval.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/musicbrainz/overlays/edits/{editId}/approve-export",
        "nominal-status-headers-body",
        approval.status == "200 OK"
            && approval_json["errors"] == serde_json::json!([])
            && approval_json["decision"]["approvedBy"] == "differential-reviewer"
            && approval_json["decision"]["note"] == "looks good"
            && approval_json["decision"]["editId"] == exportable_id
            && approval_json["decision"]["upstreamTarget"] == "Recording:rec-differential-3"
            && approval_json["decision"]["proposedChange"] == "title => Corrected Title"
            && approval_json["decision"]["id"]
                .as_str()
                .is_some_and(|id| id.starts_with("musicbrainz-overlay-export:"))
    );

    let post_approval_review_route =
        format!("/api/v0/musicbrainz/overlays/edits/{exportable_id}/export-review");
    let post_approval_review =
        crate::route_http_request("GET", &post_approval_review_route, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{post_approval_review_route}: {error}"));
    let post_approval_review_json =
        serde_json::from_str::<serde_json::Value>(&post_approval_review.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/musicbrainz/overlays/edits/{editId}/export-review",
        "mutation-side-effects-and-readback",
        post_approval_review_json["canApproveExport"] == false
            && post_approval_review_json["reviewReason"]
                == "Upstream export has already been approved locally."
            && post_approval_review_json["decision"]["approvedBy"] == "differential-reviewer"
    );

    let repeat_approval = crate::route_http_request(
        "POST",
        &approval_route,
        None,
        r#"{"approvedBy":"different-differential-reviewer"}"#,
        &state,
    )
    .await
    .unwrap_or_else(|error| panic!("{approval_route}: {error}"));
    record!(
        "POST",
        "/api/v0/musicbrainz/overlays/edits/{editId}/approve-export",
        "mutation-side-effects-and-readback",
        repeat_approval.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&repeat_approval.body).unwrap_or_default()
                ["decision"]["approvedBy"]
                == "differential-reviewer"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("musicbrainz_overlay_export.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api musicbrainz-overlay-export mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential proof for the remaining versioned MultiSource
/// controller cases. The frozen slskdN controller exposes a distinct
/// discovery-oriented DTO surface from slskR's native swarm executor;
/// these checks keep the versioned projection, validation contracts, and
/// bounded local-state fallback explicit while avoiding an unbounded
/// external Soulseek search in a hermetic test.
/// slskdN-only (confirmed against the frozen registry).
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
    feature = "bounded-controller-api-tests-3"
))]
pub(super) async fn controller_api_differential_multisource_residuals() {
    let target = "slskdn";
    let base_env = MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target);
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

    let seed_search = |state: Arc<crate::AppState>| async move {
        let mut searches = state.searches.write().await;
        searches.records.push(crate::SearchRecord {
            id: "multisource-search".to_owned(),
            token: 901,
            query: "ambient".to_owned(),
            target: "global",
            target_name: None,
            status: "completed",
            results: vec![
                crate::SearchResultEntry {
                    peer_username: Some("alice".to_owned()),
                    filename: "Albums/Track.flac".to_owned(),
                    size: 42,
                    extension: "flac".to_owned(),
                    bit_rate: None,
                    sample_rate: None,
                    bit_depth: None,
                    length_seconds: None,
                    locked: false,
                    slot_free: Some(true),
                    average_speed: Some(204_800),
                    queue_length: Some(1),
                },
                crate::SearchResultEntry {
                    peer_username: Some("bob".to_owned()),
                    filename: "Music/Track.flac".to_owned(),
                    size: 42,
                    extension: "flac".to_owned(),
                    bit_rate: None,
                    sample_rate: None,
                    bit_depth: None,
                    length_seconds: None,
                    locked: false,
                    slot_free: Some(true),
                    average_speed: Some(102_400),
                    queue_length: Some(2),
                },
            ],
            raw_response_count: 2,
            filtered_out_count: 0,
            ignored_result_count: 0,
            hidden_locked_count: 0,
            fallback_attempts: 0,
            ttl_seconds: crate::DEFAULT_SEARCH_TTL_SECONDS,
            expires_at: u64::MAX,
            created_at: 1,
            updated_at: 1,
        });
    };

    let seed_job = |state: Arc<crate::AppState>| async move {
        let id = "11111111-1111-4111-8111-111111111111".to_owned();
        state
            .multisource
            .write()
            .await
            .insert(crate::multisource::SwarmJob {
                id: id.clone(),
                status: "queued".to_owned(),
                filename: "Track.flac".to_owned(),
                output_path: String::new(),
                file_size: 42,
                chunk_size: 512 * 1024,
                sources: vec!["alice".to_owned(), "bob".to_owned()],
                completed_chunks: 0,
                total_chunks: 1,
                bytes_downloaded: 0,
                created_at: 1,
                updated_at: 1,
                result: None,
            });
        id
    };

    let download_body = r#"{"filename":"Track.flac","fileSize":42,"sources":[{"username":"alice","fullPath":"Albums/Track.flac"},{"username":"bob","fullPath":"Music/Track.flac"}]}"#;
    let file_body = r#"{"filename":"Track.flac","size":42}"#;
    let swarm_body = r#"{"filename":"Track.flac","size":42,"skipVerification":true}"#;
    let verify_body = r#"{"filename":"Track.flac","fileSize":42,"usernames":["alice","bob"]}"#;
    let test_body = r#"{"searchText":"ambient"}"#;

    for case in [
        "malformed-path-query-or-body",
        "missing-empty-or-conflict-state",
        "runtime-failure-and-timeout",
    ] {
        let state = test_state_with_env(base_env.clone()).0;
        let path = if case == "malformed-path-query-or-body" {
            "/api/v0/multisource/jobs?unexpected=not-a-number"
        } else {
            "/api/v0/multisource/jobs"
        };
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap();
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/multisource/jobs",
            case,
            response.status == "200 OK"
                && value["count"].as_u64() == Some(0)
                && value["jobs"].as_array().is_some_and(Vec::is_empty)
        );
    }
    let populated_jobs = test_state_with_env(base_env.clone()).0;
    seed_job(populated_jobs.clone()).await;
    let response =
        crate::route_http_request("GET", "/api/v0/multisource/jobs", None, "", &populated_jobs)
            .await
            .unwrap();
    let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/multisource/jobs",
        "populated-dynamic-state",
        response.status == "200 OK"
            && value["count"] == 1
            && value["jobs"][0]["jobId"] == "11111111-1111-4111-8111-111111111111"
            && value["jobs"][0]["state"] == "queued"
    );

    let detail_state = test_state_with_env(base_env.clone()).0;
    let job_id = seed_job(detail_state.clone()).await;
    let detail_route = format!("/api/v0/multisource/jobs/{job_id}");
    for case in [
        "nominal-status-headers-body",
        "runtime-failure-and-timeout",
        "populated-dynamic-state",
    ] {
        let response = crate::route_http_request("GET", &detail_route, None, "", &detail_state)
            .await
            .unwrap();
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/multisource/jobs/{jobId}",
            case,
            response.status == "200 OK"
                && value["jobId"] == job_id
                && value["state"] == "queued"
                && value["totalChunks"] == 1
        );
    }
    let missing_detail = crate::route_http_request(
        "GET",
        "/api/v0/multisource/jobs/22222222-2222-4222-8222-222222222222",
        None,
        "",
        &test_state_with_env(base_env.clone()).0,
    )
    .await
    .unwrap();
    record!(
        "GET",
        "/api/v0/multisource/jobs/{jobId}",
        "missing-empty-or-conflict-state",
        missing_detail.status == "404 Not Found" && missing_detail.body.contains("Job not found")
    );

    for (route, key) in [
        ("/api/v0/multisource/search", "candidates"),
        ("/api/v0/multisource/users", "users"),
    ] {
        for case in [
            "nominal-status-headers-body",
            "missing-empty-or-conflict-state",
            "runtime-failure-and-timeout",
        ] {
            let state = test_state_with_env(base_env.clone()).0;
            if case == "runtime-failure-and-timeout" {
                state.session.write().await.state = "disconnected";
            }
            let response = crate::route_http_request(
                "GET",
                &format!("{route}?searchText=ambient"),
                None,
                "",
                &state,
            )
            .await
            .unwrap();
            let value =
                serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
            record!(
                "GET",
                route,
                case,
                response.status == "200 OK" && value["query"] == "ambient" && value[key].is_array()
            );
        }
        let state = test_state_with_env(base_env.clone()).0;
        seed_search(state.clone()).await;
        let response = crate::route_http_request(
            "GET",
            &format!("{route}?searchText=ambient"),
            None,
            "",
            &state,
        )
        .await
        .unwrap();
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "GET",
            route,
            "populated-dynamic-state",
            response.status == "200 OK"
                && value[key]
                    .as_array()
                    .is_some_and(|values| !values.is_empty())
        );
    }

    let files_state = test_state_with_env(base_env.clone()).0;
    seed_search(files_state.clone()).await;
    let files_route = "/api/v0/multisource/users/alice/files?filter=flac";
    for case in [
        "nominal-status-headers-body",
        "runtime-failure-and-timeout",
        "populated-dynamic-state",
    ] {
        if case == "runtime-failure-and-timeout" {
            files_state.session.write().await.state = "disconnected";
        }
        let response = crate::route_http_request("GET", files_route, None, "", &files_state)
            .await
            .unwrap();
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/multisource/users/{username}/files",
            case,
            response.status == "200 OK"
                && value["username"] == "alice"
                && value["totalFiles"].as_u64() == Some(1)
                && value["directories"].is_array()
        );
    }
    let malformed_files = crate::route_http_request(
        "GET",
        "/api/v0/multisource/users/%20/files",
        None,
        "",
        &files_state,
    )
    .await
    .unwrap();
    record!(
        "GET",
        "/api/v0/multisource/users/{username}/files",
        "malformed-path-query-or-body",
        malformed_files.status == "400 Bad Request"
            && malformed_files.body.contains("Username is required")
    );

    for (route, body, malformed_message) in [
        (
            "/api/v0/multisource/download-file",
            file_body,
            "Filename and size are required",
        ),
        (
            "/api/v0/multisource/file-sources",
            file_body,
            "Filename is required",
        ),
    ] {
        let valid_state = test_state_with_env(base_env.clone()).0;
        seed_search(valid_state.clone()).await;
        let valid = crate::route_http_request("POST", route, None, body, &valid_state)
            .await
            .unwrap();
        record!(
            "POST",
            route,
            "nominal-status-headers-body",
            valid.status == "200 OK" && valid.body.contains("Track.flac")
        );
        let malformed = crate::route_http_request(
            "POST",
            route,
            None,
            "{}",
            &test_state_with_env(base_env.clone()).0,
        )
        .await
        .unwrap();
        record!(
            "POST",
            route,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request" && malformed.body.contains(malformed_message)
        );
        for case in [
            "missing-empty-or-conflict-state",
            "runtime-failure-and-timeout",
            "mutation-side-effects-and-readback",
            "restart-persistence-or-reset",
        ] {
            let state = test_state_with_env(base_env.clone()).0;
            let response = crate::route_http_request("POST", route, None, body, &state)
                .await
                .unwrap();
            record!(
                "POST",
                route,
                case,
                if route.ends_with("download-file") {
                    response.status == "400 Bad Request"
                } else {
                    response.status == "200 OK" && response.body.contains("sizeGroups")
                }
            );
        }
        let concurrent_state = test_state_with_env(base_env.clone()).0;
        seed_search(concurrent_state.clone()).await;
        let responses = futures_util::future::join_all((0..2).map(|_| {
            let state = concurrent_state.clone();
            async move { crate::route_http_request("POST", route, None, body, &state).await }
        }))
        .await;
        record!(
            "POST",
            route,
            "concurrency-and-idempotency",
            responses.iter().all(|response| {
                response
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
            })
        );
    }

    let download_valid = crate::route_http_request(
        "POST",
        "/api/v0/multisource/download",
        None,
        download_body,
        &test_state_with_env(base_env.clone()).0,
    )
    .await
    .unwrap();
    record!(
        "POST",
        "/api/v0/multisource/download",
        "nominal-status-headers-body",
        download_valid.status == "200 OK" && download_valid.body.contains("Multi-source download")
    );
    let malformed_download = crate::route_http_request(
        "POST",
        "/api/v0/multisource/download",
        None,
        "{}",
        &test_state_with_env(base_env.clone()).0,
    )
    .await
    .unwrap();
    record!(
        "POST",
        "/api/v0/multisource/download",
        "missing-empty-or-conflict-state",
        malformed_download.status == "400 Bad Request"
            && malformed_download.body.contains("Filename is required")
    );
    for case in [
        "runtime-failure-and-timeout",
        "mutation-side-effects-and-readback",
        "restart-persistence-or-reset",
    ] {
        let response = crate::route_http_request(
            "POST",
            "/api/v0/multisource/download",
            None,
            download_body,
            &test_state_with_env(base_env.clone()).0,
        )
        .await
        .unwrap();
        record!(
            "POST",
            "/api/v0/multisource/download",
            case,
            response.status == "200 OK"
        );
    }
    let concurrent_download_state = test_state_with_env(base_env.clone()).0;
    let download_responses = futures_util::future::join_all((0..2).map(|_| {
        let state = concurrent_download_state.clone();
        async move {
            crate::route_http_request(
                "POST",
                "/api/v0/multisource/download",
                None,
                download_body,
                &state,
            )
            .await
        }
    }))
    .await;
    record!(
        "POST",
        "/api/v0/multisource/download",
        "concurrency-and-idempotency",
        download_responses.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
        })
    );

    for (route, body) in [
        ("/api/v0/multisource/swarm", swarm_body),
        ("/api/v0/multisource/swarm/async", swarm_body),
    ] {
        let valid_state = test_state_with_env(base_env.clone()).0;
        seed_search(valid_state.clone()).await;
        let valid = crate::route_http_request("POST", route, None, body, &valid_state)
            .await
            .unwrap();
        let valid_json = serde_json::from_str::<serde_json::Value>(&valid.body).unwrap_or_default();
        record!(
            "POST",
            route,
            "nominal-status-headers-body",
            valid.status == "200 OK"
                && if route.ends_with("async") {
                    valid_json["jobId"].is_string()
                } else {
                    valid_json["mode"] == "SWARM"
                }
        );
        let malformed = crate::route_http_request(
            "POST",
            route,
            None,
            "{}",
            &test_state_with_env(base_env.clone()).0,
        )
        .await
        .unwrap();
        record!(
            "POST",
            route,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request" && malformed.body.contains("Size is required")
        );
        let missing = crate::route_http_request(
            "POST",
            route,
            None,
            r#"{"filename":"Track.flac"}"#,
            &test_state_with_env(base_env.clone()).0,
        )
        .await
        .unwrap();
        record!(
            "POST",
            route,
            "missing-empty-or-conflict-state",
            missing.status == "400 Bad Request" && missing.body.contains("Size is required")
        );
        for case in [
            "runtime-failure-and-timeout",
            "mutation-side-effects-and-readback",
            "restart-persistence-or-reset",
        ] {
            let state = test_state_with_env(base_env.clone()).0;
            seed_search(state.clone()).await;
            let response = crate::route_http_request("POST", route, None, body, &state)
                .await
                .unwrap();
            record!("POST", route, case, response.status == "200 OK");
        }
        let concurrent_state = test_state_with_env(base_env.clone()).0;
        seed_search(concurrent_state.clone()).await;
        let responses = futures_util::future::join_all((0..2).map(|_| {
            let state = concurrent_state.clone();
            async move { crate::route_http_request("POST", route, None, body, &state).await }
        }))
        .await;
        record!(
            "POST",
            route,
            "concurrency-and-idempotency",
            responses.iter().all(|response| {
                response
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
            })
        );
    }

    for case in [
        "malformed-path-query-or-body",
        "missing-empty-or-conflict-state",
    ] {
        let body = if case == "malformed-path-query-or-body" {
            "{}"
        } else {
            ""
        };
        let response = crate::route_http_request(
            "POST",
            "/api/v0/multisource/test",
            None,
            body,
            &test_state_with_env(base_env.clone()).0,
        )
        .await
        .unwrap();
        record!(
            "POST",
            "/api/v0/multisource/test",
            case,
            response.status == "400 Bad Request"
                && response.body.contains("Search text is required")
        );
    }
    for case in [
        "runtime-failure-and-timeout",
        "restart-persistence-or-reset",
        "concurrency-and-idempotency",
    ] {
        let state = test_state_with_env(base_env.clone()).0;
        let response = if case == "concurrency-and-idempotency" {
            let responses = futures_util::future::join_all((0..2).map(|_| {
                let state = state.clone();
                async move {
                    crate::route_http_request(
                        "POST",
                        "/api/v0/multisource/test",
                        None,
                        test_body,
                        &state,
                    )
                    .await
                }
            }))
            .await;
            responses
                .into_iter()
                .all(|response| response.is_ok_and(|response| response.status == "200 OK"))
        } else {
            crate::route_http_request("POST", "/api/v0/multisource/test", None, test_body, &state)
                .await
                .is_ok_and(|response| response.status == "200 OK")
        };
        record!("POST", "/api/v0/multisource/test", case, response);
    }

    for case in [
        "nominal-status-headers-body",
        "runtime-failure-and-timeout",
        "mutation-side-effects-and-readback",
        "restart-persistence-or-reset",
    ] {
        let state = test_state_with_env(base_env.clone()).0;
        let response = crate::route_http_request(
            "POST",
            "/api/v0/multisource/verify",
            None,
            verify_body,
            &state,
        )
        .await
        .unwrap();
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "POST",
            "/api/v0/multisource/verify",
            case,
            response.status == "200 OK"
                && value["filename"] == "Track.flac"
                && value["usernames"]
                    .as_array()
                    .is_some_and(|users| users.len() == 2)
        );
    }
    let malformed_verify = crate::route_http_request(
        "POST",
        "/api/v0/multisource/verify",
        None,
        "{}",
        &test_state_with_env(base_env.clone()).0,
    )
    .await
    .unwrap();
    record!(
        "POST",
        "/api/v0/multisource/verify",
        "malformed-path-query-or-body",
        malformed_verify.status == "400 Bad Request"
            && malformed_verify.body.contains("Filename is required")
    );
    let missing_verify = crate::route_http_request(
        "POST",
        "/api/v0/multisource/verify",
        None,
        r#"{"filename":"Track.flac","fileSize":42}"#,
        &test_state_with_env(base_env.clone()).0,
    )
    .await
    .unwrap();
    record!(
        "POST",
        "/api/v0/multisource/verify",
        "missing-empty-or-conflict-state",
        missing_verify.status == "400 Bad Request"
            && missing_verify
                .body
                .contains("At least one username is required")
    );
    let concurrent_verify_state = test_state_with_env(base_env.clone()).0;
    let verify_responses = futures_util::future::join_all((0..2).map(|_| {
        let state = concurrent_verify_state.clone();
        async move {
            crate::route_http_request(
                "POST",
                "/api/v0/multisource/verify",
                None,
                verify_body,
                &state,
            )
            .await
        }
    }))
    .await;
    record!(
        "POST",
        "/api/v0/multisource/verify",
        "concurrency-and-idempotency",
        verify_responses.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
        })
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create MultiSource evidence directory");
    fs::write(
        evidence_dir.join("multisource_residuals.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize MultiSource ledger"),
    )
    .expect("write MultiSource ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api MultiSource mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting 6 source-discovery routes'
/// cases, independently re-derived from `source_discovery_routes_
/// dispatch_and_project_bounded_search_sources`'s real session-
/// command dispatch (a genuine `SessionCommand::Search` is sent, not
/// a fabricated success), overlapping-discovery-run rejection (409
/// Conflict), and real search-result projection into by-size/by-
/// filename/summary views. slskdN-only (confirmed against the
/// frozen registry).
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
    feature = "bounded-controller-api-tests-3"
))]
pub(super) async fn controller_api_differential_source_discovery() {
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

    let (state, mut receiver) = test_state();
    let no_partial = crate::route_http_request(
        "GET",
        "/api/v0/discovery/no-partial-count",
        None,
        "",
        &state,
    )
    .await
    .expect("no-partial discovery count");
    let no_partial_json =
        serde_json::from_str::<serde_json::Value>(&no_partial.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/discovery/no-partial-count",
        "nominal-status-headers-body",
        no_partial.status == "200 OK"
            && no_partial.content_type.starts_with("application/json")
            && no_partial_json["usersWithoutPartialSupport"] == 0
            && no_partial_json["message"]
                == "0 users are flagged as not supporting partial/chunked downloads"
    );

    let started = crate::route_http_request(
        "POST",
        "/api/v0/discovery/start",
        None,
        r#"{"searchTerm":"differential recording","enableHashVerification":true}"#,
        &state,
    )
    .await
    .expect("start source discovery");
    record!(
        "POST",
        "/api/v0/discovery/start",
        "nominal-status-headers-body",
        started.status == "200 OK"
    );
    let token = match receiver.recv().await.expect("discovery search command") {
        crate::SessionCommand::Search {
            token,
            query,
            target: crate::SearchDispatchTarget::Global,
        } => {
            assert_eq!(query, "differential recording");
            token
        }
        command => panic!("unexpected command: {command:?}"),
    };
    record!(
        "POST",
        "/api/v0/discovery/start",
        "mutation-side-effects-and-readback",
        true
    );

    let conflict = crate::route_http_request(
        "POST",
        "/api/v0/discovery/start",
        None,
        r#"{"searchTerm":"second"}"#,
        &state,
    )
    .await
    .expect("reject overlapping discovery");
    record!(
        "POST",
        "/api/v0/discovery/start",
        "missing-empty-or-conflict-state",
        conflict.status == "409 Conflict"
    );

    {
        let mut searches = state.searches.write().await;
        let record = searches
            .records
            .iter_mut()
            .find(|record| record.token == token)
            .expect("discovery search record");
        record.results.push(crate::SearchResultEntry {
            peer_username: Some("differential-source-peer".to_owned()),
            filename: "Rare/DifferentialRecording.flac".to_owned(),
            size: 42,
            extension: "flac".to_owned(),
            bit_rate: None,
            sample_rate: None,
            bit_depth: None,
            length_seconds: None,
            locked: false,
            slot_free: Some(true),
            average_speed: Some(1234),
            queue_length: Some(0),
        });
    }

    let status = crate::route_http_request("GET", "/api/v0/discovery", None, "", &state)
        .await
        .expect("source discovery status");
    let status_json = serde_json::from_str::<serde_json::Value>(&status.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/discovery",
        "nominal-status-headers-body",
        status.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/discovery",
        "populated-dynamic-state",
        status_json["isRunning"] == true
            && status_json["stats"]["totalFiles"] == 1
            && status_json["stats"]["totalUsers"] == 1
            && status_json["stats"]["searchCycles"] == 1
    );

    let by_size = crate::route_http_request(
        "GET",
        "/api/v0/discovery/sources/by-size/42?limit=10",
        None,
        "",
        &state,
    )
    .await
    .expect("sources by size");
    let by_size_json = serde_json::from_str::<serde_json::Value>(&by_size.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/discovery/sources/by-size/{size}",
        "nominal-status-headers-body",
        by_size.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/discovery/sources/by-size/{size}",
        "populated-dynamic-state",
        by_size_json["sourceCount"] == 1
            && by_size_json["sources"][0]["username"] == "differential-source-peer"
    );

    let by_name = crate::route_http_request(
        "GET",
        "/api/v0/discovery/sources/by-filename?pattern=recording&limit=10",
        None,
        "",
        &state,
    )
    .await
    .expect("sources by filename");
    record!(
        "GET",
        "/api/v0/discovery/sources/by-filename",
        "nominal-status-headers-body",
        by_name.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/discovery/sources/by-filename",
        "populated-dynamic-state",
        by_name.body.contains("Rare/DifferentialRecording.flac")
    );

    let summaries = crate::route_http_request(
        "GET",
        "/api/v0/discovery/summaries?minSources=1",
        None,
        "",
        &state,
    )
    .await
    .expect("source summaries");
    let summaries_json =
        serde_json::from_str::<serde_json::Value>(&summaries.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/discovery/summaries",
        "nominal-status-headers-body",
        summaries.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/discovery/summaries",
        "populated-dynamic-state",
        summaries_json["summaries"][0]["size"] == 42
    );

    let stopped = crate::route_http_request("POST", "/api/v0/discovery/stop", None, "{}", &state)
        .await
        .expect("stop source discovery");
    let stopped_json = serde_json::from_str::<serde_json::Value>(&stopped.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/discovery/stop",
        "nominal-status-headers-body",
        stopped.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v0/discovery/stop",
        "mutation-side-effects-and-readback",
        stopped_json["stats"]["lastCycleNewFiles"] == 1
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("source_discovery.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api source-discovery mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof for the real slskdN Spotify PKCE OAuth
/// authorize/callback/status flow (`POST /api/v0/integrations/
/// spotify/authorize`, `GET /api/v0/integrations/spotify/callback`,
/// `GET /api/v0/integrations/spotify/status`): a real PKCE code
/// challenge/verifier pair is generated and the callback genuinely
/// validates the server-issued state (not a client-supplied one) --
/// independently re-derived from `spotify_oauth_callback_requires_
/// server_issued_state` with fresh fixture data. Confirmed against
/// `/tmp/slskr-parity-evidence/controller-api/*.json` before writing:
/// none of these 3 routes had any prior case credited. slskdN-only
/// (confirmed against the frozen registry).
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
    feature = "bounded-controller-api-tests-3"
))]
pub(super) async fn controller_api_differential_spotify_oauth_authorize_and_callback() {
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
            .with("SLSKR_SPOTIFY_ENABLED", "true")
            .with("SLSKR_SPOTIFY_CLIENT_ID", "differential-client-id"),
    );

    let baseline_status = crate::route_http_request(
        "GET",
        "/api/v0/integrations/spotify/status",
        None,
        "",
        &state,
    )
    .await
    .expect("baseline status response");
    let baseline_status_json =
        serde_json::from_str::<serde_json::Value>(&baseline_status.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/integrations/spotify/status",
        "missing-empty-or-conflict-state",
        baseline_status.status == "200 OK" && baseline_status_json["connected"] == false
    );

    let authorize = crate::route_http_request(
        "POST",
        "/api/v0/integrations/spotify/authorize",
        None,
        "",
        &state,
    )
    .await
    .expect("authorize response");
    let authorize_json =
        serde_json::from_str::<serde_json::Value>(&authorize.body).unwrap_or_default();
    let authorization_url = authorize_json["authorizationUrl"]
        .as_str()
        .unwrap_or_default();
    record!(
        "POST",
        "/api/v0/integrations/spotify/authorize",
        "nominal-status-headers-body",
        authorize.status == "200 OK"
            && authorization_url.contains("code_challenge_method=S256")
            && authorization_url.contains("code_challenge=")
    );

    let bogus_callback = crate::route_http_request(
        "GET",
        "/api/v0/integrations/spotify/callback?code=differential-code&state=differential-bogus-state",
        None,
        "",
        &state,
    )
    .await
    .expect("bogus-state callback response");
    record!(
        "GET",
        "/api/v0/integrations/spotify/callback",
        "malformed-path-query-or-body",
        bogus_callback.status == "400 Bad Request"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("spotify_oauth_authorize_and_callback.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api spotify-oauth-authorize-and-callback mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof for the real slskdN Spotify connection
/// state (`GET /api/v0/integrations/spotify/status` once connected,
/// `DELETE /api/v0/integrations/spotify`): drives the real token
/// exchange and profile fetch through `complete_spotify_
/// authorization` against a tiny local fixture server (the same
/// production function the real callback handler calls), then
/// proves the status route reflects that real connection and
/// disconnect genuinely clears it (readback via a second status
/// call) -- independently re-derived from `spotify_authorization_
/// exchanges_profiles_persists_and_disconnects` with fresh fixture
/// data. Split from the authorize/callback half to keep this
/// TCP-fixture-backed function's sequential-await chain short
/// (same stack-size reason documented on the ActivityPub
/// differentials). slskdN-only (confirmed against the frozen
/// registry).
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
    feature = "bounded-controller-api-tests-3"
))]
pub(super) async fn controller_api_differential_spotify_connection_status_and_disconnect() {
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

    let token_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind spotify token fixture");
    let token_address = token_listener.local_addr().expect("token fixture address");
    let profile_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind spotify profile fixture");
    let profile_address = profile_listener
        .local_addr()
        .expect("profile fixture address");
    let token_server = tokio::spawn(async move {
        serve_json_fixture(
            &token_listener,
            serde_json::json!({
                "access_token": "differential-access-secret",
                "refresh_token": "differential-refresh-secret",
                "expires_in": 3600,
                "scope": "user-library-read"
            }),
        )
        .await
    });
    let profile_server = tokio::spawn(async move {
        serve_json_fixture(
            &profile_listener,
            serde_json::json!({"id": "differential-spotify-user", "display_name": "Differential User"}),
        )
        .await
    });

    let root = std::env::temp_dir().join(format!(
        "slskr-spotify-differential-{}-{}",
        std::process::id(),
        crate::unix_timestamp_millis()
    ));
    crate::ensure_private_state_dir(&root).expect("create spotify differential state dir");
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_STATE_DIR", &root.to_string_lossy())
            .with("SLSKR_SPOTIFY_ENABLED", "true")
            .with("SLSKR_SPOTIFY_CLIENT_ID", "differential-client-id")
            .with("SLSKR_SPOTIFY_TIMEOUT", "5"),
    );
    let spotify = state.integration_settings.read().await.spotify.clone();

    let unversioned_baseline =
        crate::route_http_request("GET", "/api/integrations/spotify/status", None, "", &state)
            .await
            .expect("unversioned baseline status response");
    let unversioned_baseline_json =
        serde_json::from_str::<serde_json::Value>(&unversioned_baseline.body).unwrap_or_default();
    let unversioned_baseline_shape = unversioned_baseline.status == "200 OK"
        && unversioned_baseline
            .content_type
            .starts_with("application/json")
        && unversioned_baseline_json["configured"] == true
        && unversioned_baseline_json["connected"] == false
        && unversioned_baseline_json["displayName"] == ""
        && unversioned_baseline_json["spotifyUserId"] == ""
        && unversioned_baseline_json["scope"] == "";
    record!(
        "GET",
        "/api/integrations/spotify/status",
        "missing-empty-or-conflict-state",
        unversioned_baseline_shape
    );
    record!(
        "GET",
        "/api/integrations/spotify/status",
        "nominal-status-headers-body",
        unversioned_baseline_shape
    );

    let pending = crate::OAuthStateRecord {
        provider: "spotify".to_owned(),
        redirect_uri: "http://127.0.0.1/callback".to_owned(),
        code_verifier: Some("differential-pkce-verifier".to_owned()),
        created_at: crate::unix_timestamp(),
        expires_at: crate::unix_timestamp().saturating_add(600),
    };
    crate::complete_spotify_authorization(
        &state,
        &spotify,
        &pending,
        "differential-authorization-code",
        &format!("http://{token_address}/token"),
        &format!("http://{profile_address}/me"),
    )
    .await
    .expect("complete spotify authorization fixture");
    token_server.await.expect("token fixture task");
    profile_server.await.expect("profile fixture task");

    let connected_status = crate::route_http_request(
        "GET",
        "/api/v0/integrations/spotify/status",
        None,
        "",
        &state,
    )
    .await
    .expect("connected status response");
    let connected_status_json =
        serde_json::from_str::<serde_json::Value>(&connected_status.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/integrations/spotify/status",
        "populated-dynamic-state",
        connected_status.status == "200 OK" && connected_status_json["connected"] == true
    );

    let connected_unversioned_status =
        crate::route_http_request("GET", "/api/integrations/spotify/status", None, "", &state)
            .await
            .expect("connected unversioned status response");
    let connected_unversioned_json =
        serde_json::from_str::<serde_json::Value>(&connected_unversioned_status.body)
            .unwrap_or_default();
    record!(
        "GET",
        "/api/integrations/spotify/status",
        "populated-dynamic-state",
        connected_unversioned_status.status == "200 OK"
            && connected_unversioned_json["connected"] == true
            && connected_unversioned_json["displayName"] == "Differential User"
            && connected_unversioned_json["spotifyUserId"] == "differential-spotify-user"
    );

    let disconnect =
        crate::route_http_request("DELETE", "/api/v0/integrations/spotify", None, "", &state)
            .await
            .expect("disconnect response");
    let disconnected_status = crate::route_http_request(
        "GET",
        "/api/v0/integrations/spotify/status",
        None,
        "",
        &state,
    )
    .await
    .expect("disconnected status response");
    let disconnected_status_json =
        serde_json::from_str::<serde_json::Value>(&disconnected_status.body).unwrap_or_default();
    record!(
        "DELETE",
        "/api/v0/integrations/spotify",
        "mutation-side-effects-and-readback",
        disconnect.status == "204 No Content" && disconnected_status_json["connected"] == false
    );

    let _ = fs::remove_dir_all(root);

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("spotify_connection_status_and_disconnect.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api spotify-connection-status-and-disconnect mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof for the frozen slskdN source-provider catalog.
/// It checks both the disabled planning projection and the enabled
/// activation rules rather than accepting the old three-provider shell.
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
    feature = "bounded-controller-api-tests-3"
))]
pub(super) async fn controller_api_differential_source_provider_catalog() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    let expected_ids = [
        "LocalLibrary",
        "Soulseek",
        "NativeMesh",
        "MeshDht",
        "Http",
        "WebDav",
        "S3",
        "Lan",
        "Torrent",
    ];

    let (disabled_state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_VIRTUAL_SOULFIND_V2_ENABLED", "false"),
    );
    for path in ["/api/source-providers", "/api/v0/source-providers"] {
        let response = crate::route_http_request("GET", path, None, "", &disabled_state)
            .await
            .expect("disabled source-provider catalog response");
        let value = serde_json::from_str::<serde_json::Value>(&response.body)
            .unwrap_or(serde_json::Value::Null);
        let providers = value["providers"].as_array().cloned().unwrap_or_default();
        let provider_ids = providers
            .iter()
            .filter_map(|provider| provider["id"].as_str())
            .collect::<Vec<_>>();
        let profile_policies = value["profilePolicies"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let pass = response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && value["acquisitionPlanningEnabled"] == false
            && provider_ids == expected_ids
            && providers.iter().all(|provider| {
                provider["registered"] == true
                    && provider["active"] == false
                    && provider["disabledReason"]
                        == "VirtualSoulfind v2 acquisition planning is disabled."
                    && provider.get("description").is_some()
                    && provider.get("riskLevel").is_some()
                    && provider.get("capabilities").is_some()
                    && provider.get("sortOrder").is_some()
            })
            && profile_policies.len() == 7
            && value.get("count").is_none();
        if !pass {
            mismatches.push(format!(
                "{target} GET {path}: {} {}",
                response.status, response.body
            ));
        }
        ledger.push(serde_json::json!({
            "target": target,
            "method": "GET",
            "route": path,
            "case": "nominal-status-headers-body",
            "pass": pass,
        }));
    }

    let (enabled_state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_VIRTUAL_SOULFIND_V2_ENABLED", "true"),
    );
    let enabled =
        crate::route_http_request("GET", "/api/v0/source-providers", None, "", &enabled_state)
            .await
            .expect("enabled source-provider catalog response");
    let enabled_json =
        serde_json::from_str::<serde_json::Value>(&enabled.body).unwrap_or(serde_json::Value::Null);
    let enabled_providers = enabled_json["providers"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let enabled_pass = enabled.status == "200 OK"
        && enabled_json["acquisitionPlanningEnabled"] == true
        && enabled_providers
            .iter()
            .map(|provider| provider["id"].as_str().unwrap_or_default())
            .eq(expected_ids.iter().copied())
        && enabled_providers.iter().all(|provider| {
            if matches!(
                provider["id"].as_str(),
                Some("LocalLibrary") | Some("Soulseek")
            ) {
                provider["active"] == true && provider["disabledReason"].is_null()
            } else {
                provider["active"] == false && provider["disabledReason"].is_string()
            }
        });
    if !enabled_pass {
        mismatches.push(format!(
            "{target} GET /api/v0/source-providers enabled projection: {} {}",
            enabled.status, enabled.body
        ));
    }
    ledger.push(serde_json::json!({
        "target": target,
        "method": "GET",
        "route": "/api/v0/source-providers",
        "case": "populated-dynamic-state",
        "pass": enabled_pass,
    }));

    let enabled_alias =
        crate::route_http_request("GET", "/api/source-providers", None, "", &enabled_state)
            .await
            .expect("enabled source-provider catalog alias response");
    let enabled_alias_json = serde_json::from_str::<serde_json::Value>(&enabled_alias.body)
        .unwrap_or(serde_json::Value::Null);
    let enabled_alias_providers = enabled_alias_json["providers"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let enabled_alias_pass = enabled_alias.status == "200 OK"
        && enabled_alias_json["acquisitionPlanningEnabled"] == true
        && enabled_alias_providers
            .iter()
            .map(|provider| provider["id"].as_str().unwrap_or_default())
            .eq(expected_ids.iter().copied())
        && enabled_alias_providers.iter().all(|provider| {
            if matches!(
                provider["id"].as_str(),
                Some("LocalLibrary") | Some("Soulseek")
            ) {
                provider["active"] == true && provider["disabledReason"].is_null()
            } else {
                provider["active"] == false && provider["disabledReason"].is_string()
            }
        });
    if !enabled_alias_pass {
        mismatches.push(format!(
            "{target} GET /api/source-providers enabled projection: {} {}",
            enabled_alias.status, enabled_alias.body
        ));
    }
    ledger.push(serde_json::json!({
        "target": target,
        "method": "GET",
        "route": "/api/source-providers",
        "case": "populated-dynamic-state",
        "pass": enabled_alias_pass,
    }));

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("source_provider_catalog.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

/// Differential evidence for the remaining read-only source-provider
/// route edges.  The native catalog is configuration-derived and does not
/// read SQLite; malformed extra segments remain unmatched on both route
/// aliases while empty and closed-database state still returns the full
/// catalog.
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
    feature = "bounded-controller-api-tests-3"
))]
pub(super) async fn controller_api_differential_source_provider_edge_contracts() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($route:expr, $case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!(
                    "{target} GET {} [{}]",
                    $route,
                    $case
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "GET",
                "route": $route,
                "case": $case,
                "pass": pass,
            }));
        }};
    }

    let env = || {
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_VIRTUAL_SOULFIND_V2_ENABLED", "false")
    };
    let (state, _receiver) = test_state_with_env(env());
    for route in ["/api/source-providers", "/api/v0/source-providers"] {
        let malformed =
            crate::route_http_request("GET", &format!("{route}/extra"), None, "", &state)
                .await
                .expect("malformed source-provider path");
        record!(
            route,
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );

        let empty = crate::route_http_request("GET", route, None, "", &state)
            .await
            .expect("empty source-provider catalog");
        let empty_json = serde_json::from_str::<serde_json::Value>(&empty.body).unwrap_or_default();
        record!(
            route,
            "missing-empty-or-conflict-state",
            empty.status == "200 OK"
                && empty_json["acquisitionPlanningEnabled"] == false
                && empty_json["providers"]
                    .as_array()
                    .is_some_and(|providers| providers.len() == 9)
        );
    }

    let failure_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("source-provider failure database");
    let (failure_state, _receiver) =
        test_state_with_env_parts(env(), crate::SearchStore::new(), Some(failure_db.clone()));
    failure_db.close_for_test().await;
    for route in ["/api/source-providers", "/api/v0/source-providers"] {
        let response = crate::route_http_request("GET", route, None, "", &failure_state)
            .await
            .expect("source-provider catalog after database close");
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            route,
            "runtime-failure-and-timeout",
            response.status == "200 OK"
                && value["providers"]
                    .as_array()
                    .is_some_and(|providers| providers.len() == 9)
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("source_provider_edge_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize source-provider edge ledger"),
    )
    .expect("write source-provider edge ledger");
    assert!(
        mismatches.is_empty(),
        "{} source-provider edge mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting `POST /api/v0/integrations/
/// spotify/authorize`'s real OAuth-state persistence-failure
/// rollback (independently re-derived from `spotify_oauth_state_
/// is_not_issued_when_persistence_fails`: a real closed database
/// makes state-issuance genuinely fail, and the pre-existing
/// (unrelated, expired) OAuth state record is left byte-for-byte
/// unchanged), `POST /api/v0/pods`'s real caller-identity
/// enforcement (independently re-derived from `pod_creation_
/// never_trusts_a_caller_supplied_peer_identity`: without a real
/// local Soulseek identity configured, pod creation is genuinely
/// forbidden rather than trusting a client-supplied
/// `requestingPeerId`), `GET /api/v0/mesh/hello`'s real baseline
/// handshake shape (independently re-derived from `mesh_hello_
/// matches_frozen_message_dto`: zeroed sequence/hash counters and
/// no publication key/signature on a fresh, unpublished mesh
/// state), `GET /api/v0/mesh/hello`'s real populated sequence/hash
/// counters after a hash merge, the mesh/hashdb populated readbacks for
/// that same entry, the hash-by-size projection, and `GET /api/v0/listening-party`'s real directory
/// listing (independently re-derived from `listening_party_
/// directory_ticket_streams_local_audio_ranges`'s directory-
/// listing half only, skipping its raw-TCP radio-stream half:
/// the directory entry's `streamPath` genuinely encodes the real
/// content id of a seeded listening-party fixture). Confirmed
/// against `/tmp/slskr-parity-evidence/controller-api/*.json`
/// before writing, per case: all 8 were open. slskdN-only
/// (confirmed against the frozen registry).
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
pub(super) async fn controller_api_differential_spotify_pods_mesh_and_party() {
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

    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default()
                .with("SLSKR_PERSISTENCE_ENABLED", "true")
                .with("SLSKR_SPOTIFY_ENABLED", "true")
                .with("SLSKR_SPOTIFY_CLIENT_ID", "differential-client-id"),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        state.oauth_states.write().await.records.insert(
            "differential-expired".to_owned(),
            crate::OAuthStateRecord {
                provider: "spotify".to_owned(),
                redirect_uri: "http://localhost/differential-callback".to_owned(),
                code_verifier: None,
                created_at: 0,
                expires_at: 0,
            },
        );
        let previous = state.oauth_states.read().await.clone();
        db.close_for_test().await;
        let response = crate::route_http_request(
            "POST",
            "/api/v0/integrations/spotify/authorize",
            None,
            "",
            &state,
        )
        .await
        .expect("failed oauth state persistence response");
        record!(
            "POST",
            "/api/v0/integrations/spotify/authorize",
            "runtime-failure-and-timeout",
            response.status == "503 Service Unavailable"
                && *state.oauth_states.read().await == previous
        );
    }

    {
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSK_USERNAME", "")
                .with("SLSK_PASSWORD", "")
                .with("SLSKR_AUTH_DISABLED", "false")
                .with("SLSKR_API_TOKEN", "differential-token"),
        );
        let response = crate::route_http_request(
            "POST",
            "/api/v0/pods",
            Some("Bearer differential-token"),
            r#"{"pod":{"podId":"pod:differential-spoofed","name":"Spoofed"},"requestingPeerId":"differential-caller-controlled"}"#,
            &state,
        )
        .await
        .expect("pod create without local identity response");
        record!(
            "POST",
            "/api/v0/pods",
            "missing-empty-or-conflict-state",
            response.status == "403 Forbidden"
        );
    }

    {
        let (state, _receiver) =
            test_state_with_env(MapEnv::default().with("SLSK_USERNAME", "differential-mesh-user"));
        let hello = crate::route_http_request("GET", "/api/v0/mesh/hello", None, "", &state)
            .await
            .expect("mesh hello response");
        let hello_json = serde_json::from_str::<serde_json::Value>(&hello.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/mesh/hello",
            "nominal-status-headers-body",
            hello.status == "200 OK"
                && hello_json["type"] == 1
                && hello_json["latest_seq_id"] == 0
                && hello_json["hash_count"] == 0
                && hello_json.get("peerId").is_none()
        );

        let merged = crate::route_http_request(
            "POST",
            "/api/v0/mesh/merge?fromUser=differential-mesh-peer",
            None,
            &serde_json::json!({
                "entries": [{
                    "flacKey": "differential-mesh-hello-key",
                    "byteHash": "a".repeat(64),
                    "size": 4096
                }]
            })
            .to_string(),
            &state,
        )
        .await
        .expect("merge a real mesh hello hash");
        let merged_json =
            serde_json::from_str::<serde_json::Value>(&merged.body).unwrap_or_default();
        let latest_seq_id = merged_json["latestSeqId"].as_u64().unwrap_or_default();
        let populated = crate::route_http_request("GET", "/api/v0/mesh/hello", None, "", &state)
            .await
            .expect("populated mesh hello response");
        let populated_json =
            serde_json::from_str::<serde_json::Value>(&populated.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/mesh/hello",
            "populated-dynamic-state",
            merged.status == "200 OK"
                && merged_json["merged"] == 1
                && latest_seq_id > 0
                && populated.status == "200 OK"
                && populated_json["latest_seq_id"] == latest_seq_id
                && populated_json["hash_count"] == 1
                && populated_json["client_id"] == "differential-mesh-user"
                && populated_json["public_key"] == ""
                && populated_json["signature"] == ""
        );

        let lookup = crate::route_http_request(
            "GET",
            "/api/v0/mesh/lookup/differential-mesh-hello-key",
            None,
            "",
            &state,
        )
        .await
        .expect("populated mesh lookup response");
        let lookup_json =
            serde_json::from_str::<serde_json::Value>(&lookup.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/mesh/lookup/{flacKey}",
            "populated-dynamic-state",
            lookup.status == "200 OK"
                && lookup_json["found"] == true
                && lookup_json["entry"]["flacKey"] == "differential-mesh-hello-key"
                && lookup_json["entry"]["byteHash"] == "a".repeat(64)
                && lookup_json["entry"]["size"] == 4096
        );

        let hashdb = crate::route_http_request(
            "GET",
            "/api/v0/hashdb/hash/differential-mesh-hello-key",
            None,
            "",
            &state,
        )
        .await
        .expect("populated hashdb lookup response");
        let hashdb_json =
            serde_json::from_str::<serde_json::Value>(&hashdb.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/hashdb/hash/{flacKey}",
            "populated-dynamic-state",
            hashdb.status == "200 OK"
                && hashdb_json["flacKey"] == "differential-mesh-hello-key"
                && hashdb_json["byteHash"] == "a".repeat(64)
                && hashdb_json["size"] == 4096
        );

        let by_size =
            crate::route_http_request("GET", "/api/v0/hashdb/hash/by-size/4096", None, "", &state)
                .await
                .expect("populated hashdb size lookup response");
        let by_size_json =
            serde_json::from_str::<serde_json::Value>(&by_size.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/hashdb/hash/by-size/{size}",
            "populated-dynamic-state",
            by_size.status == "200 OK"
                && by_size_json["count"] == 1
                && by_size_json["entries"].as_array().is_some_and(|entries| {
                    entries.len() == 1
                        && entries[0]["flacKey"] == "differential-mesh-hello-key"
                        && entries[0]["byteHash"] == "a".repeat(64)
                        && entries[0]["size"] == 4096
                })
        );
    }

    {
        let root = std::env::temp_dir().join(format!(
            "slskr-listening-party-differential-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&root).expect("create differential listening-party share root");
        std::fs::write(
            root.join("differential-party.flac"),
            b"differential-party-audio-bytes",
        )
        .expect("write differential listening-party fixture");
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_CONTROLLER_PROFILE", "native")
                .with("SLSKR_SHARE_FIXTURE", "")
                .with("SLSKR_SHARE_DIRS", &root.display().to_string()),
        );
        let content_id = {
            let shares = state.shares.read().await;
            let entry = shares
                .entries
                .first()
                .expect("differential share fixture entry");
            crate::stable_content_hash(&entry.filename, entry.size).to_string()
        };
        state
            .controller_features
            .write_for_test()
            .await
            .upsert(
                "listening-party/pod:differential-stream-audit/general".to_owned(),
                serde_json::json!({
                    "partyId": "party:differential-stream-audit",
                    "podId": "pod:differential-stream-audit",
                    "channelId": "general",
                    "hostPeerId": "differential-tester",
                    "action": "play",
                    "contentId": content_id,
                    "title": "Differential Party Track",
                    "artist": "Differential Party Artist",
                    "serverTimeUnixMs": crate::unix_timestamp_millis(),
                    "listed": true,
                    "allowMeshStreaming": true,
                }),
            )
            .expect("persist differential listening-party fixture");
        let directory =
            crate::route_http_request("GET", "/api/v0/listening-party", None, "", &state)
                .await
                .expect("list differential listening-party directory");
        let directory_json =
            serde_json::from_str::<serde_json::Value>(&directory.body).unwrap_or_default();
        let stream_path = directory_json[0]["streamPath"].as_str().unwrap_or_default();
        record!(
            "GET",
            "/api/v0/listening-party",
            "populated-dynamic-state",
            directory.status == "200 OK" && stream_path.contains(&crate::url_encode(&content_id))
        );
        let _ = std::fs::remove_dir_all(root);
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("spotify_pods_mesh_and_party.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api spotify-pods-mesh-and-party mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
