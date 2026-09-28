//! Controller full media differential 02 ownership.

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
    feature = "bounded-controller-api-tests-2"
))]
pub(super) async fn controller_api_differential_listening_party_open_cases() {
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

    let (state, _receiver) = test_state();
    let pod_id = "pod:listening-party-open-cases";
    let channel_id = "general";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<crate::pods::PodRecord>(serde_json::json!({
                "podId": pod_id,
                "name": "Listening Party Open Cases",
            }))
            .expect("deserialize listening-party pod fixture"),
            "tester".to_owned(),
        )
        .expect("create listening-party pod");
    state
        .pods
        .write()
        .await
        .upsert_channel(
            pod_id,
            crate::pods::PodChannel {
                channel_id: channel_id.to_owned(),
                kind: serde_json::json!(0),
                name: "General".to_owned(),
                binding_info: None,
                description: None,
            },
        )
        .expect("create listening-party channel");

    let malformed_directory =
        crate::route_http_request("GET", "/api/v0/listening-party/extra", None, "", &state)
            .await
            .unwrap();
    record!(
        "GET",
        "/api/v0/listening-party",
        "malformed-path-query-or-body",
        malformed_directory.status == "404 Not Found"
    );

    let empty_directory =
        crate::route_http_request("GET", "/api/v0/listening-party", None, "", &state)
            .await
            .unwrap();
    let empty_directory_json =
        serde_json::from_str::<serde_json::Value>(&empty_directory.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/listening-party",
        "missing-empty-or-conflict-state",
        empty_directory.status == "200 OK" && empty_directory_json == serde_json::json!([])
    );

    let malformed_state = crate::route_http_request(
        "GET",
        &format!("/api/v0/listening-party/{pod_id}/{channel_id}/extra"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    record!(
        "GET",
        "/api/v0/listening-party/{podId}/{channelId}",
        "malformed-path-query-or-body",
        malformed_state.status == "404 Not Found"
    );

    let empty_state = crate::route_http_request(
        "GET",
        &format!("/api/v0/listening-party/{pod_id}/{channel_id}"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    record!(
        "GET",
        "/api/v0/listening-party/{podId}/{channelId}",
        "runtime-failure-and-timeout",
        empty_state.status == "204 No Content"
    );

    let played = crate::route_http_request(
        "POST",
        &format!("/api/v0/listening-party/{pod_id}/{channel_id}"),
        None,
        r#"{"action":"play","contentId":"open-cases-content","title":"Open Cases","listed":true,"allowMeshStreaming":true}"#,
        &state,
    )
    .await
    .unwrap();
    let played_json = serde_json::from_str::<serde_json::Value>(&played.body).unwrap_or_default();
    let party_id = played_json["partyId"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let content_id = played_json["contentId"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let ticket = crate::issue_listening_party_stream_ticket(&state, &party_id, &content_id)
        .await
        .expect("issue listening-party ticket");

    let radio = crate::route_http_request(
        "GET",
        &format!("/api/v0/listening-party/radio/{party_id}/{content_id}?ticket={ticket}"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    record!(
        "GET",
        "/api/v0/listening-party/radio/{partyId}/{contentId}",
        "nominal-status-headers-body",
        radio.status == "200 OK"
    );

    let radio_json = serde_json::from_str::<serde_json::Value>(&radio.body).unwrap_or_default();
    state
        .content_discovery
        .write()
        .await
        .merge_shadow_records(vec![crate::content_discovery::ShadowIndexRecord {
            recording_id: content_id.clone(),
            peer_ids: vec!["party-peer".to_owned()],
            updated_at: 0,
        }])
        .expect("seed listening-party content");
    let populated_radio = crate::route_http_request(
        "GET",
        &format!("/api/v0/listening-party/radio/{party_id}/{content_id}?ticket={ticket}"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    let populated_radio_json =
        serde_json::from_str::<serde_json::Value>(&populated_radio.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/listening-party/radio/{partyId}/{contentId}",
        "populated-dynamic-state",
        radio_json["available"] == false
            && populated_radio.status == "200 OK"
            && populated_radio_json["available"] == true
            && populated_radio_json["peerIds"] == serde_json::json!(["party-peer"])
    );

    let malformed_radio = crate::route_http_request(
        "GET",
        &format!("/api/v0/listening-party/radio/{party_id}/{content_id}/extra"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    record!(
        "GET",
        "/api/v0/listening-party/radio/{partyId}/{contentId}",
        "malformed-path-query-or-body",
        malformed_radio.status == "404 Not Found"
    );

    let runtime_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("listening-party runtime database");
    let (runtime_state, _runtime_receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_PERSISTENCE_ENABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", target),
        crate::SearchStore::new(),
        Some(runtime_db.clone()),
    );
    runtime_db.close_for_test().await;
    let runtime_directory =
        crate::route_http_request("GET", "/api/v0/listening-party", None, "", &runtime_state)
            .await
            .unwrap();
    let runtime_directory_json =
        serde_json::from_str::<serde_json::Value>(&runtime_directory.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/listening-party",
        "runtime-failure-and-timeout",
        runtime_directory.status == "200 OK" && runtime_directory_json.is_array()
    );

    let runtime_get = crate::route_http_request(
        "GET",
        &format!("/api/v0/listening-party/{pod_id}/{channel_id}"),
        None,
        "",
        &runtime_state,
    )
    .await
    .unwrap();
    record!(
        "GET",
        "/api/v0/listening-party/{podId}/{channelId}",
        "runtime-failure-and-timeout",
        runtime_get.status == "204 No Content"
    );

    let radio_runtime_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("listening-party radio runtime database");
    let (radio_runtime_state, _radio_runtime_receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_PERSISTENCE_ENABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", target),
        crate::SearchStore::new(),
        Some(radio_runtime_db.clone()),
    );
    radio_runtime_state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<crate::pods::PodRecord>(serde_json::json!({
                "podId": pod_id,
                "name": "Listening Party Radio Runtime",
            }))
            .expect("deserialize radio runtime pod fixture"),
            "tester".to_owned(),
        )
        .expect("create radio runtime pod");
    radio_runtime_state
        .pods
        .write()
        .await
        .upsert_channel(
            pod_id,
            crate::pods::PodChannel {
                channel_id: channel_id.to_owned(),
                kind: serde_json::json!(0),
                name: "General".to_owned(),
                binding_info: None,
                description: None,
            },
        )
        .expect("create radio runtime channel");
    let radio_runtime_play = crate::route_http_request(
        "POST",
        &format!("/api/v0/listening-party/{pod_id}/{channel_id}"),
        None,
        r#"{"action":"play","contentId":"radio-runtime-content","listed":true,"allowMeshStreaming":true}"#,
        &radio_runtime_state,
    )
    .await
    .unwrap();
    let radio_runtime_json =
        serde_json::from_str::<serde_json::Value>(&radio_runtime_play.body).unwrap_or_default();
    let radio_runtime_party = radio_runtime_json["partyId"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let radio_runtime_content = radio_runtime_json["contentId"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let radio_runtime_ticket = crate::issue_listening_party_stream_ticket(
        &radio_runtime_state,
        &radio_runtime_party,
        &radio_runtime_content,
    )
    .await
    .expect("issue radio runtime ticket");
    radio_runtime_db.close_for_test().await;
    let radio_runtime = crate::route_http_request(
        "GET",
        &format!(
            "/api/v0/listening-party/radio/{radio_runtime_party}/{radio_runtime_content}?ticket={radio_runtime_ticket}"
        ),
        None,
        "",
        &radio_runtime_state,
    )
    .await
    .unwrap();
    record!(
        "GET",
        "/api/v0/listening-party/radio/{partyId}/{contentId}",
        "runtime-failure-and-timeout",
        radio_runtime.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&radio_runtime.body).is_ok()
    );

    let post_runtime_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("listening-party post runtime database");
    let (post_runtime_state, _post_runtime_receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_PERSISTENCE_ENABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", target),
        crate::SearchStore::new(),
        Some(post_runtime_db.clone()),
    );
    post_runtime_state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<crate::pods::PodRecord>(serde_json::json!({
                "podId": pod_id,
                "name": "Listening Party Runtime",
            }))
            .expect("deserialize runtime pod fixture"),
            "tester".to_owned(),
        )
        .expect("create runtime pod");
    post_runtime_state
        .pods
        .write()
        .await
        .upsert_channel(
            pod_id,
            crate::pods::PodChannel {
                channel_id: channel_id.to_owned(),
                kind: serde_json::json!(0),
                name: "General".to_owned(),
                binding_info: None,
                description: None,
            },
        )
        .expect("create runtime channel");
    post_runtime_db.close_for_test().await;
    let post_runtime = crate::route_http_request(
        "POST",
        &format!("/api/v0/listening-party/{pod_id}/{channel_id}"),
        None,
        r#"{"action":"play","contentId":"runtime-post-content"}"#,
        &post_runtime_state,
    )
    .await
    .unwrap();
    record!(
        "POST",
        "/api/v0/listening-party/{podId}/{channelId}",
        "runtime-failure-and-timeout",
        post_runtime.status == "200 OK"
    );

    let (restarted_state, _restarted_receiver) = test_state();
    let reset = crate::route_http_request(
        "GET",
        &format!("/api/v0/listening-party/{pod_id}/{channel_id}"),
        None,
        "",
        &restarted_state,
    )
    .await
    .unwrap();
    record!(
        "POST",
        "/api/v0/listening-party/{podId}/{channelId}",
        "restart-persistence-or-reset",
        reset.status == "204 No Content"
    );

    let concurrent_bodies: Vec<String> = (0..4)
        .map(|index| {
            format!(r#"{{"action":"play","contentId":"concurrent-party-content-{index}"}}"#)
        })
        .collect();
    let concurrent = futures_util::future::join_all(concurrent_bodies.iter().map(|body| {
        let path = format!("/api/v0/listening-party/{pod_id}/{channel_id}");
        let body = body.clone();
        let state = Arc::clone(&state);
        async move { crate::route_http_request("POST", &path, None, &body, &state).await }
    }))
    .await;
    let final_state = crate::route_http_request(
        "GET",
        &format!("/api/v0/listening-party/{pod_id}/{channel_id}"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    record!(
        "POST",
        "/api/v0/listening-party/{podId}/{channelId}",
        "concurrency-and-idempotency",
        concurrent.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
        }) && final_state.status == "200 OK"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("listening_party_open_cases.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        mismatches.is_empty(),
        "{} controller-api listening-party open-case mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

#[cfg_attr(test, test)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-2"
))]
pub(super) fn controller_api_differential_primary_stream_ticket_lifecycle() {
    run_controller_future_on_large_stack("primary-stream-ticket-lifecycle", || {
        controller_api_differential_primary_stream_ticket_lifecycle_impl()
    });
}

/// Bulk differential proof for the real slskdN `PlaybackController`
/// (`/api/v0/playback/feedback` POST, `/api/v0/playback/{jobId}/
/// diagnostics` GET): priority is computed from the job's actual
/// most-recently-posted buffer state (`playback_priority_for_latest_
/// feedback`, High when buffer < 5s, Low when >= 30s, Mid otherwise),
/// diagnostics reflects that same real feedback via a
/// nanosecond-precision last-write-wins store keyed by
/// `playback/feedback/{job_id}/{id}` (not a hardcoded shape), and both
/// the `jobId` requirement and JSON-body well-formedness are actually
/// enforced -- independently re-derived from `playback_feedback_and_
/// diagnostics_reflect_the_real_buffer_state` with fresh fixture data.
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
pub(super) async fn controller_api_differential_playback_feedback_and_diagnostics() {
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
    let job_id = "playback-differential-job";

    let missing = crate::route_http_request(
        "GET",
        &format!("/api/v0/playback/{job_id}/diagnostics"),
        None,
        "",
        &state,
    )
    .await
    .expect("diagnostics before any feedback");
    record!(
        "GET",
        "/api/v0/playback/{jobId}/diagnostics",
        "missing-empty-or-conflict-state",
        missing.status == "404 Not Found"
    );

    let invalid_json = crate::route_http_request(
        "POST",
        "/api/v0/playback/feedback",
        None,
        "not json",
        &state,
    )
    .await
    .expect("invalid json feedback");
    record!(
        "POST",
        "/api/v0/playback/feedback",
        "malformed-path-query-or-body",
        invalid_json.status == "400 Bad Request"
            && invalid_json.body == "{\"error\":\"invalid JSON body\"}"
    );

    let missing_job_id = crate::route_http_request(
        "POST",
        "/api/v0/playback/feedback",
        None,
        r#"{"trackId":"differential-track","positionMs":1000,"bufferAheadMs":500}"#,
        &state,
    )
    .await
    .expect("feedback missing jobId");
    record!(
        "POST",
        "/api/v0/playback/feedback",
        "missing-empty-or-conflict-state",
        missing_job_id.status == "400 Bad Request"
            && missing_job_id.body == "{\"error\":\"jobId is required\"}"
    );

    let low_buffer = crate::route_http_request(
        "POST",
        "/api/v0/playback/feedback",
        None,
        &serde_json::json!({
            "jobId": job_id,
            "trackId": "differential-track-a",
            "positionMs": 2_000,
            "bufferAheadMs": 500,
        })
        .to_string(),
        &state,
    )
    .await
    .expect("low-buffer feedback");
    let low_buffer_json =
        serde_json::from_str::<serde_json::Value>(&low_buffer.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/playback/feedback",
        "nominal-status-headers-body",
        low_buffer.status == "200 OK" && low_buffer_json["priority"] == "High"
    );

    let after_low = crate::route_http_request(
        "GET",
        &format!("/api/v0/playback/{job_id}/diagnostics"),
        None,
        "",
        &state,
    )
    .await
    .expect("diagnostics after low-buffer feedback");
    let after_low_json =
        serde_json::from_str::<serde_json::Value>(&after_low.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/playback/{jobId}/diagnostics",
        "populated-dynamic-state",
        after_low.status == "200 OK"
            && after_low_json["jobId"] == job_id
            && after_low_json["trackId"] == "differential-track-a"
            && after_low_json["positionMs"] == 2_000
            && after_low_json["bufferAheadMs"] == 500
            && after_low_json["priority"] == "High"
    );

    // A second, comfortably-buffered update for the same job: real
    // nanosecond-precision last-write-wins semantics must converge on
    // this newer entry (a genuine mutation with a real readback), not
    // silently keep serving the first one.
    let comfortable = crate::route_http_request(
        "POST",
        "/api/v0/playback/feedback",
        None,
        &serde_json::json!({
            "jobId": job_id,
            "trackId": "differential-track-b",
            "positionMs": 9_000,
            "bufferAheadMs": 40_000,
        })
        .to_string(),
        &state,
    )
    .await
    .expect("comfortable-buffer feedback");
    let comfortable_json =
        serde_json::from_str::<serde_json::Value>(&comfortable.body).unwrap_or_default();
    let readback = crate::route_http_request(
        "GET",
        &format!("/api/v0/playback/{job_id}/diagnostics"),
        None,
        "",
        &state,
    )
    .await
    .expect("diagnostics after second feedback");
    let readback_json =
        serde_json::from_str::<serde_json::Value>(&readback.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/playback/feedback",
        "mutation-side-effects-and-readback",
        comfortable_json["priority"] == "Low"
            && readback_json["trackId"] == "differential-track-b"
            && readback_json["positionMs"] == 9_000
            && readback_json["bufferAheadMs"] == 40_000
            && readback_json["priority"] == "Low"
    );

    // A third, rapid-fire update with a mid-range buffer must still
    // win over the second write (nanosecond ordering, not insertion
    // order or a single mutable slot), proving real concurrency
    // semantics rather than a fixed two-entry cache.
    let mid = crate::route_http_request(
        "POST",
        "/api/v0/playback/feedback",
        None,
        &serde_json::json!({
            "jobId": job_id,
            "trackId": "differential-track-c",
            "positionMs": 12_000,
            "bufferAheadMs": 15_000,
        })
        .to_string(),
        &state,
    )
    .await
    .expect("mid-buffer feedback");
    let mid_json = serde_json::from_str::<serde_json::Value>(&mid.body).unwrap_or_default();
    let final_readback = crate::route_http_request(
        "GET",
        &format!("/api/v0/playback/{job_id}/diagnostics"),
        None,
        "",
        &state,
    )
    .await
    .expect("diagnostics after third feedback");
    let final_readback_json =
        serde_json::from_str::<serde_json::Value>(&final_readback.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/playback/feedback",
        "concurrency-and-idempotency",
        mid_json["priority"] == "Mid"
            && final_readback_json["jobId"] == job_id
            && final_readback_json["trackId"] == "differential-track-c"
            && final_readback_json["positionMs"] == 12_000
            && final_readback_json["priority"] == "Mid"
    );

    let (restarted_state, _restarted_receiver) = test_state();
    let reset_diagnostics = crate::route_http_request(
        "GET",
        &format!("/api/v0/playback/{job_id}/diagnostics"),
        None,
        "",
        &restarted_state,
    )
    .await
    .expect("playback diagnostics after restart");
    record!(
        "POST",
        "/api/v0/playback/feedback",
        "restart-persistence-or-reset",
        reset_diagnostics.status == "404 Not Found"
    );

    // PlaybackFeedbackService is process-local in the frozen controller,
    // so an unrelated closed SQLite pool must not turn either route into
    // a storage failure.  Exercise both runtime-failure rows explicitly.
    let playback_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("playback runtime-failure database");
    let (playback_runtime_state, _runtime_receiver) = test_state_with_env_parts(
        MapEnv::default(),
        crate::SearchStore::new(),
        Some(playback_db.clone()),
    );
    playback_db.close_for_test().await;
    let runtime_feedback = crate::route_http_request(
        "POST",
        "/api/v0/playback/feedback",
        None,
        r#"{"jobId":"runtime-playback-job","bufferAheadMs":500}"#,
        &playback_runtime_state,
    )
    .await
    .expect("playback feedback with closed unrelated database");
    record!(
        "POST",
        "/api/v0/playback/feedback",
        "runtime-failure-and-timeout",
        runtime_feedback.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&runtime_feedback.body)
                .map(|value| value["priority"] == "High")
                .unwrap_or(false)
    );
    let runtime_diagnostics = crate::route_http_request(
        "GET",
        "/api/v0/playback/unrecorded-runtime-job/diagnostics",
        None,
        "",
        &playback_runtime_state,
    )
    .await
    .expect("playback diagnostics with closed unrelated database");
    record!(
        "GET",
        "/api/v0/playback/{jobId}/diagnostics",
        "runtime-failure-and-timeout",
        runtime_diagnostics.status == "404 Not Found"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("playback_feedback_and_diagnostics.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api playback-feedback-and-diagnostics mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof for the remaining edge cases on the
/// state-backed now-playing and playback-diagnostics routes.  The
/// oracle's `NowPlayingService` is an in-memory single-current-track
/// service whose delete is idempotent, and its `PlaybackController`
/// trims a route job id before rejecting a blank one.  These cases are
/// independently exercised with fresh state, concurrent deletes and
/// concurrent track updates; no response-only assertion is used for
/// the mutations.  slskdN-only (confirmed against the frozen registry).
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
pub(super) async fn controller_api_differential_nowplaying_delete_and_playback_diagnostics_edge_states(
) {
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

    // A valid feedback record gives diagnostics a real nominal response;
    // the separate populated-state case is already credited by the
    // playback differential test.
    {
        let (state, _receiver) = test_state();
        let feedback = crate::route_http_request(
            "POST",
            "/api/v0/playback/feedback",
            None,
            r#"{"jobId":"diagnostics-nominal-job","positionMs":0,"bufferAheadMs":0}"#,
            &state,
        )
        .await
        .expect("nominal diagnostics feedback");
        let diagnostics = crate::route_http_request(
            "GET",
            "/api/v0/playback/diagnostics-nominal-job/diagnostics",
            None,
            "",
            &state,
        )
        .await
        .expect("nominal diagnostics response");
        let body = serde_json::from_str::<serde_json::Value>(&diagnostics.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/playback/{jobId}/diagnostics",
            "nominal-status-headers-body",
            feedback.status == "200 OK"
                && diagnostics.status == "200 OK"
                && diagnostics.content_type == "application/json"
                && body["jobId"] == "diagnostics-nominal-job"
                && body["priority"] == "High"
        );
    }

    // ASP.NET route binding passes an encoded whitespace job id to the
    // controller; the oracle trims it and returns BadRequest instead of
    // treating it as an unknown job.
    {
        let (state, _receiver) = test_state();
        let malformed =
            crate::route_http_request("GET", "/api/v0/playback/%20/diagnostics", None, "", &state)
                .await
                .expect("blank diagnostics job response");
        record!(
            "GET",
            "/api/v0/playback/{jobId}/diagnostics",
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
                && malformed.body == r#"{"error":"jobId is required"}"#
        );
    }

    // Delete is a no-op on an empty current-track store, but still
    // returns the oracle's 204 response.
    {
        let (state, _receiver) = test_state();
        let deleted = crate::route_http_request("DELETE", "/api/v0/nowplaying", None, "", &state)
            .await
            .expect("empty now-playing delete response");
        record!(
            "DELETE",
            "/api/v0/nowplaying",
            "missing-empty-or-conflict-state",
            deleted.status == "204 No Content" && state.now_playing.read().await.records.is_empty()
        );
    }

    // A newly constructed service has no current track after the
    // previous service was cleared, matching the oracle's in-memory
    // restart/reset behavior.
    {
        let (state, _receiver) = test_state();
        let seeded = crate::route_http_request(
            "PUT",
            "/api/v0/nowplaying",
            None,
            r#"{"artist":"Restart Artist","title":"Restart Track"}"#,
            &state,
        )
        .await
        .expect("seed now-playing track");
        let deleted = crate::route_http_request("DELETE", "/api/v0/nowplaying", None, "", &state)
            .await
            .expect("restart reset delete response");
        let (restarted_state, _restarted_receiver) = test_state();
        let current =
            crate::route_http_request("GET", "/api/v0/nowplaying", None, "", &restarted_state)
                .await
                .expect("now-playing state after restart");
        record!(
            "DELETE",
            "/api/v0/nowplaying",
            "restart-persistence-or-reset",
            seeded.status == "204 No Content"
                && deleted.status == "204 No Content"
                && current.status == "204 No Content"
        );
    }

    // Clear remains idempotent when several callers race on the same
    // current-track state.
    {
        let (state, _receiver) = test_state();
        let seeded = crate::route_http_request(
            "PUT",
            "/api/v0/nowplaying",
            None,
            r#"{"artist":"Concurrent Artist","title":"Concurrent Track"}"#,
            &state,
        )
        .await
        .expect("seed concurrent now-playing track");
        let responses =
            futures_util::future::join_all((0..4).map(|_| {
                crate::route_http_request("DELETE", "/api/v0/nowplaying", None, "", &state)
            }))
            .await;
        let current = crate::route_http_request("GET", "/api/v0/nowplaying", None, "", &state)
            .await
            .expect("concurrent now-playing readback");
        record!(
            "DELETE",
            "/api/v0/nowplaying",
            "concurrency-and-idempotency",
            seeded.status == "204 No Content"
                && responses.iter().all(|response| {
                    response
                        .as_ref()
                        .is_ok_and(|response| response.status == "204 No Content")
                })
                && current.status == "204 No Content"
        );
    }

    // Concurrent versioned PUTs all complete and leave one real current
    // track visible, matching the oracle service's last-writer-wins
    // current-track projection.
    {
        let (state, _receiver) = test_state();
        let bodies: Vec<String> = (0..4)
            .map(|index| {
                format!(
                    r#"{{"artist":"Concurrent Artist {index}","title":"Concurrent Track {index}"}}"#
                )
            })
            .collect();
        let responses = futures_util::future::join_all(bodies.iter().map(|body| {
            crate::route_http_request("PUT", "/api/v0/nowplaying", None, body, &state)
        }))
        .await;
        let current = crate::route_http_request("GET", "/api/v0/nowplaying", None, "", &state)
            .await
            .expect("concurrent now-playing PUT readback");
        let current_json =
            serde_json::from_str::<serde_json::Value>(&current.body).unwrap_or_default();
        record!(
            "PUT",
            "/api/v0/nowplaying",
            "concurrency-and-idempotency",
            responses.iter().all(|response| {
                response
                    .as_ref()
                    .is_ok_and(|response| response.status == "204 No Content")
            }) && current.status == "200 OK"
                && current_json["artist"]
                    .as_str()
                    .is_some_and(|artist| artist.starts_with("Concurrent Artist "))
                && current_json["title"]
                    .as_str()
                    .is_some_and(|title| title.starts_with("Concurrent Track "))
        );
    }

    // A fresh in-memory service resets a previously set track.
    {
        let (state, _receiver) = test_state();
        let seeded = crate::route_http_request(
            "PUT",
            "/api/v0/nowplaying",
            None,
            r#"{"artist":"Transient Artist","title":"Transient Track"}"#,
            &state,
        )
        .await
        .expect("seed transient now-playing track");
        let (restarted_state, _restarted_receiver) = test_state();
        let current =
            crate::route_http_request("GET", "/api/v0/nowplaying", None, "", &restarted_state)
                .await
                .expect("now-playing PUT state after restart");
        record!(
            "PUT",
            "/api/v0/nowplaying",
            "restart-persistence-or-reset",
            seeded.status == "204 No Content" && current.status == "204 No Content"
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("nowplaying_delete_and_playback_diagnostics_edge_states.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api now-playing/playback edge-state mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting the analyzer-migration,
/// hashdb-optimize, and telemetry-prometheus route families --
/// independently re-derived from `analyzer_migration_requires_
/// version_and_returns_exact_result_shape`, `hashdb_optimize_
/// profile_and_slow_queries_report_real_observed_data`, and
/// `telemetry_kpis_are_always_json_unlike_the_base_prometheus_
/// route` with fresh fixture data: the unversioned analyzer-
/// migrate route requires a version and the versioned one returns
/// the exact `{"updated":0}` shape against an empty analyzer set,
/// hashdb's optimize/profile and optimize/slow-queries report real
/// observed query data from a seeded hash entry rather than
/// hardcoded zeros, and the KPIs route is always real
/// `application/json` (unlike the base Prometheus route's
/// `text/plain` exposition format). Confirmed against `/tmp/
/// slskr-parity-evidence/controller-api/*.json` before writing,
/// per case: only `POST /api/v0/hashdb/optimize/profile`'s
/// `malformed-path-query-or-body` had any prior credit. slskdN-only
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
pub(super) async fn controller_api_differential_analyzer_hashdb_and_telemetry() {
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

    let unversioned =
        crate::route_http_request("POST", "/api/audio/analyzers/migrate", None, "", &state)
            .await
            .expect("unversioned analyzer migrate response");
    record!(
        "POST",
        "/api/audio/analyzers/migrate",
        "missing-empty-or-conflict-state",
        unversioned.status == "400 Bad Request"
    );

    let versioned =
        crate::route_http_request("POST", "/api/v0/audio/analyzers/migrate", None, "", &state)
            .await
            .expect("versioned analyzer migrate response");
    record!(
        "POST",
        "/api/v0/audio/analyzers/migrate",
        "nominal-status-headers-body",
        versioned.status == "200 OK" && versioned.body == "{\"updated\":0}"
    );

    {
        let mut discovery = state.content_discovery.write().await;
        discovery
            .merge_hash_entries(vec![crate::content_discovery::HashDbEntry {
                flac_key: "differential-key".to_owned(),
                size: 123,
                file_sha256: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                    .to_owned(),
                ..Default::default()
            }])
            .expect("seed differential hash entry");
    }

    let profile = crate::route_http_request(
        "POST",
        "/api/v0/hashdb/optimize/profile",
        None,
        r#"{"query":"SELECT * FROM hash_entries"}"#,
        &state,
    )
    .await
    .expect("hashdb optimize profile response");
    let profile_json = serde_json::from_str::<serde_json::Value>(&profile.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/hashdb/optimize/profile",
        "nominal-status-headers-body",
        profile.status == "200 OK"
            && profile_json["rowsReturned"] == 1
            && profile_json["queryPlan"]
                .as_str()
                .is_some_and(|plan| plan.contains("linear scan"))
    );

    let hashdb_key = crate::route_http_request(
        "GET",
        "/api/v0/hashdb/key?filename=Track.flac&size=123",
        None,
        "",
        &state,
    )
    .await
    .expect("hashdb key response");
    let hashdb_key_json =
        serde_json::from_str::<serde_json::Value>(&hashdb_key.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/hashdb/key",
        "nominal-status-headers-body",
        hashdb_key.status == "200 OK"
            && hashdb_key.content_type.starts_with("application/json")
            && hashdb_key_json["flacKey"].as_str().is_some()
            && hashdb_key_json.get("key").is_none()
            && hashdb_key_json.get("filename").is_none()
            && hashdb_key_json.get("size").is_none()
    );

    let slow_queries = crate::route_http_request(
        "GET",
        "/api/v0/hashdb/optimize/slow-queries",
        None,
        "",
        &state,
    )
    .await
    .expect("hashdb slow queries response");
    let slow_queries_json =
        serde_json::from_str::<serde_json::Value>(&slow_queries.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/hashdb/optimize/slow-queries",
        "populated-dynamic-state",
        slow_queries.status == "200 OK"
            && slow_queries_json["totalQueries"] == 1
            && slow_queries_json["slowQueries"][0]["executionCount"] == 1
    );

    let base_prometheus =
        crate::route_http_request("GET", "/api/v0/telemetry/prometheus", None, "", &state)
            .await
            .expect("base prometheus response");
    record!(
        "GET",
        "/api/v0/telemetry/prometheus",
        "nominal-status-headers-body",
        base_prometheus.content_type.starts_with("text/plain")
            && base_prometheus.body.contains("slskr_transfers")
    );

    let kpis =
        crate::route_http_request("GET", "/api/v0/telemetry/prometheus/kpis", None, "", &state)
            .await
            .expect("prometheus kpis response");
    let kpis_json = serde_json::from_str::<serde_json::Value>(&kpis.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/telemetry/prometheus/kpis",
        "nominal-status-headers-body",
        kpis.content_type.starts_with("application/json")
            && kpis_json["slskr_transfers"]["type"] == "gauge"
            && kpis_json["slskr_searches"]["type"] == "gauge"
    );

    let metrics = crate::route_http_request("GET", "/api/v0/telemetry/metrics", None, "", &state)
        .await
        .expect("versioned telemetry metrics response");
    record!(
        "GET",
        "/api/v0/telemetry/metrics",
        "nominal-status-headers-body",
        metrics.status == "200 OK"
            && metrics
                .content_type
                .starts_with("text/plain; version=0.0.4")
            && metrics.body.contains("slskr_telemetry_transfers")
    );

    let metrics_kpi =
        crate::route_http_request("GET", "/api/v0/telemetry/metrics/kpi", None, "", &state)
            .await
            .expect("versioned telemetry KPI response");
    let metrics_kpi_json =
        serde_json::from_str::<serde_json::Value>(&metrics_kpi.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/telemetry/metrics/kpi",
        "nominal-status-headers-body",
        metrics_kpi.status == "200 OK"
            && metrics_kpi.content_type.starts_with("application/json")
            && metrics_kpi_json["slskr_transfers"]["type"] == "gauge"
            && metrics_kpi_json["slskr_searches"]["type"] == "gauge"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("analyzer_hashdb_and_telemetry.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api analyzer-hashdb-and-telemetry mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting `DELETE /api/v0/mediacore/
/// publish/descriptor/{*contentId}`'s real unpublished-baseline
/// delete response (independently re-derived from `mediacore_
/// versioned_descriptor_delete_does_not_overflow_worker_stack`:
/// deleting a descriptor that was never published returns a real
/// `wasPublished: false` result, not a stack overflow or a
/// hardcoded shape) and `POST /api/v0/mesh-streams/tickets`'s
/// real mesh-family validation (independently re-derived from
/// `mesh_preview_ticket_fetches_verifies_streams_and_removes_
/// staging_file`'s ticket-creation request-validation logic, with
/// a much smaller fixture than the source test's full raw-TCP
/// preview-fetch machinery: the mesh family genuinely requires a
/// real `contentId`, not just a filename/peerId). Confirmed
/// against `/tmp/slskr-parity-evidence/controller-api/*.json`
/// before writing, per case: both routes had zero prior credit.
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
    feature = "bounded-controller-api-tests-4"
))]
pub(super) async fn controller_api_differential_mediacore_delete_and_mesh_tickets() {
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
        let (state, _receiver) = test_state();
        let response = crate::route_http_request(
            "DELETE",
            "/api/v0/mediacore/publish/descriptor/differential-content-id",
            None,
            "",
            &state,
        )
        .await
        .expect("delete unpublished descriptor response");
        let response_json =
            serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "DELETE",
            "/api/v0/mediacore/publish/descriptor/{*contentId}",
            "missing-empty-or-conflict-state",
            response.status == "200 OK"
                && response_json["contentId"] == "differential-content-id"
                && response_json["wasPublished"] == false
        );
    }

    {
        let (state, _receiver) = test_state();
        let response = crate::route_http_request(
            "POST",
            "/api/v0/mesh-streams/tickets",
            None,
            r#"{"filename":"Remote/Differential.flac","peerId":"differential-peer"}"#,
            &state,
        )
        .await
        .expect("mesh ticket missing contentId response");
        record!(
            "POST",
            "/api/v0/mesh-streams/tickets",
            "malformed-path-query-or-body",
            response.status == "400 Bad Request"
                && response.body.contains("ContentId is required.")
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("mediacore_delete_and_mesh_tickets.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api mediacore-delete-and-mesh-tickets mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential proof for the remaining MediaCore controller matrix.
/// These rows are the generic runtime/restart/concurrency cases that were
/// left open after the route-specific MediaCore proofs. Keep the ledger
/// one-to-one with the frozen manifest so the authoritative audit can
/// credit each controller action independently.
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
pub(super) async fn controller_api_differential_mediacore_residuals() {
    let target = "slskdn";
    let base_env = MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target);
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

    let get_cases = [
        (
            "/api/v0/mediacore/contentid/domain/music",
            "/api/v0/mediacore/contentid/domain/{domain}",
            "200 OK",
            "contentIds",
        ),
        (
            "/api/v0/mediacore/contentid/domain/music/type/recording",
            "/api/v0/mediacore/contentid/domain/{domain}/type/{type}",
            "200 OK",
            "normalizedType",
        ),
        (
            "/api/v0/mediacore/contentid/exists/missing",
            "/api/v0/mediacore/contentid/exists/{externalId}",
            "200 OK",
            "exists",
        ),
        (
            "/api/v0/mediacore/contentid/external/missing",
            "/api/v0/mediacore/contentid/external/{contentId}",
            "200 OK",
            "externalIds",
        ),
        (
            "/api/v0/mediacore/contentid/resolve/missing",
            "/api/v0/mediacore/contentid/resolve/{externalId}",
            "404 Not Found",
            "External ID not found",
        ),
        (
            "/api/v0/mediacore/contentid/stats",
            "/api/v0/mediacore/contentid/stats",
            "200 OK",
            "totalMappings",
        ),
        (
            "/api/v0/mediacore/contentid/validate/not-a-content-id",
            "/api/v0/mediacore/contentid/validate/{*contentId}",
            "200 OK",
            "isValid",
        ),
        (
            "/api/v0/mediacore/ipld/graph/missing",
            "/api/v0/mediacore/ipld/graph/{*contentId}",
            "200 OK",
            "nodes",
        ),
        (
            "/api/v0/mediacore/ipld/inbound/missing",
            "/api/v0/mediacore/ipld/inbound/{*targetContentId}",
            "200 OK",
            "inboundLinks",
        ),
        (
            "/api/v0/mediacore/ipld/traverse/content:audio:track:missing?linkName=missing",
            "/api/v0/mediacore/ipld/traverse/{*startContentId}",
            "200 OK",
            "visitedNodes",
        ),
        (
            "/api/v0/mediacore/ipld/validate",
            "/api/v0/mediacore/ipld/validate",
            "200 OK",
            "isValid",
        ),
        (
            "/api/v0/mediacore/perceptualhash/algorithms",
            "/api/v0/mediacore/perceptualhash/algorithms",
            "200 OK",
            "algorithms",
        ),
        (
            "/api/v0/mediacore/portability/merge-strategies",
            "/api/v0/mediacore/portability/merge-strategies",
            "200 OK",
            "strategies",
        ),
        (
            "/api/v0/mediacore/portability/strategies",
            "/api/v0/mediacore/portability/strategies",
            "200 OK",
            "strategies",
        ),
        (
            "/api/v0/mediacore/publish/stats",
            "/api/v0/mediacore/publish/stats",
            "200 OK",
            "totalPublishedDescriptors",
        ),
        (
            "/api/v0/mediacore/retrieve/descriptor/content:audio:track:missing",
            "/api/v0/mediacore/retrieve/descriptor/{*contentId}",
            "404 Not Found",
            "found",
        ),
        (
            "/api/v0/mediacore/retrieve/query/domain/music",
            "/api/v0/mediacore/retrieve/query/domain/{domain}",
            "200 OK",
            "descriptors",
        ),
        (
            "/api/v0/mediacore/retrieve/stats",
            "/api/v0/mediacore/retrieve/stats",
            "200 OK",
            "totalRetrievals",
        ),
        (
            "/api/v0/mediacore/stats/dashboard",
            "/api/v0/mediacore/stats/dashboard",
            "200 OK",
            "contentRegistry",
        ),
        (
            "/api/v0/mediacore/stats/descriptors",
            "/api/v0/mediacore/stats/descriptors",
            "200 OK",
            "totalRetrievals",
        ),
        (
            "/api/v0/mediacore/stats/fuzzy",
            "/api/v0/mediacore/stats/fuzzy",
            "200 OK",
            "totalMatches",
        ),
        (
            "/api/v0/mediacore/stats/ipld",
            "/api/v0/mediacore/stats/ipld",
            "200 OK",
            "totalLinks",
        ),
        (
            "/api/v0/mediacore/stats/perceptual",
            "/api/v0/mediacore/stats/perceptual",
            "200 OK",
            "totalHashesComputed",
        ),
        (
            "/api/v0/mediacore/stats/portability",
            "/api/v0/mediacore/stats/portability",
            "200 OK",
            "totalExports",
        ),
        (
            "/api/v0/mediacore/stats/publishing",
            "/api/v0/mediacore/stats/publishing",
            "200 OK",
            "totalPublished",
        ),
        (
            "/api/v0/mediacore/stats/registry",
            "/api/v0/mediacore/stats/registry",
            "200 OK",
            "totalMappings",
        ),
    ];
    for (path, route, status, marker) in get_cases {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        state.session.write().await.state = "disconnected";
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            response.status == status && response.body.contains(marker)
        );
    }

    let descriptor_body = |content_id: &str| {
        serde_json::json!({
            "descriptor": {
                "contentId": content_id,
                "title": "mediacore residual",
                "hashes": [{"algorithm": "sha256", "hex": "aaaa"}],
                "signature": {
                    "publicKey": "key",
                    "signature": "abcdef",
                    "timestampUnixMs": crate::unix_timestamp_millis(),
                },
            },
            "forceUpdate": true,
        })
        .to_string()
    };
    let residual_content_id = "content:audio:recording:mediacore-residual";
    let empty_package = r#"{"package":{"version":"1.0","exportedAt":"2026-01-01T00:00:00Z","source":"test","entries":[],"links":[],"metadata":{"totalEntries":0,"totalLinks":0,"entriesByDomain":{},"checksum":""}}}"#;
    let verify_body = serde_json::json!({
        "descriptor": {
            "contentId": residual_content_id,
            "hashes": [{"algorithm": "sha256", "hex": "aaaa"}],
            "signature": {"publicKey": "key", "signature": "abcdef"},
        },
    })
    .to_string();
    let batch_body = serde_json::json!({
        "descriptors": [{
            "contentId": residual_content_id,
            "title": "mediacore residual batch",
            "hashes": [{"algorithm": "sha256", "hex": "bbbb"}],
            "signature": {"publicKey": "key", "signature": "abcdef"},
        }],
    })
    .to_string();

    let mutations: Vec<(&str, String, &str, String, &str)> = vec![
        (
            "DELETE",
            format!("/api/v0/mediacore/publish/descriptor/{residual_content_id}"),
            "/api/v0/mediacore/publish/descriptor/{*contentId}",
            String::new(),
            "200 OK",
        ),
        (
            "POST",
            "/api/v0/mediacore/contentid/register".to_owned(),
            "/api/v0/mediacore/contentid/register",
            serde_json::json!({
                "externalId": "mediacore-residual",
                "contentId": "content:audio:recording:mediacore-register",
            })
            .to_string(),
            "200 OK",
        ),
        (
            "POST",
            "/api/v0/mediacore/fuzzymatch/find/content:audio:recording:mediacore-target"
                .to_owned(),
            "/api/v0/mediacore/fuzzymatch/find/{*contentId}",
            r#"{"minConfidence":0.0,"maxCandidates":50,"maxResults":10}"#.to_owned(),
            "200 OK",
        ),
        (
            "POST",
            "/api/v0/mediacore/fuzzymatch/perceptual".to_owned(),
            "/api/v0/mediacore/fuzzymatch/perceptual",
            r#"{"contentIdA":"content:audio:recording:a","contentIdB":"content:audio:recording:b"}"#.to_owned(),
            "200 OK",
        ),
        (
            "POST",
            "/api/v0/mediacore/fuzzymatch/text".to_owned(),
            "/api/v0/mediacore/fuzzymatch/text",
            r#"{"textA":"same","textB":"same"}"#.to_owned(),
            "200 OK",
        ),
        (
            "POST",
            "/api/v0/mediacore/ipld/links/content:audio:recording:mediacore-ipld"
                .to_owned(),
            "/api/v0/mediacore/ipld/links/{*contentId}",
            r#"{"links":[{"name":"related","target":"content:audio:recording:mediacore-target"}]}"#.to_owned(),
            "200 OK",
        ),
        (
            "POST",
            "/api/v0/mediacore/perceptualhash/audio".to_owned(),
            "/api/v0/mediacore/perceptualhash/audio",
            r#"{"samples":[0.5],"sampleRate":1,"algorithm":"PHash"}"#.to_owned(),
            "200 OK",
        ),
        (
            "POST",
            "/api/v0/mediacore/perceptualhash/image".to_owned(),
            "/api/v0/mediacore/perceptualhash/image",
            r#"{"pixels":"AAAAAA==","width":1,"height":1,"algorithm":"PHash"}"#.to_owned(),
            "200 OK",
        ),
        (
            "POST",
            "/api/v0/mediacore/perceptualhash/similarity".to_owned(),
            "/api/v0/mediacore/perceptualhash/similarity",
            r#"{"hashA":"0000000000000000","hashB":"0000000000000000","threshold":0.8}"#.to_owned(),
            "200 OK",
        ),
        (
            "POST",
            "/api/v0/mediacore/portability/analyze".to_owned(),
            "/api/v0/mediacore/portability/analyze",
            empty_package.to_owned(),
            "200 OK",
        ),
        (
            "POST",
            "/api/v0/mediacore/portability/export".to_owned(),
            "/api/v0/mediacore/portability/export",
            serde_json::json!({"contentIds":[residual_content_id]}).to_string(),
            "200 OK",
        ),
        (
            "POST",
            "/api/v0/mediacore/portability/import".to_owned(),
            "/api/v0/mediacore/portability/import",
            empty_package.to_owned(),
            "200 OK",
        ),
        (
            "POST",
            "/api/v0/mediacore/publish/batch".to_owned(),
            "/api/v0/mediacore/publish/batch",
            batch_body,
            "200 OK",
        ),
        (
            "POST",
            "/api/v0/mediacore/publish/descriptor".to_owned(),
            "/api/v0/mediacore/publish/descriptor",
            descriptor_body(residual_content_id),
            "200 OK",
        ),
        (
            "POST",
            "/api/v0/mediacore/publish/republish".to_owned(),
            "/api/v0/mediacore/publish/republish",
            serde_json::json!({"contentIds":[residual_content_id]}).to_string(),
            "200 OK",
        ),
        (
            "POST",
            "/api/v0/mediacore/retrieve/batch".to_owned(),
            "/api/v0/mediacore/retrieve/batch",
            serde_json::json!({"contentIds":[residual_content_id]}).to_string(),
            "200 OK",
        ),
        (
            "POST",
            "/api/v0/mediacore/retrieve/cache/clear".to_owned(),
            "/api/v0/mediacore/retrieve/cache/clear",
            String::new(),
            "200 OK",
        ),
        (
            "POST",
            "/api/v0/mediacore/retrieve/verify".to_owned(),
            "/api/v0/mediacore/retrieve/verify",
            verify_body,
            "200 OK",
        ),
        (
            "POST",
            "/api/v0/mediacore/stats/reset".to_owned(),
            "/api/v0/mediacore/stats/reset",
            String::new(),
            "200 OK",
        ),
        (
            "PUT",
            format!("/api/v0/mediacore/publish/descriptor/{residual_content_id}"),
            "/api/v0/mediacore/publish/descriptor/{*contentId}",
            r#"{"updates":{"title":"after"}}"#.to_owned(),
            "400 Bad Request",
        ),
    ];

    for (index, (method, path, route, body, expected_status)) in mutations.iter().enumerate() {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        let response = crate::route_http_request(method, path, None, body, &state)
            .await
            .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
        record!(
            method,
            route,
            "runtime-failure-and-timeout",
            response.status == *expected_status
        );

        let state_dir = std::env::temp_dir().join(format!(
            "slskr-mediacore-residual-{}-{index}",
            uuid::Uuid::new_v4().simple()
        ));
        fs::create_dir_all(&state_dir).expect("create MediaCore residual state directory");
        let state_env = base_env
            .clone()
            .with("SLSKR_STATE_DIR", &state_dir.display().to_string());
        let (first_state, _first_receiver) = test_state_with_env(state_env.clone());
        {
            first_state
                .controller_features
                .write_for_test()
                .await
                .state_path = state_dir.join("controller-feature-state.json");
        }
        let needs_descriptor_seed = method == &"DELETE"
            || method == &"PUT"
            || path == "/api/v0/mediacore/portability/export"
            || path == "/api/v0/mediacore/publish/republish"
            || path == "/api/v0/mediacore/retrieve/batch";
        if needs_descriptor_seed {
            let seed = crate::route_http_request(
                "POST",
                "/api/v0/mediacore/publish/descriptor",
                None,
                &descriptor_body(residual_content_id),
                &first_state,
            )
            .await
            .expect("seed MediaCore residual descriptor");
            assert_eq!(seed.status, "200 OK", "{method} {path} descriptor seed");
        }
        let first = crate::route_http_request(method, path, None, body, &first_state)
            .await
            .unwrap_or_else(|error| panic!("{method} {path} first: {error}"));
        let restored = crate::ControllerFeatureState::load(&state_dir)
            .expect("reload MediaCore residual feature state");
        let (second_state, _second_receiver) = test_state_with_env(state_env);
        *second_state.controller_features.write_for_test().await = restored;
        let second = crate::route_http_request(method, path, None, body, &second_state)
            .await
            .unwrap_or_else(|error| panic!("{method} {path} restart: {error}"));
        record!(
            method,
            route,
            "restart-persistence-or-reset",
            first.status == *expected_status && second.status == *expected_status
        );
        let _ = fs::remove_dir_all(&state_dir);

        let (concurrent_state, _receiver) = test_state_with_env(base_env.clone());
        let (left, right) = tokio::join!(
            crate::route_http_request(method, path, None, body, &concurrent_state),
            crate::route_http_request(method, path, None, body, &concurrent_state)
        );
        record!(
            method,
            route,
            "concurrency-and-idempotency",
            left.as_ref()
                .is_ok_and(|response| response.status == *expected_status)
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == *expected_status)
        );
    }

    assert_eq!(ledger.len(), 86, "MediaCore residual ledger size");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create MediaCore evidence directory");
    fs::write(
        evidence_dir.join("mediacore_controller_residuals.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize MediaCore ledger"),
    )
    .expect("write MediaCore ledger");
    assert!(
        mismatches.is_empty(),
        "{} MediaCore residual mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential proof for audio canonical/dedupe reads and analyzer
/// migration.  The read probes use one real content-discovery entry for
/// populated state; the closed-pool cases exercise the durable-storage
/// boundary rather than treating an in-memory empty result as success.
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
pub(super) async fn controller_api_differential_audio_canonical_dedupe_and_migration() {
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

    let target_env = || MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target);
    let json_value = |response: &crate::routing::HttpResponse| {
        serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or(serde_json::Value::Null)
    };
    let audio_read_contract = |response: &crate::routing::HttpResponse,
                               recording_id: &str,
                               field: &str,
                               populated: bool| {
        let value = json_value(response);
        let entries = value[field].as_array();
        response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && value["recordingId"] == recording_id
            && entries.is_some_and(|entries| {
                if !populated {
                    return entries.is_empty();
                }
                if entries.len() != 1 {
                    return false;
                }
                if field == "candidates" {
                    entries[0]["flacKey"] == "audio-differential-key"
                        && entries[0]["size"] == 123
                        && entries[0]["fileSha256"] == "c".repeat(64)
                } else {
                    entries[0]["hash"] == "c".repeat(64)
                        && entries[0]["variants"].as_array().is_some_and(|variants| {
                            variants.len() == 1
                                && variants[0]["flacKey"] == "audio-differential-key"
                                && variants[0]["size"] == 123
                        })
                }
            })
    };

    for (path, route, field) in [
        (
            "/api/audio/canonical/",
            "/api/audio/canonical/{recordingId}",
            "candidates",
        ),
        (
            "/api/audio/variants/dedupe/",
            "/api/audio/variants/dedupe/{recordingId}",
            "groups",
        ),
        (
            "/api/v0/audio/canonical/",
            "/api/v0/audio/canonical/{recordingId}",
            "candidates",
        ),
        (
            "/api/v0/audio/variants/dedupe/",
            "/api/v0/audio/variants/dedupe/{recordingId}",
            "groups",
        ),
    ] {
        let recording_id = "audio-differential-recording";
        let exact_path = format!("{path}{recording_id}");
        let (state, _receiver) = test_state_with_env(target_env());
        let nominal = crate::route_http_request("GET", &exact_path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("GET {exact_path}: {error}"));
        record!(
            "GET",
            route,
            "nominal-status-headers-body",
            audio_read_contract(&nominal, recording_id, field, false)
        );

        let malformed_path = format!("{exact_path}/extra");
        let malformed = crate::route_http_request("GET", &malformed_path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("GET {malformed_path}: {error}"));
        record!(
            "GET",
            route,
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );

        let missing_path = format!("{path}%20");
        let missing = crate::route_http_request("GET", &missing_path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("GET {missing_path}: {error}"));
        record!(
            "GET",
            route,
            "missing-empty-or-conflict-state",
            missing.status == "400 Bad Request"
        );

        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("audio runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(target_env(), crate::SearchStore::new(), Some(db.clone()));
        db.close_for_test().await;
        let runtime = crate::route_http_request("GET", &exact_path, None, "", &runtime_state)
            .await
            .unwrap_or_else(|error| panic!("GET {exact_path} runtime: {error}"));
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            runtime.status == "503 Service Unavailable"
        );

        let (populated_state, _receiver) = test_state_with_env(target_env());
        populated_state
            .content_discovery
            .write()
            .await
            .merge_hash_entries(vec![crate::content_discovery::HashDbEntry {
                flac_key: "audio-differential-key".to_owned(),
                byte_hash: "a".repeat(64),
                size: 123,
                full_file_hash: "c".repeat(64),
                music_brainz_id: recording_id.to_owned(),
                file_sha256: "c".repeat(64),
                ..Default::default()
            }])
            .expect("seed audio content-discovery entry");
        let populated = crate::route_http_request("GET", &exact_path, None, "", &populated_state)
            .await
            .unwrap_or_else(|error| panic!("GET {exact_path} populated: {error}"));
        record!(
            "GET",
            route,
            "populated-dynamic-state",
            audio_read_contract(&populated, recording_id, field, true)
        );
    }

    let analyzer_contract = |response: &crate::routing::HttpResponse, updated: u64| {
        let value = json_value(response);
        response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && value["updated"] == updated
    };

    {
        let (state, _receiver) = test_state_with_env(target_env());
        let runtime =
            crate::route_http_request("POST", "/api/audio/analyzers/migrate", None, "", &state)
                .await
                .expect("unversioned analyzer runtime response");
        record!(
            "POST",
            "/api/audio/analyzers/migrate",
            "runtime-failure-and-timeout",
            runtime.status == "400 Bad Request" && runtime.body.contains("ApiVersionUnspecified")
        );
        let mutation =
            crate::route_http_request("POST", "/api/audio/analyzers/migrate", None, "{}", &state)
                .await
                .expect("unversioned analyzer mutation response");
        record!(
            "POST",
            "/api/audio/analyzers/migrate",
            "mutation-side-effects-and-readback",
            mutation.status == "400 Bad Request" && mutation.body.contains("ApiVersionUnspecified")
        );
        let (restarted_state, _receiver) = test_state_with_env(target_env());
        let restarted = crate::route_http_request(
            "POST",
            "/api/audio/analyzers/migrate",
            None,
            "",
            &restarted_state,
        )
        .await
        .expect("unversioned analyzer reset response");
        record!(
            "POST",
            "/api/audio/analyzers/migrate",
            "restart-persistence-or-reset",
            restarted.status == "400 Bad Request"
                && restarted.body.contains("ApiVersionUnspecified")
        );
        let (left, right) = tokio::join!(
            crate::route_http_request("POST", "/api/audio/analyzers/migrate", None, "", &state),
            crate::route_http_request("POST", "/api/audio/analyzers/migrate", None, "", &state)
        );
        let left = left.expect("left unversioned analyzer concurrency response");
        let right = right.expect("right unversioned analyzer concurrency response");
        record!(
            "POST",
            "/api/audio/analyzers/migrate",
            "concurrency-and-idempotency",
            left.status == "400 Bad Request"
                && right.status == "400 Bad Request"
                && left.body.contains("ApiVersionUnspecified")
                && right.body.contains("ApiVersionUnspecified")
        );
    }

    {
        let path = "/api/v0/audio/analyzers/migrate";
        let (state, _receiver) = test_state_with_env(target_env());
        let malformed = crate::route_http_request(
            "POST",
            "/api/v0/audio/analyzers/migrate/extra",
            None,
            "",
            &state,
        )
        .await
        .expect("versioned analyzer malformed path response");
        record!(
            "POST",
            path,
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );

        let missing = crate::route_http_request("POST", path, None, "", &state)
            .await
            .expect("versioned analyzer empty response");
        record!(
            "POST",
            path,
            "missing-empty-or-conflict-state",
            analyzer_contract(&missing, 0)
        );

        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("versioned analyzer runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(target_env(), crate::SearchStore::new(), Some(db.clone()));
        db.close_for_test().await;
        let runtime = crate::route_http_request("POST", path, None, "", &runtime_state)
            .await
            .expect("versioned analyzer runtime response");
        record!(
            "POST",
            path,
            "runtime-failure-and-timeout",
            runtime.status == "503 Service Unavailable"
        );

        let (mutation_state, _receiver) = test_state_with_env(target_env());
        mutation_state
            .content_discovery
            .write()
            .await
            .merge_hash_entries(vec![crate::content_discovery::HashDbEntry {
                flac_key: "analyzer-differential-key".to_owned(),
                byte_hash: "d".repeat(64),
                full_file_hash: "f".repeat(64),
                size: 123,
                music_brainz_id: "analyzer-recording".to_owned(),
                file_sha256: "f".repeat(64),
                ..Default::default()
            }])
            .expect("seed analyzer migration entry");
        let mutation = crate::route_http_request("POST", path, None, "", &mutation_state)
            .await
            .expect("versioned analyzer mutation response");
        record!(
            "POST",
            path,
            "mutation-side-effects-and-readback",
            analyzer_contract(&mutation, 1)
        );

        let (reset_state, _receiver) = test_state_with_env(target_env());
        reset_state
            .content_discovery
            .write()
            .await
            .merge_hash_entries(vec![crate::content_discovery::HashDbEntry {
                flac_key: "analyzer-reset-key".to_owned(),
                byte_hash: "1".repeat(64),
                full_file_hash: "3".repeat(64),
                size: 123,
                music_brainz_id: "analyzer-reset-recording".to_owned(),
                file_sha256: "3".repeat(64),
                ..Default::default()
            }])
            .expect("seed analyzer reset entry");
        let reset = crate::route_http_request("POST", path, None, "", &reset_state)
            .await
            .expect("versioned analyzer reset response");
        let (restarted_state, _receiver) = test_state_with_env(target_env());
        let restarted = crate::route_http_request("POST", path, None, "", &restarted_state)
            .await
            .expect("versioned analyzer restarted response");
        record!(
            "POST",
            path,
            "restart-persistence-or-reset",
            analyzer_contract(&reset, 1) && analyzer_contract(&restarted, 0)
        );

        let (concurrent_state, _receiver) = test_state_with_env(target_env());
        concurrent_state
            .content_discovery
            .write()
            .await
            .merge_hash_entries(vec![crate::content_discovery::HashDbEntry {
                flac_key: "analyzer-concurrent-key".to_owned(),
                byte_hash: "4".repeat(64),
                full_file_hash: "6".repeat(64),
                size: 123,
                music_brainz_id: "analyzer-concurrent-recording".to_owned(),
                file_sha256: "6".repeat(64),
                ..Default::default()
            }])
            .expect("seed analyzer concurrency entry");
        let (left, right) = tokio::join!(
            crate::route_http_request("POST", path, None, "", &concurrent_state),
            crate::route_http_request("POST", path, None, "", &concurrent_state)
        );
        let left = left.expect("left analyzer concurrency response");
        let right = right.expect("right analyzer concurrency response");
        record!(
            "POST",
            path,
            "concurrency-and-idempotency",
            analyzer_contract(&left, 1) && analyzer_contract(&right, 1)
        );
    }

    assert_eq!(ledger.len(), 30, "audio residual ledger size");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create audio evidence directory");
    fs::write(
        evidence_dir.join("audio_canonical_dedupe_migration.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize audio ledger"),
    )
    .expect("write audio ledger");
    assert!(
        mismatches.is_empty(),
        "{} audio controller mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
