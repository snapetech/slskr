//! Controller full pods differential 04 ownership.

use super::*;

/// Differential proof for the PodMessageStorageController cleanup,
/// projection, rebuild-index, and vacuum contracts. The fixtures exercise
/// durable state, reloads, concurrent idempotent mutations, and confined
/// persisted-file failures instead of crediting static success responses.
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
pub(super) async fn controller_api_differential_podcore_message_storage() {
    let target = "slskdn";
    let global_cleanup_route = "/api/v0/podcore/messages/cleanup";
    let channel_cleanup_route = "/api/v0/podcore/messages/{podId}/{channelId}/cleanup";
    let stats_route = "/api/v0/podcore/messages/stats";
    let search_route = "/api/v0/podcore/messages/{podId}/search";
    let count_route = "/api/v0/podcore/messages/{podId}/{channelId}/count";
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
                    serde_json::from_value::<crate::pods::PodRecord>(serde_json::json!({
                        "podId": $pod_id,
                        "name": "Message storage differential",
                        "isPublic": true,
                        "channels": [{
                            "channelId": "general",
                            "kind": 0,
                            "name": "General"
                        }]
                    }))
                    .expect("deserialize message storage pod"),
                    "message-owner".to_owned(),
                )
                .expect("create message storage pod");
        }};
    }

    macro_rules! seed_messages {
        ($state:ident, $pod_id:expr, $old_id:expr, $new_id:expr) => {{
            let mut messages = $state.pod_channels.write().await;
            messages
                .append_with_id(
                    $old_id.to_owned(),
                    $pod_id.to_owned(),
                    "general".to_owned(),
                    "message-owner".to_owned(),
                    "old message".to_owned(),
                    String::new(),
                    1,
                    1,
                )
                .expect("seed old message");
            messages
                .append_with_id(
                    $new_id.to_owned(),
                    $pod_id.to_owned(),
                    "general".to_owned(),
                    "message-owner".to_owned(),
                    "new message".to_owned(),
                    String::new(),
                    100,
                    1,
                )
                .expect("seed new message");
        }};
    }

    macro_rules! block_message_path {
        ($state:ident) => {{
            let path = $state.config.state_dir.join("pod-channel-messages.json");
            fs::remove_file(&path).expect("remove message storage file");
            fs::create_dir(&path).expect("block message storage path");
            path
        }};
    }

    let message_env = || {
        MapEnv::default()
            .with("SLSK_USERNAME", "message-owner")
            .with("SLSK_PASSWORD", "test-secret")
    };

    // Global cleanup returns the frozen zero-count result on an empty
    // store, including when the persisted file does not yet exist.
    {
        let (state, _receiver) = test_state_with_env(message_env());
        let response = crate::route_http_request(
            "DELETE",
            "/api/v0/podcore/messages/cleanup?olderThan=10",
            None,
            "",
            &state,
        )
        .await
        .expect("empty global message cleanup");
        record!(
            "DELETE",
            global_cleanup_route,
            "missing-empty-or-conflict-state",
            response.status == "200 OK" && response.body == "0"
        );
    }

    // Global cleanup removes only messages older than the requested
    // cursor and leaves the newer row in the live store.
    {
        let (state, _receiver) = test_state_with_env(message_env());
        seed_messages!(
            state,
            "pod:message-global-mutation",
            "00000000-0000-4000-8000-000000000101",
            "00000000-0000-4000-8000-000000000102"
        );
        let response = crate::route_http_request(
            "DELETE",
            "/api/v0/podcore/messages/cleanup?olderThan=50",
            None,
            "",
            &state,
        )
        .await
        .expect("mutating global message cleanup");
        let remaining = state.pod_channels.read().await.stats().total_messages;
        record!(
            "DELETE",
            global_cleanup_route,
            "mutation-side-effects-and-readback",
            response.status == "200 OK" && response.body == "1" && remaining == 1
        );
    }

    // A cleanup result remains present after reconstructing the message
    // store from its persisted state file.
    {
        let (state, _receiver) = test_state_with_env(message_env());
        seed_messages!(
            state,
            "pod:message-global-restart",
            "00000000-0000-4000-8000-000000000201",
            "00000000-0000-4000-8000-000000000202"
        );
        let response = crate::route_http_request(
            "DELETE",
            "/api/v0/podcore/messages/cleanup?olderThan=50",
            None,
            "",
            &state,
        )
        .await
        .expect("restart global message cleanup");
        let loaded = crate::pod_channels::PodChannelStore::load(&state.config.state_dir)
            .expect("reload global message cleanup");
        record!(
            "DELETE",
            global_cleanup_route,
            "restart-persistence-or-reset",
            response.status == "200 OK"
                && response.body == "1"
                && loaded.stats().total_messages == 1
        );
    }

    // Two concurrent global cleanup requests are serialized by the store:
    // exactly one removes the old row and the other is an idempotent zero.
    {
        let (state, _receiver) = test_state_with_env(message_env());
        seed_messages!(
            state,
            "pod:message-global-concurrent",
            "00000000-0000-4000-8000-000000000301",
            "00000000-0000-4000-8000-000000000302"
        );
        let responses = futures_util::future::join_all((0..2).map(|_| {
            let state = Arc::clone(&state);
            async move {
                crate::route_http_request(
                    "DELETE",
                    "/api/v0/podcore/messages/cleanup?olderThan=50",
                    None,
                    "",
                    &state,
                )
                .await
            }
        }))
        .await;
        let removed_one = responses.iter().any(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK" && response.body == "1")
        });
        let removed_zero = responses.iter().any(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK" && response.body == "0")
        });
        let remaining = state.pod_channels.read().await.stats().total_messages;
        record!(
            "DELETE",
            global_cleanup_route,
            "concurrency-and-idempotency",
            removed_one && removed_zero && remaining == 1
        );
    }

    // A persisted-file failure returns the frozen 500 body and does not
    // silently report a successful cleanup.
    {
        let (state, _receiver) = test_state_with_env(message_env());
        seed_messages!(
            state,
            "pod:message-global-runtime",
            "00000000-0000-4000-8000-000000000401",
            "00000000-0000-4000-8000-000000000402"
        );
        let messages_path = block_message_path!(state);
        let response = crate::route_http_request(
            "DELETE",
            "/api/v0/podcore/messages/cleanup?olderThan=50",
            None,
            "",
            &state,
        )
        .await
        .expect("runtime global message cleanup");
        record!(
            "DELETE",
            global_cleanup_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response
                    .body
                    .contains("An error occurred while cleaning up messages")
        );
        fs::remove_dir(&messages_path).expect("remove blocked global message path");
    }

    // Channel-scoped cleanup has the same durable and idempotent contract
    // while retaining the pod/channel filter.
    {
        let (state, _receiver) = test_state_with_env(message_env());
        seed_pod!(state, "pod:message-channel-empty");
        let response = crate::route_http_request(
            "DELETE",
            "/api/v0/podcore/messages/pod:message-channel-empty/general/cleanup?olderThan=10",
            None,
            "",
            &state,
        )
        .await
        .expect("empty channel message cleanup");
        record!(
            "DELETE",
            channel_cleanup_route,
            "missing-empty-or-conflict-state",
            response.status == "200 OK" && response.body == "0"
        );
    }

    {
        let (state, _receiver) = test_state_with_env(message_env());
        seed_pod!(state, "pod:message-channel-mutation");
        seed_messages!(
            state,
            "pod:message-channel-mutation",
            "00000000-0000-4000-8000-000000000501",
            "00000000-0000-4000-8000-000000000502"
        );
        let response = crate::route_http_request(
            "DELETE",
            "/api/v0/podcore/messages/pod:message-channel-mutation/general/cleanup?olderThan=50",
            None,
            "",
            &state,
        )
        .await
        .expect("mutating channel message cleanup");
        let remaining = state.pod_channels.read().await.stats().total_messages;
        record!(
            "DELETE",
            channel_cleanup_route,
            "mutation-side-effects-and-readback",
            response.status == "200 OK" && response.body == "1" && remaining == 1
        );
    }

    {
        let (state, _receiver) = test_state_with_env(message_env());
        seed_pod!(state, "pod:message-channel-restart");
        seed_messages!(
            state,
            "pod:message-channel-restart",
            "00000000-0000-4000-8000-000000000601",
            "00000000-0000-4000-8000-000000000602"
        );
        let response = crate::route_http_request(
            "DELETE",
            "/api/v0/podcore/messages/pod:message-channel-restart/general/cleanup?olderThan=50",
            None,
            "",
            &state,
        )
        .await
        .expect("restart channel message cleanup");
        let loaded = crate::pod_channels::PodChannelStore::load(&state.config.state_dir)
            .expect("reload channel message cleanup");
        record!(
            "DELETE",
            channel_cleanup_route,
            "restart-persistence-or-reset",
            response.status == "200 OK"
                && response.body == "1"
                && loaded.stats().total_messages == 1
        );
    }

    {
        let (state, _receiver) = test_state_with_env(message_env());
        seed_pod!(state, "pod:message-channel-concurrent");
        seed_messages!(
            state,
            "pod:message-channel-concurrent",
            "00000000-0000-4000-8000-000000000701",
            "00000000-0000-4000-8000-000000000702"
        );
        let responses = futures_util::future::join_all((0..2).map(|_| {
            let state = Arc::clone(&state);
            async move {
                crate::route_http_request(
                    "DELETE",
                    "/api/v0/podcore/messages/pod:message-channel-concurrent/general/cleanup?olderThan=50",
                    None,
                    "",
                    &state,
                )
                .await
            }
        }))
        .await;
        let removed_one = responses.iter().any(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK" && response.body == "1")
        });
        let removed_zero = responses.iter().any(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK" && response.body == "0")
        });
        let remaining = state.pod_channels.read().await.stats().total_messages;
        record!(
            "DELETE",
            channel_cleanup_route,
            "concurrency-and-idempotency",
            removed_one && removed_zero && remaining == 1
        );
    }

    {
        let (state, _receiver) = test_state_with_env(message_env());
        seed_pod!(state, "pod:message-channel-runtime");
        seed_messages!(
            state,
            "pod:message-channel-runtime",
            "00000000-0000-4000-8000-000000000801",
            "00000000-0000-4000-8000-000000000802"
        );
        let messages_path = block_message_path!(state);
        let response = crate::route_http_request(
            "DELETE",
            "/api/v0/podcore/messages/pod:message-channel-runtime/general/cleanup?olderThan=50",
            None,
            "",
            &state,
        )
        .await
        .expect("runtime channel message cleanup");
        record!(
            "DELETE",
            channel_cleanup_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response
                    .body
                    .contains("An error occurred while cleaning up channel messages")
        );
        fs::remove_dir(&messages_path).expect("remove blocked channel message path");
    }

    // Read projections surface storage failures using their frozen
    // controller-specific messages.
    {
        let (state, _receiver) = test_state_with_env(message_env());
        seed_messages!(
            state,
            "pod:message-stats-runtime",
            "00000000-0000-4000-8000-000000000901",
            "00000000-0000-4000-8000-000000000902"
        );
        let messages_path = block_message_path!(state);
        let response = crate::route_http_request("GET", stats_route, None, "", &state)
            .await
            .expect("runtime message stats");
        record!(
            "GET",
            stats_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response
                    .body
                    .contains("An error occurred while getting storage statistics")
        );
        fs::remove_dir(&messages_path).expect("remove blocked stats message path");
    }

    {
        let (state, _receiver) = test_state_with_env(message_env());
        seed_pod!(state, "pod:message-search-runtime");
        seed_messages!(
            state,
            "pod:message-search-runtime",
            "00000000-0000-4000-8000-000000001001",
            "00000000-0000-4000-8000-000000001002"
        );
        let messages_path = block_message_path!(state);
        let response = crate::route_http_request(
            "GET",
            "/api/v0/podcore/messages/pod:message-search-runtime/search?query=message",
            None,
            "",
            &state,
        )
        .await
        .expect("runtime message search");
        record!(
            "GET",
            search_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response
                    .body
                    .contains("An error occurred while searching messages")
        );
        fs::remove_dir(&messages_path).expect("remove blocked search message path");
    }

    {
        let (state, _receiver) = test_state_with_env(message_env());
        seed_pod!(state, "pod:message-count-empty");
        let response = crate::route_http_request(
            "GET",
            "/api/v0/podcore/messages/pod:message-count-empty/general/count",
            None,
            "",
            &state,
        )
        .await
        .expect("empty message count");
        record!(
            "GET",
            count_route,
            "missing-empty-or-conflict-state",
            response.status == "200 OK" && response.body == "0"
        );
    }

    {
        let (state, _receiver) = test_state_with_env(message_env());
        seed_pod!(state, "pod:message-count-runtime");
        seed_messages!(
            state,
            "pod:message-count-runtime",
            "00000000-0000-4000-8000-000000001101",
            "00000000-0000-4000-8000-000000001102"
        );
        let messages_path = block_message_path!(state);
        let response = crate::route_http_request(
            "GET",
            "/api/v0/podcore/messages/pod:message-count-runtime/general/count",
            None,
            "",
            &state,
        )
        .await
        .expect("runtime message count");
        record!(
            "GET",
            count_route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response
                    .body
                    .contains("An error occurred while getting message count")
        );
        fs::remove_dir(&messages_path).expect("remove blocked count message path");
    }

    // Both maintenance actions retain their frozen true result on normal
    // paths and return their separate 500 contracts on storage failure.
    for (action, route, error_message) in [
        (
            "rebuild-index",
            "/api/v0/podcore/messages/rebuild-index",
            "An error occurred while rebuilding search index",
        ),
        (
            "vacuum",
            "/api/v0/podcore/messages/vacuum",
            "An error occurred while vacuuming database",
        ),
    ] {
        let action_path = format!("/api/v0/podcore/messages/{action}");

        {
            let (state, _receiver) = test_state_with_env(message_env());
            let responses =
                futures_util::future::join_all((0..2).map(|_| {
                    let state = Arc::clone(&state);
                    let action_path = action_path.clone();
                    async move {
                        crate::route_http_request("POST", &action_path, None, "", &state).await
                    }
                }))
                .await;
            record!(
                "POST",
                route,
                "concurrency-and-idempotency",
                responses.iter().all(|response| {
                    response.as_ref().is_ok_and(|response| {
                        response.status == "200 OK" && response.body == "true"
                    })
                })
            );
        }

        {
            let (state, _receiver) = test_state_with_env(message_env());
            let response =
                crate::route_http_request("POST", &action_path, None, "not-json", &state)
                    .await
                    .expect("malformed maintenance body");
            record!(
                "POST",
                route,
                "malformed-path-query-or-body",
                response.status == "200 OK" && response.body == "true"
            );
        }

        {
            let (state, _receiver) = test_state_with_env(message_env());
            let response = crate::route_http_request("POST", &action_path, None, "", &state)
                .await
                .expect("empty maintenance action");
            let loaded = crate::pod_channels::PodChannelStore::load(&state.config.state_dir)
                .expect("reload empty maintenance action");
            record!(
                "POST",
                route,
                "missing-empty-or-conflict-state",
                response.status == "200 OK"
                    && response.body == "true"
                    && loaded.stats().total_messages == 0
            );
        }

        {
            let (state, _receiver) = test_state_with_env(message_env());
            seed_messages!(
                state,
                "pod:message-maintenance-mutation",
                "00000000-0000-4000-8000-000000001201",
                "00000000-0000-4000-8000-000000001202"
            );
            let response = crate::route_http_request("POST", &action_path, None, "", &state)
                .await
                .expect("mutating maintenance action");
            let loaded = crate::pod_channels::PodChannelStore::load(&state.config.state_dir)
                .expect("reload mutating maintenance action");
            record!(
                "POST",
                route,
                "mutation-side-effects-and-readback",
                response.status == "200 OK"
                    && response.body == "true"
                    && loaded.stats().total_messages == 2
            );
        }

        {
            let (state, _receiver) = test_state_with_env(message_env());
            seed_messages!(
                state,
                "pod:message-maintenance-restart",
                "00000000-0000-4000-8000-000000001301",
                "00000000-0000-4000-8000-000000001302"
            );
            let response = crate::route_http_request("POST", &action_path, None, "", &state)
                .await
                .expect("restart maintenance action");
            let loaded = crate::pod_channels::PodChannelStore::load(&state.config.state_dir)
                .expect("reload restart maintenance action");
            record!(
                "POST",
                route,
                "restart-persistence-or-reset",
                response.status == "200 OK"
                    && response.body == "true"
                    && loaded.stats().total_messages == 2
            );
        }

        {
            let (state, _receiver) = test_state_with_env(message_env());
            seed_messages!(
                state,
                "pod:message-maintenance-runtime",
                "00000000-0000-4000-8000-000000001401",
                "00000000-0000-4000-8000-000000001402"
            );
            let messages_path = block_message_path!(state);
            let response = crate::route_http_request("POST", &action_path, None, "", &state)
                .await
                .expect("runtime maintenance action");
            record!(
                "POST",
                route,
                "runtime-failure-and-timeout",
                response.status == "500 Internal Server Error"
                    && response.body.contains(error_message)
            );
            fs::remove_dir(&messages_path).expect("remove blocked maintenance path");
        }
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("podcore_message_storage.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api podcore-message-storage mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
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
pub(super) async fn controller_api_differential_podcore_membership_storage() {
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
                    serde_json::from_value::<crate::pods::PodRecord>(serde_json::json!({
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
                    crate::pods::PodMember {
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

    let is_success = |response: &crate::HttpResponse| {
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
        let response = crate::route_http_request(
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
        let response = crate::route_http_request(
            "DELETE",
            &format!("/api/v0/podcore/membership/{pod_id}/delete-target"),
            None,
            "",
            &state,
        )
        .await
        .expect("restart membership delete");
        let loaded =
            crate::pods::PodStore::load(&state.config.state_dir).expect("reload membership delete");
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
            async move { crate::route_http_request("DELETE", &path, None, "", &state).await }
        }))
        .await;
        let loaded = crate::pods::PodStore::load(&state.config.state_dir)
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
        let response = crate::route_http_request(method, &path, None, "", &state)
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
        let response = crate::route_http_request("GET", stats_route, None, "", &state)
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
        let response = crate::route_http_request(
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
        let response = crate::route_http_request(
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
        let loaded = crate::pods::PodStore::load(&state.config.state_dir)
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
            async move { crate::route_http_request("POST", &path, None, body, &state).await }
        }))
        .await;
        let loaded = crate::pods::PodStore::load(&state.config.state_dir)
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
        let response = crate::route_http_request(
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
        let response = crate::route_http_request(
            "POST",
            &format!("/api/v0/podcore/membership/{pod_id}/members"),
            None,
            r#"{"peerId":"membership-owner","role":"owner","isBanned":false}"#,
            &state,
        )
        .await
        .expect("restart membership publish");
        let loaded = crate::pods::PodStore::load(&state.config.state_dir)
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
                crate::route_http_request(
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
        let loaded = crate::pods::PodStore::load(&state.config.state_dir)
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
        let response = crate::route_http_request(
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
        let response = crate::route_http_request(
            "PUT",
            &format!("/api/v0/podcore/membership/{pod_id}/members/update-target"),
            None,
            r#"{"role":"mod","isBanned":false}"#,
            &state,
        )
        .await
        .expect("restart membership update");
        let loaded =
            crate::pods::PodStore::load(&state.config.state_dir).expect("reload membership update");
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
                crate::route_http_request(
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
        let loaded = crate::pods::PodStore::load(&state.config.state_dir)
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
        let response = crate::route_http_request("POST", cleanup_route, None, "", &state)
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
        let response = crate::route_http_request("POST", cleanup_route, None, "", &state)
            .await
            .expect("restart membership cleanup");
        let loaded = crate::pods::PodStore::load(&state.config.state_dir)
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
pub(super) async fn controller_api_differential_podcore_discovery_storage() {
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
                    serde_json::from_value::<crate::pods::PodRecord>(serde_json::json!({
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
    let is_success = |response: &crate::HttpResponse| {
        response.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&response.body)
                .ok()
                .is_some_and(|value| value["success"] == true)
    };

    // Invalid register/update payloads must remain 400 even though the
    // success path now validates the durable feature-state boundary.
    {
        let (state, _receiver) = test_state_with_env(discovery_env());
        let response = crate::route_http_request("POST", register_route, None, "{}", &state)
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
        let response = crate::route_http_request(
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
        let response = crate::route_http_request(
            "POST",
            register_route,
            None,
            &discovery_body(pod_id, "Restart"),
            &state,
        )
        .await
        .expect("restart discovery registration");
        let loaded = crate::ControllerFeatureState::load(&state.config.state_dir)
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
                    crate::route_http_request("POST", register_route, None, &body, &state).await
                }
            }))
            .await;
        let loaded = crate::ControllerFeatureState::load(&state.config.state_dir)
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
        let response = crate::route_http_request("POST", update_route, None, "{}", &state)
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
        let response = crate::route_http_request(
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
        let registered = crate::route_http_request(
            "POST",
            register_route,
            None,
            &discovery_body(pod_id, "Before update"),
            &state,
        )
        .await
        .expect("seed restart discovery update");
        assert!(is_success(&registered));
        let response = crate::route_http_request(
            "POST",
            update_route,
            None,
            &discovery_body(pod_id, "After update"),
            &state,
        )
        .await
        .expect("restart discovery update");
        let loaded = crate::ControllerFeatureState::load(&state.config.state_dir)
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
        let registered = crate::route_http_request(
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
                        crate::route_http_request("POST", update_route, None, &body, &state).await
                    }
                }),
            )
            .await;
        let loaded = crate::ControllerFeatureState::load(&state.config.state_dir)
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
        let registered = crate::route_http_request(
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
        let response = crate::route_http_request(
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
        let registered = crate::route_http_request(
            "POST",
            register_route,
            None,
            &discovery_body(pod_id, "Restart unregister"),
            &state,
        )
        .await
        .expect("seed restart discovery unregister");
        assert!(is_success(&registered));
        let response = crate::route_http_request(
            "DELETE",
            &format!("/api/v0/podcore/discovery/unregister/{pod_id}"),
            None,
            "",
            &state,
        )
        .await
        .expect("restart discovery unregister");
        let loaded = crate::ControllerFeatureState::load(&state.config.state_dir)
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
        let registered = crate::route_http_request(
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
            async move { crate::route_http_request("DELETE", &path, None, "", &state).await }
        }))
        .await;
        let loaded = crate::ControllerFeatureState::load(&state.config.state_dir)
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
        let response = crate::route_http_request("GET", route, None, "", &state)
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
        let response = crate::route_http_request(
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
        let response = crate::route_http_request("POST", refresh_route, None, "", &state)
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
        let response = crate::route_http_request("POST", refresh_route, None, "", &state)
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
        let registered = crate::route_http_request(
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
        let response = crate::route_http_request("POST", refresh_route, None, "", &state)
            .await
            .expect("restart discovery refresh");
        let loaded = crate::ControllerFeatureState::load(&state.config.state_dir)
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
        let registered = crate::route_http_request(
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
                        crate::route_http_request("POST", refresh_route, None, "", &state).await
                    }
                }),
            )
            .await;
        let loaded = crate::ControllerFeatureState::load(&state.config.state_dir)
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
