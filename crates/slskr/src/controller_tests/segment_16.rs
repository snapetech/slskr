/// Differential proof for PodMembershipController durable mutations and
/// projections. The cases use the real PodStore file boundary to exercise
/// frozen 500 failures, reload persistence, and serialized concurrent
/// membership mutations for publish, update, delete, moderation, stats,
/// and cleanup.
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
async fn controller_api_differential_podcore_membership_storage() {
    let target = "slskdn";
    let membership_route = "/api/v0/podcore/membership/{podId}/{peerId}";
    let publish_route = "/api/v0/podcore/membership/{podId}/members";
    let update_route = "/api/v0/podcore/membership/{podId}/members/{peerId}";
    let stats_route = "/api/v0/podcore/membership/stats";
    let cleanup_route = "/api/v0/podcore/membership/cleanup";
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
        ($state:ident, $pod_id:expr) => {{
            $state
                .pods
                .write()
                .await
                .create(
                    serde_json::from_value::<super::pods::PodRecord>(serde_json::json!({
                        "podId": $pod_id,
                        "name": "Membership storage differential",
                        "isPublic": true,
                        "channels": [{
                            "channelId": "general",
                            "kind": 0,
                            "name": "General"
                        }]
                    }))
                    .expect("deserialize membership storage pod"),
                    "membership-owner".to_owned(),
                )
                .expect("create membership storage pod");
        }};
    }

    macro_rules! seed_member {
        ($state:ident, $pod_id:expr, $peer_id:expr, $role:expr, $banned:expr) => {{
            $state
                .pods
                .write()
                .await
                .upsert_member(
                    $pod_id,
                    super::pods::PodMember {
                        peer_id: $peer_id.to_owned(),
                        role: $role.to_owned(),
                        is_banned: $banned,
                        public_key: None,
                        joined_at: None,
                        last_seen: None,
                    },
                )
                .expect("seed membership storage member");
        }};
    }

    macro_rules! block_pod_path {
        ($state:ident) => {{
            let path = $state.config.state_dir.join("pods.json");
            fs::remove_file(&path).expect("remove membership storage file");
            fs::create_dir(&path).expect("block membership storage path");
            path
        }};
    }

    let membership_env = || {
        MapEnv::default()
            .with("SLSK_USERNAME", "membership-owner")
            .with("SLSK_PASSWORD", "test-secret")
    };

    let is_success = |response: &super::HttpResponse| {
        response.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&response.body)
                .ok()
                .is_some_and(|value| value["success"] == true)
    };

    // DELETE membership: runtime failure, reload persistence, and two
    // concurrent idempotent removals.
    {
        let (state, _receiver) = test_state_with_env(membership_env());
        let pod_id = "pod:membership-delete-runtime";
        seed_pod!(state, pod_id);
        seed_member!(state, pod_id, "delete-target", "member", false);
        let pods_path = block_pod_path!(state);
        let response = super::route_http_request(
            "DELETE",
            &format!("/api/v0/podcore/membership/{pod_id}/delete-target"),
            None,
            "",
            &state,
        )
        .await
        .expect("runtime membership delete");
        record!(
            "DELETE",
            membership_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("Failed to remove membership")
        );
        fs::remove_dir(&pods_path).expect("remove blocked membership delete path");
    }

    {
        let (state, _receiver) = test_state_with_env(membership_env());
        let pod_id = "pod:membership-delete-restart";
        seed_pod!(state, pod_id);
        seed_member!(state, pod_id, "delete-target", "member", false);
        let response = super::route_http_request(
            "DELETE",
            &format!("/api/v0/podcore/membership/{pod_id}/delete-target"),
            None,
            "",
            &state,
        )
        .await
        .expect("restart membership delete");
        let loaded =
            super::pods::PodStore::load(&state.config.state_dir).expect("reload membership delete");
        record!(
            "DELETE",
            membership_route,
            "restart-persistence-or-reset",
            response.status == "200 OK"
                && is_success(&response)
                && loaded
                    .member_for_verification(pod_id, "delete-target")
                    .is_none()
        );
    }

    {
        let (state, _receiver) = test_state_with_env(membership_env());
        let pod_id = "pod:membership-delete-concurrent";
        seed_pod!(state, pod_id);
        seed_member!(state, pod_id, "delete-target", "member", false);
        let responses = futures_util::future::join_all((0..2).map(|_| {
            let state = Arc::clone(&state);
            let path = format!("/api/v0/podcore/membership/{pod_id}/delete-target");
            async move { super::route_http_request("DELETE", &path, None, "", &state).await }
        }))
        .await;
        let loaded = super::pods::PodStore::load(&state.config.state_dir)
            .expect("reload concurrent membership delete");
        record!(
            "DELETE",
            membership_route,
            "concurrency-and-idempotency",
            responses
                .iter()
                .all(|response| { response.as_ref().is_ok_and(is_success) })
                && loaded
                    .member_for_verification(pod_id, "delete-target")
                    .is_none()
        );
    }

    // The two read-only membership projections must surface a damaged
    // persisted PodStore instead of returning an in-memory answer.
    for (route, method, error_message) in [
        (membership_route, "GET", "Failed to retrieve membership"),
        (
            "/api/v0/podcore/membership/{podId}/{peerId}/verify",
            "GET",
            "Failed to verify membership",
        ),
    ] {
        let (state, _receiver) = test_state_with_env(membership_env());
        let pod_id = if method == "GET" && error_message.starts_with("Failed to retrieve") {
            "pod:membership-get-runtime"
        } else {
            "pod:membership-verify-runtime"
        };
        seed_pod!(state, pod_id);
        let pods_path = block_pod_path!(state);
        let path = if error_message.starts_with("Failed to retrieve") {
            format!("/api/v0/podcore/membership/{pod_id}/membership-owner")
        } else {
            format!("/api/v0/podcore/membership/{pod_id}/membership-owner/verify")
        };
        let response = super::route_http_request(method, &path, None, "", &state)
            .await
            .expect("runtime membership projection");
        record!(
            method,
            route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error" && response.body.contains(error_message)
        );
        fs::remove_dir(&pods_path).expect("remove blocked membership projection path");
    }

    {
        let (state, _receiver) = test_state_with_env(membership_env());
        seed_pod!(state, "pod:membership-stats-runtime");
        let pods_path = block_pod_path!(state);
        let response = super::route_http_request("GET", stats_route, None, "", &state)
            .await
            .expect("runtime membership stats");
        record!(
            "GET",
            stats_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response
                    .body
                    .contains("Failed to get membership statistics")
        );
        fs::remove_dir(&pods_path).expect("remove blocked membership stats path");
    }

    // Ban, unban, and role changes share the frozen moderator gate but
    // retain distinct 500 error messages and durable state transitions.
    for (action, body, initial_banned, expected_role, expected_banned, error_message) in [
        ("ban", "", false, "member", true, "Failed to ban member"),
        ("unban", "", true, "member", false, "Failed to unban member"),
        (
            "role",
            r#"{"role":"mod"}"#,
            false,
            "mod",
            false,
            "Failed to change role",
        ),
    ] {
        let action_route = match action {
            "ban" => "/api/v0/podcore/membership/{podId}/{peerId}/ban",
            "unban" => "/api/v0/podcore/membership/{podId}/{peerId}/unban",
            "role" => "/api/v0/podcore/membership/{podId}/{peerId}/role",
            _ => unreachable!(),
        };
        let runtime_pod = format!("pod:membership-{action}-runtime");
        let (state, _receiver) = test_state_with_env(membership_env());
        seed_pod!(state, runtime_pod.as_str());
        seed_member!(
            state,
            runtime_pod.as_str(),
            "moderation-target",
            "member",
            initial_banned
        );
        let pods_path = block_pod_path!(state);
        let response = super::route_http_request(
            "POST",
            &format!(
                "/api/v0/podcore/membership/{}/moderation-target/{action}",
                runtime_pod
            ),
            None,
            body,
            &state,
        )
        .await
        .expect("runtime membership moderation");
        record!(
            "POST",
            action_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error" && response.body.contains(error_message)
        );
        fs::remove_dir(&pods_path).expect("remove blocked membership moderation path");

        let restart_pod = format!("pod:membership-{action}-restart");
        let (state, _receiver) = test_state_with_env(membership_env());
        seed_pod!(state, restart_pod.as_str());
        seed_member!(
            state,
            restart_pod.as_str(),
            "moderation-target",
            "member",
            initial_banned
        );
        let response = super::route_http_request(
            "POST",
            &format!(
                "/api/v0/podcore/membership/{}/moderation-target/{action}",
                restart_pod
            ),
            None,
            body,
            &state,
        )
        .await
        .expect("restart membership moderation");
        let loaded = super::pods::PodStore::load(&state.config.state_dir)
            .expect("reload membership moderation");
        let loaded_member = loaded.member_for_verification(&restart_pod, "moderation-target");
        record!(
            "POST",
            action_route,
            "restart-persistence-or-reset",
            is_success(&response)
                && loaded_member.as_ref().is_some_and(|member| {
                    member.role == expected_role && member.is_banned == expected_banned
                })
        );

        let concurrent_pod = format!("pod:membership-{action}-concurrent");
        let (state, _receiver) = test_state_with_env(membership_env());
        seed_pod!(state, concurrent_pod.as_str());
        seed_member!(
            state,
            concurrent_pod.as_str(),
            "moderation-target",
            "member",
            initial_banned
        );
        let responses = futures_util::future::join_all((0..2).map(|_| {
            let state = Arc::clone(&state);
            let path = format!(
                "/api/v0/podcore/membership/{}/moderation-target/{action}",
                concurrent_pod
            );
            async move { super::route_http_request("POST", &path, None, body, &state).await }
        }))
        .await;
        let loaded = super::pods::PodStore::load(&state.config.state_dir)
            .expect("reload concurrent membership moderation");
        let loaded_member = loaded.member_for_verification(&concurrent_pod, "moderation-target");
        record!(
            "POST",
            action_route,
            "concurrency-and-idempotency",
            responses
                .iter()
                .all(|response| { response.as_ref().is_ok_and(is_success) })
                && loaded_member.as_ref().is_some_and(|member| {
                    member.role == expected_role && member.is_banned == expected_banned
                })
        );
    }

    // PublishMembership and UpdateMembership both persist the membership
    // record and return the same signed result shape on success.
    {
        let (state, _receiver) = test_state_with_env(membership_env());
        let pod_id = "pod:membership-publish-runtime";
        seed_pod!(state, pod_id);
        let pods_path = block_pod_path!(state);
        let response = super::route_http_request(
            "POST",
            &format!("/api/v0/podcore/membership/{pod_id}/members"),
            None,
            r#"{"peerId":"membership-owner","role":"owner","isBanned":false}"#,
            &state,
        )
        .await
        .expect("runtime membership publish");
        record!(
            "POST",
            publish_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("Failed to publish membership")
        );
        fs::remove_dir(&pods_path).expect("remove blocked membership publish path");
    }

    {
        let (state, _receiver) = test_state_with_env(membership_env());
        let pod_id = "pod:membership-publish-restart";
        seed_pod!(state, pod_id);
        let response = super::route_http_request(
            "POST",
            &format!("/api/v0/podcore/membership/{pod_id}/members"),
            None,
            r#"{"peerId":"membership-owner","role":"owner","isBanned":false}"#,
            &state,
        )
        .await
        .expect("restart membership publish");
        let loaded = super::pods::PodStore::load(&state.config.state_dir)
            .expect("reload membership publish");
        record!(
            "POST",
            publish_route,
            "restart-persistence-or-reset",
            is_success(&response)
                && loaded
                    .member_for_verification(pod_id, "membership-owner")
                    .is_some_and(|member| member.role == "owner")
        );
    }

    {
        let (state, _receiver) = test_state_with_env(membership_env());
        let pod_id = "pod:membership-publish-concurrent";
        seed_pod!(state, pod_id);
        let responses = futures_util::future::join_all((0..2).map(|_| {
            let state = Arc::clone(&state);
            let path = format!("/api/v0/podcore/membership/{pod_id}/members");
            async move {
                super::route_http_request(
                    "POST",
                    &path,
                    None,
                    r#"{"peerId":"membership-owner","role":"owner","isBanned":false}"#,
                    &state,
                )
                .await
            }
        }))
        .await;
        let loaded = super::pods::PodStore::load(&state.config.state_dir)
            .expect("reload concurrent membership publish");
        record!(
            "POST",
            publish_route,
            "concurrency-and-idempotency",
            responses
                .iter()
                .all(|response| { response.as_ref().is_ok_and(is_success) })
                && loaded
                    .member_for_verification(pod_id, "membership-owner")
                    .is_some_and(|member| member.role == "owner")
        );
    }

    {
        let (state, _receiver) = test_state_with_env(membership_env());
        let pod_id = "pod:membership-update-runtime";
        seed_pod!(state, pod_id);
        seed_member!(state, pod_id, "update-target", "member", false);
        let pods_path = block_pod_path!(state);
        let response = super::route_http_request(
            "PUT",
            &format!("/api/v0/podcore/membership/{pod_id}/members/update-target"),
            None,
            r#"{"role":"mod","isBanned":false}"#,
            &state,
        )
        .await
        .expect("runtime membership update");
        record!(
            "PUT",
            update_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("Failed to update membership")
        );
        fs::remove_dir(&pods_path).expect("remove blocked membership update path");
    }

    {
        let (state, _receiver) = test_state_with_env(membership_env());
        let pod_id = "pod:membership-update-restart";
        seed_pod!(state, pod_id);
        seed_member!(state, pod_id, "update-target", "member", false);
        let response = super::route_http_request(
            "PUT",
            &format!("/api/v0/podcore/membership/{pod_id}/members/update-target"),
            None,
            r#"{"role":"mod","isBanned":false}"#,
            &state,
        )
        .await
        .expect("restart membership update");
        let loaded =
            super::pods::PodStore::load(&state.config.state_dir).expect("reload membership update");
        record!(
            "PUT",
            update_route,
            "restart-persistence-or-reset",
            is_success(&response)
                && loaded
                    .member_for_verification(pod_id, "update-target")
                    .is_some_and(|member| member.role == "mod")
        );
    }

    {
        let (state, _receiver) = test_state_with_env(membership_env());
        let pod_id = "pod:membership-update-concurrent";
        seed_pod!(state, pod_id);
        seed_member!(state, pod_id, "update-target", "member", false);
        let responses = futures_util::future::join_all((0..2).map(|_| {
            let state = Arc::clone(&state);
            let path = format!("/api/v0/podcore/membership/{pod_id}/members/update-target");
            async move {
                super::route_http_request(
                    "PUT",
                    &path,
                    None,
                    r#"{"role":"mod","isBanned":false}"#,
                    &state,
                )
                .await
            }
        }))
        .await;
        let loaded = super::pods::PodStore::load(&state.config.state_dir)
            .expect("reload concurrent membership update");
        record!(
            "PUT",
            update_route,
            "concurrency-and-idempotency",
            responses
                .iter()
                .all(|response| { response.as_ref().is_ok_and(is_success) })
                && loaded
                    .member_for_verification(pod_id, "update-target")
                    .is_some_and(|member| member.role == "mod")
        );
    }

    // Cleanup is process-local in this implementation, but it must still
    // honor the frozen durable-service failure contract and leave valid
    // PodStore state reloadable after a successful cleanup.
    {
        let (state, _receiver) = test_state_with_env(membership_env());
        seed_pod!(state, "pod:membership-cleanup-runtime");
        let pods_path = block_pod_path!(state);
        let response = super::route_http_request("POST", cleanup_route, None, "", &state)
            .await
            .expect("runtime membership cleanup");
        record!(
            "POST",
            cleanup_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response
                    .body
                    .contains("Failed to cleanup expired memberships")
        );
        fs::remove_dir(&pods_path).expect("remove blocked membership cleanup path");
    }

    {
        let (state, _receiver) = test_state_with_env(membership_env());
        let pod_id = "pod:membership-cleanup-restart";
        seed_pod!(state, pod_id);
        let response = super::route_http_request("POST", cleanup_route, None, "", &state)
            .await
            .expect("restart membership cleanup");
        let loaded = super::pods::PodStore::load(&state.config.state_dir)
            .expect("reload membership cleanup");
        record!(
            "POST",
            cleanup_route,
            "restart-persistence-or-reset",
            response.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&response.body)
                    .ok()
                    .is_some_and(|value| {
                        value["recordsCleaned"].is_number() && value["errorsEncountered"] == 0
                    })
                && loaded.get(pod_id).is_some()
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("podcore_membership_storage.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api podcore-membership-storage mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential proof for the PodDiscoveryController persisted registry
/// boundary and all five read projections. The cases exercise the frozen
/// runtime failures, reload behavior, and concurrent registry operations
/// against the real controller-feature state file.
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
async fn controller_api_differential_podcore_discovery_storage() {
    let target = "slskdn";
    let register_route = "/api/v0/podcore/discovery/register";
    let update_route = "/api/v0/podcore/discovery/update";
    let unregister_route = "/api/v0/podcore/discovery/unregister/{podId}";
    let refresh_route = "/api/v0/podcore/discovery/refresh";
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

    macro_rules! prepare_feature_path {
        ($state:ident) => {{
            let path = $state
                .config
                .state_dir
                .join("controller-feature-state.json");
            $state.controller_features.write_for_test().await.state_path = path.clone();
            path
        }};
    }

    macro_rules! block_feature_path {
        ($state:ident) => {{
            let path = prepare_feature_path!($state);
            if path.is_file() {
                fs::remove_file(&path).expect("remove discovery feature state file");
            }
            fs::create_dir(&path).expect("block discovery feature state path");
            path
        }};
    }

    macro_rules! seed_pod {
        ($state:ident, $pod_id:expr) => {{
            $state
                .pods
                .write()
                .await
                .create(
                    serde_json::from_value::<super::pods::PodRecord>(serde_json::json!({
                        "podId": $pod_id,
                        "name": "Discovery storage differential",
                        "visibility": "Listed",
                        "isPublic": true,
                        "channels": [{
                            "channelId": "general",
                            "kind": 0,
                            "name": "General"
                        }]
                    }))
                    .expect("deserialize discovery storage pod"),
                    "discovery-owner".to_owned(),
                )
                .expect("create discovery storage pod");
        }};
    }

    macro_rules! expire_registration {
        ($state:ident, $pod_id:expr) => {{
            let key = format!("pod/discovery/{}", $pod_id);
            let mut features = $state.controller_features.write_for_test().await;
            let mut record = features
                .get(&key)
                .cloned()
                .expect("discovery registration before refresh");
            record["expiresAt"] =
                serde_json::json!((chrono::Utc::now() + chrono::Duration::hours(1)).to_rfc3339());
            features
                .upsert(key, record)
                .expect("expire discovery registration");
        }};
    }

    let discovery_env = || {
        MapEnv::default()
            .with("SLSK_USERNAME", "discovery-owner")
            .with("SLSK_PASSWORD", "test-secret")
    };
    let discovery_body = |pod_id: &str, name: &str| {
        serde_json::json!({
            "podId": pod_id,
            "name": name,
            "description": "Discovery storage test",
            "visibility": "Listed",
            "tags": ["music", "live"],
            "focusContentId": "content:music:recording:discovery-storage",
        })
        .to_string()
    };
    let is_success = |response: &super::HttpResponse| {
        response.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&response.body)
                .ok()
                .is_some_and(|value| value["success"] == true)
    };

    // Invalid register/update payloads must remain 400 even though the
    // success path now validates the durable feature-state boundary.
    {
        let (state, _receiver) = test_state_with_env(discovery_env());
        let response = super::route_http_request("POST", register_route, None, "{}", &state)
            .await
            .expect("missing discovery registration payload");
        record!(
            "POST",
            register_route,
            "missing-empty-or-conflict-state",
            response.status == "400 Bad Request"
                && response.body.contains("Valid pod with PodId is required")
        );
    }

    // Register persists its discovery record and maps a blocked state
    // path to the controller's fixed 500 response.
    {
        let (state, _receiver) = test_state_with_env(discovery_env());
        let feature_path = block_feature_path!(state);
        let response = super::route_http_request(
            "POST",
            register_route,
            None,
            &discovery_body("pod:discovery-register-runtime", "Runtime"),
            &state,
        )
        .await
        .expect("runtime discovery registration");
        record!(
            "POST",
            register_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("Failed to register pod")
        );
        fs::remove_dir(&feature_path).expect("remove blocked discovery registration path");
    }

    {
        let (state, _receiver) = test_state_with_env(discovery_env());
        let feature_path = prepare_feature_path!(state);
        let pod_id = "pod:discovery-register-restart";
        let response = super::route_http_request(
            "POST",
            register_route,
            None,
            &discovery_body(pod_id, "Restart"),
            &state,
        )
        .await
        .expect("restart discovery registration");
        let loaded = super::ControllerFeatureState::load(&state.config.state_dir)
            .expect("reload discovery registration");
        let key = format!("pod/discovery/{pod_id}");
        record!(
            "POST",
            register_route,
            "restart-persistence-or-reset",
            is_success(&response) && loaded.get(&key).is_some() && feature_path.is_file()
        );
    }

    {
        let (state, _receiver) = test_state_with_env(discovery_env());
        let feature_path = prepare_feature_path!(state);
        let pod_id = "pod:discovery-register-concurrent";
        let responses =
            futures_util::future::join_all((0..2).map(|_| {
                let state = Arc::clone(&state);
                let body = discovery_body(pod_id, "Concurrent");
                async move {
                    super::route_http_request("POST", register_route, None, &body, &state).await
                }
            }))
            .await;
        let loaded = super::ControllerFeatureState::load(&state.config.state_dir)
            .expect("reload concurrent discovery registration");
        let key = format!("pod/discovery/{pod_id}");
        record!(
            "POST",
            register_route,
            "concurrency-and-idempotency",
            responses
                .iter()
                .all(|response| { response.as_ref().is_ok_and(is_success) })
                && loaded.get(&key).is_some()
                && feature_path.is_file()
        );
    }

    // Update keeps the same frozen validation, durable write, and
    // serialized same-key behavior.
    {
        let (state, _receiver) = test_state_with_env(discovery_env());
        let response = super::route_http_request("POST", update_route, None, "{}", &state)
            .await
            .expect("missing discovery update payload");
        record!(
            "POST",
            update_route,
            "missing-empty-or-conflict-state",
            response.status == "400 Bad Request"
                && response.body.contains("Valid pod with PodId is required")
        );
    }

    {
        let (state, _receiver) = test_state_with_env(discovery_env());
        let feature_path = block_feature_path!(state);
        let response = super::route_http_request(
            "POST",
            update_route,
            None,
            &discovery_body("pod:discovery-update-runtime", "Runtime update"),
            &state,
        )
        .await
        .expect("runtime discovery update");
        record!(
            "POST",
            update_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("Failed to update pod discovery")
        );
        fs::remove_dir(&feature_path).expect("remove blocked discovery update path");
    }

    {
        let (state, _receiver) = test_state_with_env(discovery_env());
        let feature_path = prepare_feature_path!(state);
        let pod_id = "pod:discovery-update-restart";
        let registered = super::route_http_request(
            "POST",
            register_route,
            None,
            &discovery_body(pod_id, "Before update"),
            &state,
        )
        .await
        .expect("seed restart discovery update");
        assert!(is_success(&registered));
        let response = super::route_http_request(
            "POST",
            update_route,
            None,
            &discovery_body(pod_id, "After update"),
            &state,
        )
        .await
        .expect("restart discovery update");
        let loaded = super::ControllerFeatureState::load(&state.config.state_dir)
            .expect("reload discovery update");
        let key = format!("pod/discovery/{pod_id}");
        let updated_name = loaded
            .get(&key)
            .and_then(|value| value.get("pod"))
            .and_then(|pod| pod.get("name"))
            .and_then(serde_json::Value::as_str);
        record!(
            "POST",
            update_route,
            "restart-persistence-or-reset",
            is_success(&response) && updated_name == Some("After update") && feature_path.is_file()
        );
    }

    {
        let (state, _receiver) = test_state_with_env(discovery_env());
        let feature_path = prepare_feature_path!(state);
        let pod_id = "pod:discovery-update-concurrent";
        let registered = super::route_http_request(
            "POST",
            register_route,
            None,
            &discovery_body(pod_id, "Before update"),
            &state,
        )
        .await
        .expect("seed concurrent discovery update");
        assert!(is_success(&registered));
        let responses =
            futures_util::future::join_all(
                (0..2).map(|_| {
                    let state = Arc::clone(&state);
                    let body = discovery_body(pod_id, "Concurrent update");
                    async move {
                        super::route_http_request("POST", update_route, None, &body, &state).await
                    }
                }),
            )
            .await;
        let loaded = super::ControllerFeatureState::load(&state.config.state_dir)
            .expect("reload concurrent discovery update");
        let key = format!("pod/discovery/{pod_id}");
        record!(
            "POST",
            update_route,
            "concurrency-and-idempotency",
            responses
                .iter()
                .all(|response| { response.as_ref().is_ok_and(is_success) })
                && loaded.get(&key).is_some()
                && feature_path.is_file()
        );
    }

    // Unregister is idempotent only for the first removal: the frozen
    // service reports a missing second registry entry as its fixed 500.
    {
        let (state, _receiver) = test_state_with_env(discovery_env());
        let pod_id = "pod:discovery-unregister-runtime";
        let registered = super::route_http_request(
            "POST",
            register_route,
            None,
            &discovery_body(pod_id, "Runtime unregister"),
            &state,
        )
        .await
        .expect("seed runtime discovery unregister");
        assert!(is_success(&registered));
        let feature_path = block_feature_path!(state);
        let response = super::route_http_request(
            "DELETE",
            &format!("/api/v0/podcore/discovery/unregister/{pod_id}"),
            None,
            "",
            &state,
        )
        .await
        .expect("runtime discovery unregister");
        record!(
            "DELETE",
            unregister_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("Failed to unregister pod")
        );
        fs::remove_dir(&feature_path).expect("remove blocked discovery unregister path");
    }

    {
        let (state, _receiver) = test_state_with_env(discovery_env());
        let pod_id = "pod:discovery-unregister-restart";
        prepare_feature_path!(state);
        let registered = super::route_http_request(
            "POST",
            register_route,
            None,
            &discovery_body(pod_id, "Restart unregister"),
            &state,
        )
        .await
        .expect("seed restart discovery unregister");
        assert!(is_success(&registered));
        let response = super::route_http_request(
            "DELETE",
            &format!("/api/v0/podcore/discovery/unregister/{pod_id}"),
            None,
            "",
            &state,
        )
        .await
        .expect("restart discovery unregister");
        let loaded = super::ControllerFeatureState::load(&state.config.state_dir)
            .expect("reload discovery unregister");
        record!(
            "DELETE",
            unregister_route,
            "restart-persistence-or-reset",
            response.status == "200 OK" && loaded.get(&format!("pod/discovery/{pod_id}")).is_none()
        );
    }

    {
        let (state, _receiver) = test_state_with_env(discovery_env());
        let pod_id = "pod:discovery-unregister-concurrent";
        prepare_feature_path!(state);
        let registered = super::route_http_request(
            "POST",
            register_route,
            None,
            &discovery_body(pod_id, "Concurrent unregister"),
            &state,
        )
        .await
        .expect("seed concurrent discovery unregister");
        assert!(is_success(&registered));
        let responses = futures_util::future::join_all((0..2).map(|_| {
            let state = Arc::clone(&state);
            let path = format!("/api/v0/podcore/discovery/unregister/{pod_id}");
            async move { super::route_http_request("DELETE", &path, None, "", &state).await }
        }))
        .await;
        let loaded = super::ControllerFeatureState::load(&state.config.state_dir)
            .expect("reload concurrent discovery unregister");
        let removed = loaded.get(&format!("pod/discovery/{pod_id}")).is_none();
        let one_success = responses
            .iter()
            .filter(|response| {
                response
                    .as_ref()
                    .is_ok_and(|value| value.status == "200 OK")
            })
            .count()
            == 1;
        let one_failure = responses.iter().any(|response| {
            response.as_ref().is_ok_and(|value| {
                value.status == "500 Internal Server Error"
                    && value.body.contains("Failed to unregister pod")
            })
        });
        record!(
            "DELETE",
            unregister_route,
            "concurrency-and-idempotency",
            one_success && one_failure && removed
        );
    }

    // The five read projections must not answer from stale in-memory
    // records after their persisted feature state becomes unusable.
    for (route, error_message) in [
        ("/api/v0/podcore/discovery/all", "Failed to discover pods"),
        (
            "/api/v0/podcore/discovery/content/content:music:recording:runtime",
            "Failed to discover pods by content",
        ),
        (
            "/api/v0/podcore/discovery/name/runtime",
            "Failed to discover pods by name",
        ),
        (
            "/api/v0/podcore/discovery/stats",
            "Failed to get discovery statistics",
        ),
        (
            "/api/v0/podcore/discovery/tag/runtime",
            "Failed to discover pods by tag",
        ),
        (
            "/api/v0/podcore/discovery/tags/runtime,other",
            "Failed to discover pods by tags",
        ),
    ] {
        let (state, _receiver) = test_state_with_env(discovery_env());
        let feature_path = block_feature_path!(state);
        let response = super::route_http_request("GET", route, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("runtime discovery projection {route}: {error}"));
        record!(
            "GET",
            if route.ends_with("/all") {
                "/api/v0/podcore/discovery/all"
            } else if route.ends_with("/stats") {
                "/api/v0/podcore/discovery/stats"
            } else if route.contains("/content/") {
                "/api/v0/podcore/discovery/content/{*contentId}"
            } else if route.contains("/name/") {
                "/api/v0/podcore/discovery/name/{name}"
            } else if route.contains("/tag/") && !route.contains("/tags/") {
                "/api/v0/podcore/discovery/tag/{tag}"
            } else {
                "/api/v0/podcore/discovery/tags/{tags}"
            },
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error" && response.body.contains(error_message)
        );
        fs::remove_dir(&feature_path).expect("remove blocked discovery projection path");
    }

    // Refresh has no request body; an ignored query is its malformed
    // input case, while an empty registry is a valid no-op.
    {
        let (state, _receiver) = test_state_with_env(discovery_env());
        let response = super::route_http_request(
            "POST",
            "/api/v0/podcore/discovery/refresh?unexpected=not-a-number",
            None,
            "not-json",
            &state,
        )
        .await
        .expect("malformed discovery refresh request");
        record!(
            "POST",
            refresh_route,
            "malformed-path-query-or-body",
            response.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&response.body)
                    .ok()
                    .is_some_and(|value| value["success"] == true)
        );
    }

    {
        let (state, _receiver) = test_state_with_env(discovery_env());
        let response = super::route_http_request("POST", refresh_route, None, "", &state)
            .await
            .expect("empty discovery refresh");
        record!(
            "POST",
            refresh_route,
            "missing-empty-or-conflict-state",
            response.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&response.body)
                    .ok()
                    .is_some_and(|value| {
                        value["success"] == true
                            && value["podId"] == "all"
                            && value["wasRepublished"] == false
                    })
        );
    }

    {
        let (state, _receiver) = test_state_with_env(discovery_env());
        let feature_path = block_feature_path!(state);
        let response = super::route_http_request("POST", refresh_route, None, "", &state)
            .await
            .expect("runtime discovery refresh");
        record!(
            "POST",
            refresh_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("Failed to refresh discovery")
        );
        fs::remove_dir(&feature_path).expect("remove blocked discovery refresh path");
    }

    {
        let (state, _receiver) = test_state_with_env(discovery_env());
        let feature_path = prepare_feature_path!(state);
        let pod_id = "pod:discovery-refresh-restart";
        seed_pod!(state, pod_id);
        let registered = super::route_http_request(
            "POST",
            register_route,
            None,
            &discovery_body(pod_id, "Refresh restart"),
            &state,
        )
        .await
        .expect("seed restart discovery refresh");
        assert!(is_success(&registered));
        expire_registration!(state, pod_id);
        let response = super::route_http_request("POST", refresh_route, None, "", &state)
            .await
            .expect("restart discovery refresh");
        let loaded = super::ControllerFeatureState::load(&state.config.state_dir)
            .expect("reload discovery refresh");
        let stored = loaded
            .get(&format!("pod/discovery/{pod_id}"))
            .cloned()
            .unwrap_or_default();
        let extended = stored
            .get("expiresAt")
            .and_then(serde_json::Value::as_str)
            .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
            .map(|value| value.with_timezone(&chrono::Utc))
            .is_some_and(|value| value > chrono::Utc::now() + chrono::Duration::hours(6));
        record!(
            "POST",
            refresh_route,
            "restart-persistence-or-reset",
            response.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&response.body)
                    .ok()
                    .is_some_and(|value| value["wasRepublished"] == true)
                && extended
                && feature_path.is_file()
        );
    }

    {
        let (state, _receiver) = test_state_with_env(discovery_env());
        let feature_path = prepare_feature_path!(state);
        let pod_id = "pod:discovery-refresh-concurrent";
        seed_pod!(state, pod_id);
        let registered = super::route_http_request(
            "POST",
            register_route,
            None,
            &discovery_body(pod_id, "Refresh concurrent"),
            &state,
        )
        .await
        .expect("seed concurrent discovery refresh");
        assert!(is_success(&registered));
        expire_registration!(state, pod_id);
        let responses =
            futures_util::future::join_all(
                (0..2).map(|_| {
                    let state = Arc::clone(&state);
                    async move {
                        super::route_http_request("POST", refresh_route, None, "", &state).await
                    }
                }),
            )
            .await;
        let loaded = super::ControllerFeatureState::load(&state.config.state_dir)
            .expect("reload concurrent discovery refresh");
        record!(
            "POST",
            refresh_route,
            "concurrency-and-idempotency",
            responses.iter().all(|response| {
                response
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
            }) && loaded.get(&format!("pod/discovery/{pod_id}")).is_some()
                && feature_path.is_file()
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create discovery evidence directory");
    fs::write(
        evidence_dir.join("podcore_discovery_storage.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize discovery controller ledger"),
    )
    .expect("write discovery controller ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api podcore-discovery-storage mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

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
async fn controller_api_differential_podcore_join_leave_residuals() {
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
    let pending_is_empty = |response: &super::HttpResponse, field: &str| {
        response.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&response.body)
                .ok()
                .is_some_and(|value| value[field] == serde_json::json!([]))
    };
    let one_success_one_status = |responses: &[Result<super::HttpResponse, String>],
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
        let response = super::route_http_request("DELETE", &path, None, "", &state)
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
        let joined = super::route_http_request(
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
        let response = super::route_http_request("DELETE", &path, None, "", &restarted)
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
        let joined = super::route_http_request(
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
            async move { super::route_http_request("DELETE", &path, None, "", &state).await }
        }))
        .await;
        let pending =
            super::route_http_request("GET", &pending_path("join", pod_id), None, "", &state)
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
        let response = super::route_http_request("DELETE", &path, None, "", &state)
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
        let leave = super::route_http_request(
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
        let response = super::route_http_request("DELETE", &path, None, "", &restarted)
            .await
            .expect("restart leave cancellation");
        record!(
            "DELETE",
            cancel_leave_route,
            "restart-persistence-or-reset",
            response.status == "404 Not Found"
                && pending_is_empty(
                    &super::route_http_request(
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
        let leave = super::route_http_request(
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
            async move { super::route_http_request("DELETE", &path, None, "", &state).await }
        }))
        .await;
        let pending =
            super::route_http_request("GET", &pending_path("leave", pod_id), None, "", &state)
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
        let response = super::route_http_request(
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
        let response = super::route_http_request(
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
        let response = super::route_http_request(
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
        let joined = super::route_http_request(
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
            super::route_http_request("GET", &pending_path("join", pod_id), None, "", &restarted)
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
                        super::route_http_request("POST", join_route, None, &body, &state).await
                    }
                }),
            )
            .await;
        let pending =
            super::route_http_request("GET", &pending_path("join", pod_id), None, "", &state)
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
        let response = super::route_http_request(
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
        let joined = super::route_http_request(
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
        let response = super::route_http_request(
            "POST",
            join_accept_route,
            None,
            &join_accept_body(pod_id, peer_id, owner),
            &restarted,
        )
        .await
        .expect("restart join acceptance");
        let pending =
            super::route_http_request("GET", &pending_path("join", pod_id), None, "", &restarted)
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
        let joined = super::route_http_request(
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
                super::route_http_request("POST", join_accept_route, None, &body, &state).await
            }
        }))
        .await;
        let pending =
            super::route_http_request("GET", &pending_path("join", pod_id), None, "", &state)
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
        let response = super::route_http_request(
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
        let left = super::route_http_request(
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
            super::route_http_request("GET", &pending_path("leave", pod_id), None, "", &restarted)
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
                        super::route_http_request("POST", leave_route, None, &body, &state).await
                    }
                }),
            )
            .await;
        let pending =
            super::route_http_request("GET", &pending_path("leave", pod_id), None, "", &state)
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
        let response = super::route_http_request(
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
        let left = super::route_http_request(
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
        let response = super::route_http_request(
            "POST",
            leave_accept_route,
            None,
            &leave_accept_body(pod_id, owner, owner),
            &restarted,
        )
        .await
        .expect("restart leave acceptance");
        let pending =
            super::route_http_request("GET", &pending_path("leave", pod_id), None, "", &restarted)
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
        let left = super::route_http_request(
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
                super::route_http_request("POST", leave_accept_route, None, &body, &state).await
            }
        }))
        .await;
        let pending =
            super::route_http_request("GET", &pending_path("leave", pod_id), None, "", &state)
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

