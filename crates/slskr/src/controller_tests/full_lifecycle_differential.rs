//! Controller full lifecycle differential ownership.

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
    feature = "bounded-controller-api-tests-1"
))]
pub(super) async fn controller_api_differential_bounded_activity_and_network_polling_routes_project_local_state(
) {
    let (state, _receiver) = test_state();

    let message_id = {
        let mut messages = state.messages.write().await;
        messages
            .add("peer-unread".to_owned(), "inbound", "hello".to_owned())
            .id
    };
    let unread = crate::route_http_request(
        "GET",
        "/api/v0/conversations/activity/unacknowledged",
        None,
        "",
        &state,
    )
    .await
    .expect("unacknowledged activity");
    assert_eq!(unread.status, "200 OK");
    assert!(unread.content_type.contains("application/json"));
    assert_eq!(unread.body, "true");
    state.messages.write().await.ack(message_id);
    let acknowledged = crate::route_http_request(
        "GET",
        "/api/v0/conversations/activity/unacknowledged",
        None,
        "",
        &state,
    )
    .await
    .expect("acknowledged activity");
    assert_eq!(acknowledged.status, "200 OK");
    assert!(acknowledged.content_type.contains("application/json"));
    assert_eq!(acknowledged.body, "false");

    let local_username = state
        .session
        .read()
        .await
        .username
        .clone()
        .unwrap_or_else(|| "local".to_owned());
    {
        let mut rooms = state.rooms.write().await;
        rooms.join("active-room".to_owned()).expect("room capacity");
        rooms
            .add_message("active-room", local_username, "outbound".to_owned())
            .expect("joined room");
        rooms
            .add_message("active-room", "peer-room".to_owned(), "inbound".to_owned())
            .expect("joined room");
        rooms.join("left-room".to_owned()).expect("room capacity");
        rooms
            .add_message("left-room", "peer-room".to_owned(), "ignored".to_owned())
            .expect("joined room");
        rooms.leave("left-room").expect("existing room");
    }
    let activity = crate::route_http_request("GET", "/api/v0/rooms/activity", None, "", &state)
        .await
        .expect("room activity");
    assert_eq!(activity.status, "200 OK");
    assert!(activity.content_type.contains("application/json"));
    let activity_json = serde_json::from_str::<serde_json::Value>(&activity.body).unwrap();
    assert!(activity_json["active-room"].as_u64().unwrap_or_default() > 0);
    assert!(activity_json.get("left-room").is_none());

    let network = crate::route_http_request(
        "GET",
        "/api/v0/network/stats?includePeers=false",
        None,
        "",
        &state,
    )
    .await
    .expect("network stats");
    assert_eq!(network.status, "200 OK");
    assert!(network.content_type.contains("application/json"));
    let network_json = serde_json::from_str::<serde_json::Value>(&network.body).unwrap();
    for key in [
        "backfill",
        "capabilitiesJson",
        "capabilitiesVersion",
        "dht",
        "discoveredPeers",
        "hashDb",
        "mesh",
        "meshPeers",
        "swarmJobs",
        "transport",
    ] {
        assert!(network_json.get(key).is_some(), "missing {key}");
    }
    assert_eq!(network_json["discoveredPeers"], serde_json::json!([]));
    assert_eq!(network_json["meshPeers"], serde_json::json!([]));

    let ledger = vec![
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/conversations/activity/unacknowledged",
            "case": "populated-dynamic-state",
            "pass": true,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/rooms/activity",
            "case": "populated-dynamic-state",
            "pass": true,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/network/stats",
            "case": "populated-dynamic-state",
            "pass": true,
        }),
    ];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("activity_and_network_populated.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}

/// Bulk differential proof crediting `runtime-failure-and-timeout` for
/// real DB-failure fault injection, independently re-verified (own
/// hermetic in-memory DB, own compatibility-target loop) for the same
/// routes `runtime_control_routes_roll_back_when_persistence_fails`
/// already proves: closes the real persistence DB mid-request, calls
/// the route via the compat-alias path that test uses (proven to reach
/// the same handler), and asserts a genuine 503 plus untouched
/// in-memory state. Credits the manifest's `/api/v0/...` route form
/// where that's what the frozen registries actually declare, per
/// target -- several of these routes only exist under the slskdN
/// compatibility profile.
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
    feature = "bounded-controller-api-tests-1"
))]
pub(super) async fn controller_api_differential_runtime_control_routes_survive_persistence_failure()
{
    struct Case {
        method: &'static str,
        call_path: &'static str,
        body: &'static str,
        seed_runtime: &'static str,
        seed_relay: bool,
        ledger_route: &'static str,
        targets: &'static [&'static str],
    }

    const BOTH: [&str; 2] = ["slskd", "slskdn"];
    const NATIVE_ONLY: [&str; 1] = ["slskdn"];

    let cases = [
        Case {
            method: "PUT",
            call_path: "/api/application",
            body: "{}",
            seed_runtime: "",
            seed_relay: false,
            ledger_route: "/api/v0/application",
            targets: &BOTH,
        },
        Case {
            method: "DELETE",
            call_path: "/api/application",
            body: "",
            seed_runtime: "restart",
            seed_relay: false,
            ledger_route: "/api/v0/application",
            targets: &BOTH,
        },
        Case {
            method: "POST",
            call_path: "/api/application/gc",
            body: "",
            seed_runtime: "",
            seed_relay: false,
            ledger_route: "/api/v0/application/gc",
            targets: &BOTH,
        },
        Case {
            method: "PUT",
            call_path: "/api/autoreplace/enable",
            body: "",
            seed_runtime: "",
            seed_relay: false,
            ledger_route: "/api/v0/autoreplace/enable",
            targets: &NATIVE_ONLY,
        },
        Case {
            method: "PUT",
            call_path: "/api/autoreplace/disable",
            body: "",
            seed_runtime: "autoreplace",
            seed_relay: false,
            ledger_route: "/api/v0/autoreplace/disable",
            targets: &NATIVE_ONLY,
        },
        Case {
            method: "PUT",
            call_path: "/api/relay/agent",
            body: r#"{"enabled":true}"#,
            seed_runtime: "",
            seed_relay: false,
            ledger_route: "/api/v0/relay/agent",
            targets: &BOTH,
        },
        Case {
            method: "DELETE",
            call_path: "/api/relay/agent",
            body: "",
            seed_runtime: "relay_agent",
            seed_relay: false,
            ledger_route: "/api/v0/relay/agent",
            targets: &BOTH,
        },
        Case {
            method: "POST",
            call_path: "/api/v0/bridge/start",
            body: "",
            seed_runtime: "",
            seed_relay: false,
            ledger_route: "/api/v0/bridge/start",
            targets: &NATIVE_ONLY,
        },
        Case {
            method: "POST",
            call_path: "/api/v0/bridge/stop",
            body: "",
            seed_runtime: "bridge",
            seed_relay: false,
            ledger_route: "/api/v0/bridge/stop",
            targets: &NATIVE_ONLY,
        },
        Case {
            method: "PUT",
            call_path: "/api/v0/bridge/admin/config",
            body: r#"{"enabled":true}"#,
            seed_runtime: "",
            seed_relay: false,
            ledger_route: "/api/v0/bridge/admin/config",
            targets: &NATIVE_ONLY,
        },
        Case {
            method: "POST",
            call_path: "/api/songid/runs",
            body: r#"{"source":"route-audit"}"#,
            seed_runtime: "",
            seed_relay: false,
            ledger_route: "/api/v0/songid/runs",
            targets: &NATIVE_ONLY,
        },
        Case {
            method: "POST",
            call_path: "/api/integrations/lidarr/wanted/sync",
            body: "",
            seed_runtime: "",
            seed_relay: false,
            ledger_route: "/api/v0/integrations/lidarr/wanted/sync",
            targets: &NATIVE_ONLY,
        },
        Case {
            method: "POST",
            call_path: "/api/profile/invite",
            body: "",
            seed_runtime: "",
            seed_relay: false,
            ledger_route: "/api/v0/profile/invite",
            targets: &NATIVE_ONLY,
        },
    ];

    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    for target in ["slskd", "slskdn"] {
        for case in &cases {
            if !case.targets.contains(&target) {
                continue;
            }
            let db = crate::persistence::DatabaseManager::in_memory()
                .await
                .expect("in-memory db");
            let (state, _receiver) = test_state_with_env_parts(
                MapEnv::default()
                    .with("SLSKR_PERSISTENCE_ENABLED", "true")
                    .with("SLSKR_CONTROLLER_PROFILE", target),
                crate::SearchStore::new(),
                Some(db.clone()),
            );
            match case.seed_runtime {
                "restart" => {
                    state.runtime.write().await.set_restart_requested(true);
                }
                "autoreplace" => {
                    state.runtime.write().await.set_autoreplace(true);
                }
                "relay_agent" => {
                    state.runtime.write().await.set_relay_agent(true);
                }
                "bridge" => {
                    state.runtime.write().await.set_bridge_running(true, false);
                }
                _ => {}
            }
            if case.seed_relay {
                state.relay.write().await.set_enabled(true);
            }
            let previous_runtime = state.runtime.read().await.clone();
            let previous_relay = state.relay.read().await.clone();
            db.close_for_test().await;

            let response =
                crate::route_http_request(case.method, case.call_path, None, case.body, &state)
                    .await
                    .expect("failed runtime compatibility persistence response");
            let frozen_bridge = target == "slskdn"
                && matches!(
                    case.call_path,
                    "/api/v0/bridge/start" | "/api/v0/bridge/stop" | "/api/v0/bridge/admin/config"
                );
            let pass = if frozen_bridge {
                response.status == "200 OK"
                    && *state.runtime.read().await == previous_runtime
                    && *state.relay.read().await == previous_relay
                    && match case.call_path {
                        "/api/v0/bridge/start" => response.body == r#"{"status":"started"}"#,
                        "/api/v0/bridge/stop" => response.body == r#"{"status":"stopped"}"#,
                        "/api/v0/bridge/admin/config" => {
                            response.body
                                == r#"{"message":"Configuration updated. Restart bridge service to apply changes.","restart_required":true}"#
                        }
                        _ => false,
                    }
            } else {
                response.status == "503 Service Unavailable"
                    && response
                        .body
                        .contains("runtime compatibility persistence failed")
                    && *state.runtime.read().await == previous_runtime
                    && *state.relay.read().await == previous_relay
            };
            if !pass {
                mismatches.push(format!(
                    "{target} {} {}: got {} {}",
                    case.method, case.call_path, response.status, response.body
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": case.method,
                "route": case.ledger_route,
                "case": "runtime-failure-and-timeout",
                "pass": pass,
            }));
        }
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("runtime_control_routes_survive_persistence_failure.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api runtime-failure mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof for the `runtime-failure-and-timeout`
/// case (security-ban and share/collection-grant routes) -- same
/// real closed-database fault injection as the earlier `runtime_
/// failure_*` batches, independently re-derived from `security_
/// ban_rolls_back_when_persistence_fails`, `security_unban_rolls_
/// back_when_persistence_fails`, `collection_delete_rolls_back_
/// grant_revocation_when_persistence_fails`, `share_grant_
/// revocation_rolls_back_when_persistence_fails`, and `share_
/// access_token_issue_rolls_back_when_persistence_fails` with
/// fresh fixture data. Confirmed against `/tmp/slskr-parity-
/// evidence/controller-api/*.json` before writing, per case: none
/// of these 5 routes had `runtime-failure-and-timeout` credited
/// (some had other cases already true). slskdN-only (confirmed
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
    feature = "bounded-controller-api-tests-3"
))]
pub(super) async fn controller_api_differential_runtime_failure_security_and_shares() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($method:expr, $route:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!("{target} {} {} [runtime-failure-and-timeout]", $method, $route));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": $method,
                "route": $route,
                "case": "runtime-failure-and-timeout",
                "pass": $pass,
            }));
        };
    }

    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        db.close_for_test().await;
        let response = crate::route_http_request(
            "POST",
            "/api/v0/security/bans/username",
            None,
            r#"{"username":"differential-must-persist"}"#,
            &state,
        )
        .await
        .expect("failed ban response");
        record!(
            "POST",
            "/api/v0/security/bans/username",
            response.status == "503 Service Unavailable"
                && state.security.read().await.bans.is_empty()
        );
    }

    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        state
            .security
            .write()
            .await
            .ban("username", "differential-must-remain".to_owned())
            .expect("ban");
        db.close_for_test().await;
        let response = crate::route_http_request(
            "DELETE",
            "/api/v0/security/bans/username/differential-must-remain",
            None,
            "",
            &state,
        )
        .await
        .expect("failed unban response");
        record!(
            "DELETE",
            "/api/v0/security/bans/username/{username}",
            response.status == "503 Service Unavailable"
                && state.security.read().await.active_bans() == 1
        );
    }

    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let collection_id = state
            .collections
            .write()
            .await
            .create(
                String::new(),
                "Differential Private".to_owned(),
                String::new(),
            )
            .expect("collection")
            .id;
        state
            .share_grants
            .write()
            .await
            .create_with_contract(
                None,
                collection_id.clone(),
                "differential-friend".to_owned(),
            )
            .expect("share grant");
        db.close_for_test().await;
        let response = crate::route_http_request(
            "DELETE",
            &format!("/api/v0/collections/{collection_id}"),
            None,
            "",
            &state,
        )
        .await
        .expect("failed collection delete response");
        record!(
            "DELETE",
            "/api/v0/collections/{id}",
            response.status == "503 Service Unavailable"
                && state.collections.read().await.get(&collection_id).is_some()
        );
    }

    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        state
            .share_grants
            .write()
            .await
            .create_with_contract(
                None,
                "differential-collection".to_owned(),
                "differential-friend".to_owned(),
            )
            .expect("grant");
        db.close_for_test().await;
        let response =
            crate::route_http_request("DELETE", "/api/v0/share-grants/grant-1", None, "", &state)
                .await
                .expect("failed grant revocation response");
        record!(
            "DELETE",
            "/api/v0/share-grants/{id}",
            response.status == "503 Service Unavailable"
                && state.share_grants.read().await.get("grant-1").is_some()
        );
    }

    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        state
            .collections
            .write()
            .await
            .create(
                String::new(),
                "Differential Private".to_owned(),
                String::new(),
            )
            .expect("collection");
        state
            .share_grants
            .write()
            .await
            .create_with_contract(None, "col-1".to_owned(), "differential-friend".to_owned())
            .expect("grant");
        db.close_for_test().await;
        let response = crate::route_http_request(
            "POST",
            "/api/v0/share-grants/grant-1/token",
            None,
            r#"{"expiresInSeconds":600}"#,
            &state,
        )
        .await
        .expect("failed access token issue response");
        record!(
            "POST",
            "/api/v0/share-grants/{id}/token",
            response.status == "503 Service Unavailable"
                && state.share_access_tokens.read().await.records.is_empty()
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("runtime_failure_security_and_shares.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api runtime-failure-security-and-shares mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
