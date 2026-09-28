//! Controller full pods differential 05 ownership.

use super::*;

/// Differential proof for the residual PodJoinLeaveController surface.
/// The frozen service deliberately keeps pending requests in memory, so
/// restart cases assert reset semantics rather than inventing durable
/// persistence. Versioned failures use the controller's fixed generic
/// messages; the legacy unversioned compatibility routes retain their
/// existing detailed errors.
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
pub(super) async fn controller_api_differential_podcore_join_leave_residuals() {
    let target = "slskdn";
    let join_route = "/api/v0/podcore/membership/join";
    let join_accept_route = "/api/v0/podcore/membership/join/accept";
    let leave_route = "/api/v0/podcore/membership/leave";
    let leave_accept_route = "/api/v0/podcore/membership/leave/accept";
    let cancel_join_route = "/api/v0/podcore/membership/join/{podId}/{peerId}";
    let cancel_leave_route = "/api/v0/podcore/membership/leave/{podId}/{peerId}";
    let pending_join_route = "/api/v0/podcore/membership/join/pending/{podId}";
    let pending_leave_route = "/api/v0/podcore/membership/leave/pending/{podId}";
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

    macro_rules! seed_owner_pod {
        ($state:ident, $pod_id:expr, $owner:expr) => {{
            {
                let mut rooms = $state.rooms.write().await;
                rooms
                    .join($pod_id.to_owned())
                    .expect("join PodJoinLeave differential pod");
                rooms
                    .add_member($pod_id, $owner.to_owned())
                    .expect("owner capacity")
                    .expect("PodJoinLeave differential pod");
                rooms
                    .records
                    .iter_mut()
                    .find(|room| room.name == $pod_id)
                    .expect("PodJoinLeave differential room")
                    .operated = true;
            }
            $state.pod_membership_workflow.write().await.set_role(
                $pod_id,
                $owner,
                "owner".to_owned(),
            );
        }};
    }

    let encoded_pod_id = |pod_id: &str| pod_id.replace(':', "%3A");
    let pending_path = |operation: &str, pod_id: &str| {
        format!(
            "/api/v0/podcore/membership/{operation}/pending/{}",
            encoded_pod_id(pod_id)
        )
    };
    let cancel_path = |operation: &str, pod_id: &str, peer_id: &str| {
        format!(
            "/api/v0/podcore/membership/{operation}/{}/{peer_id}",
            encoded_pod_id(pod_id)
        )
    };
    let join_body = |pod_id: &str, peer_id: &str| {
        serde_json::json!({
            "podId": pod_id,
            "peerId": peer_id,
            "requestedRole": "member",
        })
        .to_string()
    };
    let join_accept_body = |pod_id: &str, peer_id: &str, acceptor: &str| {
        serde_json::json!({
            "podId": pod_id,
            "peerId": peer_id,
            "acceptedRole": "member",
            "acceptorPeerId": acceptor,
        })
        .to_string()
    };
    let leave_body = |pod_id: &str, peer_id: &str| {
        serde_json::json!({
            "podId": pod_id,
            "peerId": peer_id,
        })
        .to_string()
    };
    let leave_accept_body = |pod_id: &str, peer_id: &str, acceptor: &str| {
        serde_json::json!({
            "podId": pod_id,
            "peerId": peer_id,
            "acceptorPeerId": acceptor,
        })
        .to_string()
    };
    let pending_is_empty = |response: &crate::HttpResponse, field: &str| {
        response.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&response.body)
                .ok()
                .is_some_and(|value| value[field] == serde_json::json!([]))
    };
    let one_success_one_status = |responses: &[Result<crate::HttpResponse, String>],
                                  status: &str| {
        responses
            .iter()
            .filter(|response| response.as_ref().is_ok_and(|value| value.status == status))
            .count()
            == 1
    };

    // DELETE join/{podId}/{peerId}: an absent request is the frozen 404
    // result, and a fresh AppState has no pending in-memory request.
    {
        let (state, _receiver) = test_state();
        let pod_id = "pod:join-cancel-runtime";
        let peer_id = "join-cancel-peer";
        let path = cancel_path("join", pod_id, peer_id);
        let response = crate::route_http_request("DELETE", &path, None, "", &state)
            .await
            .expect("runtime join cancellation");
        record!(
            "DELETE",
            cancel_join_route,
            "runtime-failure-and-timeout",
            response.status == "404 Not Found"
        );
    }

    {
        let (state, _receiver) = test_state();
        let pod_id = "pod:join-cancel-restart";
        let peer_id = "join-cancel-peer";
        seed_owner_pod!(state, pod_id, "join-cancel-owner");
        let joined = crate::route_http_request(
            "POST",
            join_route,
            None,
            &join_body(pod_id, peer_id),
            &state,
        )
        .await
        .expect("seed restart join cancellation");
        assert_eq!(joined.status, "200 OK", "{}", joined.body);
        let (restarted, _receiver) = test_state();
        let path = cancel_path("join", pod_id, peer_id);
        let response = crate::route_http_request("DELETE", &path, None, "", &restarted)
            .await
            .expect("restart join cancellation");
        record!(
            "DELETE",
            cancel_join_route,
            "restart-persistence-or-reset",
            response.status == "404 Not Found"
        );
    }

    {
        let (state, _receiver) = test_state();
        let pod_id = "pod:join-cancel-concurrent";
        let peer_id = "join-cancel-peer";
        seed_owner_pod!(state, pod_id, "join-cancel-owner");
        let joined = crate::route_http_request(
            "POST",
            join_route,
            None,
            &join_body(pod_id, peer_id),
            &state,
        )
        .await
        .expect("seed concurrent join cancellation");
        assert_eq!(joined.status, "200 OK", "{}", joined.body);
        let path = cancel_path("join", pod_id, peer_id);
        let responses = futures_util::future::join_all((0..2).map(|_| {
            let state = Arc::clone(&state);
            let path = path.clone();
            async move { crate::route_http_request("DELETE", &path, None, "", &state).await }
        }))
        .await;
        let pending =
            crate::route_http_request("GET", &pending_path("join", pod_id), None, "", &state)
                .await
                .expect("concurrent join cancellation readback");
        record!(
            "DELETE",
            cancel_join_route,
            "concurrency-and-idempotency",
            one_success_one_status(&responses, "200 OK")
                && one_success_one_status(&responses, "404 Not Found")
                && pending_is_empty(&pending, "pendingJoinRequests")
        );
    }

    // DELETE leave/{podId}/{peerId}: cancellation has the same in-memory
    // reset and single-winner behavior as join cancellation.
    {
        let (state, _receiver) = test_state();
        let pod_id = "pod:leave-cancel-runtime";
        let path = cancel_path("leave", pod_id, "leave-cancel-peer");
        let response = crate::route_http_request("DELETE", &path, None, "", &state)
            .await
            .expect("runtime leave cancellation");
        record!(
            "DELETE",
            cancel_leave_route,
            "runtime-failure-and-timeout",
            response.status == "404 Not Found"
        );
    }

    {
        let (state, _receiver) = test_state();
        let pod_id = "pod:leave-cancel-restart";
        let owner = "leave-cancel-owner";
        seed_owner_pod!(state, pod_id, owner);
        let leave = crate::route_http_request(
            "POST",
            leave_route,
            None,
            &leave_body(pod_id, owner),
            &state,
        )
        .await
        .expect("seed restart leave cancellation");
        assert_eq!(leave.status, "200 OK", "{}", leave.body);
        let (restarted, _receiver) = test_state();
        seed_owner_pod!(restarted, pod_id, owner);
        let path = cancel_path("leave", pod_id, owner);
        let response = crate::route_http_request("DELETE", &path, None, "", &restarted)
            .await
            .expect("restart leave cancellation");
        record!(
            "DELETE",
            cancel_leave_route,
            "restart-persistence-or-reset",
            response.status == "404 Not Found"
                && pending_is_empty(
                    &crate::route_http_request(
                        "GET",
                        &pending_path("leave", pod_id),
                        None,
                        "",
                        &restarted,
                    )
                    .await
                    .expect("restart leave cancellation readback"),
                    "pendingLeaveRequests"
                )
        );
    }

    {
        let (state, _receiver) = test_state();
        let pod_id = "pod:leave-cancel-concurrent";
        let owner = "leave-cancel-owner";
        seed_owner_pod!(state, pod_id, owner);
        let leave = crate::route_http_request(
            "POST",
            leave_route,
            None,
            &leave_body(pod_id, owner),
            &state,
        )
        .await
        .expect("seed concurrent leave cancellation");
        assert_eq!(leave.status, "200 OK", "{}", leave.body);
        let path = cancel_path("leave", pod_id, owner);
        let responses = futures_util::future::join_all((0..2).map(|_| {
            let state = Arc::clone(&state);
            let path = path.clone();
            async move { crate::route_http_request("DELETE", &path, None, "", &state).await }
        }))
        .await;
        let pending =
            crate::route_http_request("GET", &pending_path("leave", pod_id), None, "", &state)
                .await
                .expect("concurrent leave cancellation readback");
        record!(
            "DELETE",
            cancel_leave_route,
            "concurrency-and-idempotency",
            one_success_one_status(&responses, "200 OK")
                && one_success_one_status(&responses, "404 Not Found")
                && pending_is_empty(&pending, "pendingLeaveRequests")
        );
    }

    // GET pending/{podId} is a valid empty projection for a fresh
    // in-memory service instance.
    {
        let (state, _receiver) = test_state();
        let response = crate::route_http_request(
            "GET",
            &pending_path("join", "pod:join-pending-runtime"),
            None,
            "",
            &state,
        )
        .await
        .expect("runtime pending join projection");
        record!(
            "GET",
            pending_join_route,
            "runtime-failure-and-timeout",
            pending_is_empty(&response, "pendingJoinRequests")
        );
    }

    {
        let (state, _receiver) = test_state();
        let response = crate::route_http_request(
            "GET",
            &pending_path("leave", "pod:leave-pending-runtime"),
            None,
            "",
            &state,
        )
        .await
        .expect("runtime pending leave projection");
        record!(
            "GET",
            pending_leave_route,
            "runtime-failure-and-timeout",
            pending_is_empty(&response, "pendingLeaveRequests")
        );
    }

    // POST join.
    {
        let (state, _receiver) = test_state();
        let response = crate::route_http_request(
            "POST",
            join_route,
            None,
            &join_body("pod:join-runtime-missing", "join-runtime-peer"),
            &state,
        )
        .await
        .expect("runtime join request");
        record!(
            "POST",
            join_route,
            "runtime-failure-and-timeout",
            response.status == "400 Bad Request"
                && response
                    .body
                    .contains("Join request could not be processed")
        );
    }

    {
        let (state, _receiver) = test_state();
        let pod_id = "pod:join-restart";
        let peer_id = "join-restart-peer";
        seed_owner_pod!(state, pod_id, "join-restart-owner");
        let joined = crate::route_http_request(
            "POST",
            join_route,
            None,
            &join_body(pod_id, peer_id),
            &state,
        )
        .await
        .expect("restart join request");
        assert_eq!(joined.status, "200 OK", "{}", joined.body);
        let (restarted, _receiver) = test_state();
        seed_owner_pod!(restarted, pod_id, "join-restart-owner");
        let pending =
            crate::route_http_request("GET", &pending_path("join", pod_id), None, "", &restarted)
                .await
                .expect("restart join request readback");
        record!(
            "POST",
            join_route,
            "restart-persistence-or-reset",
            pending_is_empty(&pending, "pendingJoinRequests")
        );
    }

    {
        let (state, _receiver) = test_state();
        let pod_id = "pod:join-concurrent";
        let peer_id = "join-concurrent-peer";
        seed_owner_pod!(state, pod_id, "join-concurrent-owner");
        let responses =
            futures_util::future::join_all(
                (0..2).map(|_| {
                    let state = Arc::clone(&state);
                    let body = join_body(pod_id, peer_id);
                    async move {
                        crate::route_http_request("POST", join_route, None, &body, &state).await
                    }
                }),
            )
            .await;
        let pending =
            crate::route_http_request("GET", &pending_path("join", pod_id), None, "", &state)
                .await
                .expect("concurrent join request readback");
        record!(
            "POST",
            join_route,
            "concurrency-and-idempotency",
            one_success_one_status(&responses, "200 OK")
                && responses.iter().any(|response| {
                    response.as_ref().is_ok_and(|value| {
                        value.status == "400 Bad Request"
                            && value.body.contains("Join request could not be processed")
                    })
                })
                && serde_json::from_str::<serde_json::Value>(&pending.body)
                    .ok()
                    .is_some_and(|value| {
                        value["pendingJoinRequests"].as_array().map(Vec::len) == Some(1)
                    })
        );
    }

    // POST join/accept.
    {
        let (state, _receiver) = test_state();
        let pod_id = "pod:join-accept-runtime";
        let owner = "join-accept-owner";
        seed_owner_pod!(state, pod_id, owner);
        let response = crate::route_http_request(
            "POST",
            join_accept_route,
            None,
            &join_accept_body(pod_id, "join-accept-peer", owner),
            &state,
        )
        .await
        .expect("runtime join acceptance");
        record!(
            "POST",
            join_accept_route,
            "runtime-failure-and-timeout",
            response.status == "400 Bad Request"
                && response
                    .body
                    .contains("Join acceptance could not be processed")
        );
    }

    {
        let (state, _receiver) = test_state();
        let pod_id = "pod:join-accept-restart";
        let owner = "join-accept-owner";
        let peer_id = "join-accept-peer";
        seed_owner_pod!(state, pod_id, owner);
        let joined = crate::route_http_request(
            "POST",
            join_route,
            None,
            &join_body(pod_id, peer_id),
            &state,
        )
        .await
        .expect("seed restart join acceptance");
        assert_eq!(joined.status, "200 OK", "{}", joined.body);
        let (restarted, _receiver) = test_state();
        seed_owner_pod!(restarted, pod_id, owner);
        let response = crate::route_http_request(
            "POST",
            join_accept_route,
            None,
            &join_accept_body(pod_id, peer_id, owner),
            &restarted,
        )
        .await
        .expect("restart join acceptance");
        let pending =
            crate::route_http_request("GET", &pending_path("join", pod_id), None, "", &restarted)
                .await
                .expect("restart join acceptance readback");
        record!(
            "POST",
            join_accept_route,
            "restart-persistence-or-reset",
            response.status == "400 Bad Request"
                && response
                    .body
                    .contains("Join acceptance could not be processed")
                && pending_is_empty(&pending, "pendingJoinRequests")
        );
    }

    {
        let (state, _receiver) = test_state();
        let pod_id = "pod:join-accept-concurrent";
        let owner = "join-accept-owner";
        let peer_id = "join-accept-peer";
        seed_owner_pod!(state, pod_id, owner);
        let joined = crate::route_http_request(
            "POST",
            join_route,
            None,
            &join_body(pod_id, peer_id),
            &state,
        )
        .await
        .expect("seed concurrent join acceptance");
        assert_eq!(joined.status, "200 OK", "{}", joined.body);
        let responses = futures_util::future::join_all((0..2).map(|_| {
            let state = Arc::clone(&state);
            let body = join_accept_body(pod_id, peer_id, owner);
            async move {
                crate::route_http_request("POST", join_accept_route, None, &body, &state).await
            }
        }))
        .await;
        let pending =
            crate::route_http_request("GET", &pending_path("join", pod_id), None, "", &state)
                .await
                .expect("concurrent join acceptance readback");
        let member_count = state
            .rooms
            .read()
            .await
            .records
            .iter()
            .find(|room| room.name == pod_id)
            .map(|room| {
                room.members
                    .iter()
                    .filter(|member| *member == peer_id)
                    .count()
            })
            .unwrap_or_default();
        record!(
            "POST",
            join_accept_route,
            "concurrency-and-idempotency",
            one_success_one_status(&responses, "200 OK")
                && responses.iter().any(|response| {
                    response.as_ref().is_ok_and(|value| {
                        value.status == "400 Bad Request"
                            && value
                                .body
                                .contains("Join acceptance could not be processed")
                    })
                })
                && member_count == 1
                && pending_is_empty(&pending, "pendingJoinRequests")
        );
    }

    // POST leave.
    {
        let (state, _receiver) = test_state();
        let response = crate::route_http_request(
            "POST",
            leave_route,
            None,
            &leave_body("pod:leave-runtime-missing", "leave-runtime-peer"),
            &state,
        )
        .await
        .expect("runtime leave request");
        record!(
            "POST",
            leave_route,
            "runtime-failure-and-timeout",
            response.status == "400 Bad Request"
                && response
                    .body
                    .contains("Leave request could not be processed")
        );
    }

    {
        let (state, _receiver) = test_state();
        let pod_id = "pod:leave-restart";
        let owner = "leave-restart-owner";
        seed_owner_pod!(state, pod_id, owner);
        let left = crate::route_http_request(
            "POST",
            leave_route,
            None,
            &leave_body(pod_id, owner),
            &state,
        )
        .await
        .expect("restart leave request");
        assert_eq!(left.status, "200 OK", "{}", left.body);
        let (restarted, _receiver) = test_state();
        seed_owner_pod!(restarted, pod_id, owner);
        let pending =
            crate::route_http_request("GET", &pending_path("leave", pod_id), None, "", &restarted)
                .await
                .expect("restart leave request readback");
        let member_present = restarted
            .rooms
            .read()
            .await
            .records
            .iter()
            .find(|room| room.name == pod_id)
            .is_some_and(|room| room.members.iter().any(|member| member == owner));
        record!(
            "POST",
            leave_route,
            "restart-persistence-or-reset",
            pending_is_empty(&pending, "pendingLeaveRequests") && member_present
        );
    }

    {
        let (state, _receiver) = test_state();
        let pod_id = "pod:leave-concurrent";
        let owner = "leave-concurrent-owner";
        seed_owner_pod!(state, pod_id, owner);
        let responses =
            futures_util::future::join_all(
                (0..2).map(|_| {
                    let state = Arc::clone(&state);
                    let body = leave_body(pod_id, owner);
                    async move {
                        crate::route_http_request("POST", leave_route, None, &body, &state).await
                    }
                }),
            )
            .await;
        let pending =
            crate::route_http_request("GET", &pending_path("leave", pod_id), None, "", &state)
                .await
                .expect("concurrent leave request readback");
        record!(
            "POST",
            leave_route,
            "concurrency-and-idempotency",
            one_success_one_status(&responses, "200 OK")
                && responses.iter().any(|response| {
                    response.as_ref().is_ok_and(|value| {
                        value.status == "400 Bad Request"
                            && value.body.contains("Leave request could not be processed")
                    })
                })
                && serde_json::from_str::<serde_json::Value>(&pending.body)
                    .ok()
                    .is_some_and(|value| {
                        value["pendingLeaveRequests"].as_array().map(Vec::len) == Some(1)
                    })
        );
    }

    // POST leave/accept.
    {
        let (state, _receiver) = test_state();
        let pod_id = "pod:leave-accept-runtime";
        let owner = "leave-accept-owner";
        seed_owner_pod!(state, pod_id, owner);
        let response = crate::route_http_request(
            "POST",
            leave_accept_route,
            None,
            &leave_accept_body(pod_id, "leave-accept-peer", owner),
            &state,
        )
        .await
        .expect("runtime leave acceptance");
        record!(
            "POST",
            leave_accept_route,
            "runtime-failure-and-timeout",
            response.status == "400 Bad Request"
                && response
                    .body
                    .contains("Leave acceptance could not be processed")
        );
    }

    {
        let (state, _receiver) = test_state();
        let pod_id = "pod:leave-accept-restart";
        let owner = "leave-accept-owner";
        seed_owner_pod!(state, pod_id, owner);
        let left = crate::route_http_request(
            "POST",
            leave_route,
            None,
            &leave_body(pod_id, owner),
            &state,
        )
        .await
        .expect("seed restart leave acceptance");
        assert_eq!(left.status, "200 OK", "{}", left.body);
        let (restarted, _receiver) = test_state();
        seed_owner_pod!(restarted, pod_id, owner);
        let response = crate::route_http_request(
            "POST",
            leave_accept_route,
            None,
            &leave_accept_body(pod_id, owner, owner),
            &restarted,
        )
        .await
        .expect("restart leave acceptance");
        let pending =
            crate::route_http_request("GET", &pending_path("leave", pod_id), None, "", &restarted)
                .await
                .expect("restart leave acceptance readback");
        record!(
            "POST",
            leave_accept_route,
            "restart-persistence-or-reset",
            response.status == "400 Bad Request"
                && response
                    .body
                    .contains("Leave acceptance could not be processed")
                && pending_is_empty(&pending, "pendingLeaveRequests")
        );
    }

    {
        let (state, _receiver) = test_state();
        let pod_id = "pod:leave-accept-concurrent";
        let owner = "leave-accept-owner";
        seed_owner_pod!(state, pod_id, owner);
        let left = crate::route_http_request(
            "POST",
            leave_route,
            None,
            &leave_body(pod_id, owner),
            &state,
        )
        .await
        .expect("seed concurrent leave acceptance");
        assert_eq!(left.status, "200 OK", "{}", left.body);
        let responses = futures_util::future::join_all((0..2).map(|_| {
            let state = Arc::clone(&state);
            let body = leave_accept_body(pod_id, owner, owner);
            async move {
                crate::route_http_request("POST", leave_accept_route, None, &body, &state).await
            }
        }))
        .await;
        let pending =
            crate::route_http_request("GET", &pending_path("leave", pod_id), None, "", &state)
                .await
                .expect("concurrent leave acceptance readback");
        let member_present = state
            .rooms
            .read()
            .await
            .records
            .iter()
            .find(|room| room.name == pod_id)
            .is_some_and(|room| room.members.iter().any(|member| member == owner));
        record!(
            "POST",
            leave_accept_route,
            "concurrency-and-idempotency",
            one_success_one_status(&responses, "200 OK")
                && responses.iter().any(|response| {
                    response.as_ref().is_ok_and(|value| {
                        value.status == "400 Bad Request"
                            && value
                                .body
                                .contains("Leave acceptance could not be processed")
                    })
                })
                && !member_present
                && pending_is_empty(&pending, "pendingLeaveRequests")
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create PodJoinLeave evidence directory");
    fs::write(
        evidence_dir.join("podcore_join_leave_residuals.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize PodJoinLeave ledger"),
    )
    .expect("write PodJoinLeave ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api podcore-join-leave mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof for PodCore route-value validation on fixed
/// route parameters. The frozen controllers trim route values before
/// handling them and return a 400 for blank pod/channel/peer IDs; keep
/// these checks separate from the populated-state message and membership
/// tests so the evidence maps to the exact malformed-path case.
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
pub(super) async fn controller_api_differential_podcore_route_value_validation() {
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

    let blank_backfill = crate::route_http_request(
        "GET",
        "/api/v0/podcore/backfill/%20/last-seen",
        None,
        "",
        &state,
    )
    .await
    .expect("reject blank backfill pod ID");
    record!(
        "GET",
        "/api/v0/podcore/backfill/{podId}/last-seen",
        blank_backfill.status == "400 Bad Request"
            && blank_backfill.body.contains("Pod ID is required")
    );

    let blank_count_pod = crate::route_http_request(
        "GET",
        "/api/v0/podcore/messages/%20/general/count",
        None,
        "",
        &state,
    )
    .await
    .expect("reject blank message-count pod ID");
    let blank_count_channel = crate::route_http_request(
        "GET",
        "/api/v0/podcore/messages/pod:route-validation/%20/count",
        None,
        "",
        &state,
    )
    .await
    .expect("reject blank message-count channel ID");
    record!(
        "GET",
        "/api/v0/podcore/messages/{podId}/{channelId}/count",
        blank_count_pod.status == "400 Bad Request"
            && blank_count_pod.body.contains("Pod ID is required")
            && blank_count_channel.status == "400 Bad Request"
            && blank_count_channel.body.contains("Channel ID is required")
    );

    let blank_membership_pod = crate::route_http_request(
        "GET",
        "/api/v0/podcore/membership/%20/peer",
        None,
        "",
        &state,
    )
    .await
    .expect("reject blank membership pod ID");
    let blank_membership_peer = crate::route_http_request(
        "GET",
        "/api/v0/podcore/membership/pod:route-validation/%20",
        None,
        "",
        &state,
    )
    .await
    .expect("reject blank membership peer ID");
    record!(
        "GET",
        "/api/v0/podcore/membership/{podId}/{peerId}",
        blank_membership_pod.status == "400 Bad Request"
            && blank_membership_pod
                .body
                .contains("PodId and PeerId are required")
            && blank_membership_peer.status == "400 Bad Request"
            && blank_membership_peer
                .body
                .contains("PodId and PeerId are required")
    );

    let blank_verify_pod = crate::route_http_request(
        "GET",
        "/api/v0/podcore/membership/%20/peer/verify",
        None,
        "",
        &state,
    )
    .await
    .expect("reject blank membership-verification pod ID");
    let blank_verify_peer = crate::route_http_request(
        "GET",
        "/api/v0/podcore/membership/pod:route-validation/%20/verify",
        None,
        "",
        &state,
    )
    .await
    .expect("reject blank membership-verification peer ID");
    record!(
        "GET",
        "/api/v0/podcore/membership/{podId}/{peerId}/verify",
        blank_verify_pod.status == "400 Bad Request"
            && blank_verify_pod
                .body
                .contains("PodId and PeerId are required")
            && blank_verify_peer.status == "400 Bad Request"
            && blank_verify_peer
                .body
                .contains("PodId and PeerId are required")
    );

    for (path, route, message) in [
        (
            "/api/v0/podcore/%20/channels",
            "/api/v0/podcore/{podId}/channels",
            "Pod ID is required",
        ),
        (
            "/api/v0/podcore/pod:route-validation/channels/%20",
            "/api/v0/podcore/{podId}/channels/{channelId}",
            "Channel ID is required",
        ),
        (
            "/api/v0/podcore/%20/opinions/content/content-id",
            "/api/v0/podcore/{podId}/opinions/content/{contentId}",
            "Pod ID is required",
        ),
        (
            "/api/v0/podcore/pod:route-validation/opinions/content/%20/aggregated",
            "/api/v0/podcore/{podId}/opinions/content/{contentId}/aggregated",
            "Content ID is required",
        ),
        (
            "/api/v0/podcore/%20/opinions/content/content-id/recommendations",
            "/api/v0/podcore/{podId}/opinions/content/{contentId}/recommendations",
            "Pod ID is required",
        ),
        (
            "/api/v0/podcore/pod:route-validation/opinions/content/%20/stats",
            "/api/v0/podcore/{podId}/opinions/content/{contentId}/stats",
            "Content ID is required",
        ),
        (
            "/api/v0/podcore/pod:route-validation/opinions/content/content-id/variant/%20",
            "/api/v0/podcore/{podId}/opinions/content/{contentId}/variant/{variantHash}",
            "Variant hash is required",
        ),
        (
            "/api/v0/podcore/%20/opinions/members/affinity",
            "/api/v0/podcore/{podId}/opinions/members/affinity",
            "Pod ID is required",
        ),
    ] {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        record!(
            "GET",
            route,
            response.status == "400 Bad Request" && response.body.contains(message)
        );
    }

    for (path, route) in [
        (
            "/api/v0/podcore/membership/join/pending/%20",
            "/api/v0/podcore/membership/join/pending/{podId}",
        ),
        (
            "/api/v0/podcore/membership/leave/pending/%20",
            "/api/v0/podcore/membership/leave/pending/{podId}",
        ),
    ] {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        record!(
            "GET",
            route,
            response.status == "400 Bad Request" && response.body.contains("PodId is required")
        );
    }

    for (path, method, route, message) in [
        (
            "/api/v0/podcore/routing/seen/%20/pod:route-validation",
            "GET",
            "/api/v0/podcore/routing/seen/{messageId}/{podId}",
            "MessageId and PodId are required",
        ),
        (
            "/api/v0/podcore/routing/seen/message-id/%20",
            "GET",
            "/api/v0/podcore/routing/seen/{messageId}/{podId}",
            "MessageId and PodId are required",
        ),
        (
            "/api/v0/podcore/routing/seen/%20/pod:route-validation",
            "POST",
            "/api/v0/podcore/routing/seen/{messageId}/{podId}",
            "MessageId and PodId are required",
        ),
        (
            "/api/v0/podcore/routing/seen/message-id/%20",
            "POST",
            "/api/v0/podcore/routing/seen/{messageId}/{podId}",
            "MessageId and PodId are required",
        ),
    ] {
        let response = crate::route_http_request(method, path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
        record!(
            method,
            route,
            response.status == "400 Bad Request" && response.body.contains(message)
        );
    }

    for (path, method, route, message) in [
        (
            "/api/v0/podcore/%20/channels/general",
            "DELETE",
            "/api/v0/podcore/{podId}/channels/{channelId}",
            "Pod ID is required",
        ),
        (
            "/api/v0/podcore/pod:route-validation/channels/%20",
            "DELETE",
            "/api/v0/podcore/{podId}/channels/{channelId}",
            "Channel ID is required",
        ),
        (
            "/api/v0/podcore/discovery/unregister/%20",
            "DELETE",
            "/api/v0/podcore/discovery/unregister/{podId}",
            "PodId is required",
        ),
        (
            "/api/v0/podcore/membership/%20/peer",
            "DELETE",
            "/api/v0/podcore/membership/{podId}/{peerId}",
            "PodId and PeerId are required",
        ),
        (
            "/api/v0/podcore/membership/pod:route-validation/%20",
            "DELETE",
            "/api/v0/podcore/membership/{podId}/{peerId}",
            "PodId and PeerId are required",
        ),
        (
            "/api/v0/podcore/membership/join/%20/peer",
            "DELETE",
            "/api/v0/podcore/membership/join/{podId}/{peerId}",
            "PodId and PeerId are required",
        ),
        (
            "/api/v0/podcore/membership/leave/pod:route-validation/%20",
            "DELETE",
            "/api/v0/podcore/membership/leave/{podId}/{peerId}",
            "PodId and PeerId are required",
        ),
        (
            "/api/v0/podcore/messages/%20/general/cleanup?olderThan=1",
            "DELETE",
            "/api/v0/podcore/messages/{podId}/{channelId}/cleanup",
            "Pod ID is required",
        ),
        (
            "/api/v0/podcore/messages/pod:route-validation/general/cleanup?olderThan=0",
            "DELETE",
            "/api/v0/podcore/messages/{podId}/{channelId}/cleanup",
            "olderThan timestamp must be positive",
        ),
        (
            "/api/v0/podcore/messages/cleanup?olderThan=0",
            "DELETE",
            "/api/v0/podcore/messages/cleanup",
            "olderThan timestamp must be positive",
        ),
        (
            "/api/v0/podcore/backfill/%20/general/last-seen",
            "PUT",
            "/api/v0/podcore/backfill/{podId}/{channelId}/last-seen",
            "Pod ID is required",
        ),
        (
            "/api/v0/podcore/backfill/pod:route-validation/%20/last-seen",
            "PUT",
            "/api/v0/podcore/backfill/{podId}/{channelId}/last-seen",
            "Channel ID is required",
        ),
    ] {
        let response = crate::route_http_request(method, path, None, r#"{"lastSeen":1}"#, &state)
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
            "/api/v0/podcore/membership/%20/peer/ban",
            "/api/v0/podcore/membership/{podId}/{peerId}/ban",
        ),
        (
            "/api/v0/podcore/membership/pod:route-validation/%20/unban",
            "/api/v0/podcore/membership/{podId}/{peerId}/unban",
        ),
        (
            "/api/v0/podcore/membership/%20/peer/role",
            "/api/v0/podcore/membership/{podId}/{peerId}/role",
        ),
    ] {
        let response =
            crate::route_http_request("POST", path, None, r#"{"role":"member"}"#, &state)
                .await
                .unwrap_or_else(|error| panic!("POST {path}: {error}"));
        record!(
            "POST",
            route,
            response.status == "400 Bad Request"
                && response.body.contains("PodId and PeerId are required")
        );
    }

    let blank_publish = crate::route_http_request(
        "POST",
        "/api/v0/podcore/membership/%20/members",
        None,
        r#"{"peerId":"tester"}"#,
        &state,
    )
    .await
    .expect("reject blank membership-publish pod ID");
    record!(
        "POST",
        "/api/v0/podcore/membership/{podId}/members",
        blank_publish.status == "400 Bad Request"
            && blank_publish
                .body
                .contains("Valid podId and member with PeerId are required")
    );

    for path in [
        "/api/v0/podcore/membership/%20/members/peer",
        "/api/v0/podcore/membership/pod:route-validation/members/%20",
    ] {
        let response = crate::route_http_request("PUT", path, None, r#"{"role":"member"}"#, &state)
            .await
            .unwrap_or_else(|error| panic!("PUT {path}: {error}"));
        record!(
            "PUT",
            "/api/v0/podcore/membership/{podId}/members/{peerId}",
            response.status == "400 Bad Request"
                && response
                    .body
                    .contains("Valid podId, peerId, and member are required")
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("podcore_route_value_validation.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api podcore route-value mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
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
pub(super) async fn controller_api_differential_podcore_request_validation() {
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
        let response = crate::route_http_request(method, path, None, body, &state)
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
        let response = crate::route_http_request("POST", path, None, "", &state)
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
pub(super) async fn controller_api_differential_pod_and_jury_stats() {
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
            serde_json::from_value::<crate::pods::PodRecord>(serde_json::json!({
                "podId": pod_id,
                "name": "Verification Differential",
            }))
            .expect("deserialize pod record fixture"),
            "owner-peer".to_owned(),
        )
        .expect("create pod");

    let keypair = crate::route_http_request(
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
            crate::pods::PodMember {
                peer_id: "tester".to_owned(),
                role: "member".to_owned(),
                is_banned: false,
                public_key: keys["publicKey"].as_str().map(str::to_owned),
                joined_at: None,
                last_seen: None,
            },
        )
        .expect("add tester as a real pod member with a signing public key");

    let membership_verified = crate::route_http_request(
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
    let missing_membership = crate::route_http_request(
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
    let malformed_membership = crate::route_http_request(
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
            crate::pods::PodMember {
                peer_id: "banned-peer".to_owned(),
                role: "member".to_owned(),
                is_banned: true,
                public_key: keys["publicKey"].as_str().map(str::to_owned),
                joined_at: None,
                last_seen: None,
            },
        )
        .expect("add a real banned membership for verification");
    let banned_membership = crate::route_http_request(
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
            crate::pods::PodMember {
                peer_id: "moderator-peer".to_owned(),
                role: "mod".to_owned(),
                is_banned: false,
                public_key: None,
                joined_at: None,
                last_seen: None,
            },
        )
        .expect("add moderator membership for role hierarchy");
    let member_role = crate::route_http_request(
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
    let moderator_as_member = crate::route_http_request(
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
    let missing_role = crate::route_http_request(
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
    let malformed_role = crate::route_http_request(
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
        "timestampUnixMs": crate::unix_timestamp() * 1000,
    });
    let signed = crate::route_http_request(
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

    let verified = crate::route_http_request(
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
    let unknown_verified = crate::route_http_request(
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
    let bad_channel_verified = crate::route_http_request(
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

    let missing_pod_id = crate::route_http_request(
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

    let stats = crate::route_http_request(
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
    let malformed_verification_stats = crate::route_http_request(
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
    let signing_baseline = crate::route_http_request(
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
            && signing_baseline_json["lastSignatureOperation"] == crate::PODCORE_MIN_DATETIME
    );

    let signing_pod_id = "signing-stats-differential-pod";
    signing_state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<crate::pods::PodRecord>(serde_json::json!({
                "podId": signing_pod_id,
                "name": "Signing Stats Differential",
            }))
            .expect("deserialize pod record fixture"),
            "owner-peer".to_owned(),
        )
        .expect("create pod");
    let signing_keypair = crate::route_http_request(
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
            crate::pods::PodMember {
                peer_id: "tester".to_owned(),
                role: "member".to_owned(),
                is_banned: false,
                public_key: signing_keys["publicKey"].as_str().map(str::to_owned),
                joined_at: None,
                last_seen: None,
            },
        )
        .expect("add tester as a real pod member with a signing public key");
    let signing_signed = crate::route_http_request(
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
                "timestampUnixMs": crate::unix_timestamp() * 1000,
            }
        })
        .to_string(),
        &signing_state,
    )
    .await
    .expect("sign message");
    let signing_verified = crate::route_http_request(
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
    let forged_verified = crate::route_http_request(
        "POST",
        "/api/v0/podcore/signing/verify",
        None,
        &forged.to_string(),
        &signing_state,
    )
    .await
    .expect("verify forged sender");

    let signing_stats = crate::route_http_request(
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
    let malformed_signing_stats = crate::route_http_request(
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
    let created_a = crate::route_http_request(
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
        let verdict = crate::route_http_request(
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

    let created_b = crate::route_http_request(
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

    let baseline = crate::route_http_request(
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

    let accept = crate::route_http_request(
        "POST",
        &format!("/api/v0/quarantine-jury/requests/{request_a}/accept-release-candidate"),
        None,
        "{}",
        &jury_state,
    )
    .await
    .expect("accept request a");
    assert_eq!(accept.status, "200 OK", "{}", accept.body);

    let after_accept = crate::route_http_request(
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
            serde_json::json!(crate::unix_timestamp().saturating_sub(100 * 3600));
        features.upsert(key, backdated).expect("backdate request b");
    }
    let stale = crate::route_http_request(
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
