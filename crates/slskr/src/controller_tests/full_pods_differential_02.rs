//! Controller full pods differential 02 ownership.

use super::*;

/// Bulk differential proof crediting 3 podcore routing routes' cases,
/// independently re-derived from `podcore_routing_matches_message_
/// router_contract_and_updates_real_stats`'s real bloom-filter
/// deduplication, banned-member filtering, and validation checks.
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
pub(super) async fn controller_api_differential_podcore_routing() {
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
    let pod_id = "routing-differential-pod";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<crate::pods::PodRecord>(serde_json::json!({
                "podId": pod_id,
                "name": "Routing differential",
                "channels": [{"channelId": "general", "name": "General"}]
            }))
            .expect("deserialize routing pod"),
            "sender-peer".to_owned(),
        )
        .expect("create routing pod");
    state
        .pods
        .write()
        .await
        .upsert_member(
            pod_id,
            crate::pods::PodMember {
                peer_id: "target-peer".to_owned(),
                role: "member".to_owned(),
                is_banned: false,
                public_key: None,
                joined_at: None,
                last_seen: None,
            },
        )
        .expect("add routing target");
    state
        .pods
        .write()
        .await
        .upsert_member(
            pod_id,
            crate::pods::PodMember {
                peer_id: "banned-peer".to_owned(),
                role: "member".to_owned(),
                is_banned: true,
                public_key: None,
                joined_at: None,
                last_seen: None,
            },
        )
        .expect("add banned routing peer");

    let unseen = crate::route_http_request(
        "GET",
        &format!("/api/v0/podcore/routing/seen/routing-unseen/{pod_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("check unseen routing message");
    record!(
        "GET",
        "/api/v0/podcore/routing/seen/{messageId}/{podId}",
        "nominal-status-headers-body",
        unseen.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&unseen.body).unwrap_or_default()
                == serde_json::json!({"isSeen": false})
    );

    let missing_unseen = crate::route_http_request(
        "GET",
        "/api/v0/podcore/routing/seen/routing-unseen/pod%3Amissing-routing",
        None,
        "",
        &state,
    )
    .await
    .expect("check unseen routing message in missing pod");
    record!(
        "GET",
        "/api/v0/podcore/routing/seen/{messageId}/{podId}",
        "missing-empty-or-conflict-state",
        missing_unseen.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&missing_unseen.body).unwrap_or_default()
                == serde_json::json!({"isSeen": false})
    );

    let nominal_pod_id = "routing-nominal-pod";
    let (nominal_state, _nominal_receiver) = test_state();
    nominal_state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<crate::pods::PodRecord>(serde_json::json!({
                "podId": nominal_pod_id,
                "name": "Routing nominal differential",
                "channels": [{"channelId": "general", "name": "General"}]
            }))
            .expect("deserialize nominal routing pod"),
            "nominal-sender".to_owned(),
        )
        .expect("create nominal routing pod");
    let nominal_route = crate::route_http_request(
        "POST",
        "/api/v0/podcore/routing/route",
        None,
        &serde_json::json!({
            "messageId": "routing-nominal-1",
            "podId": nominal_pod_id,
            "channelId": "general",
            "senderPeerId": "nominal-sender",
            "body": "no eligible peers",
        })
        .to_string(),
        &nominal_state,
    )
    .await
    .expect("nominal route message");
    let nominal_route_json =
        serde_json::from_str::<serde_json::Value>(&nominal_route.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/podcore/routing/route",
        "nominal-status-headers-body",
        nominal_route.status == "200 OK"
            && nominal_route_json["success"] == true
            && nominal_route_json["messageId"] == "routing-nominal-1"
            && nominal_route_json["targetPeerCount"] == 0
            && nominal_route_json["successfullyRoutedCount"] == 0
            && nominal_route_json["failedRoutingCount"] == 0
    );

    let routed = crate::route_http_request(
        "POST",
        "/api/v0/podcore/routing/route",
        None,
        &serde_json::json!({
            "messageId": "routing-differential-1",
            "podId": pod_id,
            "channelId": "general",
            "senderPeerId": "sender-peer",
            "body": "hello",
        })
        .to_string(),
        &state,
    )
    .await
    .expect("route message");
    record!(
        "POST",
        "/api/v0/podcore/routing/route",
        "runtime-failure-and-timeout",
        routed.status == "500 Internal Server Error"
            && serde_json::from_str::<serde_json::Value>(&routed.body).unwrap()["error"]
                == "Failed to route message"
    );

    let seen_after_route = crate::route_http_request(
        "GET",
        &format!("/api/v0/podcore/routing/seen/routing-differential-1/{pod_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("check routed message state");
    record!(
        "GET",
        "/api/v0/podcore/routing/seen/{messageId}/{podId}",
        "populated-dynamic-state",
        seen_after_route.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&seen_after_route.body)
                .unwrap_or_default()
                == serde_json::json!({"isSeen": true})
    );

    let direct = crate::route_http_request(
        "POST",
        "/api/v0/podcore/routing/route-to-peers",
        None,
        &serde_json::json!({
            "message": {
                "messageId": "routing-differential-2",
                "podId": pod_id,
                "channelId": "general",
                "senderPeerId": "sender-peer",
            },
            "targetPeerIds": [" target-peer ", "target-peer"],
        })
        .to_string(),
        &state,
    )
    .await
    .expect("route message to peers");
    let direct_json = serde_json::from_str::<serde_json::Value>(&direct.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/podcore/routing/route-to-peers",
        "nominal-status-headers-body",
        direct.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v0/podcore/routing/route-to-peers",
        "mutation-side-effects-and-readback",
        direct_json["success"] == false
            && direct_json["targetPeerCount"] == 1
            && direct_json["successfullyRoutedCount"] == 0
            && direct_json["failedRoutingCount"] == 1
            && direct_json["failedPeerIds"] == serde_json::json!(["target-peer"])
    );

    let stats = crate::route_http_request("GET", "/api/v0/podcore/routing/stats", None, "", &state)
        .await
        .expect("routing stats");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/podcore/routing/stats",
        "nominal-status-headers-body",
        stats.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/podcore/routing/stats",
        "populated-dynamic-state",
        stats_json["totalMessagesRouted"] == 1
            && stats_json["totalRoutingAttempts"] == 1
            && stats_json["successfulRoutingCount"] == 0
            && stats_json["failedRoutingCount"] == 1
            && stats_json["activeDeduplicationItems"] == 1
            && stats_json["routingStatsByPod"][pod_id] == 1
    );

    let duplicate = crate::route_http_request(
        "POST",
        "/api/v0/podcore/routing/route",
        None,
        &serde_json::json!({
            "messageId": "routing-differential-1",
            "podId": pod_id,
            "channelId": "general",
            "senderPeerId": "sender-peer",
        })
        .to_string(),
        &state,
    )
    .await
    .expect("route duplicate message");
    let duplicate_json =
        serde_json::from_str::<serde_json::Value>(&duplicate.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/podcore/routing/route",
        "mutation-side-effects-and-readback",
        duplicate.status == "200 OK"
            && duplicate_json["targetPeerCount"] == 0
            && duplicate_json["errorMessage"] == "Message already routed (duplicate)"
    );

    let missing_channel = crate::route_http_request(
        "POST",
        "/api/v0/podcore/routing/route",
        None,
        &serde_json::json!({
            "messageId": "routing-differential-invalid",
            "podId": pod_id,
        })
        .to_string(),
        &state,
    )
    .await
    .expect("validate route message");
    record!(
        "POST",
        "/api/v0/podcore/routing/route",
        "malformed-path-query-or-body",
        missing_channel.status == "400 Bad Request"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("podcore_routing.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api podcore-routing mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting 13 podcore maintenance-mutation
/// routes' nominal, mutation, malformed, and missing-resource cases,
/// independently re-derived from the real DHT/discovery/membership/
/// routing/messages/backfill result-contract checks. slskdN-only
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
pub(super) async fn controller_api_differential_podcore_maintenance_mutations() {
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
    let pod_id = "pod:00000000000000000000000000000002";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<crate::pods::PodRecord>(serde_json::json!({
                "podId": pod_id,
                "name": "Maintenance differential",
                "visibility": "Listed",
                "isPublic": true,
            }))
            .expect("deserialize maintenance pod fixture"),
            "owner-peer".to_owned(),
        )
        .expect("create maintenance pod");

    for (action, fields) in [
        (
            "publish",
            vec!["success", "podId", "dhtKey", "publishedAt", "expiresAt"],
        ),
        (
            "update",
            vec!["success", "podId", "dhtKey", "publishedAt", "expiresAt"],
        ),
    ] {
        let route = format!("/api/v0/podcore/dht/{action}");
        let malformed = crate::route_http_request("POST", &route, None, "{}", &state)
            .await
            .unwrap_or_else(|error| panic!("{route} malformed: {error}"));
        record!(
            "POST",
            route,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
        let missing = crate::route_http_request(
            "POST",
            &route,
            None,
            &serde_json::json!({
                "pod": {
                    "podId": "pod:00000000000000000000000000000003",
                    "name": "Missing maintenance pod"
                }
            })
            .to_string(),
            &state,
        )
        .await
        .unwrap_or_else(|error| panic!("{route} missing: {error}"));
        record!(
            "POST",
            route,
            "missing-empty-or-conflict-state",
            missing.status == "404 Not Found"
        );
        let response = crate::route_http_request(
            "POST",
            &route,
            None,
            &serde_json::json!({"pod": {"podId": pod_id, "name": "Maintenance differential"}})
                .to_string(),
            &state,
        )
        .await
        .unwrap_or_else(|error| panic!("{route}: {error}"));
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "POST",
            route,
            "nominal-status-headers-body",
            response.status == "200 OK"
        );
        record!(
            "POST",
            route,
            "mutation-side-effects-and-readback",
            fields.iter().all(|key| value.get(*key).is_some()) && value["podId"] == pod_id
        );
    }

    for (action, fields) in [
        (
            "register",
            vec![
                "success",
                "podId",
                "discoveryKeys",
                "registeredAt",
                "expiresAt",
            ],
        ),
        (
            "update",
            vec![
                "success",
                "podId",
                "discoveryKeys",
                "registeredAt",
                "expiresAt",
            ],
        ),
    ] {
        let route = format!("/api/v0/podcore/discovery/{action}");
        let malformed = crate::route_http_request("POST", &route, None, "{}", &state)
            .await
            .unwrap_or_else(|error| panic!("{route} malformed: {error}"));
        record!(
            "POST",
            route,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
        let response = crate::route_http_request(
            "POST",
            &route,
            None,
            &serde_json::json!({
                "podId": pod_id,
                "name": "Maintenance differential",
                "visibility": "Listed",
            })
            .to_string(),
            &state,
        )
        .await
        .unwrap_or_else(|error| panic!("{route}: {error}"));
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "POST",
            route,
            "nominal-status-headers-body",
            response.status == "200 OK"
        );
        record!(
            "POST",
            route,
            "mutation-side-effects-and-readback",
            fields.iter().all(|key| value.get(*key).is_some())
        );
    }

    let refresh = crate::route_http_request(
        "POST",
        "/api/v0/podcore/discovery/refresh",
        None,
        "",
        &state,
    )
    .await
    .expect("discovery refresh");
    let refresh_json = serde_json::from_str::<serde_json::Value>(&refresh.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/podcore/discovery/refresh",
        "nominal-status-headers-body",
        refresh.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v0/podcore/discovery/refresh",
        "mutation-side-effects-and-readback",
        ["success", "podId", "wasRepublished", "nextRefresh"]
            .iter()
            .all(|key| refresh_json.get(*key).is_some())
    );

    for (route, fields) in [
        (
            "/api/v0/podcore/membership/cleanup",
            vec!["recordsCleaned", "errorsEncountered", "completedAt"],
        ),
        (
            "/api/v0/podcore/routing/cleanup",
            vec![
                "messagesCleaned",
                "messagesRetained",
                "cleanupDuration",
                "completedAt",
            ],
        ),
    ] {
        let response = crate::route_http_request("POST", route, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{route}: {error}"));
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "POST",
            route,
            "nominal-status-headers-body",
            response.status == "200 OK"
        );
        record!(
            "POST",
            route,
            "mutation-side-effects-and-readback",
            fields.iter().all(|key| value.get(*key).is_some())
        );
    }

    let malformed_membership_cleanup = crate::route_http_request(
        "POST",
        "/api/v0/podcore/membership/cleanup",
        None,
        "not-json",
        &state,
    )
    .await
    .expect("malformed membership cleanup body");
    let malformed_membership_cleanup_json =
        serde_json::from_str::<serde_json::Value>(&malformed_membership_cleanup.body)
            .unwrap_or_default();
    record!(
        "POST",
        "/api/v0/podcore/membership/cleanup",
        "malformed-path-query-or-body",
        malformed_membership_cleanup.status == "200 OK"
            && malformed_membership_cleanup_json
                .get("recordsCleaned")
                .is_some()
            && malformed_membership_cleanup_json
                .get("errorsEncountered")
                .is_some()
    );
    let missing_membership_cleanup = crate::route_http_request(
        "POST",
        "/api/v0/podcore/membership/cleanup",
        None,
        "",
        &state,
    )
    .await
    .expect("empty membership cleanup request");
    let missing_membership_cleanup_json =
        serde_json::from_str::<serde_json::Value>(&missing_membership_cleanup.body)
            .unwrap_or_default();
    record!(
        "POST",
        "/api/v0/podcore/membership/cleanup",
        "missing-empty-or-conflict-state",
        missing_membership_cleanup.status == "200 OK"
            && missing_membership_cleanup_json
                .get("recordsCleaned")
                .is_some()
            && missing_membership_cleanup_json
                .get("errorsEncountered")
                .is_some()
    );

    let (concurrent_cleanup_one, concurrent_cleanup_two) = tokio::join!(
        crate::route_http_request(
            "POST",
            "/api/v0/podcore/membership/cleanup",
            None,
            "",
            &state,
        ),
        crate::route_http_request(
            "POST",
            "/api/v0/podcore/membership/cleanup",
            None,
            "",
            &state,
        )
    );
    let concurrent_cleanup_one =
        concurrent_cleanup_one.expect("first concurrent membership cleanup");
    let concurrent_cleanup_two =
        concurrent_cleanup_two.expect("second concurrent membership cleanup");
    let concurrent_cleanup_one_json =
        serde_json::from_str::<serde_json::Value>(&concurrent_cleanup_one.body).unwrap_or_default();
    let concurrent_cleanup_two_json =
        serde_json::from_str::<serde_json::Value>(&concurrent_cleanup_two.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/podcore/membership/cleanup",
        "concurrency-and-idempotency",
        concurrent_cleanup_one.status == "200 OK"
            && concurrent_cleanup_two.status == "200 OK"
            && concurrent_cleanup_one_json.get("recordsCleaned").is_some()
            && concurrent_cleanup_two_json.get("recordsCleaned").is_some()
            && concurrent_cleanup_one_json["errorsEncountered"] == 0
            && concurrent_cleanup_two_json["errorsEncountered"] == 0
    );

    let seen_route = format!("/api/v0/podcore/routing/seen/maintenance-message-1/{pod_id}");
    let seen = crate::route_http_request("POST", &seen_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{seen_route}: {error}"));
    let seen_json = serde_json::from_str::<serde_json::Value>(&seen.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/podcore/routing/seen/{messageId}/{podId}",
        "nominal-status-headers-body",
        seen.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v0/podcore/routing/seen/{messageId}/{podId}",
        "mutation-side-effects-and-readback",
        seen_json["wasNewlyRegistered"] == true
    );

    for route in [
        "/api/v0/podcore/messages/rebuild-index",
        "/api/v0/podcore/messages/vacuum",
    ] {
        let response = crate::route_http_request("POST", route, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{route}: {error}"));
        record!(
            "POST",
            route,
            "nominal-status-headers-body",
            response.status == "200 OK" && response.body == "true"
        );
    }

    let cleanup = crate::route_http_request(
        "DELETE",
        "/api/v0/podcore/messages/cleanup?olderThan=9999999999999",
        None,
        "",
        &state,
    )
    .await
    .expect("cleanup pod messages");
    record!(
        "DELETE",
        "/api/v0/podcore/messages/cleanup",
        "nominal-status-headers-body",
        cleanup.status == "200 OK" && cleanup.body == "0"
    );

    let channel_cleanup = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/podcore/messages/{pod_id}/general/cleanup?olderThan=9999999999999"),
        None,
        "",
        &state,
    )
    .await
    .expect("cleanup pod channel messages");
    record!(
        "DELETE",
        "/api/v0/podcore/messages/{podId}/{channelId}/cleanup",
        "nominal-status-headers-body",
        channel_cleanup.status == "200 OK" && channel_cleanup.body == "0"
    );

    let sync_all = crate::route_http_request(
        "POST",
        "/api/v0/podcore/backfill/sync-all",
        None,
        "",
        &state,
    )
    .await
    .expect("backfill sync-all");
    record!(
        "POST",
        "/api/v0/podcore/backfill/sync-all",
        "nominal-status-headers-body",
        sync_all.status == "200 OK" && sync_all.body == "[]"
    );

    for (section, action) in [("dht", "unpublish"), ("discovery", "unregister")] {
        let route = format!("/api/v0/podcore/{section}/{action}/{pod_id}");
        let response = crate::route_http_request("DELETE", &route, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{route}: {error}"));
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        let route_template = match section {
            "dht" => "/api/v0/podcore/dht/unpublish/{*podId}",
            "discovery" => "/api/v0/podcore/discovery/unregister/{podId}",
            _ => unreachable!("known podcore delete section"),
        };
        record!(
            "DELETE",
            route_template,
            "nominal-status-headers-body",
            response.status == "200 OK"
        );
        record!(
            "DELETE",
            route_template,
            "mutation-side-effects-and-readback",
            value["success"] == true
        );
        let missing_route =
            format!("/api/v0/podcore/{section}/{action}/pod:00000000000000000000000000000003");
        let missing = crate::route_http_request("DELETE", &missing_route, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{missing_route}: {error}"));
        let missing_json =
            serde_json::from_str::<serde_json::Value>(&missing.body).unwrap_or_default();
        record!(
            "DELETE",
            route_template,
            "missing-empty-or-conflict-state",
            match section {
                "dht" => missing.status == "200 OK" && missing_json["success"] == true,
                "discovery" => missing.status == "500 Internal Server Error",
                _ => false,
            }
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("podcore_maintenance_mutations.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api podcore-maintenance mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting podcore channel CRUD contracts,
/// independently re-derived from the frozen PodChannelController status,
/// validation, system-channel, and readback behavior. slskdN-only
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
pub(super) async fn controller_api_differential_podcore_channel_crud() {
    let target = "slskdn";
    let channels_route = "/api/v0/podcore/{podId}/channels";
    let channel_route = "/api/v0/podcore/{podId}/channels/{channelId}";
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

    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSK_USERNAME", "channel-owner")
            .with("SLSK_PASSWORD", "test-secret"),
    );
    let pod_id = "pod:channel-crud-differential";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<crate::pods::PodRecord>(serde_json::json!({
                "podId": pod_id,
                "name": "Channel CRUD differential",
                "isPublic": true,
                "channels": [
                    {"channelId": "general", "kind": 0, "name": "general"},
                    {"channelId": "extra", "kind": 0, "name": "Extra"}
                ]
            }))
            .expect("deserialize channel CRUD pod"),
            "channel-owner".to_owned(),
        )
        .expect("create channel CRUD pod");

    let empty_channels_pod_id = "pod:channel-empty-differential";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<crate::pods::PodRecord>(serde_json::json!({
                "podId": empty_channels_pod_id,
                "name": "Empty channel differential",
                "isPublic": true,
                "channels": [],
            }))
            .expect("deserialize empty channel pod"),
            "channel-owner".to_owned(),
        )
        .expect("create empty channel pod");
    let empty_channels = crate::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{empty_channels_pod_id}/channels"),
        None,
        "",
        &state,
    )
    .await
    .expect("list empty pod channels");
    let empty_channels_json =
        serde_json::from_str::<serde_json::Value>(&empty_channels.body).unwrap_or_default();
    record!(
        "GET",
        channels_route,
        "missing-empty-or-conflict-state",
        empty_channels.status == "200 OK"
            && empty_channels.content_type == "application/json"
            && empty_channels_json == serde_json::json!([])
    );

    let missing_channel_id = crate::route_http_request(
        "POST",
        &format!("/api/v0/podcore/{pod_id}/channels"),
        None,
        r#"{"name":"Missing ID"}"#,
        &state,
    )
    .await
    .expect("create channel without ID");
    record!(
        "POST",
        channels_route,
        "malformed-path-query-or-body",
        missing_channel_id.status == "400 Bad Request"
    );

    let missing_channel_name = crate::route_http_request(
        "POST",
        &format!("/api/v0/podcore/{pod_id}/channels"),
        None,
        r#"{"channelId":"missing-name"}"#,
        &state,
    )
    .await
    .expect("create channel without name");
    record!(
        "POST",
        channels_route,
        "malformed-path-query-or-body",
        missing_channel_name.status == "400 Bad Request"
    );

    let created = crate::route_http_request(
        "POST",
        &format!("/api/v0/podcore/{pod_id}/channels"),
        None,
        r#"{"channelId":"created","name":"Created"}"#,
        &state,
    )
    .await
    .expect("create channel");
    let created_json = serde_json::from_str::<serde_json::Value>(&created.body).unwrap_or_default();
    record!(
        "POST",
        channels_route,
        "nominal-status-headers-body",
        created.status == "201 Created"
            && created.content_type == "application/json"
            && created_json["channelId"] == "created"
            && created_json["name"] == "Created"
    );
    let channels_after_create = crate::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/channels"),
        None,
        "",
        &state,
    )
    .await
    .expect("list channels after create");
    let channels_after_create_json =
        serde_json::from_str::<serde_json::Value>(&channels_after_create.body).unwrap_or_default();
    record!(
        "POST",
        channels_route,
        "mutation-side-effects-and-readback",
        channels_after_create.status == "200 OK"
            && channels_after_create_json
                .as_array()
                .is_some_and(|channels| {
                    channels.iter().any(|channel| {
                        channel["channelId"] == "created" && channel["name"] == "Created"
                    })
                })
    );

    let detail = crate::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/channels/general"),
        None,
        "",
        &state,
    )
    .await
    .expect("get channel");
    let detail_json = serde_json::from_str::<serde_json::Value>(&detail.body).unwrap_or_default();
    record!(
        "GET",
        channel_route,
        "nominal-status-headers-body",
        detail.status == "200 OK"
            && detail.content_type == "application/json"
            && detail_json["channelId"] == "general"
            && detail_json["name"] == "general"
    );
    record!(
        "GET",
        channel_route,
        "populated-dynamic-state",
        detail_json["channelId"] == "general" && detail_json["kind"] == 0
    );

    let missing_detail = crate::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/channels/does-not-exist"),
        None,
        "",
        &state,
    )
    .await
    .expect("get missing channel");
    record!(
        "GET",
        channel_route,
        "missing-empty-or-conflict-state",
        missing_detail.status == "404 Not Found"
    );

    let updated = crate::route_http_request(
        "PUT",
        &format!("/api/v0/podcore/{pod_id}/channels/extra"),
        None,
        r#"{"name":"Extra renamed"}"#,
        &state,
    )
    .await
    .expect("update channel");
    record!(
        "PUT",
        channel_route,
        "nominal-status-headers-body",
        updated.status == "200 OK" && updated.body.is_empty()
    );
    let updated_readback = crate::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/channels/extra"),
        None,
        "",
        &state,
    )
    .await
    .expect("read updated channel");
    let updated_readback_json =
        serde_json::from_str::<serde_json::Value>(&updated_readback.body).unwrap_or_default();
    record!(
        "PUT",
        channel_route,
        "mutation-side-effects-and-readback",
        updated_readback.status == "200 OK"
            && updated_readback_json["channelId"] == "extra"
            && updated_readback_json["name"] == "Extra renamed"
    );

    let update_missing_name = crate::route_http_request(
        "PUT",
        &format!("/api/v0/podcore/{pod_id}/channels/extra"),
        None,
        r#"{"channelId":"extra"}"#,
        &state,
    )
    .await
    .expect("update channel without name");
    record!(
        "PUT",
        channel_route,
        "malformed-path-query-or-body",
        update_missing_name.status == "400 Bad Request"
    );

    let update_missing_channel = crate::route_http_request(
        "PUT",
        &format!("/api/v0/podcore/{pod_id}/channels/does-not-exist"),
        None,
        r#"{"name":"Missing"}"#,
        &state,
    )
    .await
    .expect("update missing channel");
    record!(
        "PUT",
        channel_route,
        "missing-empty-or-conflict-state",
        update_missing_channel.status == "404 Not Found"
    );

    let update_system_channel = crate::route_http_request(
        "PUT",
        &format!("/api/v0/podcore/{pod_id}/channels/general"),
        None,
        r#"{"name":"General renamed"}"#,
        &state,
    )
    .await
    .expect("update system channel");
    record!(
        "PUT",
        channel_route,
        "missing-empty-or-conflict-state",
        update_system_channel.status == "400 Bad Request"
    );

    let deleted = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/podcore/{pod_id}/channels/created"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete channel");
    record!(
        "DELETE",
        channel_route,
        "nominal-status-headers-body",
        deleted.status == "200 OK" && deleted.body.is_empty()
    );
    let deleted_readback = crate::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/channels/created"),
        None,
        "",
        &state,
    )
    .await
    .expect("read deleted channel");
    record!(
        "DELETE",
        channel_route,
        "mutation-side-effects-and-readback",
        deleted_readback.status == "404 Not Found"
    );

    let delete_missing = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/podcore/{pod_id}/channels/does-not-exist"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete missing channel");
    record!(
        "DELETE",
        channel_route,
        "missing-empty-or-conflict-state",
        delete_missing.status == "404 Not Found"
    );

    let delete_system_channel = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/podcore/{pod_id}/channels/general"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete system channel");
    record!(
        "DELETE",
        channel_route,
        "missing-empty-or-conflict-state",
        delete_system_channel.status == "400 Bad Request"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("podcore_channel_crud.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api podcore-channel mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential proof for the PodChannelController's storage failure,
/// reload, and concurrent mutation behavior. The frozen controller
/// surfaces storage exceptions as 500 responses and the pod service
/// preserves successful channel mutations across a fresh service load.
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
pub(super) async fn controller_api_differential_podcore_channel_lifecycle() {
    let target = "slskdn";
    let channels_route = "/api/v0/podcore/{podId}/channels";
    let channel_route = "/api/v0/podcore/{podId}/channels/{channelId}";
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

    macro_rules! seed_pod {
        ($state:expr, $pod_id:expr, $channels:expr) => {{
            $state
                .pods
                .write()
                .await
                .create(
                    serde_json::from_value::<crate::pods::PodRecord>(serde_json::json!({
                        "podId": $pod_id,
                        "name": "Channel lifecycle differential",
                        "isPublic": true,
                        "channels": $channels,
                    }))
                    .expect("deserialize channel lifecycle pod"),
                    "channel-owner".to_owned(),
                )
                .expect("create channel lifecycle pod");
        }};
    }

    let channel_env = || {
        MapEnv::default()
            .with("SLSK_USERNAME", "channel-owner")
            .with("SLSK_PASSWORD", "test-secret")
    };

    // A valid create against a missing pod is rejected by the frozen
    // controller's access check before its service lookup.
    {
        let (state, _receiver) = test_state_with_env(channel_env());
        let response = crate::route_http_request(
            "POST",
            "/api/v0/podcore/pod:missing-channel-lifecycle/channels",
            None,
            r#"{"channelId":"missing","name":"Missing"}"#,
            &state,
        )
        .await
        .expect("missing channel-lifecycle pod");
        record!(
            "POST",
            channels_route,
            "missing-empty-or-conflict-state",
            response.status == "403 Forbidden"
        );
    }

    // Failed durable writes must return 500 and leave the in-memory pod
    // unchanged. A directory at the fixed pods.json path is a confined,
    // deterministic write failure in the temporary test state.
    {
        let (state, _receiver) = test_state_with_env(channel_env());
        let pod_id = "pod:channel-create-runtime";
        seed_pod!(&state, pod_id, serde_json::json!([]));
        let pods_path = state.config.state_dir.join("pods.json");
        fs::remove_file(&pods_path).expect("remove channel create state file");
        fs::create_dir(&pods_path).expect("block channel create state path");
        let response = crate::route_http_request(
            "POST",
            &format!("/api/v0/podcore/{pod_id}/channels"),
            None,
            r#"{"channelId":"runtime-create","name":"Runtime create"}"#,
            &state,
        )
        .await
        .expect("channel create runtime failure");
        let unchanged = state.pods.read().await.get(pod_id).is_some_and(|pod| {
            !pod.channels
                .iter()
                .any(|channel| channel.channel_id == "runtime-create")
        });
        record!(
            "POST",
            channels_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response
                    .body
                    .contains("An error occurred while creating the channel")
                && unchanged
        );
        fs::remove_dir(&pods_path).expect("remove blocked channel create state path");
    }

    {
        let (state, _receiver) = test_state_with_env(channel_env());
        let pod_id = "pod:channel-create-restart";
        seed_pod!(&state, pod_id, serde_json::json!([]));
        let created = crate::route_http_request(
            "POST",
            &format!("/api/v0/podcore/{pod_id}/channels"),
            None,
            r#"{"channelId":"restart-create","name":"Restart create"}"#,
            &state,
        )
        .await
        .expect("channel create restart fixture");
        let loaded = crate::pods::PodStore::load(&state.config.state_dir)
            .expect("reload channel create state");
        let persisted = loaded.get(pod_id).is_some_and(|pod| {
            pod.channels.iter().any(|channel| {
                channel.channel_id == "restart-create" && channel.name == "Restart create"
            })
        });
        record!(
            "POST",
            channels_route,
            "restart-persistence-or-reset",
            created.status == "201 Created" && persisted
        );
    }

    {
        let (state, _receiver) = test_state_with_env(channel_env());
        let pod_id = "pod:channel-create-concurrent";
        seed_pod!(&state, pod_id, serde_json::json!([]));
        let bodies = (0..4)
            .map(|index| {
                format!(
                    r#"{{"channelId":"concurrent-create-{index}","name":"Concurrent create {index}"}}"#
                )
            })
            .collect::<Vec<_>>();
        let responses = futures_util::future::join_all(bodies.iter().map(|body| {
            let path = format!("/api/v0/podcore/{pod_id}/channels");
            let body = body.clone();
            let state = Arc::clone(&state);
            async move { crate::route_http_request("POST", &path, None, &body, &state).await }
        }))
        .await;
        let loaded = crate::pods::PodStore::load(&state.config.state_dir)
            .expect("reload concurrent channel creates");
        let persisted = loaded.get(pod_id).is_some_and(|pod| {
            (0..4).all(|index| {
                pod.channels
                    .iter()
                    .any(|channel| channel.channel_id == format!("concurrent-create-{index}"))
            })
        });
        record!(
            "POST",
            channels_route,
            "concurrency-and-idempotency",
            responses.iter().all(|response| {
                response
                    .as_ref()
                    .is_ok_and(|response| response.status == "201 Created")
            }) && persisted
        );
    }

    // Update failure, reload, and concurrent updates use pre-existing
    // channels so the authorization and existence checks are identical
    // to the frozen controller's path.
    {
        let (state, _receiver) = test_state_with_env(channel_env());
        let pod_id = "pod:channel-update-runtime";
        seed_pod!(
            &state,
            pod_id,
            serde_json::json!([{"channelId":"runtime-update","kind":0,"name":"Before"}])
        );
        let pods_path = state.config.state_dir.join("pods.json");
        fs::remove_file(&pods_path).expect("remove channel update state file");
        fs::create_dir(&pods_path).expect("block channel update state path");
        let response = crate::route_http_request(
            "PUT",
            &format!("/api/v0/podcore/{pod_id}/channels/runtime-update"),
            None,
            r#"{"name":"After"}"#,
            &state,
        )
        .await
        .expect("channel update runtime failure");
        let unchanged = state
            .pods
            .read()
            .await
            .get(pod_id)
            .and_then(|pod| {
                pod.channels
                    .iter()
                    .find(|channel| channel.channel_id == "runtime-update")
                    .map(|channel| channel.name == "Before")
            })
            .unwrap_or(false);
        record!(
            "PUT",
            channel_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response
                    .body
                    .contains("An error occurred while updating the channel")
                && unchanged
        );
        fs::remove_dir(&pods_path).expect("remove blocked channel update state path");
    }

    {
        let (state, _receiver) = test_state_with_env(channel_env());
        let pod_id = "pod:channel-update-restart";
        seed_pod!(
            &state,
            pod_id,
            serde_json::json!([{"channelId":"restart-update","kind":0,"name":"Before"}])
        );
        let updated = crate::route_http_request(
            "PUT",
            &format!("/api/v0/podcore/{pod_id}/channels/restart-update"),
            None,
            r#"{"name":"Restarted update"}"#,
            &state,
        )
        .await
        .expect("channel update restart fixture");
        let loaded = crate::pods::PodStore::load(&state.config.state_dir)
            .expect("reload channel update state");
        let persisted = loaded.get(pod_id).is_some_and(|pod| {
            pod.channels.iter().any(|channel| {
                channel.channel_id == "restart-update" && channel.name == "Restarted update"
            })
        });
        record!(
            "PUT",
            channel_route,
            "restart-persistence-or-reset",
            updated.status == "200 OK" && persisted
        );
    }

    {
        let (state, _receiver) = test_state_with_env(channel_env());
        let pod_id = "pod:channel-update-concurrent";
        seed_pod!(
            &state,
            pod_id,
            serde_json::json!([
                {"channelId":"concurrent-update-0","kind":0,"name":"Before 0"},
                {"channelId":"concurrent-update-1","kind":0,"name":"Before 1"},
                {"channelId":"concurrent-update-2","kind":0,"name":"Before 2"},
                {"channelId":"concurrent-update-3","kind":0,"name":"Before 3"}
            ])
        );
        let responses = futures_util::future::join_all((0..4).map(|index| {
            let path = format!("/api/v0/podcore/{pod_id}/channels/concurrent-update-{index}");
            let body = format!(r#"{{"name":"After {index}"}}"#);
            let state = Arc::clone(&state);
            async move { crate::route_http_request("PUT", &path, None, &body, &state).await }
        }))
        .await;
        let loaded = crate::pods::PodStore::load(&state.config.state_dir)
            .expect("reload concurrent channel updates");
        let persisted = loaded.get(pod_id).is_some_and(|pod| {
            (0..4).all(|index| {
                pod.channels.iter().any(|channel| {
                    channel.channel_id == format!("concurrent-update-{index}")
                        && channel.name == format!("After {index}")
                })
            })
        });
        record!(
            "PUT",
            channel_route,
            "concurrency-and-idempotency",
            responses.iter().all(|response| {
                response
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
            }) && persisted
        );
    }

    // Delete failure, reload, and concurrent deletion mirror the update
    // checks and ensure a failed write does not remove the live channel.
    {
        let (state, _receiver) = test_state_with_env(channel_env());
        let pod_id = "pod:channel-delete-runtime";
        seed_pod!(
            &state,
            pod_id,
            serde_json::json!([{"channelId":"runtime-delete","kind":0,"name":"Keep"}])
        );
        let pods_path = state.config.state_dir.join("pods.json");
        fs::remove_file(&pods_path).expect("remove channel delete state file");
        fs::create_dir(&pods_path).expect("block channel delete state path");
        let response = crate::route_http_request(
            "DELETE",
            &format!("/api/v0/podcore/{pod_id}/channels/runtime-delete"),
            None,
            "",
            &state,
        )
        .await
        .expect("channel delete runtime failure");
        let unchanged = state.pods.read().await.get(pod_id).is_some_and(|pod| {
            pod.channels
                .iter()
                .any(|channel| channel.channel_id == "runtime-delete")
        });
        record!(
            "DELETE",
            channel_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response
                    .body
                    .contains("An error occurred while deleting the channel")
                && unchanged
        );
        fs::remove_dir(&pods_path).expect("remove blocked channel delete state path");
    }

    {
        let (state, _receiver) = test_state_with_env(channel_env());
        let pod_id = "pod:channel-delete-restart";
        seed_pod!(
            &state,
            pod_id,
            serde_json::json!([{"channelId":"restart-delete","kind":0,"name":"Delete"}])
        );
        let deleted = crate::route_http_request(
            "DELETE",
            &format!("/api/v0/podcore/{pod_id}/channels/restart-delete"),
            None,
            "",
            &state,
        )
        .await
        .expect("channel delete restart fixture");
        let loaded = crate::pods::PodStore::load(&state.config.state_dir)
            .expect("reload channel delete state");
        let removed = loaded.get(pod_id).is_some_and(|pod| {
            !pod.channels
                .iter()
                .any(|channel| channel.channel_id == "restart-delete")
        });
        record!(
            "DELETE",
            channel_route,
            "restart-persistence-or-reset",
            deleted.status == "200 OK" && removed
        );
    }

    {
        let (state, _receiver) = test_state_with_env(channel_env());
        let pod_id = "pod:channel-delete-concurrent";
        seed_pod!(
            &state,
            pod_id,
            serde_json::json!([
                {"channelId":"concurrent-delete-0","kind":0,"name":"Delete 0"},
                {"channelId":"concurrent-delete-1","kind":0,"name":"Delete 1"},
                {"channelId":"concurrent-delete-2","kind":0,"name":"Delete 2"},
                {"channelId":"concurrent-delete-3","kind":0,"name":"Delete 3"}
            ])
        );
        let responses = futures_util::future::join_all((0..4).map(|index| {
            let path = format!("/api/v0/podcore/{pod_id}/channels/concurrent-delete-{index}");
            let state = Arc::clone(&state);
            async move { crate::route_http_request("DELETE", &path, None, "", &state).await }
        }))
        .await;
        let loaded = crate::pods::PodStore::load(&state.config.state_dir)
            .expect("reload concurrent channel deletes");
        let removed = loaded.get(pod_id).is_some_and(|pod| {
            (0..4).all(|index| {
                !pod.channels
                    .iter()
                    .any(|channel| channel.channel_id == format!("concurrent-delete-{index}"))
            })
        });
        record!(
            "DELETE",
            channel_route,
            "concurrency-and-idempotency",
            responses.iter().all(|response| {
                response
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
            }) && removed
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("podcore_channel_lifecycle.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api podcore-channel-lifecycle mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting podcore opinion/affinity empty
/// getter contracts. The selected routes are limited to shapes that are
/// exact in the frozen oracle and slskR for an empty store: arrays for
/// opinion/recommendation reads and an object for member affinities.
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
pub(super) async fn controller_api_differential_podcore_opinion_empty_gets() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($route:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!("{target} GET {} [nominal]", $route));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "GET",
                "route": $route,
                "case": "nominal-status-headers-body",
                "pass": $pass,
            }));
        };
    }

    let (state, _receiver) = test_state();
    let pod_id = "pod:opinion-empty-differential";
    let content_id = "content-empty-differential";

    let opinions = crate::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/opinions/content/{content_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("empty content opinions");
    let opinions_json =
        serde_json::from_str::<serde_json::Value>(&opinions.body).unwrap_or_default();
    record!(
        "/api/v0/podcore/{podId}/opinions/content/{contentId}",
        opinions.status == "200 OK"
            && opinions.content_type == "application/json"
            && opinions_json == serde_json::json!([])
    );

    let recommendations = crate::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/opinions/content/{content_id}/recommendations"),
        None,
        "",
        &state,
    )
    .await
    .expect("empty opinion recommendations");
    let recommendations_json =
        serde_json::from_str::<serde_json::Value>(&recommendations.body).unwrap_or_default();
    record!(
        "/api/v0/podcore/{podId}/opinions/content/{contentId}/recommendations",
        recommendations.status == "200 OK"
            && recommendations.content_type == "application/json"
            && recommendations_json == serde_json::json!([])
    );

    let variant = crate::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/opinions/content/{content_id}/variant/variant-empty"),
        None,
        "",
        &state,
    )
    .await
    .expect("empty variant opinions");
    let variant_json = serde_json::from_str::<serde_json::Value>(&variant.body).unwrap_or_default();
    record!(
        "/api/v0/podcore/{podId}/opinions/content/{contentId}/variant/{variantHash}",
        variant.status == "200 OK"
            && variant.content_type == "application/json"
            && variant_json == serde_json::json!([])
    );

    let aggregated = crate::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/opinions/content/{content_id}/aggregated"),
        None,
        "",
        &state,
    )
    .await
    .expect("empty aggregated opinions");
    let aggregated_json =
        serde_json::from_str::<serde_json::Value>(&aggregated.body).unwrap_or_default();
    record!(
        "/api/v0/podcore/{podId}/opinions/content/{contentId}/aggregated",
        aggregated.status == "200 OK"
            && aggregated.content_type == "application/json"
            && aggregated_json["podId"] == pod_id
            && aggregated_json["contentId"] == content_id
            && aggregated_json["weightedAverageScore"] == 0.0
            && aggregated_json["unweightedAverageScore"] == 0.0
            && aggregated_json["totalOpinions"] == 0
            && aggregated_json["uniqueVariants"] == 0
            && aggregated_json["contributingMembers"] == 0
            && aggregated_json["consensusStrength"] == 0.0
            && aggregated_json["variantAggregates"]
                .as_array()
                .is_some_and(Vec::is_empty)
            && aggregated_json["memberContributions"]
                .as_object()
                .is_some_and(serde_json::Map::is_empty)
            && aggregated_json["lastUpdated"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
    );

    let stats = crate::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/opinions/content/{content_id}/stats"),
        None,
        "",
        &state,
    )
    .await
    .expect("empty opinion statistics");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap_or_default();
    record!(
        "/api/v0/podcore/{podId}/opinions/content/{contentId}/stats",
        stats.status == "200 OK"
            && stats.content_type == "application/json"
            && stats_json["podId"] == pod_id
            && stats_json["contentId"] == content_id
            && stats_json["totalOpinions"] == 0
            && stats_json["uniqueVariants"] == 0
            && stats_json["averageScore"] == 0.0
            && stats_json["minScore"] == 0.0
            && stats_json["maxScore"] == 0.0
            && stats_json["scoreDistribution"]
                .as_object()
                .is_some_and(serde_json::Map::is_empty)
            && stats_json["lastUpdated"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
    );

    let affinities = crate::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/opinions/members/affinity"),
        None,
        "",
        &state,
    )
    .await
    .expect("empty member affinities");
    let affinities_json =
        serde_json::from_str::<serde_json::Value>(&affinities.body).unwrap_or_default();
    record!(
        "/api/v0/podcore/{podId}/opinions/members/affinity",
        affinities.status == "200 OK"
            && affinities.content_type == "application/json"
            && affinities_json == serde_json::json!({})
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("podcore_opinion_empty_gets.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api podcore-opinion mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof for populated PodCore opinion/affinity getter
/// contracts. The fixture contains a real stored opinion, a pod member,
/// a content-discovery variant, and a shadow-index recommendation peer;
/// the assertions therefore cover non-empty opinion, variant, aggregate,
/// statistics, recommendation, and affinity projections rather than
/// only checking status codes. slskdN-only (confirmed against the frozen
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
    feature = "bounded-controller-api-tests-2"
))]
pub(super) async fn controller_api_differential_podcore_opinion_populated_gets() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($route:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!(
                    "{target} GET {} [populated-dynamic-state]",
                    $route
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "GET",
                "route": $route,
                "case": "populated-dynamic-state",
                "pass": $pass,
            }));
        };
    }

    let (state, _receiver) = test_state();
    let pod_id = "pod:opinion-populated-differential";
    let content_id = "content-populated-differential";
    let variant_hash = "variant-populated-differential";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<crate::pods::PodRecord>(serde_json::json!({
                "podId": pod_id,
                "name": "Opinion populated differential",
                "isPublic": true,
            }))
            .expect("deserialize populated opinion pod"),
            "opinion-member".to_owned(),
        )
        .expect("create populated opinion pod");
    {
        let mut discovery = state.content_discovery.write().await;
        discovery
            .merge_hash_entries(vec![crate::content_discovery::HashDbEntry {
                flac_key: "opinion-populated-key".to_owned(),
                size: 321,
                music_brainz_id: content_id.to_owned(),
                file_sha256: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                    .to_owned(),
                ..Default::default()
            }])
            .expect("seed populated opinion content");
        discovery
            .merge_shadow_records(vec![crate::content_discovery::ShadowIndexRecord {
                recording_id: content_id.to_owned(),
                peer_ids: vec!["recommendation-peer".to_owned()],
                updated_at: 1,
            }])
            .expect("seed populated opinion recommendation");
    }
    state
        .controller_features
        .write_for_test()
        .await
        .upsert(
            format!("pod/opinion/{pod_id}/{content_id}/opinion-populated"),
            serde_json::json!({
                "id": "opinion-populated",
                "podId": pod_id,
                "contentId": content_id,
                "variantHash": variant_hash,
                "score": 0.8,
                "note": "populated differential opinion",
                "senderPeerId": "opinion-member",
                "signature": "differential-signature",
            }),
        )
        .expect("seed populated opinion");

    let opinions = crate::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/opinions/content/{content_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("populated content opinions");
    let opinions_json =
        serde_json::from_str::<serde_json::Value>(&opinions.body).unwrap_or_default();
    record!(
        "/api/v0/podcore/{podId}/opinions/content/{contentId}",
        opinions.status == "200 OK"
            && opinions_json.as_array().is_some_and(|rows| rows.len() == 1)
            && opinions_json[0]["variantHash"] == variant_hash
            && opinions_json[0]["score"] == 0.8
    );

    let variant = crate::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/opinions/content/{content_id}/variant/{variant_hash}"),
        None,
        "",
        &state,
    )
    .await
    .expect("populated variant opinions");
    let variant_json = serde_json::from_str::<serde_json::Value>(&variant.body).unwrap_or_default();
    record!(
        "/api/v0/podcore/{podId}/opinions/content/{contentId}/variant/{variantHash}",
        variant.status == "200 OK"
            && variant_json.as_array().is_some_and(|rows| rows.len() == 1)
            && variant_json[0]["senderPeerId"] == "opinion-member"
    );

    let aggregated = crate::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/opinions/content/{content_id}/aggregated"),
        None,
        "",
        &state,
    )
    .await
    .expect("populated aggregated opinions");
    let aggregated_json =
        serde_json::from_str::<serde_json::Value>(&aggregated.body).unwrap_or_default();
    record!(
        "/api/v0/podcore/{podId}/opinions/content/{contentId}/aggregated",
        aggregated.status == "200 OK"
            && aggregated_json["totalOpinions"] == 1
            && aggregated_json["uniqueVariants"] == 1
            && aggregated_json["contributingMembers"] == 1
            && aggregated_json["weightedAverageScore"]
                .as_f64()
                .is_some_and(|value| (value - 0.8).abs() < 1e-9)
            && aggregated_json["variantAggregates"]
                .as_array()
                .is_some_and(|rows| rows.len() == 1)
            && aggregated_json["memberContributions"]
                .as_object()
                .is_some_and(|members| members.contains_key("opinion-member"))
            && aggregated_json["consensusStrength"]
                .as_f64()
                .is_some_and(|value| value > 0.0)
    );

    let recommendations = crate::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/opinions/content/{content_id}/recommendations"),
        None,
        "",
        &state,
    )
    .await
    .expect("populated opinion recommendations");
    let recommendations_json =
        serde_json::from_str::<serde_json::Value>(&recommendations.body).unwrap_or_default();
    record!(
        "/api/v0/podcore/{podId}/opinions/content/{contentId}/recommendations",
        recommendations.status == "200 OK"
            && recommendations_json == serde_json::json!(["recommendation-peer"])
    );

    let stats = crate::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/opinions/content/{content_id}/stats"),
        None,
        "",
        &state,
    )
    .await
    .expect("populated opinion statistics");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap_or_default();
    record!(
        "/api/v0/podcore/{podId}/opinions/content/{contentId}/stats",
        stats.status == "200 OK"
            && stats_json["totalOpinions"] == 1
            && stats_json["uniqueVariants"] == 1
            && stats_json["averageScore"].as_f64() == Some(0.8)
            && stats_json["minScore"].as_f64() == Some(0.8)
            && stats_json["maxScore"].as_f64() == Some(0.8)
            && stats_json["scoreDistribution"] == serde_json::json!({"0": 1})
    );

    let affinities = crate::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/opinions/members/affinity"),
        None,
        "",
        &state,
    )
    .await
    .expect("populated member affinities");
    let affinities_json =
        serde_json::from_str::<serde_json::Value>(&affinities.body).unwrap_or_default();
    record!(
        "/api/v0/podcore/{podId}/opinions/members/affinity",
        affinities.status == "200 OK"
            && affinities_json["opinion-member"]["peerId"] == "opinion-member"
            && affinities_json["opinion-member"]["affinityScore"]
                .as_f64()
                .is_some_and(|value| value > 0.0)
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("podcore_opinion_populated_gets.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api podcore-opinion-populated mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof for the PodCore opinion publication contract.
/// The publication response is the oracle's result DTO, and the
/// subsequent content read proves that a successful publication is stored
/// and observable through the sibling getter. slskdN-only (confirmed
/// against the frozen registry).
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
pub(super) async fn controller_api_differential_podcore_opinion_publish() {
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
    let pod_id = "pod:opinion-publish-differential";
    let content_id = "content-publish-differential";
    let variant_hash = "variant-publish-differential";
    let opinion_body = serde_json::json!({
        "contentId": content_id,
        "variantHash": variant_hash,
        "score": 0.9,
        "note": "published differential opinion",
        "senderPeerId": "opinion-publisher",
        "signature": format!(
            "ed25519:{}",
            base64::engine::general_purpose::STANDARD.encode([0_u8; 64])
        ),
    })
    .to_string();

    let published = crate::route_http_request(
        "POST",
        &format!("/api/v0/podcore/{pod_id}/opinions"),
        None,
        &opinion_body,
        &state,
    )
    .await
    .expect("publish opinion");
    let published_json =
        serde_json::from_str::<serde_json::Value>(&published.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/podcore/{podId}/opinions",
        "nominal-status-headers-body",
        published.status == "200 OK"
            && published.content_type == "application/json"
            && published_json["success"] == true
            && published_json["podId"] == pod_id
            && published_json["contentId"] == content_id
            && published_json["variantHash"] == variant_hash
            && published_json["publishedOpinion"]["contentId"] == content_id
            && published_json["publishedOpinion"]["variantHash"] == variant_hash
            && published_json["publishedOpinion"]["senderPeerId"] == "opinion-publisher"
    );

    let readback = crate::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/opinions/content/{content_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("read published opinion");
    let readback_json =
        serde_json::from_str::<serde_json::Value>(&readback.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/podcore/{podId}/opinions",
        "mutation-side-effects-and-readback",
        readback.status == "200 OK"
            && readback_json.as_array().is_some_and(|rows| rows.len() == 1)
            && readback_json[0]["contentId"] == content_id
            && readback_json[0]["variantHash"] == variant_hash
            && readback_json[0]["score"] == 0.9
    );

    let missing = crate::route_http_request(
        "POST",
        &format!("/api/v0/podcore/{pod_id}/opinions"),
        None,
        "{}",
        &state,
    )
    .await
    .expect("publish opinion with missing fields");
    let missing_json = serde_json::from_str::<serde_json::Value>(&missing.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/podcore/{podId}/opinions",
        "missing-empty-or-conflict-state",
        missing.status == "400 Bad Request"
            && missing.content_type == "application/json"
            && missing_json["error"] == "Content ID and variant hash are required"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("podcore_opinion_publish.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api podcore-opinion-publish mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
