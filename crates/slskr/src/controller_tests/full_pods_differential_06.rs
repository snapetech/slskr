//! Controller full pods differential 06 ownership.

use super::*;

/// Bulk differential proof for the real slskdN pod-membership
/// self-publish routes (`POST /api/v0/podcore/membership/{podId}/
/// members`, `PUT /api/v0/podcore/membership/{podId}/members/
/// {peerId}`): impersonation (publishing membership for a peer
/// other than the caller) and non-self/non-moderator updates are
/// genuinely rejected, and a self-publish/self-update that tries to
/// escalate its own role or clear its ban via the request body is
/// accepted but the role is pinned back to the real stored value,
/// not taken from client input -- independently re-derived from
/// `pod_membership_add_requires_self` and `pod_membership_update_
/// requires_moderator_or_self_and_pins_role` with fresh fixture
/// data. Confirmed against `/tmp/slskr-parity-evidence/
/// controller-api/*.json` before writing: neither route had any
/// prior case credited. slskdN-only (confirmed against the frozen
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
pub(super) async fn controller_api_differential_pod_membership_self_publish() {
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

    let pod_id = "pod:differential-membership-self-publish";
    let (state, _receiver) =
        pod_fixture_with_local_role("differential-local-peer", "differential-owner", pod_id).await;

    let impersonation = crate::route_http_request(
        "POST",
        &format!("/api/v0/podcore/membership/{pod_id}/members"),
        None,
        r#"{"peerId":"differential-someone-else","role":"member","isBanned":false}"#,
        &state,
    )
    .await
    .expect("impersonation response");
    record!(
        "POST",
        "/api/v0/podcore/membership/{podId}/members",
        "missing-empty-or-conflict-state",
        impersonation.status == "403 Forbidden"
    );

    let escalation = crate::route_http_request(
        "POST",
        &format!("/api/v0/podcore/membership/{pod_id}/members"),
        None,
        r#"{"peerId":"differential-local-peer","role":"mod","isBanned":false}"#,
        &state,
    )
    .await
    .expect("self-publish escalation response");
    let escalation_json =
        serde_json::from_str::<serde_json::Value>(&escalation.body).unwrap_or_default();
    let escalation_state_is_pinned = state
        .pods
        .read()
        .await
        .member_for_verification(pod_id, "differential-local-peer")
        .is_some_and(|member| member.role == "member");
    record!(
        "POST",
        "/api/v0/podcore/membership/{podId}/members",
        "mutation-side-effects-and-readback",
        escalation.status == "200 OK"
            && escalation_json["success"] == true
            && escalation_json["podId"] == pod_id
            && escalation_json["peerId"] == "differential-local-peer"
            && escalation_state_is_pinned
    );
    record!(
        "POST",
        "/api/v0/podcore/membership/{podId}/members",
        "nominal-status-headers-body",
        escalation.status == "200 OK"
            && escalation_json["success"] == true
            && escalation_json["dhtKey"] == format!("pod:{pod_id}:member:differential-local-peer")
            && escalation_json["publishedAt"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
            && escalation_json["expiresAt"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
    );

    state
        .pods
        .write()
        .await
        .upsert_member(
            pod_id,
            crate::pods::PodMember {
                peer_id: "differential-local-peer".to_owned(),
                role: "member".to_owned(),
                is_banned: false,
                public_key: None,
                joined_at: None,
                last_seen: None,
            },
        )
        .expect("add local peer as ordinary member");
    state
        .pods
        .write()
        .await
        .upsert_member(
            pod_id,
            crate::pods::PodMember {
                peer_id: "differential-target-member".to_owned(),
                role: "member".to_owned(),
                is_banned: false,
                public_key: None,
                joined_at: None,
                last_seen: None,
            },
        )
        .expect("add target member");

    let denied = crate::route_http_request(
        "PUT",
        &format!("/api/v0/podcore/membership/{pod_id}/members/differential-target-member"),
        None,
        r#"{"role":"member","isBanned":false}"#,
        &state,
    )
    .await
    .expect("non-moderator update response");
    record!(
        "PUT",
        "/api/v0/podcore/membership/{podId}/members/{peerId}",
        "missing-empty-or-conflict-state",
        denied.status == "403 Forbidden"
    );

    let self_update = crate::route_http_request(
        "PUT",
        &format!("/api/v0/podcore/membership/{pod_id}/members/differential-local-peer"),
        None,
        r#"{"role":"mod","isBanned":false}"#,
        &state,
    )
    .await
    .expect("self-update response");
    let self_update_json =
        serde_json::from_str::<serde_json::Value>(&self_update.body).unwrap_or_default();
    let self_update_state_is_pinned = state
        .pods
        .read()
        .await
        .member_for_verification(pod_id, "differential-local-peer")
        .is_some_and(|member| member.role == "member");
    record!(
        "PUT",
        "/api/v0/podcore/membership/{podId}/members/{peerId}",
        "mutation-side-effects-and-readback",
        self_update.status == "200 OK"
            && self_update_json["success"] == true
            && self_update_json["podId"] == pod_id
            && self_update_json["peerId"] == "differential-local-peer"
            && self_update_state_is_pinned
    );
    record!(
        "PUT",
        "/api/v0/podcore/membership/{podId}/members/{peerId}",
        "nominal-status-headers-body",
        self_update.status == "200 OK"
            && self_update_json["success"] == true
            && self_update_json["dhtKey"] == format!("pod:{pod_id}:member:differential-local-peer")
            && self_update_json["publishedAt"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
            && self_update_json["expiresAt"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
    );

    let local_membership = crate::route_http_request(
        "GET",
        &format!("/api/v0/podcore/membership/{pod_id}/differential-local-peer"),
        None,
        "",
        &state,
    )
    .await
    .expect("local membership response");
    let local_membership_json =
        serde_json::from_str::<serde_json::Value>(&local_membership.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/podcore/membership/{podId}/{peerId}",
        "nominal-status-headers-body",
        local_membership.status == "200 OK"
            && local_membership_json["found"] == true
            && local_membership_json["podId"] == pod_id
            && local_membership_json["peerId"] == "differential-local-peer"
            && local_membership_json["signedRecord"].is_object()
            && local_membership_json["isValidSignature"] == true
    );

    let target_membership = crate::route_http_request(
        "GET",
        &format!("/api/v0/podcore/membership/{pod_id}/differential-target-member"),
        None,
        "",
        &state,
    )
    .await
    .expect("target membership response");
    let target_membership_json =
        serde_json::from_str::<serde_json::Value>(&target_membership.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/podcore/membership/{podId}/{peerId}",
        "populated-dynamic-state",
        target_membership.status == "200 OK"
            && target_membership_json["found"] == true
            && target_membership_json["signedRecord"]["peerId"] == "differential-target-member"
            && target_membership_json["signedRecord"]["action"] == "join"
    );

    let missing_membership = crate::route_http_request(
        "GET",
        &format!("/api/v0/podcore/membership/{pod_id}/differential-missing-member"),
        None,
        "",
        &state,
    )
    .await
    .expect("missing membership response");
    let missing_membership_json =
        serde_json::from_str::<serde_json::Value>(&missing_membership.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/podcore/membership/{podId}/{peerId}",
        "missing-empty-or-conflict-state",
        missing_membership.status == "404 Not Found"
            && missing_membership_json["found"] == false
            && missing_membership_json["error"] == "Membership not found"
    );

    let local_verification = crate::route_http_request(
        "GET",
        &format!("/api/v0/podcore/membership/{pod_id}/differential-local-peer/verify"),
        None,
        "",
        &state,
    )
    .await
    .expect("local membership verification response");
    let local_verification_json =
        serde_json::from_str::<serde_json::Value>(&local_verification.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/podcore/membership/{podId}/{peerId}/verify",
        "nominal-status-headers-body",
        local_verification.status == "200 OK"
            && local_verification_json["isValidMember"] == true
            && local_verification_json["isBanned"] == false
            && local_verification_json["role"] == "member"
    );

    let target_verification = crate::route_http_request(
        "GET",
        &format!("/api/v0/podcore/membership/{pod_id}/differential-target-member/verify"),
        None,
        "",
        &state,
    )
    .await
    .expect("target membership verification response");
    let target_verification_json =
        serde_json::from_str::<serde_json::Value>(&target_verification.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/podcore/membership/{podId}/{peerId}/verify",
        "populated-dynamic-state",
        target_verification.status == "200 OK"
            && target_verification_json["isValidMember"] == true
            && target_verification_json["role"] == "member"
    );

    let missing_verification = crate::route_http_request(
        "GET",
        &format!("/api/v0/podcore/membership/{pod_id}/differential-missing-member/verify"),
        None,
        "",
        &state,
    )
    .await
    .expect("missing membership verification response");
    let missing_verification_json =
        serde_json::from_str::<serde_json::Value>(&missing_verification.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/podcore/membership/{podId}/{peerId}/verify",
        "missing-empty-or-conflict-state",
        missing_verification.status == "200 OK"
            && missing_verification_json["isValidMember"] == false
            && missing_verification_json["isBanned"] == false
            && missing_verification_json["role"].is_null()
            && missing_verification_json["errorMessage"] == "Membership not found"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("pod_membership_self_publish.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api pod-membership-self-publish mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof for the slskdN PodMembership moderation
/// publication routes (`POST /api/v0/podcore/membership/{podId}/
/// {peerId}/{ban|unban|role}`): successful operations return the
/// frozen `MembershipPublishResult` shape, missing members fail as a
/// service error, and the persisted member state reflects ban/unban/
/// role changes. slskdN-only (confirmed against the frozen registry).
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
pub(super) async fn controller_api_differential_pod_membership_moderation_publish() {
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

    let pod_id = "pod:differential-membership-moderation";
    let (state, _receiver) =
        pod_fixture_with_local_role("differential-moderator", "differential-moderator", pod_id)
            .await;
    for peer_id in ["differential-ban-member", "differential-role-member"] {
        state
            .pods
            .write()
            .await
            .upsert_member(
                pod_id,
                crate::pods::PodMember {
                    peer_id: peer_id.to_owned(),
                    role: "member".to_owned(),
                    is_banned: false,
                    public_key: None,
                    joined_at: None,
                    last_seen: None,
                },
            )
            .expect("add moderation target member");
    }

    let publish_result_matches = |response: &crate::HttpResponse, peer_id: &str| {
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        response.status == "200 OK"
            && value["success"] == true
            && value["podId"] == pod_id
            && value["peerId"] == peer_id
            && value["dhtKey"] == format!("pod:{pod_id}:member:{peer_id}")
            && value["publishedAt"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
            && value["expiresAt"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
    };

    let ban_route = "/api/v0/podcore/membership/{podId}/{peerId}/ban";
    let ban = crate::route_http_request(
        "POST",
        &format!("/api/v0/podcore/membership/{pod_id}/differential-ban-member/ban"),
        None,
        "",
        &state,
    )
    .await
    .expect("ban response");
    record!(
        "POST",
        ban_route,
        "nominal-status-headers-body",
        publish_result_matches(&ban, "differential-ban-member")
    );
    let ban_verify = crate::route_http_request(
        "GET",
        &format!("/api/v0/podcore/membership/{pod_id}/differential-ban-member/verify"),
        None,
        "",
        &state,
    )
    .await
    .expect("banned member verification response");
    let ban_verify_json =
        serde_json::from_str::<serde_json::Value>(&ban_verify.body).unwrap_or_default();
    record!(
        "POST",
        ban_route,
        "mutation-side-effects-and-readback",
        ban_verify.status == "200 OK"
            && ban_verify_json["isValidMember"] == false
            && ban_verify_json["isBanned"] == true
    );

    let ban_missing = crate::route_http_request(
        "POST",
        &format!("/api/v0/podcore/membership/{pod_id}/differential-missing-member/ban"),
        None,
        "",
        &state,
    )
    .await
    .expect("missing ban response");
    record!(
        "POST",
        ban_route,
        "missing-empty-or-conflict-state",
        ban_missing.status == "500 Internal Server Error"
            && ban_missing.body.contains("Failed to update membership")
    );

    let unban_route = "/api/v0/podcore/membership/{podId}/{peerId}/unban";
    let unban = crate::route_http_request(
        "POST",
        &format!("/api/v0/podcore/membership/{pod_id}/differential-ban-member/unban"),
        None,
        "",
        &state,
    )
    .await
    .expect("unban response");
    record!(
        "POST",
        unban_route,
        "nominal-status-headers-body",
        publish_result_matches(&unban, "differential-ban-member")
    );
    let unban_verify = crate::route_http_request(
        "GET",
        &format!("/api/v0/podcore/membership/{pod_id}/differential-ban-member/verify"),
        None,
        "",
        &state,
    )
    .await
    .expect("unbanned member verification response");
    let unban_verify_json =
        serde_json::from_str::<serde_json::Value>(&unban_verify.body).unwrap_or_default();
    record!(
        "POST",
        unban_route,
        "mutation-side-effects-and-readback",
        unban_verify.status == "200 OK"
            && unban_verify_json["isValidMember"] == true
            && unban_verify_json["isBanned"] == false
    );

    let unban_missing = crate::route_http_request(
        "POST",
        &format!("/api/v0/podcore/membership/{pod_id}/differential-missing-member/unban"),
        None,
        "",
        &state,
    )
    .await
    .expect("missing unban response");
    record!(
        "POST",
        unban_route,
        "missing-empty-or-conflict-state",
        unban_missing.status == "500 Internal Server Error"
            && unban_missing.body.contains("Failed to update membership")
    );

    let role_route = "/api/v0/podcore/membership/{podId}/{peerId}/role";
    let role = crate::route_http_request(
        "POST",
        &format!("/api/v0/podcore/membership/{pod_id}/differential-role-member/role"),
        None,
        r#"{"role":"mod"}"#,
        &state,
    )
    .await
    .expect("role response");
    record!(
        "POST",
        role_route,
        "nominal-status-headers-body",
        publish_result_matches(&role, "differential-role-member")
    );
    let role_verify = crate::route_http_request(
        "GET",
        &format!("/api/v0/podcore/membership/{pod_id}/differential-role-member/verify"),
        None,
        "",
        &state,
    )
    .await
    .expect("role member verification response");
    let role_verify_json =
        serde_json::from_str::<serde_json::Value>(&role_verify.body).unwrap_or_default();
    record!(
        "POST",
        role_route,
        "mutation-side-effects-and-readback",
        role_verify.status == "200 OK"
            && role_verify_json["isValidMember"] == true
            && role_verify_json["isBanned"] == false
            && role_verify_json["role"] == "mod"
    );

    let role_missing = crate::route_http_request(
        "POST",
        &format!("/api/v0/podcore/membership/{pod_id}/differential-missing-member/role"),
        None,
        r#"{"role":"mod"}"#,
        &state,
    )
    .await
    .expect("missing role response");
    record!(
        "POST",
        role_route,
        "missing-empty-or-conflict-state",
        role_missing.status == "500 Internal Server Error"
            && role_missing.body.contains("Failed to update membership")
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("pod_membership_moderation_publish.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api pod-membership-moderation-publish mismatches:\n{}",
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
pub(super) async fn controller_api_differential_pods_controller_residuals() {
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
            crate::route_http_request($method, $path, None, $body, $state)
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
                    serde_json::from_value::<crate::pods::PodRecord>(serde_json::json!({
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
            crate::pods::PodStore::load(&state.config.state_dir).expect("reload deleted pod state");
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
                crate::unix_timestamp_millis(),
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
            crate::pods::PodStore::load(&state.config.state_dir).expect("reload created pod state");
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
        let loaded = crate::pods::PodStore::load(&state.config.state_dir)
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
            crate::pods::PodStore::load(&state.config.state_dir).expect("reload banned pod state");
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
        let loaded = crate::pods::PodStore::load(&state.config.state_dir)
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
            crate::pods::PodStore::load(&state.config.state_dir).expect("reload bound pod state");
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
        let loaded = crate::pods::PodStore::load(&state.config.state_dir)
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
        let loaded = crate::pod_channels::PodChannelStore::load(&state.config.state_dir)
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
        let loaded = crate::pod_channels::PodChannelStore::load(&state.config.state_dir)
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
            crate::pods::PodStore::load(&state.config.state_dir).expect("reload unbound pod state");
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
        *state.runtime_credentials.write().await = Some(crate::LoginCredentials::default_client(
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
        *state.runtime_credentials.write().await = Some(crate::LoginCredentials::default_client(
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
        *state.runtime_credentials.write().await = Some(crate::LoginCredentials::default_client(
            "join-restart-peer",
            "secret",
        ));
        let response = request!(&state, "POST", "/api/v0/pods/pods-join-restart/join", "{}");
        let loaded =
            crate::pods::PodStore::load(&state.config.state_dir).expect("reload joined pod state");
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
        *state.runtime_credentials.write().await = Some(crate::LoginCredentials::default_client(
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
        *state.runtime_credentials.write().await = Some(crate::LoginCredentials::default_client(
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
        *state.runtime_credentials.write().await = Some(crate::LoginCredentials::default_client(
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
        *state.runtime_credentials.write().await = Some(crate::LoginCredentials::default_client(
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
        *state.runtime_credentials.write().await = Some(crate::LoginCredentials::default_client(
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
            crate::pods::PodStore::load(&state.config.state_dir).expect("reload left pod state");
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
        *state.runtime_credentials.write().await = Some(crate::LoginCredentials::default_client(
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
            crate::pods::PodStore::load(&state.config.state_dir).expect("reload updated pod state");
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
        let loaded = crate::pods::PodStore::load(&state.config.state_dir)
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

/// Differential proof for the remaining frozen slskdN PodCore controller
/// cases.  The ledger deliberately covers the source controller's
/// storage-error contracts as well as the process-local reset and
/// concurrent mutation behavior, rather than proving route presence only.
#[cfg_attr(test, test)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-4"
))]
pub(super) fn controller_api_differential_podcore_residuals() {
    run_controller_future_on_large_stack("podcore-residuals", || {
        controller_api_differential_podcore_residuals_impl()
    });
}

/// Differential proof for the remaining versioned taste-recommendation
/// controller cases.  Each subroute is exercised with its real WorkRef
/// shape, a null/empty request, a closed persistence pool, a fresh-state
/// reset, and concurrent requests.  Wishlist promotion additionally
/// checks the actual stored item and duplicate behavior.
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
pub(super) async fn controller_api_differential_taste_recommendation_open_cases() {
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
    let root_contract = |response: &crate::routing::HttpResponse| {
        let value = json_value(response);
        response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && value["minimumTrustedSources"].is_u64()
            && value["trustedActorCount"].is_u64()
            && value["candidateCount"].is_u64()
            && value["recommendations"].is_array()
    };
    let subroute_contract = |kind: &str, response: &crate::routing::HttpResponse| {
        let value = json_value(response);
        match kind {
            "graph" => {
                response.status == "200 OK"
                    && value["graph_data"].is_array()
                    && value["nodes"].is_u64()
                    && value["edges"].is_u64()
            }
            "release" => {
                response.status == "200 OK"
                    && value["recommendations"].is_array()
                    && value["count"].is_u64()
                    && value["status"] == "processing"
            }
            "wishlist" => {
                response.status == "200 OK"
                    && value["created"].is_boolean()
                    && value["searchText"].is_string()
            }
            _ => false,
        }
    };
    let root_body = r#"{"minimumTrustedSources":3}"#;
    let valid_work_ref = r#"{"workRef":{"domain":"music","title":"Taste Differential","creator":"Differential Artist","externalIds":{"musicbrainz":"abcd1234-1234-4a12-9a12-0a1234567890"}}}"#;

    {
        let (state, _receiver) = test_state_with_env(target_env());
        let malformed = crate::route_http_request(
            "POST",
            "/api/v0/taste-recommendations",
            None,
            "not-json",
            &state,
        )
        .await
        .expect("taste recommendations malformed response");
        record!(
            "POST",
            "/api/v0/taste-recommendations",
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );

        let missing =
            crate::route_http_request("POST", "/api/v0/taste-recommendations", None, "", &state)
                .await
                .expect("taste recommendations empty response");
        record!(
            "POST",
            "/api/v0/taste-recommendations",
            "missing-empty-or-conflict-state",
            root_contract(&missing)
        );

        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("taste recommendations runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(target_env(), crate::SearchStore::new(), Some(db.clone()));
        db.close_for_test().await;
        let runtime = crate::route_http_request(
            "POST",
            "/api/v0/taste-recommendations",
            None,
            root_body,
            &runtime_state,
        )
        .await
        .expect("taste recommendations runtime response");
        record!(
            "POST",
            "/api/v0/taste-recommendations",
            "runtime-failure-and-timeout",
            root_contract(&runtime)
        );

        let (reset_state, _receiver) = test_state_with_env(target_env());
        let reset = crate::route_http_request(
            "POST",
            "/api/v0/taste-recommendations",
            None,
            root_body,
            &reset_state,
        )
        .await
        .expect("taste recommendations reset response");
        let (restarted_state, _receiver) = test_state_with_env(target_env());
        let restarted = crate::route_http_request(
            "POST",
            "/api/v0/taste-recommendations",
            None,
            root_body,
            &restarted_state,
        )
        .await
        .expect("taste recommendations restarted response");
        record!(
            "POST",
            "/api/v0/taste-recommendations",
            "restart-persistence-or-reset",
            root_contract(&reset) && root_contract(&restarted)
        );

        let (left, right) = tokio::join!(
            crate::route_http_request(
                "POST",
                "/api/v0/taste-recommendations",
                None,
                root_body,
                &state
            ),
            crate::route_http_request(
                "POST",
                "/api/v0/taste-recommendations",
                None,
                root_body,
                &state
            )
        );
        let left = left.expect("left taste recommendations concurrency response");
        let right = right.expect("right taste recommendations concurrency response");
        record!(
            "POST",
            "/api/v0/taste-recommendations",
            "concurrency-and-idempotency",
            root_contract(&left) && root_contract(&right)
        );
    }

    for (path, kind) in [
        ("/api/v0/taste-recommendations/graph-preview", "graph"),
        ("/api/v0/taste-recommendations/release-radar", "release"),
        ("/api/v0/taste-recommendations/wishlist", "wishlist"),
    ] {
        let (state, _receiver) = test_state_with_env(target_env());
        let nominal = crate::route_http_request("POST", path, None, valid_work_ref, &state)
            .await
            .unwrap_or_else(|error| panic!("POST {path} nominal: {error}"));
        record!(
            "POST",
            path,
            "nominal-status-headers-body",
            subroute_contract(kind, &nominal)
        );

        let missing = crate::route_http_request("POST", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("POST {path} missing: {error}"));
        record!(
            "POST",
            path,
            "missing-empty-or-conflict-state",
            missing.status == "400 Bad Request"
        );

        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("taste subroute runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(target_env(), crate::SearchStore::new(), Some(db.clone()));
        db.close_for_test().await;
        let runtime = crate::route_http_request("POST", path, None, valid_work_ref, &runtime_state)
            .await
            .unwrap_or_else(|error| panic!("POST {path} runtime: {error}"));
        record!(
            "POST",
            path,
            "runtime-failure-and-timeout",
            if kind == "wishlist" {
                runtime.status == "503 Service Unavailable"
            } else {
                subroute_contract(kind, &runtime)
            }
        );

        let (mutation_state, _receiver) = test_state_with_env(target_env());
        let mutation = if kind == "graph" {
            mutation_state
                .interests
                .write()
                .await
                .add_liked("graph-differential-interest".to_owned());
            crate::route_http_request("POST", path, None, valid_work_ref, &mutation_state)
                .await
                .expect("graph preview mutation response")
        } else if kind == "release" {
            crate::route_http_request(
                "POST",
                "/api/v0/taste-recommendations/wishlist",
                None,
                valid_work_ref,
                &mutation_state,
            )
            .await
            .expect("seed taste wishlist before release radar");
            crate::route_http_request("POST", path, None, valid_work_ref, &mutation_state)
                .await
                .expect("release radar mutation response")
        } else {
            crate::route_http_request("POST", path, None, valid_work_ref, &mutation_state)
                .await
                .expect("wishlist mutation response")
        };
        let mutation_value = json_value(&mutation);
        let mutation_readback = if kind == "graph" {
            mutation_value["nodes"] == 1
                && mutation_value["graph_data"]
                    .as_array()
                    .is_some_and(|nodes| !nodes.is_empty())
        } else if kind == "release" {
            mutation_value["count"] == 1
                && mutation_value["recommendations"]
                    .as_array()
                    .is_some_and(|items| !items.is_empty())
        } else {
            mutation_value["created"] == true
                && mutation_state
                    .wishlist
                    .read()
                    .await
                    .records
                    .iter()
                    .flat_map(|record| record.items.iter())
                    .count()
                    == 1
        };
        record!(
            "POST",
            path,
            "mutation-side-effects-and-readback",
            subroute_contract(kind, &mutation) && mutation_readback
        );

        let (reset_state, _receiver) = test_state_with_env(target_env());
        let reset = crate::route_http_request("POST", path, None, valid_work_ref, &reset_state)
            .await
            .unwrap_or_else(|error| panic!("POST {path} reset: {error}"));
        let (restarted_state, _receiver) = test_state_with_env(target_env());
        let restarted =
            crate::route_http_request("POST", path, None, valid_work_ref, &restarted_state)
                .await
                .unwrap_or_else(|error| panic!("POST {path} restarted: {error}"));
        record!(
            "POST",
            path,
            "restart-persistence-or-reset",
            subroute_contract(kind, &reset) && subroute_contract(kind, &restarted)
        );

        let (concurrent_state, _receiver) = test_state_with_env(target_env());
        let (left, right) = tokio::join!(
            crate::route_http_request("POST", path, None, valid_work_ref, &concurrent_state),
            crate::route_http_request("POST", path, None, valid_work_ref, &concurrent_state)
        );
        let left = left.expect("left taste subroute concurrency response");
        let right = right.expect("right taste subroute concurrency response");
        let concurrency_readback = if kind == "wishlist" {
            concurrent_state
                .wishlist
                .read()
                .await
                .records
                .iter()
                .flat_map(|record| record.items.iter())
                .count()
                == 1
        } else {
            true
        };
        record!(
            "POST",
            path,
            "concurrency-and-idempotency",
            subroute_contract(kind, &left)
                && subroute_contract(kind, &right)
                && concurrency_readback
        );
    }

    assert_eq!(
        ledger.len(),
        23,
        "taste recommendation residual ledger size"
    );
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create taste recommendation evidence directory");
    fs::write(
        evidence_dir.join("taste_recommendation_open_cases.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize taste recommendation ledger"),
    )
    .expect("write taste recommendation ledger");
    assert!(
        mismatches.is_empty(),
        "{} taste recommendation controller mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