/// Differential proof for the versioned SecurityController ban and unban
/// endpoints. The default route-test state has no database, so restart
/// cases intentionally assert reset semantics; concurrent deletes assert
/// the frozen single-winner/not-found behavior.
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
async fn controller_api_differential_security_ban_residuals() {
    let target = "slskdn";
    let ip_post_route = "/api/v0/security/bans/ip";
    let ip_delete_route = "/api/v0/security/bans/ip/{ipAddress}";
    let username_post_route = "/api/v0/security/bans/username";
    let username_delete_route = "/api/v0/security/bans/username/{username}";
    let list_route = "/api/v0/security/bans";
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

    macro_rules! seed_ip {
        ($state:ident, $ip:expr) => {{
            let response = super::route_http_request(
                "POST",
                ip_post_route,
                None,
                &format!(r#"{{"ipAddress":"{}"}}"#, $ip),
                &$state,
            )
            .await
            .expect("seed IP ban");
            assert_eq!(response.status, "200 OK", "{}", response.body);
        }};
    }

    macro_rules! seed_username {
        ($state:ident, $username:expr) => {{
            let response = super::route_http_request(
                "POST",
                username_post_route,
                None,
                &format!(r#"{{"username":"{}"}}"#, $username),
                &$state,
            )
            .await
            .expect("seed username ban");
            assert_eq!(response.status, "200 OK", "{}", response.body);
        }};
    }

    let list_is_empty = |response: &super::HttpResponse| {
        response.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&response.body)
                .ok()
                .is_some_and(|value| value == serde_json::json!([]))
    };
    let list_len = |response: &super::HttpResponse, expected: usize| {
        response.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&response.body)
                .ok()
                .and_then(|value| value.as_array().map(|entries| entries.len()))
                == Some(expected)
    };
    let one_status = |responses: &[Result<super::HttpResponse, String>], status: &str| {
        responses
            .iter()
            .filter(|response| response.as_ref().is_ok_and(|value| value.status == status))
            .count()
            == 1
    };

    {
        let (state, _receiver) = test_state();
        let invalid =
            super::route_http_request("POST", ip_post_route, None, r#"{"ip":"not-an-ip"}"#, &state)
                .await
                .expect("invalid IP ban");
        let missing = super::route_http_request("POST", ip_post_route, None, "{}", &state)
            .await
            .expect("missing IP ban");
        record!(
            "POST",
            ip_post_route,
            "malformed-path-query-or-body",
            invalid.status == "400 Bad Request"
        );
        record!(
            "POST",
            ip_post_route,
            "missing-empty-or-conflict-state",
            missing.status == "400 Bad Request"
        );
    }

    {
        let (state, _receiver) = test_state();
        seed_ip!(state, "203.0.113.10");
        let (restarted, _receiver) = test_state();
        let listed = super::route_http_request("GET", list_route, None, "", &restarted)
            .await
            .expect("reset IP bans");
        record!(
            "POST",
            ip_post_route,
            "restart-persistence-or-reset",
            list_is_empty(&listed)
        );
    }

    {
        let (state, _receiver) = test_state();
        let responses =
            futures_util::future::join_all(
                [
                    r#"{"ipAddress":"203.0.113.11"}"#,
                    r#"{"ipAddress":"203.0.113.12"}"#,
                ]
                .into_iter()
                .map(|body| {
                    let state = Arc::clone(&state);
                    async move {
                        super::route_http_request("POST", ip_post_route, None, body, &state).await
                    }
                }),
            )
            .await;
        let listed = super::route_http_request("GET", list_route, None, "", &state)
            .await
            .expect("concurrent IP bans readback");
        record!(
            "POST",
            ip_post_route,
            "concurrency-and-idempotency",
            responses.iter().all(|response| {
                response
                    .as_ref()
                    .is_ok_and(|value| value.status == "200 OK")
            }) && list_len(&listed, 2)
        );
    }

    {
        let (state, _receiver) = test_state();
        seed_ip!(state, "203.0.113.20");
        let deleted = super::route_http_request(
            "DELETE",
            "/api/v0/security/bans/ip/203.0.113.20",
            None,
            "",
            &state,
        )
        .await
        .expect("nominal IP unban");
        record!(
            "DELETE",
            ip_delete_route,
            "nominal-status-headers-body",
            deleted.status == "200 OK"
        );
    }

    {
        let (state, _receiver) = test_state();
        let malformed =
            super::route_http_request("DELETE", "/api/v0/security/bans/ip/%20", None, "", &state)
                .await
                .expect("malformed IP unban");
        let missing = super::route_http_request(
            "DELETE",
            "/api/v0/security/bans/ip/203.0.113.21",
            None,
            "",
            &state,
        )
        .await
        .expect("missing IP unban");
        record!(
            "DELETE",
            ip_delete_route,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
        record!(
            "DELETE",
            ip_delete_route,
            "missing-empty-or-conflict-state",
            missing.status == "404 Not Found"
        );
    }

    {
        let (state, _receiver) = test_state();
        seed_ip!(state, "203.0.113.22");
        let deleted = super::route_http_request(
            "DELETE",
            "/api/v0/security/bans/ip/203.0.113.22",
            None,
            "",
            &state,
        )
        .await
        .expect("readback IP unban");
        let listed = super::route_http_request("GET", list_route, None, "", &state)
            .await
            .expect("IP unban readback");
        record!(
            "DELETE",
            ip_delete_route,
            "mutation-side-effects-and-readback",
            deleted.status == "200 OK" && list_is_empty(&listed)
        );
    }

    {
        let (state, _receiver) = test_state();
        seed_ip!(state, "203.0.113.23");
        let (restarted, _receiver) = test_state();
        let deleted = super::route_http_request(
            "DELETE",
            "/api/v0/security/bans/ip/203.0.113.23",
            None,
            "",
            &restarted,
        )
        .await
        .expect("reset IP unban");
        record!(
            "DELETE",
            ip_delete_route,
            "restart-persistence-or-reset",
            deleted.status == "404 Not Found"
        );
    }

    {
        let (state, _receiver) = test_state();
        seed_ip!(state, "203.0.113.24");
        let responses = futures_util::future::join_all((0..2).map(|_| {
            let state = Arc::clone(&state);
            async move {
                super::route_http_request(
                    "DELETE",
                    "/api/v0/security/bans/ip/203.0.113.24",
                    None,
                    "",
                    &state,
                )
                .await
            }
        }))
        .await;
        let listed = super::route_http_request("GET", list_route, None, "", &state)
            .await
            .expect("concurrent IP unban readback");
        record!(
            "DELETE",
            ip_delete_route,
            "concurrency-and-idempotency",
            one_status(&responses, "200 OK")
                && one_status(&responses, "404 Not Found")
                && list_is_empty(&listed)
        );
    }

    {
        let (state, _receiver) = test_state();
        let invalid = super::route_http_request(
            "POST",
            username_post_route,
            None,
            r#"{"username":" "}"#,
            &state,
        )
        .await
        .expect("invalid username ban");
        let missing = super::route_http_request("POST", username_post_route, None, "{}", &state)
            .await
            .expect("missing username ban");
        record!(
            "POST",
            username_post_route,
            "malformed-path-query-or-body",
            invalid.status == "400 Bad Request"
        );
        record!(
            "POST",
            username_post_route,
            "missing-empty-or-conflict-state",
            missing.status == "400 Bad Request"
        );
    }

    {
        let (state, _receiver) = test_state();
        seed_username!(state, "security-reset-user");
        let (restarted, _receiver) = test_state();
        let listed = super::route_http_request("GET", list_route, None, "", &restarted)
            .await
            .expect("reset username bans");
        record!(
            "POST",
            username_post_route,
            "restart-persistence-or-reset",
            list_is_empty(&listed)
        );
    }

    {
        let (state, _receiver) = test_state();
        let responses = futures_util::future::join_all(
            [
                r#"{"username":"security-user-a"}"#,
                r#"{"username":"security-user-b"}"#,
            ]
            .into_iter()
            .map(|body| {
                let state = Arc::clone(&state);
                async move {
                    super::route_http_request("POST", username_post_route, None, body, &state).await
                }
            }),
        )
        .await;
        let listed = super::route_http_request("GET", list_route, None, "", &state)
            .await
            .expect("concurrent username bans readback");
        record!(
            "POST",
            username_post_route,
            "concurrency-and-idempotency",
            responses.iter().all(|response| {
                response
                    .as_ref()
                    .is_ok_and(|value| value.status == "200 OK")
            }) && list_len(&listed, 2)
        );
    }

    {
        let (state, _receiver) = test_state();
        let malformed = super::route_http_request(
            "DELETE",
            "/api/v0/security/bans/username/%20",
            None,
            "",
            &state,
        )
        .await
        .expect("malformed username unban");
        let missing = super::route_http_request(
            "DELETE",
            "/api/v0/security/bans/username/security-missing-user",
            None,
            "",
            &state,
        )
        .await
        .expect("missing username unban");
        record!(
            "DELETE",
            username_delete_route,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
        record!(
            "DELETE",
            username_delete_route,
            "missing-empty-or-conflict-state",
            missing.status == "404 Not Found"
        );
    }

    {
        let (state, _receiver) = test_state();
        seed_username!(state, "security-reset-delete-user");
        let (restarted, _receiver) = test_state();
        let deleted = super::route_http_request(
            "DELETE",
            "/api/v0/security/bans/username/security-reset-delete-user",
            None,
            "",
            &restarted,
        )
        .await
        .expect("reset username unban");
        record!(
            "DELETE",
            username_delete_route,
            "restart-persistence-or-reset",
            deleted.status == "404 Not Found"
        );
    }

    {
        let (state, _receiver) = test_state();
        seed_username!(state, "security-concurrent-delete-user");
        let responses = futures_util::future::join_all((0..2).map(|_| {
            let state = Arc::clone(&state);
            async move {
                super::route_http_request(
                    "DELETE",
                    "/api/v0/security/bans/username/security-concurrent-delete-user",
                    None,
                    "",
                    &state,
                )
                .await
            }
        }))
        .await;
        let listed = super::route_http_request("GET", list_route, None, "", &state)
            .await
            .expect("concurrent username unban readback");
        record!(
            "DELETE",
            username_delete_route,
            "concurrency-and-idempotency",
            one_status(&responses, "200 OK")
                && one_status(&responses, "404 Not Found")
                && list_is_empty(&listed)
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create security ban evidence directory");
    fs::write(
        evidence_dir.join("security_ban_residuals.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize security ban ledger"),
    )
    .expect("write security ban ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api security-ban mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential proof for the fixed and read-only SecurityController
/// diagnostics. Missing optional services intentionally retain the frozen
/// 404/503 responses, while local projections are checked for their
/// empty and populated shapes.
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
async fn controller_api_differential_security_diagnostics_residuals() {
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
    state
        .incoming_connection_ips
        .lock()
        .expect("security network fixture lock")
        .insert(
            "203.0.113.40".parse().expect("security network fixture IP"),
            2,
        );

    for (route, case, query, expected_status) in [
        (
            "/api/v0/security/adversarial/stats",
            "malformed-path-query-or-body",
            "?unexpected=not-a-number",
            "200 OK",
        ),
        (
            "/api/v0/security/adversarial/stats",
            "missing-empty-or-conflict-state",
            "",
            "200 OK",
        ),
        (
            "/api/v0/security/circuits/stats",
            "malformed-path-query-or-body",
            "?unexpected=not-a-number",
            "200 OK",
        ),
        (
            "/api/v0/security/circuits/stats",
            "missing-empty-or-conflict-state",
            "",
            "200 OK",
        ),
        (
            "/api/v0/security/network",
            "malformed-path-query-or-body",
            "?unexpected=not-a-number",
            "200 OK",
        ),
        (
            "/api/v0/security/network",
            "missing-empty-or-conflict-state",
            "",
            "200 OK",
        ),
        (
            "/api/v0/security/network",
            "populated-dynamic-state",
            "",
            "200 OK",
        ),
        (
            "/api/v0/security/peers/stats",
            "malformed-path-query-or-body",
            "?unexpected=not-a-number",
            "200 OK",
        ),
        (
            "/api/v0/security/peers/stats",
            "missing-empty-or-conflict-state",
            "",
            "200 OK",
        ),
    ] {
        let response =
            super::route_http_request("GET", &format!("{route}{query}"), None, "", &state)
                .await
                .unwrap_or_else(|error| panic!("GET {route}: {error}"));
        record!(
            "GET",
            route,
            case,
            response.status == expected_status
                && !response.body.is_empty()
                && (route != "/api/v0/security/network"
                    || case != "populated-dynamic-state"
                    || response.body.contains("\"trackedIps\":1"))
        );
    }

    for (route, query, case) in [
        (
            "/api/v0/security/anomalies",
            "?count=0",
            "malformed-path-query-or-body",
        ),
        (
            "/api/v0/security/anomalies",
            "",
            "missing-empty-or-conflict-state",
        ),
        (
            "/api/v0/security/events",
            "?count=0",
            "malformed-path-query-or-body",
        ),
        (
            "/api/v0/security/events",
            "",
            "missing-empty-or-conflict-state",
        ),
    ] {
        let response =
            super::route_http_request("GET", &format!("{route}{query}"), None, "", &state)
                .await
                .unwrap_or_else(|error| panic!("GET {route}: {error}"));
        record!(
            "GET",
            route,
            case,
            if query.is_empty() {
                response.status == "200 OK"
                    && serde_json::from_str::<serde_json::Value>(&response.body)
                        .ok()
                        .is_some_and(|value| value == serde_json::json!([]))
            } else {
                response.status == "400 Bad Request" && response.body.contains("must be positive")
            }
        );
    }

    {
        state.events.write().await.record(
            "security.differential",
            "diagnostic",
            Some("populated security event".to_owned()),
        );
        let events =
            super::route_http_request("GET", "/api/v0/security/events?count=10", None, "", &state)
                .await
                .expect("populated security events");
        let anomalies = super::route_http_request(
            "GET",
            "/api/v0/security/anomalies?count=10",
            None,
            "",
            &state,
        )
        .await
        .expect("populated security anomalies");
        record!(
            "GET",
            "/api/v0/security/events",
            "populated-dynamic-state",
            events.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&events.body)
                    .ok()
                    .is_some_and(|value| value.as_array().is_some_and(|rows| !rows.is_empty()))
        );
        record!(
            "GET",
            "/api/v0/security/anomalies",
            "populated-dynamic-state",
            anomalies.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&anomalies.body)
                    .ok()
                    .is_some_and(|value| value.as_array().is_some_and(|rows| !rows.is_empty()))
        );
    }

    for (route, case, query, expected) in [
        (
            "/api/v0/security/network/top",
            "malformed-path-query-or-body",
            "?limit=0",
            "limit must be positive",
        ),
        (
            "/api/v0/security/peers",
            "malformed-path-query-or-body",
            "?unexpected=not-a-number",
            "",
        ),
    ] {
        let response =
            super::route_http_request("GET", &format!("{route}{query}"), None, "", &state)
                .await
                .unwrap_or_else(|error| panic!("GET {route}: {error}"));
        record!(
            "GET",
            route,
            case,
            if expected.is_empty() {
                response.status == "200 OK"
            } else {
                response.status == "400 Bad Request" && response.body.contains(expected)
            }
        );
    }

    for (route, case, query) in [
        (
            "/api/v0/security/network/top",
            "missing-empty-or-conflict-state",
            "",
        ),
        (
            "/api/v0/security/peers",
            "missing-empty-or-conflict-state",
            "",
        ),
        (
            "/api/v0/security/scanners",
            "missing-empty-or-conflict-state",
            "",
        ),
        (
            "/api/v0/security/threats",
            "missing-empty-or-conflict-state",
            "",
        ),
        (
            "/api/v0/security/transports",
            "missing-empty-or-conflict-state",
            "",
        ),
    ] {
        let response =
            super::route_http_request("GET", &format!("{route}{query}"), None, "", &state)
                .await
                .unwrap_or_else(|error| panic!("GET {route}: {error}"));
        record!(
            "GET",
            route,
            case,
            response.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&response.body).is_ok()
        );
    }

    {
        let mut users = state.users.write().await;
        users.records.push(super::UserRecord {
            username: "security-peer".to_owned(),
            watched: true,
            status: Some("online".to_owned()),
            privileged: false,
            average_speed: None,
            upload_count: None,
            file_count: None,
            directory_count: None,
            updated_at: super::unix_timestamp(),
        });
    }
    for route in ["/api/v0/security/network/top", "/api/v0/security/peers"] {
        let response = super::route_http_request("GET", route, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("GET {route}: {error}"));
        record!(
            "GET",
            route,
            "populated-dynamic-state",
            response.status == "200 OK"
                && (route.ends_with("/peers") && response.body.contains("security-peer")
                    || route.ends_with("/network/top") && response.body.contains("security-peer"))
        );
    }

    for route in ["/api/v0/security/canaries", "/api/v0/security/tor/status"] {
        for case in [
            "nominal-status-headers-body",
            "malformed-path-query-or-body",
        ] {
            let path = if case == "malformed-path-query-or-body" {
                format!("{route}?unexpected=not-a-number")
            } else {
                route.to_owned()
            };
            let response = super::route_http_request("GET", &path, None, "", &state)
                .await
                .unwrap_or_else(|error| panic!("GET {route}: {error}"));
            record!("GET", route, case, response.status == "404 Not Found");
        }
    }

    for (route, expected_status) in [
        ("/api/v0/security/tor/test", "404 Not Found"),
        (
            "/api/v0/security/transports/test",
            "503 Service Unavailable",
        ),
    ] {
        for case in [
            "nominal-status-headers-body",
            "malformed-path-query-or-body",
            "missing-empty-or-conflict-state",
            "mutation-side-effects-and-readback",
            "restart-persistence-or-reset",
            "concurrency-and-idempotency",
        ] {
            let path = if case == "malformed-path-query-or-body" {
                format!("{route}?unexpected=not-a-number")
            } else {
                route.to_owned()
            };
            let pass = match case {
                "restart-persistence-or-reset" => {
                    let (restarted, _receiver) = test_state();
                    super::route_http_request("POST", &path, None, "not-json", &restarted)
                        .await
                        .is_ok_and(|response| response.status == expected_status)
                }
                "concurrency-and-idempotency" => {
                    let responses = futures_util::future::join_all((0..2).map(|_| {
                        let state = Arc::clone(&state);
                        let path = path.clone();
                        async move {
                            super::route_http_request("POST", &path, None, "not-json", &state).await
                        }
                    }))
                    .await;
                    responses.iter().all(|response| {
                        response
                            .as_ref()
                            .is_ok_and(|value| value.status == expected_status)
                    })
                }
                _ => super::route_http_request("POST", &path, None, "not-json", &state)
                    .await
                    .is_ok_and(|response| response.status == expected_status),
            };
            record!("POST", route, case, pass);
        }
    }

    for (route, case, query) in [
        (
            "/api/v0/security/circuits",
            "malformed-path-query-or-body",
            "?unexpected=not-a-number",
        ),
        (
            "/api/v0/security/circuits",
            "missing-empty-or-conflict-state",
            "",
        ),
    ] {
        let response =
            super::route_http_request("GET", &format!("{route}{query}"), None, "", &state)
                .await
                .unwrap_or_else(|error| panic!("GET {route}: {error}"));
        record!(
            "GET",
            route,
            case,
            response.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&response.body)
                    .ok()
                    .is_some_and(|value| value.as_array().is_some_and(|rows| rows.is_empty()))
        );
    }

    {
        state
            .controller_features
            .write_for_test()
            .await
            .upsert(
                "security/circuit/differential".to_owned(),
                serde_json::json!({
                    "circuitId": "differential",
                    "peerId": "security-peer",
                    "active": true,
                }),
            )
            .expect("populate circuit stats");
        let response =
            super::route_http_request("GET", "/api/v0/security/circuits/stats", None, "", &state)
                .await
                .expect("empty circuit stats");
        record!(
            "GET",
            "/api/v0/security/circuits/stats",
            "populated-dynamic-state",
            response.status == "200 OK"
                && response.body.contains("totalCircuits")
                && response.body.contains("\"totalCircuits\":1")
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create security diagnostics evidence directory");
    fs::write(
        evidence_dir.join("security_diagnostics_residuals.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize security diagnostics ledger"),
    )
    .expect("write security diagnostics ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api security-diagnostics mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential proof for the remaining versioned SoulseekDiscovery
/// controller cases. The frozen controller normalizes body and route
/// items before invoking the client, returns NoContent for every valid
/// interest mutation (including duplicate/absent removals), and exposes
/// protocol DTO shapes for recommendations, similar users, user
/// interests, and capability projections. Disabled rendezvous operations
/// intentionally retain the frozen 403 contract.
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
async fn controller_api_differential_soulseek_discovery_residuals() {
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

    // The frozen service receives item values, not slskR's internal
    // generated IDs. Exercise both kinds so the versioned route is
    // compatible with the frozen web client and remains backwards-safe.
    for (route, read_route, hated) in [
        (
            "/api/v0/soulseek/interests",
            "/api/v0/soulseek/interests",
            false,
        ),
        (
            "/api/v0/soulseek/hated-interests",
            "/api/v0/soulseek/hated-interests",
            true,
        ),
    ] {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        state.session.write().await.state = "connected";
        let nominal = super::route_http_request(
            "POST",
            route,
            None,
            if hated {
                r#"{"item":"nominal-hated"}"#
            } else {
                r#"{"item":"nominal-liked"}"#
            },
            &state,
        )
        .await
        .unwrap();
        record!(
            "POST",
            route,
            "nominal-status-headers-body",
            nominal.status == "204 No Content"
                && nominal.content_type == "application/json"
                && nominal.body.is_empty()
        );
        let malformed = super::route_http_request("POST", route, None, "{}", &state)
            .await
            .unwrap();
        record!(
            "POST",
            route,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request" && malformed.body.contains("item is required")
        );
        let missing = super::route_http_request("POST", route, None, "", &state)
            .await
            .unwrap();
        record!(
            "POST",
            route,
            "missing-empty-or-conflict-state",
            missing.status == "400 Bad Request" && missing.body.contains("item is required")
        );

        let (first_state, _receiver) = test_state_with_env(base_env.clone());
        first_state.session.write().await.state = "connected";
        let created = super::route_http_request(
            "POST",
            route,
            None,
            if hated {
                r#"{"item":"reset-hated"}"#
            } else {
                r#"{"item":"reset-liked"}"#
            },
            &first_state,
        )
        .await
        .unwrap();
        let (restarted_state, _receiver) = test_state_with_env(base_env.clone());
        let restarted_read =
            super::route_http_request("GET", read_route, None, "", &restarted_state)
                .await
                .unwrap();
        let restarted_json =
            serde_json::from_str::<serde_json::Value>(&restarted_read.body).unwrap_or_default();
        record!(
            "POST",
            route,
            "restart-persistence-or-reset",
            created.status == "204 No Content"
                && restarted_read.status == "200 OK"
                && restarted_json["count"] == 0
        );

        let (concurrent_state, _receiver) = test_state_with_env(base_env.clone());
        concurrent_state.session.write().await.state = "connected";
        let body = if hated {
            r#"{"item":"concurrent-hated"}"#
        } else {
            r#"{"item":"concurrent-liked"}"#
        };
        let responses = futures_util::future::join_all((0..2).map(|_| {
            let state = Arc::clone(&concurrent_state);
            async move { super::route_http_request("POST", route, None, body, &state).await }
        }))
        .await;
        let concurrent_read =
            super::route_http_request("GET", read_route, None, "", &concurrent_state)
                .await
                .unwrap();
        let concurrent_json =
            serde_json::from_str::<serde_json::Value>(&concurrent_read.body).unwrap_or_default();
        record!(
            "POST",
            route,
            "concurrency-and-idempotency",
            responses.iter().all(|response| {
                response
                    .as_ref()
                    .is_ok_and(|response| response.status == "204 No Content")
            }) && concurrent_read.status == "200 OK"
                && concurrent_json["count"] == 1
        );

        if hated {
            let (mutation_state, _receiver) = test_state_with_env(base_env.clone());
            mutation_state.session.write().await.state = "connected";
            let created = super::route_http_request(
                "POST",
                route,
                None,
                r#"{"item":"readback-hated"}"#,
                &mutation_state,
            )
            .await
            .unwrap();
            let readback = super::route_http_request("GET", read_route, None, "", &mutation_state)
                .await
                .unwrap();
            let readback_json =
                serde_json::from_str::<serde_json::Value>(&readback.body).unwrap_or_default();
            record!(
                "POST",
                route,
                "mutation-side-effects-and-readback",
                created.status == "204 No Content"
                    && readback_json["count"] == 1
                    && readback.body.contains("readback-hated")
            );
        }
    }

    for (route, read_route, hated) in [
        (
            "/api/v0/soulseek/interests/{item}",
            "/api/v0/soulseek/interests",
            false,
        ),
        (
            "/api/v0/soulseek/hated-interests/{item}",
            "/api/v0/soulseek/hated-interests",
            true,
        ),
    ] {
        let route_path = if hated {
            "/api/v0/soulseek/hated-interests"
        } else {
            "/api/v0/soulseek/interests"
        };
        let (state, _receiver) = test_state_with_env(base_env.clone());
        state.session.write().await.state = "connected";
        let nominal_item = if hated {
            "nominal-delete-hated"
        } else {
            "nominal-delete-liked"
        };
        let nominal_added = super::route_http_request(
            "POST",
            route_path,
            None,
            &format!(r#"{{"item":"{nominal_item}"}}"#),
            &state,
        )
        .await
        .unwrap();
        let nominal = super::route_http_request(
            "DELETE",
            &format!("{route_path}/{nominal_item}"),
            None,
            "",
            &state,
        )
        .await
        .unwrap();
        record!(
            "DELETE",
            route,
            "nominal-status-headers-body",
            nominal_added.status == "204 No Content"
                && nominal.status == "204 No Content"
                && nominal.content_type == "application/json"
                && nominal.body.is_empty()
        );
        let malformed =
            super::route_http_request("DELETE", &format!("{route_path}/%20"), None, "", &state)
                .await
                .unwrap();
        record!(
            "DELETE",
            route,
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request" && malformed.body.contains("item is required")
        );

        let missing =
            super::route_http_request("DELETE", &format!("{route_path}/missing"), None, "", &state)
                .await
                .unwrap();
        record!(
            "DELETE",
            route,
            "missing-empty-or-conflict-state",
            missing.status == "204 No Content"
        );

        let (mutation_state, _receiver) = test_state_with_env(base_env.clone());
        mutation_state.session.write().await.state = "connected";
        let add_body = if hated {
            r#"{"item":"delete-hated"}"#
        } else {
            r#"{"item":"delete-liked"}"#
        };
        let added = super::route_http_request("POST", route_path, None, add_body, &mutation_state)
            .await
            .unwrap();
        let deleted = super::route_http_request(
            "DELETE",
            &format!(
                "{route_path}/{}",
                if hated {
                    "delete-hated"
                } else {
                    "delete-liked"
                }
            ),
            None,
            "",
            &mutation_state,
        )
        .await
        .unwrap();
        let after_delete = super::route_http_request("GET", read_route, None, "", &mutation_state)
            .await
            .unwrap();
        let after_delete_json =
            serde_json::from_str::<serde_json::Value>(&after_delete.body).unwrap_or_default();
        record!(
            "DELETE",
            route,
            "mutation-side-effects-and-readback",
            added.status == "204 No Content"
                && deleted.status == "204 No Content"
                && after_delete_json["count"] == 0
        );

        let (before_restart, _receiver) = test_state_with_env(base_env.clone());
        before_restart.session.write().await.state = "connected";
        let _ = super::route_http_request(
            "POST",
            route_path,
            None,
            r#"{"item":"restart-delete"}"#,
            &before_restart,
        )
        .await
        .unwrap();
        let (restarted_state, _receiver) = test_state_with_env(base_env.clone());
        restarted_state.session.write().await.state = "connected";
        let restarted_delete = super::route_http_request(
            "DELETE",
            &format!("{route_path}/restart-delete"),
            None,
            "",
            &restarted_state,
        )
        .await
        .unwrap();
        record!(
            "DELETE",
            route,
            "restart-persistence-or-reset",
            restarted_delete.status == "204 No Content"
        );

        let (concurrent_state, _receiver) = test_state_with_env(base_env.clone());
        concurrent_state.session.write().await.state = "connected";
        let _ = super::route_http_request(
            "POST",
            route_path,
            None,
            r#"{"item":"concurrent-delete"}"#,
            &concurrent_state,
        )
        .await
        .unwrap();
        let delete_responses = futures_util::future::join_all((0..2).map(|_| {
            let state = Arc::clone(&concurrent_state);
            let path = format!("{route_path}/concurrent-delete");
            async move { super::route_http_request("DELETE", &path, None, "", &state).await }
        }))
        .await;
        record!(
            "DELETE",
            route,
            "concurrency-and-idempotency",
            delete_responses.iter().all(|response| {
                response
                    .as_ref()
                    .is_ok_and(|response| response.status == "204 No Content")
            })
        );
    }

    // Interest rendezvous mutation is disabled by default in both the
    // frozen MeshOptions and slskR's versioned guard. Exercise each
    // residual request shape against that real 403 contract.
    for (method, route) in [
        ("POST", "/api/v0/soulseek/mesh-rendezvous/interest"),
        ("DELETE", "/api/v0/soulseek/mesh-rendezvous/interest"),
    ] {
        for case in [
            "nominal-status-headers-body",
            "malformed-path-query-or-body",
            "runtime-failure-and-timeout",
            "mutation-side-effects-and-readback",
            "restart-persistence-or-reset",
            "concurrency-and-idempotency",
        ] {
            let path = if case == "malformed-path-query-or-body" {
                format!("{route}?unexpected=not-a-number")
            } else {
                route.to_owned()
            };
            let pass = if case == "restart-persistence-or-reset" {
                let (fresh, _receiver) = test_state_with_env(base_env.clone());
                super::route_http_request(method, &path, None, "not-json", &fresh)
                    .await
                    .is_ok_and(|response| response.status == "403 Forbidden")
            } else if case == "concurrency-and-idempotency" {
                let (fresh, _receiver) = test_state_with_env(base_env.clone());
                let responses = futures_util::future::join_all((0..2).map(|_| {
                    let state = Arc::clone(&fresh);
                    let path = path.clone();
                    async move {
                        super::route_http_request(method, &path, None, "not-json", &state).await
                    }
                }))
                .await;
                responses.iter().all(|response| {
                    response
                        .as_ref()
                        .is_ok_and(|response| response.status == "403 Forbidden")
                })
            } else {
                let (fresh, _receiver) = test_state_with_env(base_env.clone());
                super::route_http_request(method, &path, None, "not-json", &fresh)
                    .await
                    .is_ok_and(|response| response.status == "403 Forbidden")
            };
            record!(method, route, case, pass);
        }
    }

    for (route, ledger_route, similar_users) in [
        (
            "/api/v0/soulseek/items/ambient/recommendations",
            "/api/v0/soulseek/items/{item}/recommendations",
            false,
        ),
        (
            "/api/v0/soulseek/items/ambient/similar-users",
            "/api/v0/soulseek/items/{item}/similar-users",
            true,
        ),
    ] {
        let malformed = super::route_http_request(
            "GET",
            &format!("{route}?unexpected=not-a-number"),
            None,
            "",
            &test_state_with_env(base_env.clone()).0,
        )
        .await
        .unwrap();
        let malformed_json =
            serde_json::from_str::<serde_json::Value>(&malformed.body).unwrap_or_default();
        record!(
            "GET",
            ledger_route,
            "malformed-path-query-or-body",
            malformed.status == "200 OK"
                && malformed_json["item"] == "ambient"
                && malformed_json
                    .get(if similar_users {
                        "usernames"
                    } else {
                        "recommendations"
                    })
                    .is_some()
        );

        let missing_state = test_state_with_env(base_env.clone()).0;
        let missing = super::route_http_request(
            "GET",
            "/api/v0/soulseek/items/%20/recommendations"
                .replace(
                    "/recommendations",
                    if similar_users {
                        "/similar-users"
                    } else {
                        "/recommendations"
                    },
                )
                .as_str(),
            None,
            "",
            &missing_state,
        )
        .await
        .unwrap();
        record!(
            "GET",
            ledger_route,
            "missing-empty-or-conflict-state",
            missing.status == "400 Bad Request" && missing.body.contains("item is required")
        );

        let runtime_state = test_state_with_env(base_env.clone()).0;
        let runtime = super::route_http_request("GET", route, None, "", &runtime_state)
            .await
            .unwrap();
        let runtime_json =
            serde_json::from_str::<serde_json::Value>(&runtime.body).unwrap_or_default();
        record!(
            "GET",
            ledger_route,
            "runtime-failure-and-timeout",
            runtime.status == "200 OK" && runtime_json["item"] == "ambient"
        );

        let populated_state = test_state_with_env(base_env.clone()).0;
        if similar_users {
            populated_state
                .users
                .write()
                .await
                .watch("discovery-similar-peer".to_owned());
        } else {
            populated_state
                .interests
                .write()
                .await
                .add_liked("ambient".to_owned());
        }
        let populated = super::route_http_request("GET", route, None, "", &populated_state)
            .await
            .unwrap();
        let populated_json =
            serde_json::from_str::<serde_json::Value>(&populated.body).unwrap_or_default();
        let populated_values = populated_json
            .get(if similar_users {
                "usernames"
            } else {
                "recommendations"
            })
            .and_then(serde_json::Value::as_array);
        record!(
            "GET",
            ledger_route,
            "populated-dynamic-state",
            populated.status == "200 OK"
                && populated_values.is_some_and(|values| !values.is_empty())
        );
    }

    for route in [
        "/api/v0/soulseek/recommendations",
        "/api/v0/soulseek/recommendations/global",
    ] {
        let malformed_state = test_state_with_env(base_env.clone()).0;
        let malformed = super::route_http_request(
            "GET",
            &format!("{route}?unexpected=not-a-number"),
            None,
            "",
            &malformed_state,
        )
        .await
        .unwrap();
        let malformed_json =
            serde_json::from_str::<serde_json::Value>(&malformed.body).unwrap_or_default();
        record!(
            "GET",
            route,
            "malformed-path-query-or-body",
            malformed.status == "200 OK"
                && malformed_json["recommendations"] == serde_json::json!([])
                && malformed_json["unrecommendations"] == serde_json::json!([])
        );

        let missing_state = test_state_with_env(base_env.clone()).0;
        let missing = super::route_http_request("GET", route, None, "", &missing_state)
            .await
            .unwrap();
        let missing_json =
            serde_json::from_str::<serde_json::Value>(&missing.body).unwrap_or_default();
        record!(
            "GET",
            route,
            "missing-empty-or-conflict-state",
            missing.status == "200 OK"
                && missing_json["recommendations"] == serde_json::json!([])
                && missing_json["unrecommendations"] == serde_json::json!([])
        );

        let runtime_state = test_state_with_env(base_env.clone()).0;
        runtime_state.session.write().await.state = "disconnected";
        let runtime = super::route_http_request("GET", route, None, "", &runtime_state)
            .await
            .unwrap();
        let runtime_json =
            serde_json::from_str::<serde_json::Value>(&runtime.body).unwrap_or_default();
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            runtime.status == "200 OK" && runtime_json["recommendations"] == serde_json::json!([])
        );

        let populated_state = test_state_with_env(base_env.clone()).0;
        populated_state
            .interests
            .write()
            .await
            .add_liked("ambient".to_owned());
        populated_state
            .interests
            .write()
            .await
            .add_hated("noise".to_owned());
        let populated = super::route_http_request("GET", route, None, "", &populated_state)
            .await
            .unwrap();
        let populated_json =
            serde_json::from_str::<serde_json::Value>(&populated.body).unwrap_or_default();
        record!(
            "GET",
            route,
            "populated-dynamic-state",
            populated.status == "200 OK"
                && populated_json["recommendations"]
                    .as_array()
                    .is_some_and(|values| !values.is_empty())
                && populated_json["unrecommendations"]
                    .as_array()
                    .is_some_and(|values| !values.is_empty())
        );
    }

    for route in [
        "/api/v0/soulseek/mesh-rendezvous/status",
        "/api/v0/soulseek/peer-capabilities",
    ] {
        let malformed_state = test_state_with_env(base_env.clone()).0;
        let malformed = super::route_http_request(
            "GET",
            &format!("{route}?unexpected=not-a-number"),
            None,
            "",
            &malformed_state,
        )
        .await
        .unwrap();
        let malformed_json =
            serde_json::from_str::<serde_json::Value>(&malformed.body).unwrap_or_default();
        let malformed_pass = if route.ends_with("status") {
            malformed.status == "200 OK"
                && malformed_json["interestTag"].is_string()
                && malformed_json["privacy"].is_string()
        } else {
            malformed.status == "200 OK" && malformed_json.as_array().is_some()
        };
        record!("GET", route, "malformed-path-query-or-body", malformed_pass);

        let missing_state = test_state_with_env(base_env.clone()).0;
        let missing = super::route_http_request("GET", route, None, "", &missing_state)
            .await
            .unwrap();
        let missing_json =
            serde_json::from_str::<serde_json::Value>(&missing.body).unwrap_or_default();
        let missing_pass = if route.ends_with("status") {
            missing.status == "200 OK" && missing_json["enabled"].is_boolean()
        } else {
            missing.status == "200 OK"
                && missing_json
                    .as_array()
                    .is_some_and(|values| values.is_empty())
        };
        record!(
            "GET",
            route,
            "missing-empty-or-conflict-state",
            missing_pass
        );

        let runtime_state = test_state_with_env(base_env.clone()).0;
        let runtime = super::route_http_request("GET", route, None, "", &runtime_state)
            .await
            .unwrap();
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            runtime.status == "200 OK"
        );

        let populated_state = test_state_with_env(base_env.clone()).0;
        if route.ends_with("status") {
            populated_state
                .users
                .write()
                .await
                .watch("mesh-populated-peer".to_owned());
            populated_state
                .mesh
                .write()
                .await
                .capability_records
                .push(test_capability_descriptor(
                    "mesh-populated-peer",
                    vec![slskr_client::capabilities::FEATURE_MESH_V1.to_owned()],
                ));
        } else {
            populated_state
                .mesh
                .write()
                .await
                .capability_records
                .push(test_capability_descriptor(
                    "capability-populated-peer",
                    vec![slskr_client::capabilities::FEATURE_MESH_V1.to_owned()],
                ));
        }
        let populated = super::route_http_request("GET", route, None, "", &populated_state)
            .await
            .unwrap();
        let populated_json =
            serde_json::from_str::<serde_json::Value>(&populated.body).unwrap_or_default();
        let populated_pass = if route.ends_with("status") {
            populated.status == "200 OK" && populated_json["candidateCount"] == 1
        } else {
            populated.status == "200 OK"
                && populated_json.as_array().is_some_and(|values| {
                    values
                        .iter()
                        .any(|value| value["username"] == "capability-populated-peer")
                })
        };
        record!("GET", route, "populated-dynamic-state", populated_pass);
    }

    for route in [
        "/api/v0/soulseek/mesh-rendezvous/discover",
        "/api/v0/soulseek/mesh-rendezvous/users",
    ] {
        for case in [
            "nominal-status-headers-body",
            "malformed-path-query-or-body",
            "runtime-failure-and-timeout",
            "populated-dynamic-state",
        ] {
            let state = test_state_with_env(base_env.clone()).0;
            let path = if case == "malformed-path-query-or-body" {
                format!("{route}?unexpected=not-a-number")
            } else {
                route.to_owned()
            };
            let response = super::route_http_request("GET", &path, None, "", &state)
                .await
                .unwrap();
            record!(
                "GET",
                route,
                case,
                response.status == "403 Forbidden" && response.body.contains("feature is disabled")
            );
        }
    }

    let similar_users_route = "/api/v0/soulseek/users/similar";
    let malformed_state = test_state_with_env(base_env.clone()).0;
    let malformed = super::route_http_request(
        "GET",
        &format!("{similar_users_route}?unexpected=not-a-number"),
        None,
        "",
        &malformed_state,
    )
    .await
    .unwrap();
    let malformed_json =
        serde_json::from_str::<serde_json::Value>(&malformed.body).unwrap_or_default();
    record!(
        "GET",
        similar_users_route,
        "malformed-path-query-or-body",
        malformed.status == "200 OK" && malformed_json.as_array().is_some()
    );

    let missing_state = test_state_with_env(base_env.clone()).0;
    let missing = super::route_http_request("GET", similar_users_route, None, "", &missing_state)
        .await
        .unwrap();
    let missing_json = serde_json::from_str::<serde_json::Value>(&missing.body).unwrap_or_default();
    record!(
        "GET",
        similar_users_route,
        "missing-empty-or-conflict-state",
        missing.status == "200 OK"
            && missing_json
                .as_array()
                .is_some_and(|values| values.is_empty())
    );

    let runtime_state = test_state_with_env(base_env.clone()).0;
    let runtime = super::route_http_request("GET", similar_users_route, None, "", &runtime_state)
        .await
        .unwrap();
    let runtime_json = serde_json::from_str::<serde_json::Value>(&runtime.body).unwrap_or_default();
    record!(
        "GET",
        similar_users_route,
        "runtime-failure-and-timeout",
        runtime.status == "200 OK" && runtime_json.as_array().is_some()
    );

    let populated_state = test_state_with_env(base_env.clone()).0;
    populated_state
        .users
        .write()
        .await
        .watch("similar-populated-peer".to_owned());
    let populated =
        super::route_http_request("GET", similar_users_route, None, "", &populated_state)
            .await
            .unwrap();
    let populated_json =
        serde_json::from_str::<serde_json::Value>(&populated.body).unwrap_or_default();
    record!(
        "GET",
        similar_users_route,
        "populated-dynamic-state",
        populated.status == "200 OK"
            && populated_json.as_array().is_some_and(|values| {
                values
                    .iter()
                    .any(|value| value["username"] == "similar-populated-peer")
            })
    );

    for case in [
        "malformed-path-query-or-body",
        "missing-empty-or-conflict-state",
        "runtime-failure-and-timeout",
    ] {
        let path = if case == "malformed-path-query-or-body" {
            "/api/v0/soulseek/users/%20/interests"
        } else {
            "/api/v0/soulseek/users/missing-peer/interests"
        };
        let state = test_state_with_env(base_env.clone()).0;
        if case != "malformed-path-query-or-body" {
            state.session.write().await.state = "disconnected";
        }
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap();
        record!(
            "GET",
            "/api/v0/soulseek/users/{username}/interests",
            case,
            if case == "malformed-path-query-or-body" {
                response.status == "400 Bad Request"
                    && response.body.contains("username is required")
            } else {
                response.status == "503 Service Unavailable"
            }
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create SoulseekDiscovery evidence directory");
    fs::write(
        evidence_dir.join("soulseek_discovery_residuals.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize SoulseekDiscovery ledger"),
    )
    .expect("write SoulseekDiscovery ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api SoulseekDiscovery mismatches:\n{}",
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
async fn controller_api_differential_multisource_residuals() {
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

    let seed_search = |state: Arc<super::AppState>| async move {
        let mut searches = state.searches.write().await;
        searches.records.push(super::SearchRecord {
            id: "multisource-search".to_owned(),
            token: 901,
            query: "ambient".to_owned(),
            target: "global",
            target_name: None,
            status: "completed",
            results: vec![
                super::SearchResultEntry {
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
                super::SearchResultEntry {
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
            ttl_seconds: super::DEFAULT_SEARCH_TTL_SECONDS,
            expires_at: u64::MAX,
            created_at: 1,
            updated_at: 1,
        });
    };

    let seed_job = |state: Arc<super::AppState>| async move {
        let id = "11111111-1111-4111-8111-111111111111".to_owned();
        state
            .multisource
            .write()
            .await
            .insert(super::multisource::SwarmJob {
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
        let response = super::route_http_request("GET", path, None, "", &state)
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
        super::route_http_request("GET", "/api/v0/multisource/jobs", None, "", &populated_jobs)
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
        let response = super::route_http_request("GET", &detail_route, None, "", &detail_state)
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
    let missing_detail = super::route_http_request(
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
            let response = super::route_http_request(
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
        let response = super::route_http_request(
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
        let response = super::route_http_request("GET", files_route, None, "", &files_state)
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
    let malformed_files = super::route_http_request(
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
        let valid = super::route_http_request("POST", route, None, body, &valid_state)
            .await
            .unwrap();
        record!(
            "POST",
            route,
            "nominal-status-headers-body",
            valid.status == "200 OK" && valid.body.contains("Track.flac")
        );
        let malformed = super::route_http_request(
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
            let response = super::route_http_request("POST", route, None, body, &state)
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
            async move { super::route_http_request("POST", route, None, body, &state).await }
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

    let download_valid = super::route_http_request(
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
    let malformed_download = super::route_http_request(
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
        let response = super::route_http_request(
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
            super::route_http_request(
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
        let valid = super::route_http_request("POST", route, None, body, &valid_state)
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
        let malformed = super::route_http_request(
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
        let missing = super::route_http_request(
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
            let response = super::route_http_request("POST", route, None, body, &state)
                .await
                .unwrap();
            record!("POST", route, case, response.status == "200 OK");
        }
        let concurrent_state = test_state_with_env(base_env.clone()).0;
        seed_search(concurrent_state.clone()).await;
        let responses = futures_util::future::join_all((0..2).map(|_| {
            let state = concurrent_state.clone();
            async move { super::route_http_request("POST", route, None, body, &state).await }
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
        let response = super::route_http_request(
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
                    super::route_http_request(
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
            super::route_http_request("POST", "/api/v0/multisource/test", None, test_body, &state)
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
        let response = super::route_http_request(
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
    let malformed_verify = super::route_http_request(
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
    let missing_verify = super::route_http_request(
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
            super::route_http_request(
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
async fn controller_api_differential_podcore_route_value_validation() {
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

    let blank_backfill = super::route_http_request(
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

    let blank_count_pod = super::route_http_request(
        "GET",
        "/api/v0/podcore/messages/%20/general/count",
        None,
        "",
        &state,
    )
    .await
    .expect("reject blank message-count pod ID");
    let blank_count_channel = super::route_http_request(
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

    let blank_membership_pod = super::route_http_request(
        "GET",
        "/api/v0/podcore/membership/%20/peer",
        None,
        "",
        &state,
    )
    .await
    .expect("reject blank membership pod ID");
    let blank_membership_peer = super::route_http_request(
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

    let blank_verify_pod = super::route_http_request(
        "GET",
        "/api/v0/podcore/membership/%20/peer/verify",
        None,
        "",
        &state,
    )
    .await
    .expect("reject blank membership-verification pod ID");
    let blank_verify_peer = super::route_http_request(
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
        let response = super::route_http_request("GET", path, None, "", &state)
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
        let response = super::route_http_request("GET", path, None, "", &state)
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
        let response = super::route_http_request(method, path, None, "", &state)
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
        let response = super::route_http_request(method, path, None, r#"{"lastSeen":1}"#, &state)
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
            super::route_http_request("POST", path, None, r#"{"role":"member"}"#, &state)
                .await
                .unwrap_or_else(|error| panic!("POST {path}: {error}"));
        record!(
            "POST",
            route,
            response.status == "400 Bad Request"
                && response.body.contains("PodId and PeerId are required")
        );
    }

    let blank_publish = super::route_http_request(
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
        let response = super::route_http_request("PUT", path, None, r#"{"role":"member"}"#, &state)
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
