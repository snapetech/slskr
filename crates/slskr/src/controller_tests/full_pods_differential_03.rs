//! Controller full pods differential 03 ownership.

use super::*;

/// Bulk differential proof crediting the empty-state opinion actions.
/// The frozen services return successful zero-count result records when
/// there is no cached opinion or membership state for a valid route.
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
    feature = "bounded-controller-api-tests-2"
))]
pub(super) async fn controller_api_differential_podcore_opinion_actions() {
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

    let (state, _receiver) = test_state();
    let pod_id = "pod:opinion-actions-empty";

    let refresh = crate::route_http_request(
        "POST",
        &format!("/api/v0/podcore/{pod_id}/opinions/refresh"),
        None,
        "",
        &state,
    )
    .await
    .expect("empty opinion refresh");
    let refresh_json = serde_json::from_str::<serde_json::Value>(&refresh.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/podcore/{podId}/opinions/refresh",
        "nominal-status-headers-body",
        refresh.status == "200 OK"
            && refresh.content_type == "application/json"
            && refresh_json["success"] == true
            && refresh_json["podId"] == pod_id
            && refresh_json["opinionsRefreshed"] == 0
            && refresh_json["newOpinions"] == 0
            && refresh_json["duration"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
    );

    let update = crate::route_http_request(
        "POST",
        &format!("/api/v0/podcore/{pod_id}/opinions/members/affinity/update"),
        None,
        "",
        &state,
    )
    .await
    .expect("empty affinity update");
    let update_json = serde_json::from_str::<serde_json::Value>(&update.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/podcore/{podId}/opinions/members/affinity/update",
        "nominal-status-headers-body",
        update.status == "200 OK"
            && update.content_type == "application/json"
            && update_json["success"] == true
            && update_json["podId"] == pod_id
            && update_json["membersUpdated"] == 0
            && update_json["duration"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
    );

    let (populated_state, _receiver) = test_state();
    let populated_pod_id = "pod:opinion-actions-populated";
    populated_state
        .controller_features
        .write_for_test()
        .await
        .upsert(
            format!("pod/opinion/{populated_pod_id}/content-actions/opinion-refresh"),
            serde_json::json!({
                "id": "opinion-refresh",
                "podId": populated_pod_id,
                "contentId": "content-actions",
                "variantHash": "variant-actions",
                "score": 0.7,
                "senderPeerId": "affinity-member",
                "signature": "action-differential-signature",
            }),
        )
        .expect("seed populated opinion action");
    let refreshed = crate::route_http_request(
        "POST",
        &format!("/api/v0/podcore/{populated_pod_id}/opinions/refresh"),
        None,
        "",
        &populated_state,
    )
    .await
    .expect("populated opinion refresh");
    let refreshed_json =
        serde_json::from_str::<serde_json::Value>(&refreshed.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/podcore/{podId}/opinions/refresh",
        "mutation-side-effects-and-readback",
        refreshed.status == "200 OK"
            && refreshed.content_type == "application/json"
            && refreshed_json["success"] == true
            && refreshed_json["podId"] == populated_pod_id
            && refreshed_json["opinionsRefreshed"] == 1
            && refreshed_json["newOpinions"] == 1
            && refreshed_json["duration"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
    );

    populated_state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<crate::pods::PodRecord>(serde_json::json!({
                "podId": populated_pod_id,
                "name": "Opinion action differential",
                "isPublic": true,
            }))
            .expect("deserialize populated opinion action pod"),
            "affinity-member".to_owned(),
        )
        .expect("create populated opinion action pod");
    let populated_update = crate::route_http_request(
        "POST",
        &format!("/api/v0/podcore/{populated_pod_id}/opinions/members/affinity/update"),
        None,
        "",
        &populated_state,
    )
    .await
    .expect("populated affinity update");
    let populated_update_json =
        serde_json::from_str::<serde_json::Value>(&populated_update.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/podcore/{podId}/opinions/members/affinity/update",
        "mutation-side-effects-and-readback",
        populated_update.status == "200 OK"
            && populated_update.content_type == "application/json"
            && populated_update_json["success"] == true
            && populated_update_json["podId"] == populated_pod_id
            && populated_update_json["membersUpdated"] == 1
            && populated_update_json["duration"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("podcore_opinion_actions.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api podcore-opinion action mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting PodCore opinion routes when the pod
/// has no cached opinion or membership state. The frozen services expose
/// successful zero/empty DTOs for these valid routes rather than a 404.
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
    feature = "bounded-controller-api-tests-2"
))]
pub(super) async fn controller_api_differential_podcore_opinion_missing_gets_and_actions() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($method:expr, $route:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!(
                    "{target} {} {} [missing-empty-or-conflict-state]",
                    $method,
                    $route
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": $method,
                "route": $route,
                "case": "missing-empty-or-conflict-state",
                "pass": $pass,
            }));
        };
    }

    let (state, _receiver) = test_state();
    let pod_id = "pod:opinion-missing-differential";
    let content_id = "content-missing-differential";

    let opinions = crate::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/opinions/content/{content_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("missing-state content opinions");
    let opinions_json =
        serde_json::from_str::<serde_json::Value>(&opinions.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/podcore/{podId}/opinions/content/{contentId}",
        opinions.status == "200 OK" && opinions_json == serde_json::json!([])
    );

    let aggregated = crate::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/opinions/content/{content_id}/aggregated"),
        None,
        "",
        &state,
    )
    .await
    .expect("missing-state aggregated opinions");
    let aggregated_json =
        serde_json::from_str::<serde_json::Value>(&aggregated.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/podcore/{podId}/opinions/content/{contentId}/aggregated",
        aggregated.status == "200 OK"
            && aggregated_json["podId"] == pod_id
            && aggregated_json["contentId"] == content_id
            && aggregated_json["totalOpinions"] == 0
            && aggregated_json["variantAggregates"]
                .as_array()
                .is_some_and(Vec::is_empty)
            && aggregated_json["memberContributions"]
                .as_object()
                .is_some_and(serde_json::Map::is_empty)
    );

    let recommendations = crate::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/opinions/content/{content_id}/recommendations"),
        None,
        "",
        &state,
    )
    .await
    .expect("missing-state opinion recommendations");
    let recommendations_json =
        serde_json::from_str::<serde_json::Value>(&recommendations.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/podcore/{podId}/opinions/content/{contentId}/recommendations",
        recommendations.status == "200 OK" && recommendations_json == serde_json::json!([])
    );

    let stats = crate::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/opinions/content/{content_id}/stats"),
        None,
        "",
        &state,
    )
    .await
    .expect("missing-state opinion statistics");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/podcore/{podId}/opinions/content/{contentId}/stats",
        stats.status == "200 OK"
            && stats_json["podId"] == pod_id
            && stats_json["contentId"] == content_id
            && stats_json["totalOpinions"] == 0
            && stats_json["uniqueVariants"] == 0
            && stats_json["scoreDistribution"]
                .as_object()
                .is_some_and(serde_json::Map::is_empty)
    );

    let variant = crate::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/opinions/content/{content_id}/variant/variant-missing"),
        None,
        "",
        &state,
    )
    .await
    .expect("missing-state variant opinions");
    let variant_json = serde_json::from_str::<serde_json::Value>(&variant.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/podcore/{podId}/opinions/content/{contentId}/variant/{variantHash}",
        variant.status == "200 OK" && variant_json == serde_json::json!([])
    );

    let refresh = crate::route_http_request(
        "POST",
        &format!("/api/v0/podcore/{pod_id}/opinions/refresh"),
        None,
        "",
        &state,
    )
    .await
    .expect("missing-state opinion refresh");
    let refresh_json = serde_json::from_str::<serde_json::Value>(&refresh.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/podcore/{podId}/opinions/refresh",
        refresh.status == "200 OK"
            && refresh_json["success"] == true
            && refresh_json["podId"] == pod_id
            && refresh_json["opinionsRefreshed"] == 0
            && refresh_json["newOpinions"] == 0
    );

    let update = crate::route_http_request(
        "POST",
        &format!("/api/v0/podcore/{pod_id}/opinions/members/affinity/update"),
        None,
        "",
        &state,
    )
    .await
    .expect("missing-state affinity update");
    let update_json = serde_json::from_str::<serde_json::Value>(&update.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/podcore/{podId}/opinions/members/affinity/update",
        update.status == "200 OK"
            && update_json["success"] == true
            && update_json["podId"] == pod_id
            && update_json["membersUpdated"] == 0
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("podcore_opinion_missing_gets_and_actions.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api podcore-opinion missing-state mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential proof that PodCore message-signature verification updates
/// its observable counters while preserving the oracle's boolean result
/// DTO for a legacy signature in the default non-enforced mode. slskdN-only
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
    feature = "bounded-controller-api-tests-2"
))]
pub(super) async fn controller_api_differential_podcore_signing_verify() {
    let target = "slskdn";
    let route = "/api/v0/podcore/signing/verify";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($pass:expr) => {
            if !$pass {
                mismatches.push(format!(
                    "{target} POST {route} [mutation-side-effects-and-readback]"
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "POST",
                "route": route,
                "case": "mutation-side-effects-and-readback",
                "pass": $pass,
            }));
        };
    }

    let (state, _receiver) = test_state();
    let stats_before =
        crate::route_http_request("GET", "/api/v0/podcore/signing/stats", None, "", &state)
            .await
            .expect("read signing stats before verification");
    let stats_before_json =
        serde_json::from_str::<serde_json::Value>(&stats_before.body).unwrap_or_default();
    let verified = crate::route_http_request(
        "POST",
        route,
        None,
        r#"{"messageId":"signing-differential","podId":"pod:signing-differential","channelId":"general","senderPeerId":"signing-peer","body":"signing differential","timestampUnixMs":1,"signature":"legacy","sigVersion":1}"#,
        &state,
    )
    .await
    .expect("verify signing differential message");
    let verified_json =
        serde_json::from_str::<serde_json::Value>(&verified.body).unwrap_or_default();
    let stats_after =
        crate::route_http_request("GET", "/api/v0/podcore/signing/stats", None, "", &state)
            .await
            .expect("read signing stats after verification");
    let stats_after_json =
        serde_json::from_str::<serde_json::Value>(&stats_after.body).unwrap_or_default();
    let before_count = stats_before_json["totalSignaturesVerified"]
        .as_u64()
        .unwrap_or(0);
    let after_count = stats_after_json["totalSignaturesVerified"]
        .as_u64()
        .unwrap_or(0);
    let before_successes = stats_before_json["successfulVerifications"]
        .as_u64()
        .unwrap_or(0);
    let after_successes = stats_after_json["successfulVerifications"]
        .as_u64()
        .unwrap_or(0);
    record!(
        verified.status == "200 OK"
            && verified.content_type == "application/json"
            && verified_json == serde_json::json!({"isValid": true})
            && stats_after.status == "200 OK"
            && after_count == before_count + 1
            && after_successes == before_successes + 1
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("podcore_signing_verify.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api podcore-signing mismatches:\n{}",
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
    feature = "bounded-controller-api-tests-2"
))]
pub(super) async fn controller_api_differential_opinion_open_cases() {
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

    let malformed_list = crate::route_http_request(
        "GET",
        "/api/v0/opinions?limit=not-a-number",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    record!(
        "GET",
        "/api/v0/opinions",
        "malformed-path-query-or-body",
        malformed_list.status == "400 Bad Request"
    );

    let empty_list = crate::route_http_request("GET", "/api/v0/opinions", None, "", &state)
        .await
        .unwrap();
    let empty_list_json =
        serde_json::from_str::<serde_json::Value>(&empty_list.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/opinions",
        "missing-empty-or-conflict-state",
        empty_list.status == "200 OK" && empty_list_json == serde_json::json!([])
    );

    let summary = crate::route_http_request(
        "GET",
        "/api/v0/opinions/summary?subjectType=Track&subjectId=empty-track",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    let summary_json = serde_json::from_str::<serde_json::Value>(&summary.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/opinions/summary",
        "nominal-status-headers-body",
        summary.status == "200 OK"
            && summary_json["subjectType"] == "Track"
            && summary_json["subjectId"] == "empty-track"
    );

    let missing_summary =
        crate::route_http_request("GET", "/api/v0/opinions/summary", None, "", &state)
            .await
            .unwrap();
    record!(
        "GET",
        "/api/v0/opinions/summary",
        "missing-empty-or-conflict-state",
        missing_summary.status == "400 Bad Request"
    );

    let posted = crate::route_http_request(
        "POST",
        "/api/v0/opinions",
        None,
        r#"{"id":"opinion-open-populated","issuer":"open-cases","subjectType":"Track","subjectId":"summary-track","kind":"Like","strength":0.8,"confidence":0.75}"#,
        &state,
    )
    .await
    .unwrap();
    let posted_json = serde_json::from_str::<serde_json::Value>(&posted.body).unwrap_or_default();
    let populated_summary = crate::route_http_request(
        "GET",
        "/api/v0/opinions/summary?subjectType=Track&subjectId=summary-track",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    let populated_summary_json =
        serde_json::from_str::<serde_json::Value>(&populated_summary.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/opinions/summary",
        "populated-dynamic-state",
        posted.status == "200 OK"
            && posted_json["id"] == "opinion-open-populated"
            && populated_summary.status == "200 OK"
            && populated_summary_json["total"] == 1
            && populated_summary_json["positive"] == 1
            && populated_summary_json["opinions"].as_array().map(Vec::len) == Some(1)
    );

    let summary_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("opinion summary runtime database");
    let (summary_runtime_state, _summary_runtime_receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_PERSISTENCE_ENABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", target),
        crate::SearchStore::new(),
        Some(summary_db.clone()),
    );
    summary_db.close_for_test().await;
    let summary_runtime = crate::route_http_request(
        "GET",
        "/api/v0/opinions/summary?subjectType=Track&subjectId=runtime-track",
        None,
        "",
        &summary_runtime_state,
    )
    .await
    .unwrap();
    record!(
        "GET",
        "/api/v0/opinions/summary",
        "runtime-failure-and-timeout",
        summary_runtime.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&summary_runtime.body).is_ok()
    );

    let list_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("opinion list runtime database");
    let (list_runtime_state, _list_runtime_receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_PERSISTENCE_ENABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", target),
        crate::SearchStore::new(),
        Some(list_db.clone()),
    );
    list_db.close_for_test().await;
    let list_runtime =
        crate::route_http_request("GET", "/api/v0/opinions", None, "", &list_runtime_state)
            .await
            .unwrap();
    record!(
        "GET",
        "/api/v0/opinions",
        "runtime-failure-and-timeout",
        list_runtime.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&list_runtime.body)
                .is_ok_and(|value| value.is_array())
    );

    let missing_post = crate::route_http_request("POST", "/api/v0/opinions", None, "{}", &state)
        .await
        .unwrap();
    record!(
        "POST",
        "/api/v0/opinions",
        "missing-empty-or-conflict-state",
        missing_post.status == "400 Bad Request"
    );

    let post_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("opinion post runtime database");
    let (post_runtime_state, _post_runtime_receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_PERSISTENCE_ENABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", target),
        crate::SearchStore::new(),
        Some(post_db.clone()),
    );
    post_db.close_for_test().await;
    let post_runtime = crate::route_http_request(
        "POST",
        "/api/v0/opinions",
        None,
        r#"{"id":"opinion-runtime","issuer":"runtime","subjectType":"Track","subjectId":"runtime-track","kind":"Like","strength":1,"confidence":1}"#,
        &post_runtime_state,
    )
    .await
    .unwrap();
    record!(
        "POST",
        "/api/v0/opinions",
        "runtime-failure-and-timeout",
        post_runtime.status == "200 OK"
    );

    let (post_restart_state, _post_restart_receiver) = test_state();
    let post_restart =
        crate::route_http_request("GET", "/api/v0/opinions", None, "", &post_restart_state)
            .await
            .unwrap();
    let post_restart_json =
        serde_json::from_str::<serde_json::Value>(&post_restart.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/opinions",
        "restart-persistence-or-reset",
        post_restart.status == "200 OK" && post_restart_json == serde_json::json!([])
    );

    let concurrent_bodies: Vec<String> = (0..4)
        .map(|index| {
            format!(
                r#"{{"id":"opinion-concurrent-{index}","issuer":"concurrent","subjectType":"Track","subjectId":"concurrent-track-{index}","kind":"Like","strength":1,"confidence":1}}"#
            )
        })
        .collect();
    let concurrent_posts = futures_util::future::join_all(
        concurrent_bodies
            .iter()
            .map(|body| crate::route_http_request("POST", "/api/v0/opinions", None, body, &state)),
    )
    .await;
    let concurrent_list = crate::route_http_request("GET", "/api/v0/opinions", None, "", &state)
        .await
        .unwrap();
    let concurrent_list_json =
        serde_json::from_str::<serde_json::Value>(&concurrent_list.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/opinions",
        "concurrency-and-idempotency",
        concurrent_posts.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
        }) && concurrent_list_json
            .as_array()
            .is_some_and(|opinions| opinions.len() == 5)
    );

    let malformed_delete = crate::route_http_request(
        "DELETE",
        "/api/v0/opinions/opinion-open-populated/extra",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    record!(
        "DELETE",
        "/api/v0/opinions/{id}",
        "malformed-path-query-or-body",
        malformed_delete.status == "404 Not Found"
    );

    let delete_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("opinion delete runtime database");
    let (delete_runtime_state, _delete_runtime_receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_PERSISTENCE_ENABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", target),
        crate::SearchStore::new(),
        Some(delete_db.clone()),
    );
    let delete_runtime_post = crate::route_http_request(
        "POST",
        "/api/v0/opinions",
        None,
        r#"{"id":"opinion-delete-runtime","issuer":"runtime","subjectType":"Track","subjectId":"delete-runtime","kind":"Like","strength":1,"confidence":1}"#,
        &delete_runtime_state,
    )
    .await
    .unwrap();
    delete_db.close_for_test().await;
    let delete_runtime = crate::route_http_request(
        "DELETE",
        "/api/v0/opinions/opinion-delete-runtime",
        None,
        "",
        &delete_runtime_state,
    )
    .await
    .unwrap();
    record!(
        "DELETE",
        "/api/v0/opinions/{id}",
        "runtime-failure-and-timeout",
        delete_runtime_post.status == "200 OK" && delete_runtime.status == "204 No Content"
    );

    let (delete_restart_state, _delete_restart_receiver) = test_state();
    let delete_restart = crate::route_http_request(
        "DELETE",
        "/api/v0/opinions/opinion-open-populated",
        None,
        "",
        &delete_restart_state,
    )
    .await
    .unwrap();
    record!(
        "DELETE",
        "/api/v0/opinions/{id}",
        "restart-persistence-or-reset",
        delete_restart.status == "404 Not Found"
    );

    let delete_ids: Vec<String> = (0..4)
        .map(|index| format!("opinion-delete-concurrent-{index}"))
        .collect();
    for id in &delete_ids {
        let body = format!(
            r#"{{"id":"{id}","issuer":"delete-concurrent","subjectType":"Track","subjectId":"{id}","kind":"Like","strength":1,"confidence":1}}"#
        );
        crate::route_http_request("POST", "/api/v0/opinions", None, &body, &state)
            .await
            .unwrap();
    }
    let concurrent_deletes = futures_util::future::join_all(delete_ids.iter().map(|id| {
        let path = format!("/api/v0/opinions/{id}");
        let state = Arc::clone(&state);
        async move { crate::route_http_request("DELETE", &path, None, "", &state).await }
    }))
    .await;
    record!(
        "DELETE",
        "/api/v0/opinions/{id}",
        "concurrency-and-idempotency",
        concurrent_deletes.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "204 No Content")
        })
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("opinion_open_cases.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        mismatches.is_empty(),
        "{} controller-api opinion open-case mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting 5 realm-subject-index routes'
/// cases, independently re-derived from `realm_subject_indexes_
/// persist_authority_and_compute_conflicts`'s real conflict-
/// detection (external-id, recording-subject, workref-identity,
/// alias-subject) and authority-decision-disables-conflicts checks.
/// Uses the same `compute_payload_hash`-based fixture-building
/// closure pattern as the source test (computes a real hash over its
/// own content, so index content can be freely varied per fixture
/// without breaking signature validation -- unlike the earlier
/// session bug with a copy-pasted fixed hash literal). slskdN-only
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
    feature = "bounded-controller-api-tests-2"
))]
pub(super) async fn controller_api_differential_realm_subject_indexes() {
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
    let index = |id: &str, subject: &str, title: &str, discogs: &str| {
        let mut index = serde_json::json!({
            "id": id,
            "realmId": crate::realm_subject_index::DEFAULT_REALM_ID,
            "subjectNamespace": "music",
            "revision": 1,
            "publishedAt": "2026-08-01T00:00:00Z",
            "entries": [{
                "subjectId": subject,
                "workRef": {
                    "domain": "music",
                    "title": title,
                    "creator": "Artist",
                },
                "externalIds": {
                    "musicbrainz:recording": "recording-differential",
                    "discogs": discogs,
                },
                "aliases": ["shared-alias-differential"],
            }],
            "signature": {
                "signer": crate::realm_subject_index::DEFAULT_GOVERNANCE_ROOT,
                "algorithm": "realm-governance-sha256",
                "payloadHash": "",
                "value": "signature",
            },
        });
        index["signature"]["payloadHash"] =
            serde_json::json!(crate::realm_subject_index::compute_payload_hash(&index));
        index
    };

    let merged = crate::route_http_request(
        "POST",
        "/api/v0/virtualsoulfind/shadow-index/sync/merge",
        None,
        &serde_json::json!({
            "records": [{"recordingId":"recording-differential","peerIds":["peer-a"],"updatedAt":1}],
            "realmIndexes": [
                index("index-differential-a", "subject-a", "Title A", "discogs-a"),
                index("index-differential-b", "subject-b", "Title B", "discogs-b"),
                index("index-differential-c", "subject-a", "Title C", "discogs-c"),
            ],
        })
        .to_string(),
        &state,
    )
    .await
    .expect("register realm index fixtures");
    assert_eq!(merged.status, "200 OK", "{}", merged.body);

    let indexes = crate::route_http_request(
        "GET",
        "/api/v0/realm-subject-indexes/default-realm",
        None,
        "",
        &state,
    )
    .await
    .expect("list realm indexes");
    record!(
        "GET",
        "/api/v0/realm-subject-indexes/{realmId}",
        "nominal-status-headers-body",
        indexes.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/realm-subject-indexes/{realmId}",
        "populated-dynamic-state",
        serde_json::from_str::<serde_json::Value>(&indexes.body)
            .unwrap_or_default()
            .as_array()
            .map(Vec::len)
            == Some(3)
    );

    let conflicts_route = "/api/v0/realm-subject-indexes/default-realm/conflicts";
    let conflicts = crate::route_http_request("GET", conflicts_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{conflicts_route}: {error}"));
    let conflicts_json =
        serde_json::from_str::<serde_json::Value>(&conflicts.body).unwrap_or_default();
    let conflict_types = conflicts_json["conflicts"]
        .as_array()
        .map(|entries| {
            entries
                .iter()
                .map(|entry| entry["type"].as_str().unwrap_or_default().to_owned())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    record!(
        "GET",
        "/api/v0/realm-subject-indexes/{realmId}/conflicts",
        "nominal-status-headers-body",
        conflicts.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/realm-subject-indexes/{realmId}/conflicts",
        "populated-dynamic-state",
        conflicts_json["indexCount"] == 3
            && conflicts_json["entryCount"] == 3
            && conflicts_json["hasConflicts"] == true
            && conflict_types.contains(&"external-id".to_owned())
            && conflict_types.contains(&"recording-subject".to_owned())
            && conflict_types.contains(&"workref-identity".to_owned())
            && conflict_types.contains(&"alias-subject".to_owned())
    );

    let resolutions = crate::route_http_request(
        "GET",
        "/api/v0/realm-subject-indexes/recordings/recording-differential/resolutions",
        None,
        "",
        &state,
    )
    .await
    .expect("recording resolutions");
    record!(
        "GET",
        "/api/v0/realm-subject-indexes/recordings/{recordingId}/resolutions",
        "nominal-status-headers-body",
        resolutions.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/realm-subject-indexes/recordings/{recordingId}/resolutions",
        "populated-dynamic-state",
        serde_json::from_str::<serde_json::Value>(&resolutions.body)
            .unwrap_or_default()
            .as_array()
            .map(Vec::len)
            == Some(3)
    );

    for index_id in ["index-differential-b", "index-differential-c"] {
        let route =
            format!("/api/v0/realm-subject-indexes/default-realm/{index_id}/authority-decision");
        let disabled = crate::route_http_request(
            "POST",
            &route,
            None,
            r#"{"enabled":false,"decidedBy":"differential-operator","note":"conflicting authority"}"#,
            &state,
        )
        .await
        .unwrap_or_else(|error| panic!("{route}: {error}"));
        assert_eq!(disabled.status, "200 OK", "{}", disabled.body);
    }
    record!(
        "POST",
        "/api/v0/realm-subject-indexes/{realmId}/{indexId}/authority-decision",
        "mutation-side-effects-and-readback",
        true
    );

    let decisions_route = "/api/v0/realm-subject-indexes/default-realm/authority-decisions";
    let decisions = crate::route_http_request("GET", decisions_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{decisions_route}: {error}"));
    record!(
        "GET",
        "/api/v0/realm-subject-indexes/{realmId}/authority-decisions",
        "nominal-status-headers-body",
        decisions.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/realm-subject-indexes/{realmId}/authority-decisions",
        "populated-dynamic-state",
        serde_json::from_str::<serde_json::Value>(&decisions.body)
            .unwrap_or_default()
            .as_array()
            .map(Vec::len)
            == Some(2)
    );

    let after_disable = crate::route_http_request("GET", conflicts_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{conflicts_route}: {error}"));
    let after_disable_json =
        serde_json::from_str::<serde_json::Value>(&after_disable.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/realm-subject-indexes/{realmId}/conflicts",
        "mutation-side-effects-and-readback",
        after_disable_json["disabledAuthorityCount"] == 2
            && after_disable_json["hasConflicts"] == false
    );

    let missing_route =
        "/api/v0/realm-subject-indexes/default-realm/missing-differential/authority-decision";
    let missing_decision = crate::route_http_request(
        "POST",
        missing_route,
        None,
        r#"{"enabled":true,"decidedBy":"differential-operator"}"#,
        &state,
    )
    .await
    .unwrap_or_else(|error| panic!("{missing_route}: {error}"));
    record!(
        "POST",
        "/api/v0/realm-subject-indexes/{realmId}/{indexId}/authority-decision",
        "missing-empty-or-conflict-state",
        missing_decision.status == "400 Bad Request"
            && missing_decision
                .body
                .contains("Index authority was not found")
    );

    let (empty_state, _empty_receiver) = test_state();
    let malformed_indexes = crate::route_http_request(
        "GET",
        "/api/v0/realm-subject-indexes/default-realm/extra",
        None,
        "",
        &empty_state,
    )
    .await
    .expect("malformed realm indexes path");
    record!(
        "GET",
        "/api/v0/realm-subject-indexes/{realmId}",
        "malformed-path-query-or-body",
        malformed_indexes.status == "404 Not Found"
    );
    let malformed_authority_decisions = crate::route_http_request(
        "GET",
        "/api/v0/realm-subject-indexes/default-realm/authority-decisions/extra",
        None,
        "",
        &empty_state,
    )
    .await
    .expect("malformed authority decisions path");
    record!(
        "GET",
        "/api/v0/realm-subject-indexes/{realmId}/authority-decisions",
        "malformed-path-query-or-body",
        malformed_authority_decisions.status == "404 Not Found"
    );
    let malformed_conflicts = crate::route_http_request(
        "GET",
        "/api/v0/realm-subject-indexes/default-realm/conflicts/extra",
        None,
        "",
        &empty_state,
    )
    .await
    .expect("malformed realm conflicts path");
    record!(
        "GET",
        "/api/v0/realm-subject-indexes/{realmId}/conflicts",
        "malformed-path-query-or-body",
        malformed_conflicts.status == "404 Not Found"
    );
    let malformed_resolutions = crate::route_http_request(
        "GET",
        "/api/v0/realm-subject-indexes/recordings/recording-differential/resolutions/extra",
        None,
        "",
        &empty_state,
    )
    .await
    .expect("malformed recording resolutions path");
    record!(
        "GET",
        "/api/v0/realm-subject-indexes/recordings/{recordingId}/resolutions",
        "malformed-path-query-or-body",
        malformed_resolutions.status == "404 Not Found"
    );

    let empty_indexes = crate::route_http_request(
        "GET",
        "/api/v0/realm-subject-indexes/default-realm",
        None,
        "",
        &empty_state,
    )
    .await
    .expect("empty realm indexes");
    let empty_indexes_json =
        serde_json::from_str::<serde_json::Value>(&empty_indexes.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/realm-subject-indexes/{realmId}",
        "missing-empty-or-conflict-state",
        empty_indexes.status == "200 OK" && empty_indexes_json == serde_json::json!([])
    );
    let empty_authority_decisions = crate::route_http_request(
        "GET",
        "/api/v0/realm-subject-indexes/default-realm/authority-decisions",
        None,
        "",
        &empty_state,
    )
    .await
    .expect("empty authority decisions");
    let empty_authority_decisions_json =
        serde_json::from_str::<serde_json::Value>(&empty_authority_decisions.body)
            .unwrap_or_default();
    record!(
        "GET",
        "/api/v0/realm-subject-indexes/{realmId}/authority-decisions",
        "missing-empty-or-conflict-state",
        empty_authority_decisions.status == "200 OK"
            && empty_authority_decisions_json == serde_json::json!([])
    );
    let empty_conflicts = crate::route_http_request(
        "GET",
        "/api/v0/realm-subject-indexes/default-realm/conflicts",
        None,
        "",
        &empty_state,
    )
    .await
    .expect("empty realm conflicts");
    let empty_conflicts_json =
        serde_json::from_str::<serde_json::Value>(&empty_conflicts.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/realm-subject-indexes/{realmId}/conflicts",
        "missing-empty-or-conflict-state",
        empty_conflicts.status == "200 OK"
            && empty_conflicts_json["indexCount"] == 0
            && empty_conflicts_json["disabledAuthorityCount"] == 0
            && empty_conflicts_json["entryCount"] == 0
            && empty_conflicts_json["hasConflicts"] == false
            && empty_conflicts_json["conflicts"] == serde_json::json!([])
    );
    let empty_resolutions = crate::route_http_request(
        "GET",
        "/api/v0/realm-subject-indexes/recordings/recording-empty/resolutions",
        None,
        "",
        &empty_state,
    )
    .await
    .expect("empty recording resolutions");
    let empty_resolutions_json =
        serde_json::from_str::<serde_json::Value>(&empty_resolutions.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/realm-subject-indexes/recordings/{recordingId}/resolutions",
        "missing-empty-or-conflict-state",
        empty_resolutions.status == "200 OK" && empty_resolutions_json == serde_json::json!([])
    );

    let runtime_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("realm subject-index runtime database");
    let (runtime_state, _runtime_receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_PERSISTENCE_ENABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", target),
        crate::SearchStore::new(),
        Some(runtime_db.clone()),
    );
    runtime_db.close_for_test().await;
    let runtime_indexes = crate::route_http_request(
        "GET",
        "/api/v0/realm-subject-indexes/default-realm",
        None,
        "",
        &runtime_state,
    )
    .await
    .expect("runtime realm indexes");
    record!(
        "GET",
        "/api/v0/realm-subject-indexes/{realmId}",
        "runtime-failure-and-timeout",
        runtime_indexes.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&runtime_indexes.body)
                .is_ok_and(|value| value.is_array())
    );
    let runtime_authority_decisions = crate::route_http_request(
        "GET",
        "/api/v0/realm-subject-indexes/default-realm/authority-decisions",
        None,
        "",
        &runtime_state,
    )
    .await
    .expect("runtime authority decisions");
    record!(
        "GET",
        "/api/v0/realm-subject-indexes/{realmId}/authority-decisions",
        "runtime-failure-and-timeout",
        runtime_authority_decisions.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&runtime_authority_decisions.body)
                .is_ok_and(|value| value.is_array())
    );
    let runtime_conflicts = crate::route_http_request(
        "GET",
        "/api/v0/realm-subject-indexes/default-realm/conflicts",
        None,
        "",
        &runtime_state,
    )
    .await
    .expect("runtime realm conflicts");
    record!(
        "GET",
        "/api/v0/realm-subject-indexes/{realmId}/conflicts",
        "runtime-failure-and-timeout",
        runtime_conflicts.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&runtime_conflicts.body)
                .is_ok_and(|value| value.is_object())
    );
    let runtime_resolutions = crate::route_http_request(
        "GET",
        "/api/v0/realm-subject-indexes/recordings/recording-runtime/resolutions",
        None,
        "",
        &runtime_state,
    )
    .await
    .expect("runtime recording resolutions");
    record!(
        "GET",
        "/api/v0/realm-subject-indexes/recordings/{recordingId}/resolutions",
        "runtime-failure-and-timeout",
        runtime_resolutions.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&runtime_resolutions.body)
                .is_ok_and(|value| value.is_array())
    );

    let runtime_index = index(
        "index-runtime-differential",
        "subject-runtime-differential",
        "Runtime Title",
        "discogs-runtime-differential",
    );
    runtime_state
        .realm_subject_indexes
        .write()
        .await
        .merge_indexes(vec![runtime_index])
        .expect("seed runtime authority index");
    let runtime_decision = crate::route_http_request(
        "POST",
        "/api/v0/realm-subject-indexes/default-realm/index-runtime-differential/authority-decision",
        None,
        r#"{"enabled":false,"decidedBy":"runtime-operator"}"#,
        &runtime_state,
    )
    .await
    .expect("runtime authority decision");
    record!(
        "POST",
        "/api/v0/realm-subject-indexes/{realmId}/{indexId}/authority-decision",
        "runtime-failure-and-timeout",
        runtime_decision.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&runtime_decision.body)
                .is_ok_and(|value| value["isAccepted"] == true)
    );

    let persisted_state_dir = state.config.state_dir.clone();
    *state.realm_subject_indexes.write().await =
        crate::realm_subject_index::Store::load_with_identity(
            &persisted_state_dir,
            crate::realm_subject_index::DEFAULT_REALM_ID,
            [crate::realm_subject_index::DEFAULT_GOVERNANCE_ROOT],
        )
        .expect("load persistent realm subject-index store");
    let persisted_index = index(
        "index-restart-differential",
        "subject-restart-differential",
        "Restart Title",
        "discogs-restart-differential",
    );
    state
        .realm_subject_indexes
        .write()
        .await
        .merge_indexes(vec![persisted_index])
        .expect("persist restart authority index");
    let persisted_decision = crate::route_http_request(
        "POST",
        "/api/v0/realm-subject-indexes/default-realm/index-restart-differential/authority-decision",
        None,
        r#"{"enabled":false,"decidedBy":"restart-operator"}"#,
        &state,
    )
    .await
    .expect("persist restart authority decision");
    let (restarted_state, _restarted_receiver) = test_state();
    *restarted_state.realm_subject_indexes.write().await =
        crate::realm_subject_index::Store::load_with_identity(
            &persisted_state_dir,
            crate::realm_subject_index::DEFAULT_REALM_ID,
            [crate::realm_subject_index::DEFAULT_GOVERNANCE_ROOT],
        )
        .expect("reload persistent realm subject-index store");
    let restarted_decisions = crate::route_http_request(
        "GET",
        "/api/v0/realm-subject-indexes/default-realm/authority-decisions",
        None,
        "",
        &restarted_state,
    )
    .await
    .expect("read restarted authority decisions");
    record!(
        "POST",
        "/api/v0/realm-subject-indexes/{realmId}/{indexId}/authority-decision",
        "restart-persistence-or-reset",
        persisted_decision.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&restarted_decisions.body).is_ok_and(
                |value| {
                    value.as_array().is_some_and(|decisions| {
                        decisions.iter().any(|decision| {
                            decision["indexId"] == "index-restart-differential"
                                && decision["enabled"] == false
                        })
                    })
                }
            )
    );

    let concurrent_bodies: Vec<String> = (0..4)
        .map(|index| {
            format!(
                r#"{{"enabled":{},"decidedBy":"concurrent-operator-{index}"}}"#,
                index % 2 == 0
            )
        })
        .collect();
    let concurrent_posts = futures_util::future::join_all(concurrent_bodies.iter().map(|body| {
        let path =
            "/api/v0/realm-subject-indexes/default-realm/index-restart-differential/authority-decision"
                .to_owned();
        let body = body.clone();
        let state = Arc::clone(&state);
        async move { crate::route_http_request("POST", &path, None, &body, &state).await }
    }))
    .await;
    let concurrent_decisions = crate::route_http_request(
        "GET",
        "/api/v0/realm-subject-indexes/default-realm/authority-decisions",
        None,
        "",
        &state,
    )
    .await
    .expect("read concurrent authority decisions");
    record!(
        "POST",
        "/api/v0/realm-subject-indexes/{realmId}/{indexId}/authority-decision",
        "concurrency-and-idempotency",
        concurrent_posts.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
        }) && serde_json::from_str::<serde_json::Value>(&concurrent_decisions.body).is_ok_and(
            |value| {
                value.as_array().is_some_and(|decisions| {
                    decisions
                        .iter()
                        .any(|decision| decision["indexId"] == "index-restart-differential")
                })
            }
        )
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("realm_subject_indexes.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api realm-subject-indexes mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting 7 pod-membership-workflow
/// routes' cases, independently re-derived from `pod_membership_
/// workflow_queues_accepts_lists_leaves_and_cancels`'s real
/// queued-request lifecycle: joining queues a pending request rather
/// than adding the member immediately, only an authorized acceptor
/// (a real pod moderator/owner, not any member) can accept it, and
/// leave/cancel follow the same real queued pattern. slskdN-only
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
    feature = "bounded-controller-api-tests-2"
))]
pub(super) async fn controller_api_differential_pod_membership_workflow() {
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
        let mut rooms = state.rooms.write().await;
        rooms
            .join("pod:differential-workflow".to_owned())
            .expect("workflow pod");
        rooms
            .add_member(
                "pod:differential-workflow",
                "differential-owner-peer".to_owned(),
            )
            .expect("owner capacity")
            .expect("workflow pod");
        rooms
            .add_member(
                "pod:differential-workflow",
                "differential-ordinary-peer".to_owned(),
            )
            .expect("ordinary member capacity")
            .expect("workflow pod");
        rooms.records[0].operated = true;
    }
    state.pod_membership_workflow.write().await.set_role(
        "pod:differential-workflow",
        "differential-owner-peer",
        "owner".to_owned(),
    );

    let join = crate::route_http_request(
        "POST",
        "/api/v0/podcore/membership/join",
        None,
        r#"{"podId":"pod:differential-workflow","peerId":"differential-applicant","requestedRole":"moderator"}"#,
        &state,
    )
    .await
    .expect("join request");
    record!(
        "POST",
        "/api/v0/podcore/membership/join",
        "nominal-status-headers-body",
        join.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v0/podcore/membership/join",
        "mutation-side-effects-and-readback",
        !state.rooms.read().await.records[0]
            .members
            .iter()
            .any(|member| member == "differential-applicant")
    );

    let pending_route = "/api/v0/podcore/membership/join/pending/pod%3Adifferential-workflow";
    let pending = crate::route_http_request("GET", pending_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{pending_route}: {error}"));
    let pending_json = serde_json::from_str::<serde_json::Value>(&pending.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/podcore/membership/join/pending/{podId}",
        "nominal-status-headers-body",
        pending.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/podcore/membership/join/pending/{podId}",
        "populated-dynamic-state",
        pending_json["pendingJoinRequests"][0]["peerId"] == "differential-applicant"
    );

    let unauthorized = crate::route_http_request(
        "POST",
        "/api/v0/podcore/membership/join/accept",
        None,
        r#"{"podId":"pod:differential-workflow","peerId":"differential-applicant","acceptedRole":"moderator","acceptorPeerId":"differential-ordinary-peer"}"#,
        &state,
    )
    .await
    .expect("unauthorized join acceptance");
    record!(
        "POST",
        "/api/v0/podcore/membership/join/accept",
        "missing-empty-or-conflict-state",
        unauthorized.status == "400 Bad Request"
            && unauthorized
                .body
                .contains("Join acceptance could not be processed")
            && state
                .pod_membership_workflow
                .read()
                .await
                .pending_joins("pod:differential-workflow")
                .len()
                == 1
    );

    let accepted = crate::route_http_request(
        "POST",
        "/api/v0/podcore/membership/join/accept",
        None,
        r#"{"podId":"pod:differential-workflow","peerId":"differential-applicant","acceptedRole":"moderator","acceptorPeerId":"differential-owner-peer"}"#,
        &state,
    )
    .await
    .expect("join acceptance");
    record!(
        "POST",
        "/api/v0/podcore/membership/join/accept",
        "nominal-status-headers-body",
        accepted.status == "200 OK"
            && state.rooms.read().await.records[0]
                .members
                .iter()
                .any(|member| member == "differential-applicant")
    );
    record!(
        "POST",
        "/api/v0/podcore/membership/join/accept",
        "mutation-side-effects-and-readback",
        accepted.status == "200 OK"
            && accepted.body.contains("differential-applicant")
            && state
                .pod_membership_workflow
                .read()
                .await
                .pending_joins("pod:differential-workflow")
                .is_empty()
            && state.rooms.read().await.records[0]
                .members
                .iter()
                .any(|member| member == "differential-applicant")
    );

    let leave = crate::route_http_request(
        "POST",
        "/api/v0/podcore/membership/leave",
        None,
        r#"{"podId":"pod:differential-workflow","peerId":"differential-applicant"}"#,
        &state,
    )
    .await
    .expect("leave request");
    record!(
        "POST",
        "/api/v0/podcore/membership/leave",
        "nominal-status-headers-body",
        leave.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&leave.body).unwrap_or_default()
                ["pending"]
                == true
    );
    record!(
        "POST",
        "/api/v0/podcore/membership/leave",
        "mutation-side-effects-and-readback",
        leave.status == "200 OK"
            && leave.body.contains("differential-applicant")
            && state
                .pod_membership_workflow
                .read()
                .await
                .pending_leaves("pod:differential-workflow")
                .iter()
                .any(|request| request.peer_id == "differential-applicant")
    );

    let pending_leave_route =
        "/api/v0/podcore/membership/leave/pending/pod%3Adifferential-workflow";
    let pending_leave = crate::route_http_request("GET", pending_leave_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{pending_leave_route}: {error}"));
    record!(
        "GET",
        "/api/v0/podcore/membership/leave/pending/{podId}",
        "nominal-status-headers-body",
        pending_leave.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/podcore/membership/leave/pending/{podId}",
        "populated-dynamic-state",
        pending_leave.body.contains("differential-applicant")
    );

    let missing_pending_join = crate::route_http_request(
        "GET",
        "/api/v0/podcore/membership/join/pending/pod%3Amissing-workflow",
        None,
        "",
        &state,
    )
    .await
    .expect("missing pending join requests");
    let missing_pending_join_json =
        serde_json::from_str::<serde_json::Value>(&missing_pending_join.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/podcore/membership/join/pending/{podId}",
        "missing-empty-or-conflict-state",
        missing_pending_join.status == "200 OK"
            && missing_pending_join_json["pendingJoinRequests"] == serde_json::json!([])
    );

    let missing_pending_leave = crate::route_http_request(
        "GET",
        "/api/v0/podcore/membership/leave/pending/pod%3Amissing-workflow",
        None,
        "",
        &state,
    )
    .await
    .expect("missing pending leave requests");
    let missing_pending_leave_json =
        serde_json::from_str::<serde_json::Value>(&missing_pending_leave.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/podcore/membership/leave/pending/{podId}",
        "missing-empty-or-conflict-state",
        missing_pending_leave.status == "200 OK"
            && missing_pending_leave_json["pendingLeaveRequests"] == serde_json::json!([])
    );

    let accepted_leave = crate::route_http_request(
        "POST",
        "/api/v0/podcore/membership/leave/accept",
        None,
        r#"{"podId":"pod:differential-workflow","peerId":"differential-applicant","acceptorPeerId":"differential-owner-peer"}"#,
        &state,
    )
    .await
    .expect("leave acceptance");
    record!(
        "POST",
        "/api/v0/podcore/membership/leave/accept",
        "nominal-status-headers-body",
        accepted_leave.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v0/podcore/membership/leave/accept",
        "mutation-side-effects-and-readback",
        !state.rooms.read().await.records[0]
            .members
            .iter()
            .any(|member| member == "differential-applicant")
    );

    let missing_leave = crate::route_http_request(
        "POST",
        "/api/v0/podcore/membership/leave",
        None,
        r#"{"podId":"pod:differential-workflow","peerId":"differential-applicant"}"#,
        &state,
    )
    .await
    .expect("missing leave request");
    record!(
        "POST",
        "/api/v0/podcore/membership/leave",
        "missing-empty-or-conflict-state",
        missing_leave.status == "400 Bad Request"
    );

    let missing_leave_accept = crate::route_http_request(
        "POST",
        "/api/v0/podcore/membership/leave/accept",
        None,
        r#"{"podId":"pod:differential-workflow","peerId":"differential-applicant","acceptorPeerId":"differential-owner-peer"}"#,
        &state,
    )
    .await
    .expect("missing leave acceptance");
    record!(
        "POST",
        "/api/v0/podcore/membership/leave/accept",
        "missing-empty-or-conflict-state",
        missing_leave_accept.status == "400 Bad Request"
    );

    let second_join = crate::route_http_request(
        "POST",
        "/api/v0/podcore/membership/join",
        None,
        r#"{"podId":"pod:differential-workflow","peerId":"differential-peer-two"}"#,
        &state,
    )
    .await
    .expect("second join request");
    assert_eq!(second_join.status, "200 OK");
    let duplicate_join = crate::route_http_request(
        "POST",
        "/api/v0/podcore/membership/join",
        None,
        r#"{"podId":"pod:differential-workflow","peerId":"differential-peer-two"}"#,
        &state,
    )
    .await
    .expect("duplicate join request");
    record!(
        "POST",
        "/api/v0/podcore/membership/join",
        "missing-empty-or-conflict-state",
        duplicate_join.status == "400 Bad Request"
    );

    let cancel_route =
        "/api/v0/podcore/membership/join/pod%3Adifferential-workflow/differential-peer-two";
    let cancelled = crate::route_http_request("DELETE", cancel_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{cancel_route}: {error}"));
    record!(
        "DELETE",
        "/api/v0/podcore/membership/join/{podId}/{peerId}",
        "nominal-status-headers-body",
        cancelled.status == "200 OK" && cancelled.body == r#"{"cancelled":true}"#
    );
    record!(
        "DELETE",
        "/api/v0/podcore/membership/join/{podId}/{peerId}",
        "mutation-side-effects-and-readback",
        state
            .pod_membership_workflow
            .read()
            .await
            .pending_joins("pod:differential-workflow")
            .is_empty()
    );
    let repeated_cancel = crate::route_http_request("DELETE", cancel_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{cancel_route}: {error}"));
    record!(
        "DELETE",
        "/api/v0/podcore/membership/join/{podId}/{peerId}",
        "missing-empty-or-conflict-state",
        repeated_cancel.status == "404 Not Found"
    );

    // The frozen PodJoinLeaveController uses the same pending-request
    // contract for leave cancellation: a privileged member creates a
    // pending leave, DELETE consumes it with the exact cancelled body,
    // and a repeat DELETE is a real not-found branch.
    let leave_request = crate::route_http_request(
        "POST",
        "/api/v0/podcore/membership/leave",
        None,
        r#"{"podId":"pod:differential-workflow","peerId":"differential-owner-peer"}"#,
        &state,
    )
    .await
    .expect("leave request for cancellation");
    assert_eq!(leave_request.status, "200 OK", "{}", leave_request.body);
    let leave_cancel_route =
        "/api/v0/podcore/membership/leave/pod%3Adifferential-workflow/differential-owner-peer";
    let cancelled_leave = crate::route_http_request("DELETE", leave_cancel_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{leave_cancel_route}: {error}"));
    record!(
        "DELETE",
        "/api/v0/podcore/membership/leave/{podId}/{peerId}",
        "nominal-status-headers-body",
        cancelled_leave.status == "200 OK" && cancelled_leave.body == r#"{"cancelled":true}"#
    );
    record!(
        "DELETE",
        "/api/v0/podcore/membership/leave/{podId}/{peerId}",
        "mutation-side-effects-and-readback",
        state
            .pod_membership_workflow
            .read()
            .await
            .pending_leaves("pod:differential-workflow")
            .is_empty()
    );
    let repeated_leave_cancel =
        crate::route_http_request("DELETE", leave_cancel_route, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{leave_cancel_route}: {error}"));
    record!(
        "DELETE",
        "/api/v0/podcore/membership/leave/{podId}/{peerId}",
        "missing-empty-or-conflict-state",
        repeated_leave_cancel.status == "404 Not Found"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("pod_membership_workflow.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api pod-membership-workflow mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting the pod-channel-messages route's
/// cases, independently re-derived from `pod_channel_messages_are_
/// durable_shaped_and_incremental`'s real sender-identity-spoofing
/// rejection (a caller cannot post as a different `senderPeerId`),
/// membership-required history reads (a non-member 403s even for a
/// public pod), and `?since=` incremental-cursor pagination.
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
    feature = "bounded-controller-api-tests-2"
))]
pub(super) async fn controller_api_differential_pod_channel_messages() {
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
    let path = "/api/v0/pods/pod-differential-1/channels/general/messages";
    let created = crate::route_http_request(
        "POST",
        "/api/v0/pods",
        None,
        r#"{"pod":{"podId":"pod-differential-1","name":"Pod Differential One","isPublic":true,"channels":[{"channelId":"general","kind":0,"name":"General"}]},"requestingPeerId":"differential-peer-1"}"#,
        &state,
    )
    .await
    .expect("create pod for messages");
    assert_eq!(created.status, "201 Created");

    state
        .pods
        .write()
        .await
        .join("pod-differential-1", "differential-peer-1".to_owned())
        .expect("join pod as fixture member");

    // PodMessageStorageController exposes count and search as separate
    // read contracts from channel history. The frozen oracle returns a
    // numeric zero / empty array for a member pod with no matching
    // messages, then returns the durable message rows after publish.
    let empty_count = crate::route_http_request(
        "GET",
        "/api/v0/podcore/messages/pod-differential-1/general/count",
        None,
        "",
        &state,
    )
    .await
    .expect("empty message count");
    record!(
        "GET",
        "/api/v0/podcore/messages/{podId}/{channelId}/count",
        "nominal-status-headers-body",
        empty_count.status == "200 OK" && empty_count.body == "0"
    );
    let empty_search = crate::route_http_request(
        "GET",
        "/api/v0/podcore/messages/pod-differential-1/search?query=first",
        None,
        "",
        &state,
    )
    .await
    .expect("empty message search");
    record!(
        "GET",
        "/api/v0/podcore/messages/{podId}/search",
        "nominal-status-headers-body",
        empty_search.status == "200 OK" && empty_search.body == "[]"
    );

    let spoofed = crate::route_http_request(
        "POST",
        path,
        None,
        r#"{"body":"spoofed","senderPeerId":"differential-peer-1"}"#,
        &state,
    )
    .await
    .unwrap_or_else(|error| panic!("{path}: {error}"));
    record!(
        "POST",
        "/api/v0/pods/{podId}/channels/{channelId}/messages",
        "missing-empty-or-conflict-state",
        spoofed.status == "403 Forbidden"
    );

    *state.runtime_credentials.write().await = Some(crate::LoginCredentials::default_client(
        "differential-public-intruder",
        "secret",
    ));
    let forbidden = crate::route_http_request("GET", path, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{path}: {error}"));
    record!(
        "GET",
        "/api/v0/pods/{podId}/channels/{channelId}/messages",
        "missing-empty-or-conflict-state",
        forbidden.status == "403 Forbidden"
    );
    *state.runtime_credentials.write().await = None;

    let first = crate::route_http_request(
        "POST",
        path,
        None,
        r#"{"body":"first","senderPeerId":"tester","signature":"sig"}"#,
        &state,
    )
    .await
    .unwrap_or_else(|error| panic!("{path}: {error}"));
    let first_json = serde_json::from_str::<serde_json::Value>(&first.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/pods/{podId}/channels/{channelId}/messages",
        "nominal-status-headers-body",
        first.status == "200 OK"
            && first_json["sent"] == true
            && first_json["messageId"].as_str().map(str::len) == Some(32)
    );

    let initial = crate::route_http_request("GET", path, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{path}: {error}"));
    let initial_json = serde_json::from_str::<serde_json::Value>(&initial.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/pods/{podId}/channels/{channelId}/messages",
        "nominal-status-headers-body",
        initial.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/pods/{podId}/channels/{channelId}/messages",
        "populated-dynamic-state",
        initial_json.as_array().map(Vec::len) == Some(1)
            && initial_json[0]["podId"] == "pod-differential-1"
            && initial_json[0]["channelId"] == "general"
            && initial_json[0]["senderPeerId"] == "tester"
            && initial_json[0]["body"] == "first"
            && initial_json[0]["sigVersion"] == 1
    );
    let populated_count = crate::route_http_request(
        "GET",
        "/api/v0/podcore/messages/pod-differential-1/general/count",
        None,
        "",
        &state,
    )
    .await
    .expect("populated message count");
    record!(
        "GET",
        "/api/v0/podcore/messages/{podId}/{channelId}/count",
        "populated-dynamic-state",
        populated_count.status == "200 OK" && populated_count.body == "1"
    );
    let populated_search = crate::route_http_request(
        "GET",
        "/api/v0/podcore/messages/pod-differential-1/search?query=first",
        None,
        "",
        &state,
    )
    .await
    .expect("populated message search");
    let populated_search_json =
        serde_json::from_str::<serde_json::Value>(&populated_search.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/podcore/messages/{podId}/search",
        "populated-dynamic-state",
        populated_search.status == "200 OK"
            && populated_search_json.as_array().is_some_and(|entries| {
                entries.len() == 1
                    && entries[0]["podId"] == "pod-differential-1"
                    && entries[0]["channelId"] == "general"
                    && entries[0]["body"] == "first"
            })
    );
    let no_result_search = crate::route_http_request(
        "GET",
        "/api/v0/podcore/messages/pod-differential-1/search?query=not-present",
        None,
        "",
        &state,
    )
    .await
    .expect("no-result message search");
    record!(
        "GET",
        "/api/v0/podcore/messages/{podId}/search",
        "missing-empty-or-conflict-state",
        no_result_search.status == "200 OK" && no_result_search.body == "[]"
    );
    let malformed_search = crate::route_http_request(
        "GET",
        "/api/v0/podcore/messages/pod-differential-1/search?query=",
        None,
        "",
        &state,
    )
    .await
    .expect("malformed message search");
    record!(
        "GET",
        "/api/v0/podcore/messages/{podId}/search",
        "malformed-path-query-or-body",
        malformed_search.status == "400 Bad Request"
    );
    record!(
        "POST",
        "/api/v0/pods/{podId}/channels/{channelId}/messages",
        "mutation-side-effects-and-readback",
        first.status == "200 OK"
            && first_json["sent"] == true
            && initial_json.as_array().map(Vec::len) == Some(1)
            && initial_json[0]["body"] == "first"
            && initial_json[0]["senderPeerId"] == "tester"
    );
    let cursor = initial_json[0]["timestampUnixMs"]
        .as_u64()
        .unwrap_or_default();

    let second = crate::route_http_request(
        "POST",
        path,
        None,
        r#"{"body":"second","senderPeerId":"tester"}"#,
        &state,
    )
    .await
    .unwrap_or_else(|error| panic!("{path}: {error}"));
    assert_eq!(second.status, "200 OK");
    {
        let mut channels = state.pod_channels.write().await;
        let latest = channels
            .list("pod-differential-1", "general", None)
            .pop()
            .expect("at least one message");
        if latest.timestamp_unix_ms == cursor {
            channels
                .append(
                    "pod-differential-1".to_owned(),
                    "general".to_owned(),
                    "tester".to_owned(),
                    "third".to_owned(),
                    String::new(),
                    cursor + 1,
                )
                .expect("append disambiguating message");
        }
    }
    let incremental_route = format!("{path}?since={cursor}");
    let incremental = crate::route_http_request("GET", &incremental_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{incremental_route}: {error}"));
    let incremental_json =
        serde_json::from_str::<serde_json::Value>(&incremental.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/pods/{podId}/channels/{channelId}/messages",
        "mutation-side-effects-and-readback",
        incremental.status == "200 OK"
            && incremental_json.as_array().is_some_and(|entries| {
                !entries.is_empty()
                    && entries.iter().all(|message| {
                        message["timestampUnixMs"].as_u64().unwrap_or_default() > cursor
                    })
            })
    );

    let invalid_cursor_route = format!("{path}?since=-1");
    let invalid_cursor = crate::route_http_request("GET", &invalid_cursor_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{invalid_cursor_route}: {error}"));
    record!(
        "GET",
        "/api/v0/pods/{podId}/channels/{channelId}/messages",
        "malformed-path-query-or-body",
        invalid_cursor.status == "400 Bad Request"
    );

    let invalid_body = crate::route_http_request(
        "POST",
        path,
        None,
        r#"{"body":"","senderPeerId":"tester"}"#,
        &state,
    )
    .await
    .unwrap_or_else(|error| panic!("{path}: {error}"));
    record!(
        "POST",
        "/api/v0/pods/{podId}/channels/{channelId}/messages",
        "malformed-path-query-or-body",
        invalid_body.status == "400 Bad Request"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("pod_channel_messages.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api pod-channel-messages mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
