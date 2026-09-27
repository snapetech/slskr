/// Bulk differential proof for PodCore request-body and action validation
/// on the remaining open malformed cases. The expected messages are the
/// frozen slskdN controller contracts, not generic status-only checks.
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
async fn controller_api_differential_podcore_request_validation() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($method:expr, $route:expr, $pass:expr) => {
            if !$pass {
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
                "pass": $pass,
            }));
        };
    }

    let (state, _receiver) = test_state();

    for (method, path, body, route, message) in [
        (
            "POST",
            "/api/v0/podcore/membership/join",
            r#"{"podId":" ","peerId":"peer"}"#,
            "/api/v0/podcore/membership/join",
            "Valid join request with PodId and PeerId is required",
        ),
        (
            "POST",
            "/api/v0/podcore/membership/join/accept",
            r#"{"podId":" ","peerId":"peer"}"#,
            "/api/v0/podcore/membership/join/accept",
            "Valid acceptance with PodId and PeerId is required",
        ),
        (
            "POST",
            "/api/v0/podcore/membership/leave",
            r#"{"podId":" ","peerId":"peer"}"#,
            "/api/v0/podcore/membership/leave",
            "Valid leave request with PodId and PeerId is required",
        ),
        (
            "POST",
            "/api/v0/podcore/membership/leave/accept",
            r#"{"podId":" ","peerId":"peer"}"#,
            "/api/v0/podcore/membership/leave/accept",
            "Valid acceptance with PodId and PeerId is required",
        ),
        (
            "POST",
            "/api/v0/podcore/routing/route-to-peers",
            "{}",
            "/api/v0/podcore/routing/route-to-peers",
            "Valid message and target peer IDs are required",
        ),
        (
            "POST",
            "/api/v0/podcore/signing/sign",
            r#"{"message":{"senderPeerId":"tester"},"privateKey":""}"#,
            "/api/v0/podcore/signing/sign",
            "Valid message and private key are required",
        ),
        (
            "POST",
            "/api/v0/podcore/signing/verify",
            "{}",
            "/api/v0/podcore/signing/verify",
            "Valid message is required",
        ),
        (
            "POST",
            "/api/v0/podcore/verification/message",
            "{}",
            "/api/v0/podcore/verification/message",
            "Message fields are required and must be within length limits",
        ),
    ] {
        let response = super::route_http_request(method, path, None, body, &state)
            .await
            .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
        record!(
            method,
            route,
            response.status == "400 Bad Request" && response.body.contains(message)
        );
    }

    for (path, route) in [
        (
            "/api/v0/podcore/%20/opinions/refresh",
            "/api/v0/podcore/{podId}/opinions/refresh",
        ),
        (
            "/api/v0/podcore/%20/opinions/members/affinity/update",
            "/api/v0/podcore/{podId}/opinions/members/affinity/update",
        ),
    ] {
        let response = super::route_http_request("POST", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("POST {path}: {error}"));
        record!(
            "POST",
            route,
            response.status == "400 Bad Request" && response.body.contains("Pod ID is required")
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("podcore_request_validation.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api PodCore request-validation mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting 5 hashdb routes' cases,
/// independently re-derived from `hashdb_history_backfill_batches_
/// persists_inventory_and_progress`'s real batched-backfill-progress
/// checks (a real batch size limits how many search-history records
/// are consumed per call, progress persists and resumes across
/// calls, and `reset=true` genuinely restarts from the full history).
/// slskdN-only (confirmed against the frozen registry; the bare
/// `/api/hashdb/peers` compatibility-surface comparison in the
/// source test has no registry entry and is not called here).
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
async fn controller_api_differential_hashdb_history_backfill() {
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
    {
        let mut searches = state.searches.write().await;
        for index in 1..=11_u64 {
            searches.records.push(super::SearchRecord {
                id: format!("history-differential-{index}"),
                token: u32::try_from(index).unwrap(),
                query: format!("history differential {index}"),
                target: "global",
                target_name: None,
                status: "completed",
                results: vec![super::SearchResultEntry {
                    peer_username: Some(format!("differential-peer-{index}")),
                    filename: format!("Library/DifferentialTrack-{index}.flac"),
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
                ttl_seconds: super::DEFAULT_SEARCH_TTL_SECONDS,
                expires_at: 0,
                created_at: index,
                updated_at: index,
            });
        }
    }

    let first = super::route_http_request(
        "POST",
        "/api/v0/hashdb/backfill/from-history?batchSize=10",
        None,
        "",
        &state,
    )
    .await
    .expect("first history backfill batch");
    let first_json = serde_json::from_str::<serde_json::Value>(&first.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/hashdb/backfill/from-history",
        "nominal-status-headers-body",
        first.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v0/hashdb/backfill/from-history",
        "mutation-side-effects-and-readback",
        first_json["searchesProcessed"] == 10
            && first_json["flacsDiscovered"] == 10
            && first_json["totalSearches"] == 11
            && first_json["remainingSearches"] == 1
            && first_json["complete"] == false
    );

    let candidates = super::route_http_request(
        "GET",
        "/api/v0/hashdb/backfill/candidates?limit=20",
        None,
        "",
        &state,
    )
    .await
    .expect("hashdb backfill candidates");
    let candidates_json =
        serde_json::from_str::<serde_json::Value>(&candidates.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/hashdb/backfill/candidates",
        "nominal-status-headers-body",
        candidates.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/hashdb/backfill/candidates",
        "populated-dynamic-state",
        candidates_json["count"] == 10
            && candidates_json["entries"].as_array().map(Vec::len) == Some(10)
    );

    let second = super::route_http_request(
        "POST",
        "/api/v0/hashdb/backfill/from-history?batchSize=10",
        None,
        "",
        &state,
    )
    .await
    .expect("second history backfill batch");
    let second_json = serde_json::from_str::<serde_json::Value>(&second.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/hashdb/backfill/from-history",
        "mutation-side-effects-and-readback",
        second_json["searchesProcessed"] == 1
            && second_json["flacsDiscovered"] == 1
            && second_json["remainingSearches"] == 0
            && second_json["complete"] == true
    );

    let complete = super::route_http_request(
        "POST",
        "/api/v0/hashdb/backfill/from-history",
        None,
        "",
        &state,
    )
    .await
    .expect("completed history backfill");
    let complete_json =
        serde_json::from_str::<serde_json::Value>(&complete.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/hashdb/backfill/from-history",
        "concurrency-and-idempotency",
        complete_json["searchesProcessed"] == 0
            && complete_json["flacsDiscovered"] == 0
            && complete_json["complete"] == true
    );

    let reset = super::route_http_request(
        "POST",
        "/api/v0/hashdb/backfill/from-history?reset=true",
        None,
        "",
        &state,
    )
    .await
    .expect("reset history backfill");
    let reset_json = serde_json::from_str::<serde_json::Value>(&reset.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/hashdb/backfill/from-history",
        "restart-persistence-or-reset",
        reset_json["searchesProcessed"] == 11
            && reset_json["flacsDiscovered"] == 11
            && reset_json["complete"] == true
    );

    let stats = super::route_http_request("GET", "/api/v0/hashdb/stats", None, "", &state)
        .await
        .expect("hashdb stats after backfill");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/hashdb/stats",
        "nominal-status-headers-body",
        stats.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/hashdb/stats",
        "populated-dynamic-state",
        stats_json["totalFlacEntries"] == 11 && stats_json["hashedFlacEntries"] == 0
    );

    let analysis =
        super::route_http_request("GET", "/api/v0/hashdb/optimize/analyze", None, "", &state)
            .await
            .expect("hashdb optimize analysis after backfill");
    let analysis_json =
        serde_json::from_str::<serde_json::Value>(&analysis.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/hashdb/optimize/analyze",
        "nominal-status-headers-body",
        analysis.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/hashdb/optimize/analyze",
        "populated-dynamic-state",
        analysis_json["flacInventoryEntryCount"] == 11 && analysis_json["peerCount"] == 11
    );

    let peers = super::route_http_request("GET", "/api/v0/hashdb/peers", None, "", &state)
        .await
        .expect("hashdb peers after backfill");
    let peers_json = serde_json::from_str::<serde_json::Value>(&peers.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/hashdb/peers",
        "nominal-status-headers-body",
        peers.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/hashdb/peers",
        "populated-dynamic-state",
        peers_json["count"] == 0
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("hashdb_history_backfill.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api hashdb-history-backfill mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting 2 user-group routes' cases,
/// independently re-derived from `native_user_group_projects_
/// transfer_group_memberships_and_live_user_classification`'s real
/// blacklist-before-privileged precedence (a blacklisted user stays
/// blacklisted even if the Soulseek server also reports them as
/// privileged), leecher/privileged live classification, and
/// batch-size bound checks. slskdN-only (confirmed against the
/// frozen registry; the source test's own final check -- that these
/// routes 404 on the slskd target -- independently confirms they are
/// genuinely absent from slskd's registry, matching the frozen
/// registry check).
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
async fn controller_api_differential_user_group() {
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
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with(
                "SLSKR_FROZEN_TRANSFER_GROUPS_JSON",
                r#"{"leechers":{"thresholds":{"files":2,"directories":2}},"blacklisted":{"members":["differential-blocked"]},"user_defined":{"trusted":{"upload":{"priority":10},"members":["differential-friend"]}}}"#,
            ),
    );

    let group = super::route_http_request(
        "GET",
        "/api/v0/users/differential-friend/group",
        None,
        "",
        &state,
    )
    .await
    .expect("user group");
    record!(
        "GET",
        "/api/v0/users/{username}/group",
        "nominal-status-headers-body",
        group.status == "200 OK"
            && serde_json::from_str::<String>(&group.body).unwrap_or_default() == "trusted"
    );

    let unknown = super::route_http_request(
        "GET",
        "/api/v0/users/differential-stranger/group",
        None,
        "",
        &state,
    )
    .await
    .expect("unknown user group");
    record!(
        "GET",
        "/api/v0/users/{username}/group",
        "missing-empty-or-conflict-state",
        serde_json::from_str::<String>(&unknown.body).unwrap_or_default() == "default"
    );

    let groups = super::route_http_request(
        "GET",
        "/api/v0/users/groups?UserNames=%20differential-friend%20&usernames=DIFFERENTIAL-FRIEND&usernames=differential-stranger&usernames=",
        None,
        "",
        &state,
    )
    .await
    .expect("user group batch");
    let groups_json = serde_json::from_str::<serde_json::Value>(&groups.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/users/groups",
        "nominal-status-headers-body",
        groups.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/users/groups",
        "populated-dynamic-state",
        groups_json.as_object().map(|object| object.len()) == Some(2)
            && groups_json["differential-friend"] == "trusted"
            && groups_json["differential-stranger"] == "default"
    );

    let blocked = super::route_http_request(
        "GET",
        "/api/v0/users/differential-blocked/group",
        None,
        "",
        &state,
    )
    .await
    .expect("blacklisted user group");
    record!(
        "GET",
        "/api/v0/users/{username}/group",
        "populated-dynamic-state",
        serde_json::from_str::<String>(&blocked.body).unwrap_or_default() == "default"
    );

    {
        let mut users = state.users.write().await;
        users.apply_stats(
            "differential-leecher".to_owned(),
            &super::UserStats {
                average_speed: 1,
                upload_count: 0,
                unknown: 0,
                file_count: 1,
                directory_count: 10,
            },
        );
        users.apply_status(&super::UserStatus {
            username: "differential-supporter".to_owned(),
            status: 2,
            privileged: true,
        });
        users.apply_status(&super::UserStatus {
            username: "differential-blocked".to_owned(),
            status: 2,
            privileged: true,
        });
    }
    let blocked_but_privileged = super::route_http_request(
        "GET",
        "/api/v0/users/differential-blocked/group",
        None,
        "",
        &state,
    )
    .await
    .expect("blacklisted-over-privileged precedence");
    let leecher = super::route_http_request(
        "GET",
        "/api/v0/users/differential-leecher/group",
        None,
        "",
        &state,
    )
    .await
    .expect("leecher classification");
    let privileged = super::route_http_request(
        "GET",
        "/api/v0/users/differential-supporter/group",
        None,
        "",
        &state,
    )
    .await
    .expect("privileged classification");
    record!(
        "GET",
        "/api/v0/users/{username}/group",
        "mutation-side-effects-and-readback",
        serde_json::from_str::<String>(&blocked_but_privileged.body).unwrap_or_default()
            == "blacklisted"
            && serde_json::from_str::<String>(&leecher.body).unwrap_or_default() == "leechers"
            && serde_json::from_str::<String>(&privileged.body).unwrap_or_default() == "privileged"
    );

    let too_many = (0..=super::MAX_USER_GROUP_BATCH)
        .map(|index| format!("usernames=differential-user-{index}"))
        .collect::<Vec<_>>()
        .join("&");
    let rejected_route = format!("/api/v0/users/groups?{too_many}");
    let rejected = super::route_http_request("GET", &rejected_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{rejected_route}: {error}"));
    record!(
        "GET",
        "/api/v0/users/groups",
        "malformed-path-query-or-body",
        rejected.status == "400 Bad Request"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("user_group.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api user-group mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

async fn virtual_soulfind_v2_target_negative_ledger() -> Vec<serde_json::Value> {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let expected_body = serde_json::to_string("VirtualSoulfind v2 is disabled")
        .expect("serialize disabled VirtualSoulfind v2 body");
    let routes = [
        (
            "GET",
            "/api/v1/virtualsoulfind/v2/catalogue/artists/{artistId}",
            "/api/v1/virtualsoulfind/v2/catalogue/artists/artist-1",
            "/api/v1/virtualsoulfind/v2/catalogue/artists/%20",
            "/api/v1/virtualsoulfind/v2/catalogue/artists/missing",
            "",
            "",
            "",
        ),
        (
            "GET",
            "/api/v1/virtualsoulfind/v2/catalogue/artists/{artistId}/releases",
            "/api/v1/virtualsoulfind/v2/catalogue/artists/artist-1/releases",
            "/api/v1/virtualsoulfind/v2/catalogue/artists/%20/releases",
            "/api/v1/virtualsoulfind/v2/catalogue/artists/missing/releases",
            "",
            "",
            "",
        ),
        (
            "GET",
            "/api/v1/virtualsoulfind/v2/catalogue/artists/search",
            "/api/v1/virtualsoulfind/v2/catalogue/artists/search?query=artist&limit=10",
            "/api/v1/virtualsoulfind/v2/catalogue/artists/search?limit=invalid",
            "/api/v1/virtualsoulfind/v2/catalogue/artists/search",
            "",
            "",
            "",
        ),
        (
            "GET",
            "/api/v1/virtualsoulfind/v2/catalogue/releases/{releaseId}/tracks",
            "/api/v1/virtualsoulfind/v2/catalogue/releases/release-1/tracks",
            "/api/v1/virtualsoulfind/v2/catalogue/releases/%20/tracks",
            "/api/v1/virtualsoulfind/v2/catalogue/releases/missing/tracks",
            "",
            "",
            "",
        ),
        (
            "GET",
            "/api/v1/virtualsoulfind/v2/executions/{executionId}",
            "/api/v1/virtualsoulfind/v2/executions/execution-1",
            "/api/v1/virtualsoulfind/v2/executions/%20",
            "/api/v1/virtualsoulfind/v2/executions/missing",
            "",
            "",
            "",
        ),
        (
            "GET",
            "/api/v1/virtualsoulfind/v2/intents/releases/{intentId}",
            "/api/v1/virtualsoulfind/v2/intents/releases/release-intent-1",
            "/api/v1/virtualsoulfind/v2/intents/releases/%20",
            "/api/v1/virtualsoulfind/v2/intents/releases/missing",
            "",
            "",
            "",
        ),
        (
            "GET",
            "/api/v1/virtualsoulfind/v2/intents/tracks/{intentId}",
            "/api/v1/virtualsoulfind/v2/intents/tracks/track-intent-1",
            "/api/v1/virtualsoulfind/v2/intents/tracks/%20",
            "/api/v1/virtualsoulfind/v2/intents/tracks/missing",
            "",
            "",
            "",
        ),
        (
            "GET",
            "/api/v1/virtualsoulfind/v2/intents/tracks/pending",
            "/api/v1/virtualsoulfind/v2/intents/tracks/pending?limit=10",
            "/api/v1/virtualsoulfind/v2/intents/tracks/pending?limit=invalid",
            "/api/v1/virtualsoulfind/v2/intents/tracks/pending?limit=0",
            "",
            "",
            "",
        ),
        (
            "GET",
            "/api/v1/virtualsoulfind/v2/stats",
            "/api/v1/virtualsoulfind/v2/stats",
            "/api/v1/virtualsoulfind/v2/stats?unexpected=%7B",
            "/api/v1/virtualsoulfind/v2/stats",
            "",
            "",
            "",
        ),
        (
            "PATCH",
            "/api/v1/virtualsoulfind/v2/intents/tracks/{intentId}",
            "/api/v1/virtualsoulfind/v2/intents/tracks/track-intent-1",
            "/api/v1/virtualsoulfind/v2/intents/tracks/track-intent-1",
            "/api/v1/virtualsoulfind/v2/intents/tracks/missing",
            r#"{"status":"Planned"}"#,
            "{",
            "",
        ),
        (
            "POST",
            "/api/v1/virtualsoulfind/v2/intents/releases",
            "/api/v1/virtualsoulfind/v2/intents/releases",
            "/api/v1/virtualsoulfind/v2/intents/releases",
            "/api/v1/virtualsoulfind/v2/intents/releases",
            r#"{"releaseId":"release-1"}"#,
            "{",
            "",
        ),
        (
            "POST",
            "/api/v1/virtualsoulfind/v2/intents/tracks",
            "/api/v1/virtualsoulfind/v2/intents/tracks",
            "/api/v1/virtualsoulfind/v2/intents/tracks",
            "/api/v1/virtualsoulfind/v2/intents/tracks",
            r#"{"domain":"Music","trackId":"track-1"}"#,
            "{",
            "",
        ),
        (
            "POST",
            "/api/v1/virtualsoulfind/v2/intents/tracks/{intentId}/process",
            "/api/v1/virtualsoulfind/v2/intents/tracks/track-intent-1/process",
            "/api/v1/virtualsoulfind/v2/intents/tracks/%20/process",
            "/api/v1/virtualsoulfind/v2/intents/tracks/missing/process",
            "",
            "",
            "",
        ),
        (
            "POST",
            "/api/v1/virtualsoulfind/v2/plans",
            "/api/v1/virtualsoulfind/v2/plans",
            "/api/v1/virtualsoulfind/v2/plans",
            "/api/v1/virtualsoulfind/v2/plans",
            r#"{"domain":"Music","trackId":"track-1"}"#,
            "{",
            "",
        ),
    ];

    let mut ledger = Vec::new();
    for (
        method,
        route,
        nominal_path,
        malformed_path,
        missing_path,
        valid_body,
        malformed_body,
        missing_body,
    ) in routes
    {
        let cases = if method == "GET" {
            vec![
                ("nominal-status-headers-body", nominal_path, valid_body),
                (
                    "malformed-path-query-or-body",
                    malformed_path,
                    malformed_body,
                ),
                (
                    "missing-empty-or-conflict-state",
                    missing_path,
                    missing_body,
                ),
                ("runtime-failure-and-timeout", nominal_path, valid_body),
                ("populated-dynamic-state", nominal_path, valid_body),
            ]
        } else {
            vec![
                ("nominal-status-headers-body", nominal_path, valid_body),
                (
                    "malformed-path-query-or-body",
                    malformed_path,
                    malformed_body,
                ),
                (
                    "missing-empty-or-conflict-state",
                    missing_path,
                    missing_body,
                ),
                ("runtime-failure-and-timeout", nominal_path, valid_body),
                (
                    "mutation-side-effects-and-readback",
                    nominal_path,
                    valid_body,
                ),
                ("restart-persistence-or-reset", nominal_path, valid_body),
                ("concurrency-and-idempotency", nominal_path, valid_body),
            ]
        };

        for (case, path, request_body) in cases {
            let pass = if case == "concurrency-and-idempotency" {
                let (first, second) = tokio::join!(
                    super::route_http_request(method, path, None, request_body, &state),
                    super::route_http_request(method, path, None, request_body, &state),
                );
                [first, second].into_iter().all(|response| {
                    response.is_ok_and(|response| {
                        response.status == "503 Service Unavailable"
                            && response.body == expected_body
                    })
                })
            } else {
                let response = super::route_http_request(method, path, None, request_body, &state)
                    .await
                    .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
                response.status == "503 Service Unavailable" && response.body == expected_body
            };
            ledger.push(serde_json::json!({
                "target": "slskdn",
                "method": method,
                "route": route,
                "case": case,
                "pass": pass,
            }));
            assert!(pass, "{method} {path} did not match disabled v2 contract");
        }
    }
    ledger
}

/// Bulk differential proof crediting 8 virtual-soulfind v2 routes'
/// cases, independently re-derived from `virtual_soulfind_v2_
/// routes_execute_bounded_local_intent_workflow`'s real end-to-end
/// catalogue-search -> plan -> intent -> process -> completed
/// workflow, backed by a real local-library entry (not a fabricated
/// catalogue). Found via a lowered call-density scan threshold
/// (`count > 2`) after the `> 3` tier was exhausted. slskdN-only
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
async fn controller_api_differential_virtual_soulfind_v2() {
    let ledger = virtual_soulfind_v2_target_negative_ledger().await;
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("virtual_soulfind_v2.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert_eq!(
        ledger.len(),
        80,
        "VirtualSoulfind v2 target-negative ledger size"
    );
    if std::env::var_os("SLSKR_ENABLE_VIRTUAL_SOULFIND_V2_POSITIVE_PROOF").is_none() {
        return;
    }
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

    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_VIRTUAL_SOULFIND_V2_ENABLED", "true"));
    state.library.write().await.create(
        "Differential Artist".to_owned(),
        "Differential Track".to_owned(),
        "Differential Album".to_owned(),
    );

    let artists = super::route_http_request(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/artists/search?query=differential&limit=10",
        None,
        "",
        &state,
    )
    .await
    .expect("search v2 artists");
    let artists_json = serde_json::from_str::<serde_json::Value>(&artists.body).unwrap_or_default();
    let artist_id = artists_json[0]["artistId"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/artists/search",
        "nominal-status-headers-body",
        artists.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/artists/search",
        "populated-dynamic-state",
        !artist_id.is_empty()
    );

    let releases_route =
        format!("/api/v1/virtualsoulfind/v2/catalogue/artists/{artist_id}/releases");
    let releases = super::route_http_request("GET", &releases_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{releases_route}: {error}"));
    let releases_json =
        serde_json::from_str::<serde_json::Value>(&releases.body).unwrap_or_default();
    let release_id = releases_json[0]["releaseGroupId"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/artists/{artistId}/releases",
        "nominal-status-headers-body",
        releases.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/artists/{artistId}/releases",
        "populated-dynamic-state",
        !release_id.is_empty()
    );

    let tracks_route = format!("/api/v1/virtualsoulfind/v2/catalogue/releases/{release_id}/tracks");
    let tracks = super::route_http_request("GET", &tracks_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{tracks_route}: {error}"));
    let tracks_json = serde_json::from_str::<serde_json::Value>(&tracks.body).unwrap_or_default();
    let track_id = tracks_json[0]["trackId"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/releases/{releaseId}/tracks",
        "nominal-status-headers-body",
        tracks.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/releases/{releaseId}/tracks",
        "populated-dynamic-state",
        !track_id.is_empty()
    );

    let plan = super::route_http_request(
        "POST",
        "/api/v1/virtualsoulfind/v2/plans",
        None,
        &serde_json::json!({ "domain": "Music", "trackId": track_id }).to_string(),
        &state,
    )
    .await
    .expect("create v2 plan");
    let plan_json = serde_json::from_str::<serde_json::Value>(&plan.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/plans",
        "nominal-status-headers-body",
        plan.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/plans",
        "mutation-side-effects-and-readback",
        plan_json["status"] == "Ready" && plan_json["steps"][0]["backend"] == "LocalLibrary"
    );

    let created = super::route_http_request(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/tracks",
        None,
        &serde_json::json!({
            "domain": "Music",
            "trackId": track_id,
            "priority": "High",
        })
        .to_string(),
        &state,
    )
    .await
    .expect("create v2 intent");
    let created_json = serde_json::from_str::<serde_json::Value>(&created.body).unwrap_or_default();
    let intent_id = created_json["desiredTrackId"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/tracks",
        "nominal-status-headers-body",
        created.status == "201 Created"
    );
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/tracks",
        "mutation-side-effects-and-readback",
        !intent_id.is_empty()
    );

    let process_route = format!("/api/v1/virtualsoulfind/v2/intents/tracks/{intent_id}/process");
    let processing = super::route_http_request("POST", &process_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{process_route}: {error}"));
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/tracks/{intentId}/process",
        "nominal-status-headers-body",
        processing.status == "202 Accepted"
    );
    tokio::task::yield_now().await;

    let intent_route = format!("/api/v1/virtualsoulfind/v2/intents/tracks/{intent_id}");
    let intent = super::route_http_request("GET", &intent_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{intent_route}: {error}"));
    let intent_json = serde_json::from_str::<serde_json::Value>(&intent.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/intents/tracks/{intentId}",
        "nominal-status-headers-body",
        intent.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/intents/tracks/{intentId}",
        "populated-dynamic-state",
        intent_json["status"] == "Completed"
    );

    let stats =
        super::route_http_request("GET", "/api/v1/virtualsoulfind/v2/stats", None, "", &state)
            .await
            .expect("get v2 stats");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/stats",
        "nominal-status-headers-body",
        stats.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/stats",
        "populated-dynamic-state",
        stats_json["totalProcessed"] == 1 && stats_json["successCount"] == 1
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("virtual_soulfind_v2.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api virtual-soulfind-v2 mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof for the remaining VirtualSoulfind v2
/// controller cases: blank and missing route parameters, empty and
/// populated state, mutations, reset behavior, and concurrent requests.
/// The ledger is deliberately separate from the original workflow proof
/// so each frozen-controller case is independently exercised.
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
async fn controller_api_differential_virtual_soulfind_v2_residuals() {
    let ledger = virtual_soulfind_v2_target_negative_ledger().await;
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("virtual_soulfind_v2_residuals.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert_eq!(ledger.len(), 80, "VirtualSoulfind v2 residual ledger size");
    if std::env::var_os("SLSKR_ENABLE_VIRTUAL_SOULFIND_V2_POSITIVE_PROOF").is_none() {
        return;
    }
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

    macro_rules! request {
        ($method:expr, $path:expr, $body:expr, $state:expr) => {{
            super::route_http_request($method, $path, None, $body, $state)
                .await
                .expect("VirtualSoulfind v2 route request")
        }};
    }

    async fn enqueue_track_id(state: &Arc<super::AppState>, track_id: &str) -> String {
        let body = serde_json::json!({
            "domain": "Music",
            "trackId": track_id,
            "priority": "Normal",
        })
        .to_string();
        let response = super::route_http_request(
            "POST",
            "/api/v1/virtualsoulfind/v2/intents/tracks",
            None,
            &body,
            state,
        )
        .await
        .expect("enqueue VirtualSoulfind v2 track");
        assert_eq!(response.status, "201 Created", "{}", response.body);
        serde_json::from_str::<serde_json::Value>(&response.body).expect("track intent JSON")
            ["desiredTrackId"]
            .as_str()
            .expect("track intent ID")
            .to_owned()
    }

    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_VIRTUAL_SOULFIND_V2_ENABLED", "true"));
    state.library.write().await.create(
        "Residual Artist".to_owned(),
        "Residual Track".to_owned(),
        "Residual Album".to_owned(),
    );
    let unknown = "00000000-0000-0000-0000-000000000000";

    let artists = request!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/artists/search?query=residual&limit=10",
        "",
        &state
    );
    let artists_json = serde_json::from_str::<serde_json::Value>(&artists.body)
        .expect("residual artist search JSON");
    let artist_id = artists_json[0]["artistId"]
        .as_str()
        .expect("residual artist ID")
        .to_owned();
    let releases = request!(
        "GET",
        &format!("/api/v1/virtualsoulfind/v2/catalogue/artists/{artist_id}/releases"),
        "",
        &state
    );
    let releases_json = serde_json::from_str::<serde_json::Value>(&releases.body)
        .expect("residual artist releases JSON");
    let release_id = releases_json[0]["releaseGroupId"]
        .as_str()
        .expect("residual release ID")
        .to_owned();
    let tracks = request!(
        "GET",
        &format!("/api/v1/virtualsoulfind/v2/catalogue/releases/{release_id}/tracks"),
        "",
        &state
    );
    let tracks_json = serde_json::from_str::<serde_json::Value>(&tracks.body)
        .expect("residual release tracks JSON");
    let track_id = tracks_json[0]["trackId"]
        .as_str()
        .expect("residual track ID")
        .to_owned();

    let pending_id = enqueue_track_id(&state, &track_id).await;
    let execution_intent_id = enqueue_track_id(&state, &track_id).await;
    let catalogue = super::virtual_soulfind_catalogue(&state).await;
    assert!(state
        .virtual_soulfind_v2
        .write()
        .await
        .process_track(&execution_intent_id, &catalogue));
    let execution_id = state
        .virtual_soulfind_v2
        .read()
        .await
        .latest_execution_id()
        .expect("execution ID");

    let search_malformed = request!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/artists/search",
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/artists/search",
        "malformed-path-query-or-body",
        search_malformed.status == "400 Bad Request"
    );
    let search_missing = request!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/artists/search?query=does-not-exist",
        "",
        &state
    );
    let search_missing_json =
        serde_json::from_str::<serde_json::Value>(&search_missing.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/artists/search",
        "missing-empty-or-conflict-state",
        search_missing.status == "200 OK"
            && search_missing_json.as_array().is_some_and(Vec::is_empty)
    );
    let search_runtime = request!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/artists/search?query=residual",
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/artists/search",
        "runtime-failure-and-timeout",
        search_runtime.status == "200 OK"
    );

    let artist_nominal = request!(
        "GET",
        &format!("/api/v1/virtualsoulfind/v2/catalogue/artists/{artist_id}"),
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/artists/{artistId}",
        "nominal-status-headers-body",
        artist_nominal.status == "200 OK"
    );
    let artist_populated_json =
        serde_json::from_str::<serde_json::Value>(&artist_nominal.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/artists/{artistId}",
        "populated-dynamic-state",
        artist_populated_json["artistId"] == artist_id
    );
    let artist_malformed = request!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/artists/%20",
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/artists/{artistId}",
        "malformed-path-query-or-body",
        artist_malformed.status == "400 Bad Request"
    );
    let artist_missing = request!(
        "GET",
        &format!("/api/v1/virtualsoulfind/v2/catalogue/artists/{unknown}"),
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/artists/{artistId}",
        "missing-empty-or-conflict-state",
        artist_missing.status == "404 Not Found"
    );
    let artist_runtime = request!(
        "GET",
        &format!("/api/v1/virtualsoulfind/v2/catalogue/artists/{artist_id}"),
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/artists/{artistId}",
        "runtime-failure-and-timeout",
        artist_runtime.status == "200 OK"
    );

    let artist_releases_malformed = request!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/artists/%20/releases",
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/artists/{artistId}/releases",
        "malformed-path-query-or-body",
        artist_releases_malformed.status == "400 Bad Request"
    );
    let artist_releases_missing = request!(
        "GET",
        &format!("/api/v1/virtualsoulfind/v2/catalogue/artists/{unknown}/releases"),
        "",
        &state
    );
    let artist_releases_missing_json =
        serde_json::from_str::<serde_json::Value>(&artist_releases_missing.body)
            .unwrap_or_default();
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/artists/{artistId}/releases",
        "missing-empty-or-conflict-state",
        artist_releases_missing.status == "200 OK"
            && artist_releases_missing_json
                .as_array()
                .is_some_and(|items| items.is_empty())
    );
    let artist_releases_runtime = request!(
        "GET",
        &format!("/api/v1/virtualsoulfind/v2/catalogue/artists/{artist_id}/releases"),
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/artists/{artistId}/releases",
        "runtime-failure-and-timeout",
        artist_releases_runtime.status == "200 OK"
    );

    let release_tracks_malformed = request!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/releases/%20/tracks",
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/releases/{releaseId}/tracks",
        "malformed-path-query-or-body",
        release_tracks_malformed.status == "400 Bad Request"
    );
    let release_tracks_missing = request!(
        "GET",
        &format!("/api/v1/virtualsoulfind/v2/catalogue/releases/{unknown}/tracks"),
        "",
        &state
    );
    let release_tracks_missing_json =
        serde_json::from_str::<serde_json::Value>(&release_tracks_missing.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/releases/{releaseId}/tracks",
        "missing-empty-or-conflict-state",
        release_tracks_missing.status == "200 OK"
            && release_tracks_missing_json
                .as_array()
                .is_some_and(|items| items.is_empty())
    );
    let release_tracks_runtime = request!(
        "GET",
        &format!("/api/v1/virtualsoulfind/v2/catalogue/releases/{release_id}/tracks"),
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/releases/{releaseId}/tracks",
        "runtime-failure-and-timeout",
        release_tracks_runtime.status == "200 OK"
    );

    let execution_nominal = request!(
        "GET",
        &format!("/api/v1/virtualsoulfind/v2/executions/{execution_id}"),
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/executions/{executionId}",
        "nominal-status-headers-body",
        execution_nominal.status == "200 OK"
    );
    let execution_populated_json =
        serde_json::from_str::<serde_json::Value>(&execution_nominal.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/executions/{executionId}",
        "populated-dynamic-state",
        execution_populated_json["executionId"] == execution_id
    );
    let execution_malformed = request!(
        "GET",
        "/api/v1/virtualsoulfind/v2/executions/%20",
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/executions/{executionId}",
        "malformed-path-query-or-body",
        execution_malformed.status == "400 Bad Request"
    );
    let execution_missing = request!(
        "GET",
        &format!("/api/v1/virtualsoulfind/v2/executions/{unknown}"),
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/executions/{executionId}",
        "missing-empty-or-conflict-state",
        execution_missing.status == "404 Not Found"
    );
    let execution_runtime = request!(
        "GET",
        &format!("/api/v1/virtualsoulfind/v2/executions/{execution_id}"),
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/executions/{executionId}",
        "runtime-failure-and-timeout",
        execution_runtime.status == "200 OK"
    );

    let release_nominal_body = serde_json::json!({ "releaseId": "residual-release" }).to_string();
    let release_nominal = request!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/releases",
        &release_nominal_body,
        &state
    );
    let release_nominal_json =
        serde_json::from_str::<serde_json::Value>(&release_nominal.body).unwrap_or_default();
    let release_intent_id = release_nominal_json["desiredReleaseId"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/releases",
        "nominal-status-headers-body",
        release_nominal.status == "201 Created" && !release_intent_id.is_empty()
    );
    let release_get = request!(
        "GET",
        &format!("/api/v1/virtualsoulfind/v2/intents/releases/{release_intent_id}"),
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/intents/releases/{intentId}",
        "nominal-status-headers-body",
        release_get.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/intents/releases/{intentId}",
        "populated-dynamic-state",
        release_get.status == "200 OK"
    );
    let release_get_malformed = request!(
        "GET",
        "/api/v1/virtualsoulfind/v2/intents/releases/%20",
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/intents/releases/{intentId}",
        "malformed-path-query-or-body",
        release_get_malformed.status == "400 Bad Request"
    );
    let release_get_missing = request!(
        "GET",
        &format!("/api/v1/virtualsoulfind/v2/intents/releases/{unknown}"),
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/intents/releases/{intentId}",
        "missing-empty-or-conflict-state",
        release_get_missing.status == "404 Not Found"
    );
    let release_get_runtime = request!(
        "GET",
        &format!("/api/v1/virtualsoulfind/v2/intents/releases/{release_intent_id}"),
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/intents/releases/{intentId}",
        "runtime-failure-and-timeout",
        release_get_runtime.status == "200 OK"
    );

    let track_route = format!("/api/v1/virtualsoulfind/v2/intents/tracks/{pending_id}");
    let track_malformed = request!(
        "GET",
        "/api/v1/virtualsoulfind/v2/intents/tracks/%20",
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/intents/tracks/{intentId}",
        "malformed-path-query-or-body",
        track_malformed.status == "400 Bad Request"
    );
    let track_missing = request!(
        "GET",
        &format!("/api/v1/virtualsoulfind/v2/intents/tracks/{unknown}"),
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/intents/tracks/{intentId}",
        "missing-empty-or-conflict-state",
        track_missing.status == "404 Not Found"
    );
    let track_runtime = request!("GET", &track_route, "", &state);
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/intents/tracks/{intentId}",
        "runtime-failure-and-timeout",
        track_runtime.status == "200 OK"
    );

    let pending_nominal = request!(
        "GET",
        "/api/v1/virtualsoulfind/v2/intents/tracks/pending",
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/intents/tracks/pending",
        "nominal-status-headers-body",
        pending_nominal.status == "200 OK"
    );
    let pending_malformed = request!(
        "GET",
        "/api/v1/virtualsoulfind/v2/intents/tracks/pending?limit=invalid",
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/intents/tracks/pending",
        "malformed-path-query-or-body",
        pending_malformed.status == "400 Bad Request"
    );
    let (empty_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_VIRTUAL_SOULFIND_V2_ENABLED", "true"));
    let pending_missing = request!(
        "GET",
        "/api/v1/virtualsoulfind/v2/intents/tracks/pending",
        "",
        &empty_state
    );
    let pending_missing_json =
        serde_json::from_str::<serde_json::Value>(&pending_missing.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/intents/tracks/pending",
        "missing-empty-or-conflict-state",
        pending_missing.status == "200 OK"
            && pending_missing_json
                .as_array()
                .is_some_and(|items| items.is_empty())
    );
    let pending_runtime = request!(
        "GET",
        "/api/v1/virtualsoulfind/v2/intents/tracks/pending?limit=1",
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/intents/tracks/pending",
        "runtime-failure-and-timeout",
        pending_runtime.status == "200 OK"
    );
    let pending_populated_json =
        serde_json::from_str::<serde_json::Value>(&pending_nominal.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/intents/tracks/pending",
        "populated-dynamic-state",
        pending_populated_json.as_array().is_some_and(|items| items
            .iter()
            .any(|item| item["desiredTrackId"] == pending_id))
    );

    let stats_malformed = request!(
        "GET",
        "/api/v1/virtualsoulfind/v2/stats?unexpected=%7B",
        "",
        &state
    );
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/stats",
        "malformed-path-query-or-body",
        stats_malformed.status == "200 OK"
    );
    let stats_missing = request!("GET", "/api/v1/virtualsoulfind/v2/stats", "", &empty_state);
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/stats",
        "missing-empty-or-conflict-state",
        stats_missing.status == "200 OK"
    );
    let stats_runtime = request!("GET", "/api/v1/virtualsoulfind/v2/stats", "", &state);
    let stats_runtime_json =
        serde_json::from_str::<serde_json::Value>(&stats_runtime.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v1/virtualsoulfind/v2/stats",
        "runtime-failure-and-timeout",
        stats_runtime.status == "200 OK" && stats_runtime_json["totalProcessed"].is_number()
    );

    let patch_nominal_id = enqueue_track_id(&state, &track_id).await;
    let patch_nominal_body = serde_json::json!({ "status": "Planned" }).to_string();
    let patch_nominal = request!(
        "PATCH",
        &format!("/api/v1/virtualsoulfind/v2/intents/tracks/{patch_nominal_id}"),
        &patch_nominal_body,
        &state
    );
    record!(
        "PATCH",
        "/api/v1/virtualsoulfind/v2/intents/tracks/{intentId}",
        "nominal-status-headers-body",
        patch_nominal.status == "204 No Content"
    );
    let patch_malformed_id = enqueue_track_id(&state, &track_id).await;
    let patch_malformed = request!(
        "PATCH",
        &format!("/api/v1/virtualsoulfind/v2/intents/tracks/{patch_malformed_id}"),
        "{}",
        &state
    );
    record!(
        "PATCH",
        "/api/v1/virtualsoulfind/v2/intents/tracks/{intentId}",
        "malformed-path-query-or-body",
        patch_malformed.status == "400 Bad Request"
    );
    let patch_missing = request!(
        "PATCH",
        &format!("/api/v1/virtualsoulfind/v2/intents/tracks/{unknown}"),
        &patch_nominal_body,
        &state
    );
    record!(
        "PATCH",
        "/api/v1/virtualsoulfind/v2/intents/tracks/{intentId}",
        "missing-empty-or-conflict-state",
        patch_missing.status == "404 Not Found"
    );
    let patch_runtime_id = enqueue_track_id(&state, &track_id).await;
    let patch_runtime = request!(
        "PATCH",
        &format!("/api/v1/virtualsoulfind/v2/intents/tracks/{patch_runtime_id}"),
        &patch_nominal_body,
        &state
    );
    record!(
        "PATCH",
        "/api/v1/virtualsoulfind/v2/intents/tracks/{intentId}",
        "runtime-failure-and-timeout",
        patch_runtime.status == "204 No Content"
    );
    let patch_mutation_id = enqueue_track_id(&state, &track_id).await;
    let patch_mutation = request!(
        "PATCH",
        &format!("/api/v1/virtualsoulfind/v2/intents/tracks/{patch_mutation_id}"),
        &serde_json::json!({ "status": "OnHold" }).to_string(),
        &state
    );
    let patch_mutation_readback = request!(
        "GET",
        &format!("/api/v1/virtualsoulfind/v2/intents/tracks/{patch_mutation_id}"),
        "",
        &state
    );
    let patch_mutation_json =
        serde_json::from_str::<serde_json::Value>(&patch_mutation_readback.body)
            .unwrap_or_default();
    record!(
        "PATCH",
        "/api/v1/virtualsoulfind/v2/intents/tracks/{intentId}",
        "mutation-side-effects-and-readback",
        patch_mutation.status == "204 No Content" && patch_mutation_json["status"] == "OnHold"
    );
    let (patch_restart_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_VIRTUAL_SOULFIND_V2_ENABLED", "true"));
    let patch_restart = request!(
        "PATCH",
        &format!("/api/v1/virtualsoulfind/v2/intents/tracks/{patch_mutation_id}"),
        &patch_nominal_body,
        &patch_restart_state
    );
    record!(
        "PATCH",
        "/api/v1/virtualsoulfind/v2/intents/tracks/{intentId}",
        "restart-persistence-or-reset",
        patch_restart.status == "404 Not Found"
    );
    let patch_concurrent_id = enqueue_track_id(&state, &track_id).await;
    let patch_concurrent_path =
        format!("/api/v1/virtualsoulfind/v2/intents/tracks/{patch_concurrent_id}");
    let (patch_a, patch_b) = tokio::join!(
        super::route_http_request(
            "PATCH",
            &patch_concurrent_path,
            None,
            &patch_nominal_body,
            &state
        ),
        super::route_http_request(
            "PATCH",
            &patch_concurrent_path,
            None,
            &patch_nominal_body,
            &state
        ),
    );
    record!(
        "PATCH",
        "/api/v1/virtualsoulfind/v2/intents/tracks/{intentId}",
        "concurrency-and-idempotency",
        patch_a
            .as_ref()
            .is_ok_and(|response| response.status == "204 No Content")
            && patch_b
                .as_ref()
                .is_ok_and(|response| response.status == "204 No Content")
    );

    let release_malformed = request!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/releases",
        "{}",
        &state
    );
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/releases",
        "malformed-path-query-or-body",
        release_malformed.status == "400 Bad Request"
    );
    let release_missing = request!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/releases",
        "",
        &state
    );
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/releases",
        "missing-empty-or-conflict-state",
        release_missing.status == "400 Bad Request"
    );
    let release_runtime_body = serde_json::json!({ "releaseId": "runtime-release" }).to_string();
    let release_runtime = request!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/releases",
        &release_runtime_body,
        &state
    );
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/releases",
        "runtime-failure-and-timeout",
        release_runtime.status == "201 Created"
    );
    let release_mutation_body = serde_json::json!({ "releaseId": "mutation-release" }).to_string();
    let release_mutation = request!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/releases",
        &release_mutation_body,
        &state
    );
    let release_mutation_id = serde_json::from_str::<serde_json::Value>(&release_mutation.body)
        .unwrap_or_default()["desiredReleaseId"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let release_mutation_readback = request!(
        "GET",
        &format!("/api/v1/virtualsoulfind/v2/intents/releases/{release_mutation_id}"),
        "",
        &state
    );
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/releases",
        "mutation-side-effects-and-readback",
        release_mutation.status == "201 Created" && release_mutation_readback.status == "200 OK"
    );
    let (release_restart_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_VIRTUAL_SOULFIND_V2_ENABLED", "true"));
    let release_restart = request!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/releases",
        &release_runtime_body,
        &release_restart_state
    );
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/releases",
        "restart-persistence-or-reset",
        release_restart.status == "201 Created"
    );
    let release_concurrent_body_a =
        serde_json::json!({ "releaseId": "concurrent-release-a" }).to_string();
    let release_concurrent_body_b =
        serde_json::json!({ "releaseId": "concurrent-release-b" }).to_string();
    let (release_a, release_b) = tokio::join!(
        super::route_http_request(
            "POST",
            "/api/v1/virtualsoulfind/v2/intents/releases",
            None,
            &release_concurrent_body_a,
            &state
        ),
        super::route_http_request(
            "POST",
            "/api/v1/virtualsoulfind/v2/intents/releases",
            None,
            &release_concurrent_body_b,
            &state
        ),
    );
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/releases",
        "concurrency-and-idempotency",
        release_a
            .as_ref()
            .is_ok_and(|response| response.status == "201 Created")
            && release_b
                .as_ref()
                .is_ok_and(|response| response.status == "201 Created")
    );

    let track_malformed_body = "{}";
    let track_malformed_post = request!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/tracks",
        track_malformed_body,
        &state
    );
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/tracks",
        "malformed-path-query-or-body",
        track_malformed_post.status == "400 Bad Request"
    );
    let track_missing_post = request!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/tracks",
        "",
        &state
    );
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/tracks",
        "missing-empty-or-conflict-state",
        track_missing_post.status == "400 Bad Request"
    );
    let track_runtime_body =
        serde_json::json!({ "domain": "Music", "trackId": track_id }).to_string();
    let track_runtime_post = request!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/tracks",
        &track_runtime_body,
        &state
    );
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/tracks",
        "runtime-failure-and-timeout",
        track_runtime_post.status == "201 Created"
    );
    let (track_restart_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_VIRTUAL_SOULFIND_V2_ENABLED", "true"));
    let track_restart_post = request!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/tracks",
        &track_runtime_body,
        &track_restart_state
    );
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/tracks",
        "restart-persistence-or-reset",
        track_restart_post.status == "201 Created"
    );
    let (track_a, track_b) = tokio::join!(
        super::route_http_request(
            "POST",
            "/api/v1/virtualsoulfind/v2/intents/tracks",
            None,
            &track_runtime_body,
            &state
        ),
        super::route_http_request(
            "POST",
            "/api/v1/virtualsoulfind/v2/intents/tracks",
            None,
            &track_runtime_body,
            &state
        ),
    );
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/tracks",
        "concurrency-and-idempotency",
        track_a
            .as_ref()
            .is_ok_and(|response| response.status == "201 Created")
            && track_b
                .as_ref()
                .is_ok_and(|response| response.status == "201 Created")
    );

    let process_runtime_id = enqueue_track_id(&state, &track_id).await;
    let process_runtime = request!(
        "POST",
        &format!("/api/v1/virtualsoulfind/v2/intents/tracks/{process_runtime_id}/process"),
        "",
        &state
    );
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/tracks/{intentId}/process",
        "runtime-failure-and-timeout",
        process_runtime.status == "202 Accepted"
    );
    let process_malformed = request!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/tracks/%20/process",
        "",
        &state
    );
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/tracks/{intentId}/process",
        "malformed-path-query-or-body",
        process_malformed.status == "400 Bad Request"
    );
    let process_missing = request!(
        "POST",
        &format!("/api/v1/virtualsoulfind/v2/intents/tracks/{unknown}/process"),
        "",
        &state
    );
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/tracks/{intentId}/process",
        "missing-empty-or-conflict-state",
        process_missing.status == "404 Not Found"
    );
    let process_mutation_id = enqueue_track_id(&state, &track_id).await;
    let process_mutation = request!(
        "POST",
        &format!("/api/v1/virtualsoulfind/v2/intents/tracks/{process_mutation_id}/process"),
        "",
        &state
    );
    tokio::task::yield_now().await;
    tokio::task::yield_now().await;
    let process_mutation_readback = request!(
        "GET",
        &format!("/api/v1/virtualsoulfind/v2/intents/tracks/{process_mutation_id}"),
        "",
        &state
    );
    let process_mutation_json =
        serde_json::from_str::<serde_json::Value>(&process_mutation_readback.body)
            .unwrap_or_default();
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/tracks/{intentId}/process",
        "mutation-side-effects-and-readback",
        process_mutation.status == "202 Accepted" && process_mutation_json["status"] == "Completed"
    );
    let (process_restart_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_VIRTUAL_SOULFIND_V2_ENABLED", "true"));
    let process_restart = request!(
        "POST",
        &format!("/api/v1/virtualsoulfind/v2/intents/tracks/{process_mutation_id}/process"),
        "",
        &process_restart_state
    );
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/tracks/{intentId}/process",
        "restart-persistence-or-reset",
        process_restart.status == "404 Not Found"
    );
    let process_concurrent_id = enqueue_track_id(&state, &track_id).await;
    let process_concurrent_path =
        format!("/api/v1/virtualsoulfind/v2/intents/tracks/{process_concurrent_id}/process");
    let (process_a, process_b) = tokio::join!(
        super::route_http_request("POST", &process_concurrent_path, None, "", &state),
        super::route_http_request("POST", &process_concurrent_path, None, "", &state),
    );
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/tracks/{intentId}/process",
        "concurrency-and-idempotency",
        process_a
            .as_ref()
            .is_ok_and(|response| response.status == "202 Accepted")
            && process_b
                .as_ref()
                .is_ok_and(|response| response.status == "202 Accepted")
    );

    let plan_malformed = request!("POST", "/api/v1/virtualsoulfind/v2/plans", "{}", &state);
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/plans",
        "malformed-path-query-or-body",
        plan_malformed.status == "400 Bad Request"
    );
    let plan_missing = request!("POST", "/api/v1/virtualsoulfind/v2/plans", "", &state);
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/plans",
        "missing-empty-or-conflict-state",
        plan_missing.status == "400 Bad Request"
    );
    let plan_runtime_body = serde_json::json!({
        "domain": "Music",
        "trackId": track_id,
        "mode": "OfflinePlanning",
    })
    .to_string();
    let plan_runtime = request!(
        "POST",
        "/api/v1/virtualsoulfind/v2/plans",
        &plan_runtime_body,
        &state
    );
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/plans",
        "runtime-failure-and-timeout",
        plan_runtime.status == "200 OK"
    );
    let (plan_restart_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_VIRTUAL_SOULFIND_V2_ENABLED", "true"));
    let plan_restart = request!(
        "POST",
        "/api/v1/virtualsoulfind/v2/plans",
        &plan_runtime_body,
        &plan_restart_state
    );
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/plans",
        "restart-persistence-or-reset",
        plan_restart.status == "200 OK"
    );
    let (plan_a, plan_b) = tokio::join!(
        super::route_http_request(
            "POST",
            "/api/v1/virtualsoulfind/v2/plans",
            None,
            &plan_runtime_body,
            &state
        ),
        super::route_http_request(
            "POST",
            "/api/v1/virtualsoulfind/v2/plans",
            None,
            &plan_runtime_body,
            &state
        ),
    );
    record!(
        "POST",
        "/api/v1/virtualsoulfind/v2/plans",
        "concurrency-and-idempotency",
        plan_a
            .as_ref()
            .is_ok_and(|response| response.status == "200 OK")
            && plan_b
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("virtual_soulfind_v2_residuals.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert_eq!(ledger.len(), 65, "VirtualSoulfind v2 residual ledger size");
    assert!(
        mismatches.is_empty(),
        "{} controller-api VirtualSoulfind v2 residual mismatches:\n{}",
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
async fn controller_api_differential_source_discovery() {
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
    let no_partial = super::route_http_request(
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

    let started = super::route_http_request(
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
        super::SessionCommand::Search {
            token,
            query,
            target: super::SearchDispatchTarget::Global,
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

    let conflict = super::route_http_request(
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
        record.results.push(super::SearchResultEntry {
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

    let status = super::route_http_request("GET", "/api/v0/discovery", None, "", &state)
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

    let by_size = super::route_http_request(
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

    let by_name = super::route_http_request(
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

    let summaries = super::route_http_request(
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

    let stopped = super::route_http_request("POST", "/api/v0/discovery/stop", None, "{}", &state)
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

/// Differential proof for the residual source-discovery controller cases.
/// The frozen service keeps an empty status/SQLite projection available
/// when the Soulseek client is idle, returns a no-op response when stop is
/// requested before start, and reports its full conflict DTO for an
/// overlapping start.  These probes cover those boundaries plus the
/// malformed, reset, restart, and concurrent request paths.
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
async fn controller_api_differential_discovery_open_cases() {
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
    let json_value = |response: &super::routing::HttpResponse| {
        serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or(serde_json::Value::Null)
    };
    let status_contract = |response: &super::routing::HttpResponse| {
        let value = json_value(response);
        response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && value["isRunning"].is_boolean()
            && value["currentSearchTerm"].is_string()
            && value["stats"]["totalFiles"].is_u64()
            && value["stats"]["totalUsers"].is_u64()
            && value["stats"]["searchCycles"].is_u64()
            && value["stats"]["lastCycleNewFiles"].is_u64()
            && value["stats"]["hashVerificationEnabled"].is_boolean()
            && value["stats"]["filesWithHash"].is_u64()
    };
    let empty_status_contract = |response: &super::routing::HttpResponse| {
        let value = json_value(response);
        status_contract(response)
            && value["isRunning"] == false
            && value["currentSearchTerm"] == ""
            && value["stats"]["totalFiles"] == 0
            && value["stats"]["totalUsers"] == 0
            && value["stats"]["searchCycles"] == 0
            && value["stats"]["lastCycleNewFiles"] == 0
            && value["stats"]["hashVerificationEnabled"] == false
            && value["stats"]["filesWithHash"] == 0
    };

    {
        let (state, _receiver) = test_state_with_env(target_env());
        let malformed =
            super::route_http_request("GET", "/api/v0/discovery/extra", None, "", &state)
                .await
                .expect("discovery status malformed response");
        record!(
            "GET",
            "/api/v0/discovery",
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );

        let empty = super::route_http_request("GET", "/api/v0/discovery", None, "", &state)
            .await
            .expect("discovery empty status response");
        record!(
            "GET",
            "/api/v0/discovery",
            "missing-empty-or-conflict-state",
            empty_status_contract(&empty)
        );
    }

    {
        // Status is local service state in the frozen controller; a
        // disconnected session must not turn this read into a 5xx.
        let (runtime_state, runtime_receiver) = test_state_with_env(target_env());
        drop(runtime_receiver);
        let runtime =
            super::route_http_request("GET", "/api/v0/discovery", None, "", &runtime_state)
                .await
                .expect("discovery runtime status response");
        record!(
            "GET",
            "/api/v0/discovery",
            "runtime-failure-and-timeout",
            empty_status_contract(&runtime)
        );
    }

    {
        let (state, _receiver) = test_state_with_env(target_env());
        let malformed = super::route_http_request(
            "GET",
            "/api/v0/discovery/no-partial-count/extra",
            None,
            "",
            &state,
        )
        .await
        .expect("no-partial malformed response");
        record!(
            "GET",
            "/api/v0/discovery/no-partial-count",
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );

        let empty = super::route_http_request(
            "GET",
            "/api/v0/discovery/no-partial-count",
            None,
            "",
            &state,
        )
        .await
        .expect("no-partial empty response");
        let empty_json = json_value(&empty);
        record!(
            "GET",
            "/api/v0/discovery/no-partial-count",
            "missing-empty-or-conflict-state",
            empty.status == "200 OK"
                && empty_json["usersWithoutPartialSupport"] == 0
                && empty_json["message"]
                    == "0 users are flagged as not supporting partial/chunked downloads"
        );
    }

    {
        let (runtime_state, runtime_receiver) = test_state_with_env(target_env());
        drop(runtime_receiver);
        let runtime = super::route_http_request(
            "GET",
            "/api/v0/discovery/no-partial-count",
            None,
            "",
            &runtime_state,
        )
        .await
        .expect("no-partial runtime response");
        let runtime_json = json_value(&runtime);
        record!(
            "GET",
            "/api/v0/discovery/no-partial-count",
            "runtime-failure-and-timeout",
            runtime.status == "200 OK" && runtime_json["usersWithoutPartialSupport"] == 0
        );

        let (populated_state, mut receiver) = test_state_with_env(target_env());
        let started = super::route_http_request(
            "POST",
            "/api/v0/discovery/start",
            None,
            r#"{"searchTerm":"partial support probe"}"#,
            &populated_state,
        )
        .await
        .expect("start no-partial populated probe");
        let token = match receiver.recv().await.expect("no-partial discovery command") {
            super::SessionCommand::Search { token, .. } => token,
            command => panic!("unexpected no-partial command: {command:?}"),
        };
        {
            let mut searches = populated_state.searches.write().await;
            searches
                .records
                .iter_mut()
                .find(|record| record.token == token)
                .expect("no-partial populated search")
                .results
                .push(super::SearchResultEntry {
                    peer_username: Some("partial-probe-peer".to_owned()),
                    filename: "Partial/Probe.flac".to_owned(),
                    size: 123,
                    extension: "flac".to_owned(),
                    bit_rate: None,
                    sample_rate: None,
                    bit_depth: None,
                    length_seconds: None,
                    locked: false,
                    slot_free: Some(true),
                    average_speed: Some(1),
                    queue_length: Some(0),
                });
        }
        let populated = super::route_http_request(
            "GET",
            "/api/v0/discovery/no-partial-count",
            None,
            "",
            &populated_state,
        )
        .await
        .expect("no-partial populated response");
        let populated_json = json_value(&populated);
        record!(
            "GET",
            "/api/v0/discovery/no-partial-count",
            "populated-dynamic-state",
            started.status == "200 OK"
                && populated.status == "200 OK"
                && populated_json["usersWithoutPartialSupport"] == 0
        );
    }

    {
        let (state, _receiver) = test_state_with_env(target_env());
        let malformed = super::route_http_request(
            "GET",
            "/api/v0/discovery/sources/by-filename/extra",
            None,
            "",
            &state,
        )
        .await
        .expect("discovery filename malformed response");
        record!(
            "GET",
            "/api/v0/discovery/sources/by-filename",
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );

        let missing = super::route_http_request(
            "GET",
            "/api/v0/discovery/sources/by-filename",
            None,
            "",
            &state,
        )
        .await
        .expect("discovery filename missing response");
        record!(
            "GET",
            "/api/v0/discovery/sources/by-filename",
            "missing-empty-or-conflict-state",
            missing.status == "400 Bad Request"
                && missing.body.contains("pattern query parameter is required")
        );
    }

    {
        let (runtime_state, runtime_receiver) = test_state_with_env(target_env());
        drop(runtime_receiver);
        let runtime = super::route_http_request(
            "GET",
            "/api/v0/discovery/sources/by-filename?pattern=recording",
            None,
            "",
            &runtime_state,
        )
        .await
        .expect("discovery filename runtime response");
        let runtime_json = json_value(&runtime);
        record!(
            "GET",
            "/api/v0/discovery/sources/by-filename",
            "runtime-failure-and-timeout",
            runtime.status == "200 OK"
                && runtime_json["pattern"] == "recording"
                && runtime_json["sourceCount"] == 0
                && runtime_json["sources"]
                    .as_array()
                    .is_some_and(Vec::is_empty)
        );
    }

    {
        let (state, _receiver) = test_state_with_env(target_env());
        let malformed = super::route_http_request(
            "GET",
            "/api/v0/discovery/sources/by-size/not-a-size",
            None,
            "",
            &state,
        )
        .await
        .expect("discovery size malformed response");
        record!(
            "GET",
            "/api/v0/discovery/sources/by-size/{size}",
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
                && malformed.body.contains("size must be greater than zero")
        );

        let missing = super::route_http_request(
            "GET",
            "/api/v0/discovery/sources/by-size/42",
            None,
            "",
            &state,
        )
        .await
        .expect("discovery size missing response");
        let missing_json = json_value(&missing);
        record!(
            "GET",
            "/api/v0/discovery/sources/by-size/{size}",
            "missing-empty-or-conflict-state",
            missing.status == "200 OK"
                && missing_json["size"] == 42
                && missing_json["sourceCount"] == 0
                && missing_json["sources"]
                    .as_array()
                    .is_some_and(Vec::is_empty)
        );
    }

    {
        let (runtime_state, runtime_receiver) = test_state_with_env(target_env());
        drop(runtime_receiver);
        let runtime = super::route_http_request(
            "GET",
            "/api/v0/discovery/sources/by-size/42?limit=10",
            None,
            "",
            &runtime_state,
        )
        .await
        .expect("discovery size runtime response");
        let runtime_json = json_value(&runtime);
        record!(
            "GET",
            "/api/v0/discovery/sources/by-size/{size}",
            "runtime-failure-and-timeout",
            runtime.status == "200 OK"
                && runtime_json["size"] == 42
                && runtime_json["sourceCount"] == 0
        );
    }

    {
        let (state, _receiver) = test_state_with_env(target_env());
        let malformed = super::route_http_request(
            "GET",
            "/api/v0/discovery/summaries?minSources=0",
            None,
            "",
            &state,
        )
        .await
        .expect("discovery summaries malformed response");
        record!(
            "GET",
            "/api/v0/discovery/summaries",
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
                && malformed
                    .body
                    .contains("minSources must be greater than zero")
        );

        let missing =
            super::route_http_request("GET", "/api/v0/discovery/summaries", None, "", &state)
                .await
                .expect("discovery summaries missing response");
        let missing_json = json_value(&missing);
        record!(
            "GET",
            "/api/v0/discovery/summaries",
            "missing-empty-or-conflict-state",
            missing.status == "200 OK"
                && missing_json["minSources"] == 2
                && missing_json["count"] == 0
                && missing_json["summaries"]
                    .as_array()
                    .is_some_and(Vec::is_empty)
        );
    }

    {
        let (runtime_state, runtime_receiver) = test_state_with_env(target_env());
        drop(runtime_receiver);
        let runtime = super::route_http_request(
            "GET",
            "/api/v0/discovery/summaries?minSources=1",
            None,
            "",
            &runtime_state,
        )
        .await
        .expect("discovery summaries runtime response");
        let runtime_json = json_value(&runtime);
        record!(
            "GET",
            "/api/v0/discovery/summaries",
            "runtime-failure-and-timeout",
            runtime.status == "200 OK"
                && runtime_json["minSources"] == 1
                && runtime_json["count"] == 0
        );
    }

    {
        let (state, _receiver) = test_state_with_env(target_env());
        let nominal = super::route_http_request(
            "POST",
            "/api/v0/discovery/reset-partial-flags",
            None,
            "",
            &state,
        )
        .await
        .expect("discovery reset nominal response");
        record!(
            "POST",
            "/api/v0/discovery/reset-partial-flags",
            "nominal-status-headers-body",
            nominal.status == "200 OK"
                && nominal
                    .body
                    .contains("Reset partial support flags for 0 users")
        );

        let malformed = super::route_http_request(
            "POST",
            "/api/v0/discovery/reset-partial-flags/extra",
            None,
            "",
            &state,
        )
        .await
        .expect("discovery reset malformed response");
        record!(
            "POST",
            "/api/v0/discovery/reset-partial-flags",
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );

        let missing = super::route_http_request(
            "POST",
            "/api/v0/discovery/reset-partial-flags",
            None,
            "",
            &state,
        )
        .await
        .expect("discovery reset missing response");
        record!(
            "POST",
            "/api/v0/discovery/reset-partial-flags",
            "missing-empty-or-conflict-state",
            missing.status == "200 OK"
                && missing
                    .body
                    .contains("Reset partial support flags for 0 users")
        );

        let followup = super::route_http_request(
            "GET",
            "/api/v0/discovery/no-partial-count",
            None,
            "",
            &state,
        )
        .await
        .expect("discovery reset readback response");
        let followup_json = json_value(&followup);
        record!(
            "POST",
            "/api/v0/discovery/reset-partial-flags",
            "mutation-side-effects-and-readback",
            followup.status == "200 OK" && followup_json["usersWithoutPartialSupport"] == 0
        );
    }

    {
        let (runtime_state, runtime_receiver) = test_state_with_env(target_env());
        drop(runtime_receiver);
        let runtime = super::route_http_request(
            "POST",
            "/api/v0/discovery/reset-partial-flags",
            None,
            "",
            &runtime_state,
        )
        .await
        .expect("discovery reset runtime response");
        record!(
            "POST",
            "/api/v0/discovery/reset-partial-flags",
            "runtime-failure-and-timeout",
            runtime.status == "200 OK"
                && runtime
                    .body
                    .contains("Reset partial support flags for 0 users")
        );

        let (restarted_state, _receiver) = test_state_with_env(target_env());
        let restarted = super::route_http_request(
            "POST",
            "/api/v0/discovery/reset-partial-flags",
            None,
            "",
            &restarted_state,
        )
        .await
        .expect("discovery reset restarted response");
        record!(
            "POST",
            "/api/v0/discovery/reset-partial-flags",
            "restart-persistence-or-reset",
            restarted.status == "200 OK"
                && restarted
                    .body
                    .contains("Reset partial support flags for 0 users")
        );

        let (concurrent_state, _receiver) = test_state_with_env(target_env());
        let (left, right) = tokio::join!(
            super::route_http_request(
                "POST",
                "/api/v0/discovery/reset-partial-flags",
                None,
                "",
                &concurrent_state
            ),
            super::route_http_request(
                "POST",
                "/api/v0/discovery/reset-partial-flags",
                None,
                "",
                &concurrent_state
            )
        );
        let left = left.expect("left discovery reset concurrency response");
        let right = right.expect("right discovery reset concurrency response");
        record!(
            "POST",
            "/api/v0/discovery/reset-partial-flags",
            "concurrency-and-idempotency",
            left.status == "200 OK" && right.status == "200 OK" && left.body == right.body
        );
    }

    {
        let (state, _receiver) = test_state_with_env(target_env());
        let malformed = super::route_http_request(
            "POST",
            "/api/v0/discovery/start/extra",
            None,
            "not-json",
            &state,
        )
        .await
        .expect("discovery start malformed response");
        record!(
            "POST",
            "/api/v0/discovery/start",
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );
    }

    {
        let (runtime_state, runtime_receiver) = test_state_with_env(target_env());
        drop(runtime_receiver);
        let runtime = super::route_http_request(
            "POST",
            "/api/v0/discovery/start",
            None,
            r#"{"searchTerm":"runtime discovery","enableHashVerification":false}"#,
            &runtime_state,
        )
        .await
        .expect("discovery start runtime response");
        let runtime_json = json_value(&runtime);
        record!(
            "POST",
            "/api/v0/discovery/start",
            "runtime-failure-and-timeout",
            runtime.status == "200 OK"
                && runtime_json["message"] == "Discovery started"
                && runtime_json["searchTerm"] == "runtime discovery"
                && runtime_json["hashVerificationEnabled"] == false
        );
    }

    {
        let (restart_state, mut receiver) = test_state_with_env(target_env());
        let first = super::route_http_request(
            "POST",
            "/api/v0/discovery/start",
            None,
            r#"{"searchTerm":"first discovery"}"#,
            &restart_state,
        )
        .await
        .expect("discovery first start response");
        let _ = receiver.recv().await.expect("first discovery command");
        let stopped =
            super::route_http_request("POST", "/api/v0/discovery/stop", None, "", &restart_state)
                .await
                .expect("discovery restart stop response");
        let second = super::route_http_request(
            "POST",
            "/api/v0/discovery/start",
            None,
            r#"{"searchTerm":"second discovery"}"#,
            &restart_state,
        )
        .await
        .expect("discovery second start response");
        let _ = receiver.recv().await.expect("second discovery command");
        let second_json = json_value(&second);
        record!(
            "POST",
            "/api/v0/discovery/start",
            "restart-persistence-or-reset",
            first.status == "200 OK"
                && stopped.status == "200 OK"
                && second.status == "200 OK"
                && second_json["searchTerm"] == "second discovery"
        );
    }

    {
        let (concurrent_state, _receiver) = test_state_with_env(target_env());
        let (left, right) = tokio::join!(
            super::route_http_request(
                "POST",
                "/api/v0/discovery/start",
                None,
                r#"{"searchTerm":"concurrent one"}"#,
                &concurrent_state
            ),
            super::route_http_request(
                "POST",
                "/api/v0/discovery/start",
                None,
                r#"{"searchTerm":"concurrent two"}"#,
                &concurrent_state
            )
        );
        let left = left.expect("left discovery start concurrency response");
        let right = right.expect("right discovery start concurrency response");
        let left_ok = left.status == "200 OK";
        let right_ok = right.status == "200 OK";
        let left_allowed = left_ok || left.status == "409 Conflict";
        let right_allowed = right_ok || right.status == "409 Conflict";
        record!(
            "POST",
            "/api/v0/discovery/start",
            "concurrency-and-idempotency",
            left_allowed
                && right_allowed
                && (left_ok || right_ok)
                && concurrent_state.source_discovery.read().await.running
        );
    }

    {
        let (state, _receiver) = test_state_with_env(target_env());
        let malformed =
            super::route_http_request("POST", "/api/v0/discovery/stop/extra", None, "", &state)
                .await
                .expect("discovery stop malformed response");
        record!(
            "POST",
            "/api/v0/discovery/stop",
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );

        let missing = super::route_http_request("POST", "/api/v0/discovery/stop", None, "", &state)
            .await
            .expect("discovery stop missing response");
        let missing_json = json_value(&missing);
        record!(
            "POST",
            "/api/v0/discovery/stop",
            "missing-empty-or-conflict-state",
            missing.status == "200 OK"
                && missing_json == serde_json::json!({"message": "Discovery not running"})
        );
    }

    {
        let (runtime_state, runtime_receiver) = test_state_with_env(target_env());
        drop(runtime_receiver);
        let started = super::route_http_request(
            "POST",
            "/api/v0/discovery/start",
            None,
            r#"{"searchTerm":"runtime stop"}"#,
            &runtime_state,
        )
        .await
        .expect("discovery runtime stop start response");
        let runtime =
            super::route_http_request("POST", "/api/v0/discovery/stop", None, "", &runtime_state)
                .await
                .expect("discovery stop runtime response");
        let runtime_json = json_value(&runtime);
        record!(
            "POST",
            "/api/v0/discovery/stop",
            "runtime-failure-and-timeout",
            started.status == "200 OK"
                && runtime.status == "200 OK"
                && runtime_json["message"] == "Discovery stopped"
                && runtime_json["stats"].is_object()
        );

        let (restarted_state, _receiver) = test_state_with_env(target_env());
        let restarted =
            super::route_http_request("POST", "/api/v0/discovery/stop", None, "", &restarted_state)
                .await
                .expect("discovery stop restarted response");
        let restarted_json = json_value(&restarted);
        record!(
            "POST",
            "/api/v0/discovery/stop",
            "restart-persistence-or-reset",
            restarted.status == "200 OK"
                && restarted_json == serde_json::json!({"message": "Discovery not running"})
        );

        let (concurrent_state, mut receiver) = test_state_with_env(target_env());
        let started = super::route_http_request(
            "POST",
            "/api/v0/discovery/start",
            None,
            r#"{"searchTerm":"concurrent stop"}"#,
            &concurrent_state,
        )
        .await
        .expect("discovery concurrent stop start response");
        let _ = receiver
            .recv()
            .await
            .expect("concurrent stop discovery command");
        let (left, right) = tokio::join!(
            super::route_http_request(
                "POST",
                "/api/v0/discovery/stop",
                None,
                "",
                &concurrent_state
            ),
            super::route_http_request(
                "POST",
                "/api/v0/discovery/stop",
                None,
                "",
                &concurrent_state
            )
        );
        let left = left.expect("left discovery stop concurrency response");
        let right = right.expect("right discovery stop concurrency response");
        record!(
            "POST",
            "/api/v0/discovery/stop",
            "concurrency-and-idempotency",
            started.status == "200 OK"
                && left.status == "200 OK"
                && right.status == "200 OK"
                && !concurrent_state.source_discovery.read().await.running
        );
    }

    assert_eq!(ledger.len(), 32, "discovery residual ledger size");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create discovery evidence directory");
    fs::write(
        evidence_dir.join("discovery_open_cases.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize discovery ledger"),
    )
    .expect("write discovery ledger");
    assert!(
        mismatches.is_empty(),
        "{} discovery controller mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting 4 pod/quarantine-jury stats and
/// verification routes' cases, independently re-derived from 3
/// source tests: `pod_verification_message_checks_real_membership_
/// and_signature` (real membership/signature checks, honest
/// structural-error reporting, and real attempt-counted stats),
/// `quarantine_jury_audit_report_reflects_real_status_not_
/// hardcoded_zeros` (real per-status counts and real request-age
/// staleness, not hardcoded zeros), and `pod_signing_stats_reflect_
/// real_activity_not_hardcoded_zeros` (a forged-sender rejection
/// still counts as a real failed verification, not silently
/// dropped). slskdN-only (confirmed against the frozen registry).
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
async fn controller_api_differential_pod_and_jury_stats() {
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

    // -- podcore verification/message + verification/stats --
    let (state, _receiver) = test_state();
    let pod_id = "verification-differential-pod";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<super::pods::PodRecord>(serde_json::json!({
                "podId": pod_id,
                "name": "Verification Differential",
            }))
            .expect("deserialize pod record fixture"),
            "owner-peer".to_owned(),
        )
        .expect("create pod");

    let keypair = super::route_http_request(
        "POST",
        "/api/v0/podcore/signing/generate-keypair",
        None,
        "{}",
        &state,
    )
    .await
    .expect("generate keypair");
    let keys = serde_json::from_str::<serde_json::Value>(&keypair.body).unwrap_or_default();
    state
        .pods
        .write()
        .await
        .upsert_member(
            pod_id,
            super::pods::PodMember {
                peer_id: "tester".to_owned(),
                role: "member".to_owned(),
                is_banned: false,
                public_key: keys["publicKey"].as_str().map(str::to_owned),
                joined_at: None,
                last_seen: None,
            },
        )
        .expect("add tester as a real pod member with a signing public key");

    let membership_verified = super::route_http_request(
        "GET",
        &format!("/api/v0/podcore/verification/membership/{pod_id}/tester"),
        None,
        "",
        &state,
    )
    .await
    .expect("verify membership projection");
    let membership_verified_json =
        serde_json::from_str::<serde_json::Value>(&membership_verified.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/podcore/verification/membership/{podId}/{peerId}",
        "nominal-status-headers-body",
        membership_verified.status == "200 OK"
            && membership_verified_json
                == serde_json::json!({
                    "isValidMember": true,
                    "isBanned": false,
                    "role": "member",
                })
    );
    let missing_membership = super::route_http_request(
        "GET",
        &format!("/api/v0/podcore/verification/membership/{pod_id}/missing-peer"),
        None,
        "",
        &state,
    )
    .await
    .expect("missing membership projection");
    let missing_membership_json =
        serde_json::from_str::<serde_json::Value>(&missing_membership.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/podcore/verification/membership/{podId}/{peerId}",
        "missing-empty-or-conflict-state",
        missing_membership.status == "200 OK"
            && missing_membership_json
                == serde_json::json!({
                    "isValidMember": false,
                    "isBanned": false,
                    "errorMessage": "Membership not found",
                })
    );
    let malformed_membership = super::route_http_request(
        "GET",
        &format!(
            "/api/v0/podcore/verification/membership/{}/tester",
            "x".repeat(129)
        ),
        None,
        "",
        &state,
    )
    .await
    .expect("reject oversized membership verification route value");
    record!(
        "GET",
        "/api/v0/podcore/verification/membership/{podId}/{peerId}",
        "malformed-path-query-or-body",
        malformed_membership.status == "400 Bad Request"
    );
    state
        .pods
        .write()
        .await
        .upsert_member(
            pod_id,
            super::pods::PodMember {
                peer_id: "banned-peer".to_owned(),
                role: "member".to_owned(),
                is_banned: true,
                public_key: keys["publicKey"].as_str().map(str::to_owned),
                joined_at: None,
                last_seen: None,
            },
        )
        .expect("add a real banned membership for verification");
    let banned_membership = super::route_http_request(
        "GET",
        &format!("/api/v0/podcore/verification/membership/{pod_id}/banned-peer"),
        None,
        "",
        &state,
    )
    .await
    .expect("verify banned membership projection");
    let banned_membership_json =
        serde_json::from_str::<serde_json::Value>(&banned_membership.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/podcore/verification/membership/{podId}/{peerId}",
        "populated-dynamic-state",
        banned_membership.status == "200 OK"
            && banned_membership_json
                == serde_json::json!({
                    "isValidMember": true,
                    "isBanned": true,
                    "role": "member",
                })
    );
    state
        .pods
        .write()
        .await
        .upsert_member(
            pod_id,
            super::pods::PodMember {
                peer_id: "moderator-peer".to_owned(),
                role: "mod".to_owned(),
                is_banned: false,
                public_key: None,
                joined_at: None,
                last_seen: None,
            },
        )
        .expect("add moderator membership for role hierarchy");
    let member_role = super::route_http_request(
        "GET",
        &format!("/api/v0/podcore/verification/role/{pod_id}/tester/member"),
        None,
        "",
        &state,
    )
    .await
    .expect("check member role");
    record!(
        "GET",
        "/api/v0/podcore/verification/role/{podId}/{peerId}/{requiredRole}",
        "nominal-status-headers-body",
        member_role.status == "200 OK" && member_role.body == r#"{"hasRole":true}"#
    );
    let moderator_as_member = super::route_http_request(
        "GET",
        &format!("/api/v0/podcore/verification/role/{pod_id}/moderator-peer/member"),
        None,
        "",
        &state,
    )
    .await
    .expect("check role hierarchy");
    record!(
        "GET",
        "/api/v0/podcore/verification/role/{podId}/{peerId}/{requiredRole}",
        "populated-dynamic-state",
        moderator_as_member.status == "200 OK" && moderator_as_member.body == r#"{"hasRole":true}"#
    );
    let missing_role = super::route_http_request(
        "GET",
        &format!("/api/v0/podcore/verification/role/{pod_id}/missing-peer/member"),
        None,
        "",
        &state,
    )
    .await
    .expect("check missing role membership");
    record!(
        "GET",
        "/api/v0/podcore/verification/role/{podId}/{peerId}/{requiredRole}",
        "missing-empty-or-conflict-state",
        missing_role.status == "200 OK" && missing_role.body == r#"{"hasRole":false}"#
    );
    let malformed_role = super::route_http_request(
        "GET",
        &format!(
            "/api/v0/podcore/verification/role/{pod_id}/tester/{}",
            "member".repeat(22)
        ),
        None,
        "",
        &state,
    )
    .await
    .expect("reject oversized role verification route value");
    record!(
        "GET",
        "/api/v0/podcore/verification/role/{podId}/{peerId}/{requiredRole}",
        "malformed-path-query-or-body",
        malformed_role.status == "400 Bad Request"
    );

    let message = serde_json::json!({
        "messageId": "message-differential-1",
        "podId": pod_id,
        "channelId": format!("{pod_id}:general"),
        "senderPeerId": "tester",
        "body": "hello",
        "timestampUnixMs": super::unix_timestamp() * 1000,
    });
    let signed = super::route_http_request(
        "POST",
        "/api/v0/podcore/signing/sign",
        None,
        &serde_json::json!({"privateKey": keys["privateKey"], "message": message}).to_string(),
        &state,
    )
    .await
    .expect("sign message");
    let signed_json = serde_json::from_str::<serde_json::Value>(&signed.body).unwrap_or_default();
    let mut verified_message = message.clone();
    verified_message["signature"] = signed_json["signature"].clone();

    let verified = super::route_http_request(
        "POST",
        "/api/v0/podcore/verification/message",
        None,
        &verified_message.to_string(),
        &state,
    )
    .await
    .expect("verify message");
    let verified_json =
        serde_json::from_str::<serde_json::Value>(&verified.body).unwrap_or_default();
    let verified_pass = verified.status == "200 OK"
        && verified_json
            == serde_json::json!({
                "isValid": true,
                "isFromValidMember": true,
                "hasValidSignature": true,
                "isNotBanned": true,
            });

    let mut unknown_sender = verified_message.clone();
    unknown_sender["senderPeerId"] = serde_json::json!("differential-stranger");
    let unknown_verified = super::route_http_request(
        "POST",
        "/api/v0/podcore/verification/message",
        None,
        &unknown_sender.to_string(),
        &state,
    )
    .await
    .expect("verify unknown sender");
    let unknown_json =
        serde_json::from_str::<serde_json::Value>(&unknown_verified.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/podcore/verification/message",
        "missing-empty-or-conflict-state",
        verified_pass
            && unknown_json["isValid"] == false
            && unknown_json["isFromValidMember"] == false
            && unknown_json["isNotBanned"] == true
    );

    let mut bad_channel = verified_message.clone();
    bad_channel["channelId"] = serde_json::json!("no-colon-here-differential");
    let bad_channel_verified = super::route_http_request(
        "POST",
        "/api/v0/podcore/verification/message",
        None,
        &bad_channel.to_string(),
        &state,
    )
    .await
    .expect("verify bad channel id");
    let bad_channel_pass = bad_channel_verified.body
        == serde_json::json!({
            "isValid": false,
            "isFromValidMember": false,
            "hasValidSignature": false,
            "isNotBanned": false,
            "errorMessage": "Invalid channel ID format",
        });

    let missing_pod_id = super::route_http_request(
        "POST",
        "/api/v0/podcore/verification/message",
        None,
        r#"{"messageId":"message-differential-1"}"#,
        &state,
    )
    .await
    .expect("verify missing podId");
    record!(
        "POST",
        "/api/v0/podcore/verification/message",
        "malformed-path-query-or-body",
        bad_channel_pass && missing_pod_id.status == "400 Bad Request"
    );

    let stats = super::route_http_request(
        "GET",
        "/api/v0/podcore/verification/stats",
        None,
        "",
        &state,
    )
    .await
    .expect("verification stats");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/podcore/verification/stats",
        "nominal-status-headers-body",
        stats.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/podcore/verification/stats",
        "populated-dynamic-state",
        stats_json["totalVerifications"] == 2
            && stats_json["successfulVerifications"] == 1
            && stats_json["failedMembershipChecks"] == 1
            && stats_json["lastVerification"].is_string()
    );
    let malformed_verification_stats = super::route_http_request(
        "GET",
        "/api/v0/podcore/verification/stats?unexpected=not-a-number",
        None,
        "",
        &state,
    )
    .await
    .expect("malformed verification stats query");
    let malformed_verification_stats_json =
        serde_json::from_str::<serde_json::Value>(&malformed_verification_stats.body)
            .unwrap_or_default();
    record!(
        "GET",
        "/api/v0/podcore/verification/stats",
        "malformed-path-query-or-body",
        malformed_verification_stats.status == "200 OK"
            && malformed_verification_stats_json["totalVerifications"] == 2
    );

    // -- podcore signing/stats --
    let (signing_state, _signing_receiver) = test_state();
    let signing_baseline = super::route_http_request(
        "GET",
        "/api/v0/podcore/signing/stats",
        None,
        "",
        &signing_state,
    )
    .await
    .expect("baseline signing stats");
    let signing_baseline_json =
        serde_json::from_str::<serde_json::Value>(&signing_baseline.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/podcore/signing/stats",
        "missing-empty-or-conflict-state",
        signing_baseline.status == "200 OK"
            && signing_baseline_json["totalSignaturesCreated"] == 0
            && signing_baseline_json["totalSignaturesVerified"] == 0
            && signing_baseline_json["lastSignatureOperation"] == super::PODCORE_MIN_DATETIME
    );

    let signing_pod_id = "signing-stats-differential-pod";
    signing_state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<super::pods::PodRecord>(serde_json::json!({
                "podId": signing_pod_id,
                "name": "Signing Stats Differential",
            }))
            .expect("deserialize pod record fixture"),
            "owner-peer".to_owned(),
        )
        .expect("create pod");
    let signing_keypair = super::route_http_request(
        "POST",
        "/api/v0/podcore/signing/generate-keypair",
        None,
        "{}",
        &signing_state,
    )
    .await
    .expect("generate keypair");
    let signing_keys =
        serde_json::from_str::<serde_json::Value>(&signing_keypair.body).unwrap_or_default();
    signing_state
        .pods
        .write()
        .await
        .upsert_member(
            signing_pod_id,
            super::pods::PodMember {
                peer_id: "tester".to_owned(),
                role: "member".to_owned(),
                is_banned: false,
                public_key: signing_keys["publicKey"].as_str().map(str::to_owned),
                joined_at: None,
                last_seen: None,
            },
        )
        .expect("add tester as a real pod member with a signing public key");
    let signing_signed = super::route_http_request(
        "POST",
        "/api/v0/podcore/signing/sign",
        None,
        &serde_json::json!({
            "privateKey": signing_keys["privateKey"],
            "message": {
                "messageId": "message-differential-2",
                "podId": signing_pod_id,
                "senderPeerId": "tester",
                "body": "hello",
                "timestampUnixMs": super::unix_timestamp() * 1000,
            }
        })
        .to_string(),
        &signing_state,
    )
    .await
    .expect("sign message");
    let signing_verified = super::route_http_request(
        "POST",
        "/api/v0/podcore/signing/verify",
        None,
        &signing_signed.body,
        &signing_state,
    )
    .await
    .expect("verify message");
    let signing_signed_json =
        serde_json::from_str::<serde_json::Value>(&signing_signed.body).unwrap_or_default();
    let mut forged = signing_signed_json.clone();
    forged["message"]["senderPeerId"] = serde_json::json!("someone-else-differential");
    let forged_verified = super::route_http_request(
        "POST",
        "/api/v0/podcore/signing/verify",
        None,
        &forged.to_string(),
        &signing_state,
    )
    .await
    .expect("verify forged sender");

    let signing_stats = super::route_http_request(
        "GET",
        "/api/v0/podcore/signing/stats",
        None,
        "",
        &signing_state,
    )
    .await
    .expect("signing stats");
    let signing_stats_json =
        serde_json::from_str::<serde_json::Value>(&signing_stats.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/podcore/signing/stats",
        "nominal-status-headers-body",
        signing_verified.body == r#"{"isValid":true}"#
            && forged_verified.body == r#"{"isValid":false}"#
    );
    record!(
        "GET",
        "/api/v0/podcore/signing/stats",
        "populated-dynamic-state",
        signing_stats_json["totalSignaturesCreated"] == 1
            && signing_stats_json["totalSignaturesVerified"] == 2
            && signing_stats_json["successfulVerifications"] == 1
            && signing_stats_json["failedVerifications"] == 1
            && signing_stats_json["lastSignatureOperation"].is_string()
    );
    let malformed_signing_stats = super::route_http_request(
        "GET",
        "/api/v0/podcore/signing/stats?unexpected=not-a-number",
        None,
        "",
        &signing_state,
    )
    .await
    .expect("malformed signing stats query");
    let malformed_signing_stats_json =
        serde_json::from_str::<serde_json::Value>(&malformed_signing_stats.body)
            .unwrap_or_default();
    record!(
        "GET",
        "/api/v0/podcore/signing/stats",
        "malformed-path-query-or-body",
        malformed_signing_stats.status == "200 OK"
            && malformed_signing_stats_json["totalSignaturesCreated"] == 1
            && malformed_signing_stats_json["totalSignaturesVerified"] == 2
    );

    // -- quarantine-jury/audit --
    let (jury_state, _jury_receiver) = test_state();
    let created_a = super::route_http_request(
        "POST",
        "/api/v0/quarantine-jury/requests",
        None,
        r#"{"localReason":"audit-differential-a","jurors":["juror-a","juror-b"],"evidence":[{"type":"hash","reference":"opaque-ref-a"}],"minJurorVotes":2}"#,
        &jury_state,
    )
    .await
    .expect("create request a");
    let request_a = serde_json::from_str::<serde_json::Value>(&created_a.body).unwrap_or_default()
        ["request"]["requestId"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    for juror in ["juror-a", "juror-b"] {
        let verdict = super::route_http_request(
            "POST",
            "/api/v0/quarantine-jury/verdicts",
            None,
            &quarantine_signed_verdict_json(&request_a, juror, "ReleaseCandidate").to_string(),
            &jury_state,
        )
        .await
        .expect("cast verdict");
        assert_eq!(verdict.status, "200 OK", "{}", verdict.body);
    }

    let created_b = super::route_http_request(
        "POST",
        "/api/v0/quarantine-jury/requests",
        None,
        r#"{"localReason":"audit-differential-b","jurors":["juror-c"],"evidence":[{"type":"hash","reference":"opaque-ref-b"}],"minJurorVotes":1}"#,
        &jury_state,
    )
    .await
    .expect("create request b");
    let request_b = serde_json::from_str::<serde_json::Value>(&created_b.body).unwrap_or_default()
        ["request"]["requestId"]
        .as_str()
        .unwrap_or_default()
        .to_owned();

    let baseline = super::route_http_request(
        "GET",
        "/api/v0/quarantine-jury/audit",
        None,
        "",
        &jury_state,
    )
    .await
    .expect("audit report before acceptance");
    let baseline_json =
        serde_json::from_str::<serde_json::Value>(&baseline.body).unwrap_or_default();
    let entries = baseline_json["entries"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let entry_a = entries
        .iter()
        .find(|entry| entry["requestId"] == request_a)
        .cloned()
        .unwrap_or_default();
    let entry_b = entries
        .iter()
        .find(|entry| entry["requestId"] == request_b)
        .cloned()
        .unwrap_or_default();
    record!(
        "GET",
        "/api/v0/quarantine-jury/audit",
        "nominal-status-headers-body",
        baseline.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/quarantine-jury/audit",
        "populated-dynamic-state",
        baseline_json["requestCount"] == 2
            && baseline_json["pendingReleaseCandidateCount"] == 1
            && baseline_json["pendingManualReviewCount"] == 1
            && baseline_json["acceptedReleaseCandidateCount"] == 0
            && baseline_json["upholdQuarantineCount"] == 0
            && entry_a["status"] == "pending-release-acceptance"
            && entry_a["verdictCount"] == 2
            && entry_a["quorumReached"] == true
            && entry_a["canAcceptReleaseCandidate"] == true
            && entry_b["status"] == "manual-review"
            && entry_b["verdictCount"] == 0
            && entry_b["quorumReached"] == false
    );

    let accept = super::route_http_request(
        "POST",
        &format!("/api/v0/quarantine-jury/requests/{request_a}/accept-release-candidate"),
        None,
        "{}",
        &jury_state,
    )
    .await
    .expect("accept request a");
    assert_eq!(accept.status, "200 OK", "{}", accept.body);

    let after_accept = super::route_http_request(
        "GET",
        "/api/v0/quarantine-jury/audit",
        None,
        "",
        &jury_state,
    )
    .await
    .expect("audit report after acceptance");
    let after_accept_json =
        serde_json::from_str::<serde_json::Value>(&after_accept.body).unwrap_or_default();

    {
        let key = format!("quarantine/request/{request_b}");
        let mut features = jury_state.controller_features.write_for_test().await;
        let mut backdated = features.get(&key).cloned().expect("request b exists");
        backdated["createdAt"] =
            serde_json::json!(super::unix_timestamp().saturating_sub(100 * 3600));
        features.upsert(key, backdated).expect("backdate request b");
    }
    let stale = super::route_http_request(
        "GET",
        "/api/v0/quarantine-jury/audit",
        None,
        "",
        &jury_state,
    )
    .await
    .expect("audit report after backdating request b");
    let stale_json = serde_json::from_str::<serde_json::Value>(&stale.body).unwrap_or_default();
    let stale_entries = stale_json["entries"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let stale_entry_a = stale_entries
        .iter()
        .find(|entry| entry["requestId"] == request_a)
        .cloned()
        .unwrap_or_default();
    let stale_entry_b = stale_entries
        .iter()
        .find(|entry| entry["requestId"] == request_b)
        .cloned()
        .unwrap_or_default();
    record!(
        "GET",
        "/api/v0/quarantine-jury/audit",
        "mutation-side-effects-and-readback",
        after_accept_json["acceptedReleaseCandidateCount"] == 1
            && after_accept_json["pendingReleaseCandidateCount"] == 0
            && stale_json["staleRequestCount"] == 1
            && stale_entry_b["isStale"] == true
            && stale_entry_a["isStale"] == false
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("pod_and_jury_stats.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api pod-and-jury-stats mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
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
async fn controller_api_differential_playback_feedback_and_diagnostics() {
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

    let missing = super::route_http_request(
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

    let invalid_json = super::route_http_request(
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

    let missing_job_id = super::route_http_request(
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

    let low_buffer = super::route_http_request(
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

    let after_low = super::route_http_request(
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
    let comfortable = super::route_http_request(
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
    let readback = super::route_http_request(
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
    let mid = super::route_http_request(
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
    let final_readback = super::route_http_request(
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
    let reset_diagnostics = super::route_http_request(
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
    let playback_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("playback runtime-failure database");
    let (playback_runtime_state, _runtime_receiver) = test_state_with_env_parts(
        MapEnv::default(),
        super::SearchStore::new(),
        Some(playback_db.clone()),
    );
    playback_db.close_for_test().await;
    let runtime_feedback = super::route_http_request(
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
    let runtime_diagnostics = super::route_http_request(
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
async fn controller_api_differential_nowplaying_delete_and_playback_diagnostics_edge_states() {
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
        let feedback = super::route_http_request(
            "POST",
            "/api/v0/playback/feedback",
            None,
            r#"{"jobId":"diagnostics-nominal-job","positionMs":0,"bufferAheadMs":0}"#,
            &state,
        )
        .await
        .expect("nominal diagnostics feedback");
        let diagnostics = super::route_http_request(
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
            super::route_http_request("GET", "/api/v0/playback/%20/diagnostics", None, "", &state)
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
        let deleted = super::route_http_request("DELETE", "/api/v0/nowplaying", None, "", &state)
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
        let seeded = super::route_http_request(
            "PUT",
            "/api/v0/nowplaying",
            None,
            r#"{"artist":"Restart Artist","title":"Restart Track"}"#,
            &state,
        )
        .await
        .expect("seed now-playing track");
        let deleted = super::route_http_request("DELETE", "/api/v0/nowplaying", None, "", &state)
            .await
            .expect("restart reset delete response");
        let (restarted_state, _restarted_receiver) = test_state();
        let current =
            super::route_http_request("GET", "/api/v0/nowplaying", None, "", &restarted_state)
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
        let seeded = super::route_http_request(
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
                super::route_http_request("DELETE", "/api/v0/nowplaying", None, "", &state)
            }))
            .await;
        let current = super::route_http_request("GET", "/api/v0/nowplaying", None, "", &state)
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
            super::route_http_request("PUT", "/api/v0/nowplaying", None, body, &state)
        }))
        .await;
        let current = super::route_http_request("GET", "/api/v0/nowplaying", None, "", &state)
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
        let seeded = super::route_http_request(
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
            super::route_http_request("GET", "/api/v0/nowplaying", None, "", &restarted_state)
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

/// Differential evidence for the slskdN NowPlayingController webhook and
/// its remaining malformed/runtime cases.  The frozen webhook accepts
/// generic, Plex, and Jellyfin JSON, clears on stop/pause notifications,
/// rejects empty or invalid payloads, and remains process-local rather
/// than depending on SQLite.
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
async fn controller_api_differential_native_nowplaying_webhook_contracts() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
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
                "pass": pass,
            }));
        }};
    }

    let env = || MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target);
    let (state, _receiver) = test_state_with_env(env());

    let malformed_get =
        super::route_http_request("GET", "/api/v0/nowplaying/extra", None, "", &state)
            .await
            .expect("malformed now-playing GET");
    record!(
        "GET",
        "/api/v0/nowplaying",
        "malformed-path-query-or-body",
        malformed_get.status == "404 Not Found"
    );

    let runtime_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("now-playing runtime database");
    let (runtime_state, _receiver) =
        test_state_with_env_parts(env(), super::SearchStore::new(), Some(runtime_db.clone()));
    runtime_db.close_for_test().await;
    let runtime_get =
        super::route_http_request("GET", "/api/v0/nowplaying", None, "", &runtime_state)
            .await
            .expect("now-playing GET after database close");
    record!(
        "GET",
        "/api/v0/nowplaying",
        "runtime-failure-and-timeout",
        runtime_get.status == "204 No Content"
    );

    let malformed_delete =
        super::route_http_request("DELETE", "/api/v0/nowplaying/extra", None, "", &state)
            .await
            .expect("malformed now-playing DELETE");
    record!(
        "DELETE",
        "/api/v0/nowplaying",
        "malformed-path-query-or-body",
        malformed_delete.status == "404 Not Found"
    );

    let malformed_put = super::route_http_request("PUT", "/api/v0/nowplaying", None, "{}", &state)
        .await
        .expect("malformed now-playing PUT");
    record!(
        "PUT",
        "/api/v0/nowplaying",
        "malformed-path-query-or-body",
        malformed_put.status == "400 Bad Request"
            && malformed_put.body.contains("Artist and title are required")
    );

    let missing_put = super::route_http_request("PUT", "/api/v0/nowplaying", None, "", &state)
        .await
        .expect("missing now-playing PUT");
    record!(
        "PUT",
        "/api/v0/nowplaying",
        "missing-empty-or-conflict-state",
        missing_put.status == "400 Bad Request"
            && missing_put.body.contains("Track data is required")
    );

    let plex = super::route_http_request(
        "POST",
        "/api/v0/nowplaying/webhook",
        None,
        r#"{"event":"media.play","Metadata":{"grandparentTitle":"Plex Artist","title":"Plex Track","parentTitle":"Plex Album"}}"#,
        &state,
    )
    .await
    .expect("nominal Plex now-playing webhook");
    let plex_current = super::route_http_request("GET", "/api/v0/nowplaying", None, "", &state)
        .await
        .expect("Plex now-playing readback");
    let plex_json =
        serde_json::from_str::<serde_json::Value>(&plex_current.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/nowplaying/webhook",
        "nominal-status-headers-body",
        plex.status == "200 OK"
            && plex.body.is_empty()
            && plex.content_type.starts_with("application/json")
            && plex_json["artist"] == "Plex Artist"
            && plex_json["title"] == "Plex Track"
            && plex_json["album"].is_null()
    );

    let generic = super::route_http_request(
        "POST",
        "/api/v0/nowplaying/webhook",
        None,
        r#"{"artist":"Generic Artist","title":"Generic Track","event":"play"}"#,
        &state,
    )
    .await
    .expect("generic now-playing webhook");
    let generic_current = super::route_http_request("GET", "/api/v0/nowplaying", None, "", &state)
        .await
        .expect("generic now-playing readback");
    let generic_json =
        serde_json::from_str::<serde_json::Value>(&generic_current.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/nowplaying/webhook",
        "mutation-side-effects-and-readback",
        generic.status == "200 OK"
            && generic_current.status == "200 OK"
            && generic_json["artist"] == "Generic Artist"
            && generic_json["title"] == "Generic Track"
    );

    let jellyfin = super::route_http_request(
        "POST",
        "/api/v0/nowplaying/webhook",
        None,
        r#"{"NotificationType":"PlaybackStart","Artist":"Jelly Artist","Name":"Jelly Track","Album":"Jelly Album"}"#,
        &state,
    )
    .await
    .expect("Jellyfin now-playing webhook");
    let jellyfin_stop = super::route_http_request(
        "POST",
        "/api/v0/nowplaying/webhook",
        None,
        r#"{"NotificationType":"PlaybackStop"}"#,
        &state,
    )
    .await
    .expect("Jellyfin stop webhook");
    assert_eq!(jellyfin.status, "200 OK");
    assert_eq!(jellyfin_stop.status, "200 OK");
    assert!(state.now_playing.read().await.records.is_empty());

    let malformed_webhook = super::route_http_request(
        "POST",
        "/api/v0/nowplaying/webhook",
        None,
        "not-json",
        &state,
    )
    .await
    .expect("malformed now-playing webhook");
    record!(
        "POST",
        "/api/v0/nowplaying/webhook",
        "malformed-path-query-or-body",
        malformed_webhook.status == "400 Bad Request"
            && malformed_webhook.body.contains("Invalid JSON payload")
    );

    let missing_webhook =
        super::route_http_request("POST", "/api/v0/nowplaying/webhook", None, "", &state)
            .await
            .expect("missing now-playing webhook");
    record!(
        "POST",
        "/api/v0/nowplaying/webhook",
        "missing-empty-or-conflict-state",
        missing_webhook.status == "400 Bad Request"
            && missing_webhook.body.contains("Empty payload")
    );

    let seeded = super::route_http_request(
        "POST",
        "/api/v0/nowplaying/webhook",
        None,
        r#"{"artist":"Transient Artist","title":"Transient Track"}"#,
        &state,
    )
    .await
    .expect("seed transient webhook track");
    let (restarted_state, _receiver) = test_state_with_env(env());
    let restarted =
        super::route_http_request("GET", "/api/v0/nowplaying", None, "", &restarted_state)
            .await
            .expect("now-playing webhook after restart");
    record!(
        "POST",
        "/api/v0/nowplaying/webhook",
        "restart-persistence-or-reset",
        seeded.status == "200 OK" && restarted.status == "204 No Content"
    );

    let concurrent_responses = futures_util::future::join_all([
        super::route_http_request(
            "POST",
            "/api/v0/nowplaying/webhook",
            None,
            r#"{"artist":"Concurrent A","title":"Track A"}"#,
            &state,
        ),
        super::route_http_request(
            "POST",
            "/api/v0/nowplaying/webhook",
            None,
            r#"{"artist":"Concurrent B","title":"Track B"}"#,
            &state,
        ),
    ])
    .await;
    let concurrent_current =
        super::route_http_request("GET", "/api/v0/nowplaying", None, "", &state)
            .await
            .expect("concurrent now-playing webhook readback");
    let concurrent_json =
        serde_json::from_str::<serde_json::Value>(&concurrent_current.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/nowplaying/webhook",
        "concurrency-and-idempotency",
        concurrent_responses.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
        }) && concurrent_current.status == "200 OK"
            && ["Concurrent A", "Concurrent B"]
                .contains(&concurrent_json["artist"].as_str().unwrap_or_default())
    );

    let failure_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("now-playing webhook failure database");
    let (failure_state, _receiver) =
        test_state_with_env_parts(env(), super::SearchStore::new(), Some(failure_db.clone()));
    failure_db.close_for_test().await;
    let failure_webhook = super::route_http_request(
        "POST",
        "/api/v0/nowplaying/webhook",
        None,
        r#"{"artist":"Failure Artist","title":"Failure Track"}"#,
        &failure_state,
    )
    .await
    .expect("now-playing webhook after database close");
    record!(
        "POST",
        "/api/v0/nowplaying/webhook",
        "runtime-failure-and-timeout",
        failure_webhook.status == "200 OK"
            && failure_state.now_playing.read().await.records.len() == 1
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("native_nowplaying_webhook_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskdn now-playing webhook ledger"),
    )
    .expect("write slskdn now-playing webhook ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskdn now-playing webhook mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
