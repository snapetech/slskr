//! Controller full pods differential fixtures ownership.

use super::*;

#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-4"
))]
pub(super) async fn controller_api_differential_podcore_residuals_impl() {
    let target = "slskdn";
    let pod_env = || MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target);
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

    let json_body = |response: &crate::HttpResponse| {
        serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or(serde_json::Value::Null)
    };
    let block_file = |path: PathBuf| {
        if path.exists() {
            fs::remove_file(&path).expect("remove PodCore state file before blocking it");
        }
        fs::create_dir(&path).expect("block PodCore state file with a directory");
    };
    let block_feature_file = |state: &Arc<crate::AppState>| {
        let state = Arc::clone(state);
        async move {
            let path = state.config.state_dir.join("controller-feature-state.json");
            state.controller_features.write_for_test().await.state_path = path.clone();
            block_file(path);
        }
    };
    let prepare_feature_file = |state: &Arc<crate::AppState>| {
        let state = Arc::clone(state);
        async move {
            let path = state.config.state_dir.join("controller-feature-state.json");
            if path.is_dir() {
                fs::remove_dir_all(&path).expect("remove prepared PodCore feature directory");
            } else if path.exists() {
                fs::remove_file(&path).expect("remove prepared PodCore feature file");
            }
            state.controller_features.write_for_test().await.state_path = path;
        }
    };
    let seed_pod = |state: &Arc<crate::AppState>, pod_id: &str| {
        let state = Arc::clone(state);
        let pod_id = pod_id.to_owned();
        async move {
            state
                .pods
                .write()
                .await
                .create(
                    serde_json::from_value::<crate::pods::PodRecord>(serde_json::json!({
                        "podId": pod_id,
                        "name": format!("PodCore residual {pod_id}"),
                        "visibility": "Listed",
                        "isPublic": true,
                        "channels": [{
                            "channelId": "general",
                            "kind": 0,
                            "name": "General"
                        }]
                    }))
                    .expect("deserialize PodCore residual pod"),
                    "tester".to_owned(),
                )
                .expect("persist PodCore residual pod");
        }
    };
    let pod_body = |pod_id: &str| {
        serde_json::json!({
            "pod": {
                "podId": pod_id,
                "name": format!("Published {pod_id}"),
                "visibility": "Listed",
                "isPublic": true,
                "channels": [{"channelId":"general","kind":0,"name":"General"}]
            }
        })
        .to_string()
    };
    let content_id = "content:video:movie:podcore-residual";
    let valid_signature = crate::STANDARD.encode([0_u8; 64]);
    let opinion_body = |pod_id: &str, id: &str| {
        serde_json::json!({
            "id": id,
            "contentId": content_id,
            "variantHash": "variant-residual",
            "score": 4.0,
            "note": "PodCore residual opinion",
            "senderPeerId": "tester",
            "signature": format!("ed25519:{valid_signature}"),
            "podId": pod_id,
        })
        .to_string()
    };
    let message_body = |message_id: &str, pod_id: &str| {
        serde_json::json!({
            "messageId": message_id,
            "podId": pod_id,
            "channelId": "general",
            "senderPeerId": "tester",
            "body": "PodCore residual message",
            "timestampUnixMs": crate::unix_timestamp_millis(),
            "signature": "",
            "sigVersion": 1,
        })
        .to_string()
    };
    let signing_private_key = crate::STANDARD.encode([7_u8; 32]);

    // GET /podcore/{podId}/channels and /channels/{channelId} -- the
    // frozen services read the durable pod store and normalize failures
    // to their controller-specific 500 responses.
    for (path, route, message) in [
        (
            "/api/v0/podcore/podcore-channels-runtime/channels",
            "/api/v0/podcore/{podId}/channels",
            "An error occurred while getting channels",
        ),
        (
            "/api/v0/podcore/podcore-channels-runtime/channels/general",
            "/api/v0/podcore/{podId}/channels/{channelId}",
            "An error occurred while getting the channel",
        ),
    ] {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-channels-runtime").await;
        block_file(state.config.state_dir.join("pods.json"));
        let response = request!(&state, "GET", path, "");
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error" && response.body.contains(message)
        );
    }

    // Opinion projections use the controller-feature persistence boundary.
    for (path, route, message) in [
        (
            "/api/v0/podcore/podcore-opinions-runtime/opinions/content/content:video:movie:residual",
            "/api/v0/podcore/{podId}/opinions/content/{contentId}",
            "An error occurred while getting opinions",
        ),
        (
            "/api/v0/podcore/podcore-opinions-runtime/opinions/content/content:video:movie:residual/aggregated",
            "/api/v0/podcore/{podId}/opinions/content/{contentId}/aggregated",
            "An error occurred while getting opinions",
        ),
        (
            "/api/v0/podcore/podcore-opinions-runtime/opinions/content/content:video:movie:residual/recommendations",
            "/api/v0/podcore/{podId}/opinions/content/{contentId}/recommendations",
            "An error occurred while getting opinions",
        ),
        (
            "/api/v0/podcore/podcore-opinions-runtime/opinions/content/content:video:movie:residual/stats",
            "/api/v0/podcore/{podId}/opinions/content/{contentId}/stats",
            "An error occurred while getting opinions",
        ),
        (
            "/api/v0/podcore/podcore-opinions-runtime/opinions/content/content:video:movie:residual/variant/variant-residual",
            "/api/v0/podcore/{podId}/opinions/content/{contentId}/variant/{variantHash}",
            "An error occurred while getting opinions",
        ),
    ] {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-opinions-runtime").await;
        block_feature_file(&state).await;
        let response = request!(&state, "GET", path, "");
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error" && response.body.contains(message)
        );
    }

    {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-affinity-runtime").await;
        block_feature_file(&state).await;
        let response = request!(
            &state,
            "GET",
            "/api/v0/podcore/podcore-affinity-runtime/opinions/members/affinity",
            ""
        );
        record!(
            "GET",
            "/api/v0/podcore/{podId}/opinions/members/affinity",
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response
                    .body
                    .contains("An error occurred while getting member affinities")
        );
    }

    {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-last-seen-runtime").await;
        let updated = request!(
            &state,
            "PUT",
            "/api/v0/podcore/backfill/podcore-last-seen-runtime/general/last-seen",
            r#"{"lastSeen":10}"#
        );
        block_feature_file(&state).await;
        let response = request!(
            &state,
            "GET",
            "/api/v0/podcore/backfill/podcore-last-seen-runtime/last-seen",
            ""
        );
        record!(
            "GET",
            "/api/v0/podcore/backfill/{podId}/last-seen",
            "runtime-failure-and-timeout",
            updated.status == "200 OK"
                && response.status == "500 Internal Server Error"
                && response
                    .body
                    .contains("An error occurred while getting last seen timestamps")
        );
    }

    // These statistics and content-link operations are intentionally
    // process-local in the frozen services; their runtime contract is a
    // successful JSON projection even when the optional remote provider
    // is empty or unavailable.
    for (path, route) in [
        (
            "/api/v0/podcore/backfill/stats",
            "/api/v0/podcore/backfill/stats",
        ),
        (
            "/api/v0/podcore/content/metadata?contentId=content:video:movie:podcore-residual",
            "/api/v0/podcore/content/metadata",
        ),
        (
            "/api/v0/podcore/content/search?query=podcore-residual&domain=video",
            "/api/v0/podcore/content/search",
        ),
        ("/api/v0/podcore/dht/stats", "/api/v0/podcore/dht/stats"),
        (
            "/api/v0/podcore/signing/stats",
            "/api/v0/podcore/signing/stats",
        ),
        (
            "/api/v0/podcore/verification/stats",
            "/api/v0/podcore/verification/stats",
        ),
    ] {
        let (state, _receiver) = test_state_with_env(pod_env());
        let response = request!(&state, "GET", path, "");
        let body_shape_ok = if path.contains("/content/search") {
            json_body(&response).is_array() || json_body(&response).is_object()
        } else {
            json_body(&response).is_object()
        };
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            response.status == "200 OK" && body_shape_ok
        );
    }

    {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-dht-metadata-runtime").await;
        let published = request!(
            &state,
            "POST",
            "/api/v0/podcore/dht/publish",
            &pod_body("podcore-dht-metadata-runtime")
        );
        block_feature_file(&state).await;
        let response = request!(
            &state,
            "GET",
            "/api/v0/podcore/dht/metadata/podcore-dht-metadata-runtime",
            ""
        );
        record!(
            "GET",
            "/api/v0/podcore/dht/metadata/{*podId}",
            "runtime-failure-and-timeout",
            published.status == "200 OK"
                && response.status == "500 Internal Server Error"
                && response.body.contains("Failed to retrieve pod metadata")
        );
    }

    {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-routing-seen-runtime").await;
        let registered = request!(
            &state,
            "POST",
            "/api/v0/podcore/routing/seen/message-runtime/podcore-routing-seen-runtime",
            ""
        );
        block_feature_file(&state).await;
        let response = request!(
            &state,
            "GET",
            "/api/v0/podcore/routing/seen/message-runtime/podcore-routing-seen-runtime",
            ""
        );
        record!(
            "GET",
            "/api/v0/podcore/routing/seen/{messageId}/{podId}",
            "runtime-failure-and-timeout",
            registered.status == "200 OK"
                && response.status == "500 Internal Server Error"
                && response
                    .body
                    .contains("Failed to check message seen status")
        );
    }

    {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-verification-runtime").await;
        block_file(state.config.state_dir.join("pods.json"));
        let membership = request!(
            &state,
            "GET",
            "/api/v0/podcore/verification/membership/podcore-verification-runtime/tester",
            ""
        );
        let role = request!(
            &state,
            "GET",
            "/api/v0/podcore/verification/role/podcore-verification-runtime/tester/member",
            ""
        );
        record!(
            "GET",
            "/api/v0/podcore/verification/membership/{podId}/{peerId}",
            "runtime-failure-and-timeout",
            membership.status == "500 Internal Server Error"
                && membership.body.contains("Failed to verify membership")
        );
        record!(
            "GET",
            "/api/v0/podcore/verification/role/{podId}/{peerId}/{requiredRole}",
            "runtime-failure-and-timeout",
            role.status == "500 Internal Server Error"
                && role.body.contains("Failed to check role")
        );
    }

    {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-routing-stats-runtime").await;
        let registered = request!(
            &state,
            "POST",
            "/api/v0/podcore/routing/seen/message-stats/podcore-routing-stats-runtime",
            ""
        );
        block_feature_file(&state).await;
        let response = request!(&state, "GET", "/api/v0/podcore/routing/stats", "");
        record!(
            "GET",
            "/api/v0/podcore/routing/stats",
            "runtime-failure-and-timeout",
            registered.status == "200 OK"
                && response.status == "500 Internal Server Error"
                && response.body.contains("Failed to get routing statistics")
        );
    }

    // DHT unpublish: the frozen controller returns a fixed 500 when the
    // publisher/storage boundary fails, persists a successful removal,
    // and treats repeated removal as idempotent.
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-unpublish-runtime").await;
        let published = request!(
            &state,
            "POST",
            "/api/v0/podcore/dht/publish",
            &pod_body("podcore-unpublish-runtime")
        );
        block_feature_file(&state).await;
        let response = request!(
            &state,
            "DELETE",
            "/api/v0/podcore/dht/unpublish/podcore-unpublish-runtime",
            ""
        );
        record!(
            "DELETE",
            "/api/v0/podcore/dht/unpublish/{*podId}",
            "runtime-failure-and-timeout",
            published.status == "200 OK"
                && response.status == "500 Internal Server Error"
                && response.body.contains("Failed to unpublish pod")
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-unpublish-restart").await;
        let published = request!(
            &state,
            "POST",
            "/api/v0/podcore/dht/publish",
            &pod_body("podcore-unpublish-restart")
        );
        let response = request!(
            &state,
            "DELETE",
            "/api/v0/podcore/dht/unpublish/podcore-unpublish-restart",
            ""
        );
        let loaded = crate::ControllerFeatureState::load(&state.config.state_dir)
            .expect("reload DHT unpublish state");
        record!(
            "DELETE",
            "/api/v0/podcore/dht/unpublish/{*podId}",
            "restart-persistence-or-reset",
            published.status == "200 OK"
                && response.status == "200 OK"
                && loaded.get("pod/dht/podcore-unpublish-restart").is_none()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-unpublish-concurrent").await;
        let published = request!(
            &state,
            "POST",
            "/api/v0/podcore/dht/publish",
            &pod_body("podcore-unpublish-concurrent")
        );
        let (left, right) = tokio::join!(
            crate::route_http_request(
                "DELETE",
                "/api/v0/podcore/dht/unpublish/podcore-unpublish-concurrent",
                None,
                "",
                &state
            ),
            crate::route_http_request(
                "DELETE",
                "/api/v0/podcore/dht/unpublish/podcore-unpublish-concurrent",
                None,
                "",
                &state
            )
        );
        let loaded = crate::ControllerFeatureState::load(&state.config.state_dir)
            .expect("reload concurrent DHT unpublish state");
        record!(
            "DELETE",
            "/api/v0/podcore/dht/unpublish/{*podId}",
            "concurrency-and-idempotency",
            published.status == "200 OK"
                && left
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
                && loaded.get("pod/dht/podcore-unpublish-concurrent").is_none()
        );
    }

    // DHT publish and update share the same durable publication record.
    for (route, pod_id, action) in [
        (
            "/api/v0/podcore/dht/publish",
            "podcore-publish-runtime",
            "publish",
        ),
        (
            "/api/v0/podcore/dht/update",
            "podcore-update-runtime",
            "update",
        ),
    ] {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, pod_id).await;
        let initial = if action == "update" {
            request!(
                &state,
                "POST",
                "/api/v0/podcore/dht/publish",
                &pod_body(pod_id)
            )
        } else {
            crate::HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: String::new(),
            }
        };
        block_feature_file(&state).await;
        let response = request!(&state, "POST", route, &pod_body(pod_id));
        record!(
            "POST",
            route,
            "runtime-failure-and-timeout",
            initial.status == "200 OK"
                && response.status == "500 Internal Server Error"
                && response.body.contains(if action == "publish" {
                    "Failed to publish pod"
                } else {
                    "Failed to update pod"
                })
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        prepare_feature_file(&state).await;
        seed_pod(&state, "podcore-publish-restart").await;
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/dht/publish",
            &pod_body("podcore-publish-restart")
        );
        let loaded = crate::ControllerFeatureState::load(&state.config.state_dir)
            .expect("reload DHT publication state");
        record!(
            "POST",
            "/api/v0/podcore/dht/publish",
            "restart-persistence-or-reset",
            response.status == "200 OK" && loaded.get("pod/dht/podcore-publish-restart").is_some()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        prepare_feature_file(&state).await;
        seed_pod(&state, "podcore-publish-concurrent").await;
        let body = pod_body("podcore-publish-concurrent");
        let (left, right) = tokio::join!(
            crate::route_http_request("POST", "/api/v0/podcore/dht/publish", None, &body, &state),
            crate::route_http_request("POST", "/api/v0/podcore/dht/publish", None, &body, &state)
        );
        let loaded = crate::ControllerFeatureState::load(&state.config.state_dir)
            .expect("reload concurrent DHT publication state");
        record!(
            "POST",
            "/api/v0/podcore/dht/publish",
            "concurrency-and-idempotency",
            left.as_ref()
                .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
                && loaded.get("pod/dht/podcore-publish-concurrent").is_some()
        );
    }

    {
        let (state, _receiver) = test_state_with_env(pod_env());
        prepare_feature_file(&state).await;
        seed_pod(&state, "podcore-update-restart").await;
        let first = request!(
            &state,
            "POST",
            "/api/v0/podcore/dht/publish",
            &pod_body("podcore-update-restart")
        );
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/dht/update",
            &pod_body("podcore-update-restart")
        );
        let loaded = crate::ControllerFeatureState::load(&state.config.state_dir)
            .expect("reload DHT update state");
        record!(
            "POST",
            "/api/v0/podcore/dht/update",
            "restart-persistence-or-reset",
            first.status == "200 OK"
                && response.status == "200 OK"
                && loaded.get("pod/dht/podcore-update-restart").is_some()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        prepare_feature_file(&state).await;
        seed_pod(&state, "podcore-update-concurrent").await;
        let initial = request!(
            &state,
            "POST",
            "/api/v0/podcore/dht/publish",
            &pod_body("podcore-update-concurrent")
        );
        let body = pod_body("podcore-update-concurrent");
        let (left, right) = tokio::join!(
            crate::route_http_request("POST", "/api/v0/podcore/dht/update", None, &body, &state),
            crate::route_http_request("POST", "/api/v0/podcore/dht/update", None, &body, &state)
        );
        record!(
            "POST",
            "/api/v0/podcore/dht/update",
            "concurrency-and-idempotency",
            initial.status == "200 OK"
                && left
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
        );
    }

    let expire_dht_publication = |state: &Arc<crate::AppState>, pod_id: &str| {
        let state = Arc::clone(state);
        let pod_id = pod_id.to_owned();
        async move {
            let key = format!("pod/dht/{pod_id}");
            let mut features = state.controller_features.write_for_test().await;
            let mut publication = features
                .get(&key)
                .cloned()
                .expect("DHT publication exists before expiry fixture");
            publication["expiresAt"] = serde_json::json!("1970-01-01T00:00:00+00:00");
            features
                .upsert(key, publication)
                .expect("expire DHT publication fixture");
        }
    };

    // Refresh has distinct missing, publisher-failure, republish, reset,
    // and concurrent contracts.
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/dht/refresh/podcore-refresh-missing",
            ""
        );
        record!(
            "POST",
            "/api/v0/podcore/dht/refresh/{*podId}",
            "missing-empty-or-conflict-state",
            response.status == "500 Internal Server Error"
                && response.body.contains("Failed to refresh pod")
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-refresh-runtime").await;
        let published = request!(
            &state,
            "POST",
            "/api/v0/podcore/dht/publish",
            &pod_body("podcore-refresh-runtime")
        );
        expire_dht_publication(&state, "podcore-refresh-runtime").await;
        block_feature_file(&state).await;
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/dht/refresh/podcore-refresh-runtime",
            ""
        );
        record!(
            "POST",
            "/api/v0/podcore/dht/refresh/{*podId}",
            "runtime-failure-and-timeout",
            published.status == "200 OK"
                && response.status == "500 Internal Server Error"
                && response.body.contains("Failed to refresh pod")
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-refresh-mutation").await;
        let published = request!(
            &state,
            "POST",
            "/api/v0/podcore/dht/publish",
            &pod_body("podcore-refresh-mutation")
        );
        expire_dht_publication(&state, "podcore-refresh-mutation").await;
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/dht/refresh/podcore-refresh-mutation",
            ""
        );
        let metadata = request!(
            &state,
            "GET",
            "/api/v0/podcore/dht/metadata/podcore-refresh-mutation",
            ""
        );
        record!(
            "POST",
            "/api/v0/podcore/dht/refresh/{*podId}",
            "mutation-side-effects-and-readback",
            published.status == "200 OK"
                && response.status == "200 OK"
                && json_body(&response)["wasRepublished"] == true
                && metadata.status == "200 OK"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-refresh-reset").await;
        let published = request!(
            &state,
            "POST",
            "/api/v0/podcore/dht/publish",
            &pod_body("podcore-refresh-reset")
        );
        let (fresh, _fresh_receiver) = test_state_with_env(pod_env());
        let response = request!(
            &fresh,
            "POST",
            "/api/v0/podcore/dht/refresh/podcore-refresh-reset",
            ""
        );
        record!(
            "POST",
            "/api/v0/podcore/dht/refresh/{*podId}",
            "restart-persistence-or-reset",
            published.status == "200 OK" && response.status == "500 Internal Server Error"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-refresh-concurrent").await;
        let published = request!(
            &state,
            "POST",
            "/api/v0/podcore/dht/publish",
            &pod_body("podcore-refresh-concurrent")
        );
        expire_dht_publication(&state, "podcore-refresh-concurrent").await;
        let (left, right) = tokio::join!(
            crate::route_http_request(
                "POST",
                "/api/v0/podcore/dht/refresh/podcore-refresh-concurrent",
                None,
                "",
                &state
            ),
            crate::route_http_request(
                "POST",
                "/api/v0/podcore/dht/refresh/podcore-refresh-concurrent",
                None,
                "",
                &state
            )
        );
        record!(
            "POST",
            "/api/v0/podcore/dht/refresh/{*podId}",
            "concurrency-and-idempotency",
            published.status == "200 OK"
                && left
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
        );
    }

    // Opinion publication, affinity refresh, and DHT-backed opinion
    // refresh share the frozen controller's explicit persistence/error
    // boundary and remain idempotent across repeated calls.
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        prepare_feature_file(&state).await;
        block_feature_file(&state).await;
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/podcore-opinion-runtime/opinions",
            &opinion_body("podcore-opinion-runtime", "opinion-runtime")
        );
        record!(
            "POST",
            "/api/v0/podcore/{podId}/opinions",
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response
                    .body
                    .contains("An error occurred while publishing the opinion")
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        prepare_feature_file(&state).await;
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/podcore-opinion-restart/opinions",
            &opinion_body("podcore-opinion-restart", "opinion-restart")
        );
        let loaded = crate::ControllerFeatureState::load(&state.config.state_dir)
            .expect("reload published opinion state");
        record!(
            "POST",
            "/api/v0/podcore/{podId}/opinions",
            "restart-persistence-or-reset",
            response.status == "200 OK"
                && loaded
                    .get("pod/opinion/podcore-opinion-restart/content:video:movie:podcore-residual/opinion-restart")
                    .is_some()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        prepare_feature_file(&state).await;
        let body = opinion_body("podcore-opinion-concurrent", "opinion-concurrent");
        let (left, right) = tokio::join!(
            crate::route_http_request(
                "POST",
                "/api/v0/podcore/podcore-opinion-concurrent/opinions",
                None,
                &body,
                &state
            ),
            crate::route_http_request(
                "POST",
                "/api/v0/podcore/podcore-opinion-concurrent/opinions",
                None,
                &body,
                &state
            )
        );
        let loaded = crate::ControllerFeatureState::load(&state.config.state_dir)
            .expect("reload concurrent opinion state");
        record!(
            "POST",
            "/api/v0/podcore/{podId}/opinions",
            "concurrency-and-idempotency",
            left.as_ref().is_ok_and(|response| response.status == "200 OK")
                && right.as_ref().is_ok_and(|response| response.status == "200 OK")
                && loaded
                    .get("pod/opinion/podcore-opinion-concurrent/content:video:movie:podcore-residual/opinion-concurrent")
                    .is_some()
        );
    }

    for (pod_id, case, expected_status) in [
        (
            "podcore-affinity-runtime",
            "runtime-failure-and-timeout",
            "500 Internal Server Error",
        ),
        (
            "podcore-affinity-restart",
            "restart-persistence-or-reset",
            "200 OK",
        ),
        (
            "podcore-affinity-concurrent",
            "concurrency-and-idempotency",
            "200 OK",
        ),
    ] {
        let (state, _receiver) = test_state_with_env(pod_env());
        if case == "runtime-failure-and-timeout" {
            block_feature_file(&state).await;
        }
        let response = request!(
            &state,
            "POST",
            &format!("/api/v0/podcore/{pod_id}/opinions/members/affinity/update"),
            ""
        );
        let pass = response.status == expected_status
            && (case != "runtime-failure-and-timeout"
                || response
                    .body
                    .contains("An error occurred while updating member affinities"));
        record!(
            "POST",
            "/api/v0/podcore/{podId}/opinions/members/affinity/update",
            case,
            pass
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        block_feature_file(&state).await;
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/podcore-opinion-refresh-runtime/opinions/refresh",
            ""
        );
        record!(
            "POST",
            "/api/v0/podcore/{podId}/opinions/refresh",
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response
                    .body
                    .contains("An error occurred while refreshing opinions")
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        prepare_feature_file(&state).await;
        let published = request!(
            &state,
            "POST",
            "/api/v0/podcore/podcore-opinion-refresh-restart/opinions",
            &opinion_body("podcore-opinion-refresh-restart", "opinion-refresh-restart")
        );
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/podcore-opinion-refresh-restart/opinions/refresh",
            ""
        );
        record!(
            "POST",
            "/api/v0/podcore/{podId}/opinions/refresh",
            "restart-persistence-or-reset",
            published.status == "200 OK" && response.status == "200 OK"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let (left, right) = tokio::join!(
            crate::route_http_request(
                "POST",
                "/api/v0/podcore/podcore-opinion-refresh-concurrent/opinions/refresh",
                None,
                "",
                &state
            ),
            crate::route_http_request(
                "POST",
                "/api/v0/podcore/podcore-opinion-refresh-concurrent/opinions/refresh",
                None,
                "",
                &state
            )
        );
        record!(
            "POST",
            "/api/v0/podcore/{podId}/opinions/refresh",
            "concurrency-and-idempotency",
            left.as_ref()
                .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
        );
    }

    // Backfill sync validates both durable stores before invoking the
    // local projection.  sync-all is body-independent, returns an empty
    // list when there is no work, and includes persisted work when it is
    // present.
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/backfill/podcore-backfill-missing/sync",
            "{}"
        );
        record!(
            "POST",
            "/api/v0/podcore/backfill/{podId}/sync",
            "missing-empty-or-conflict-state",
            response.status == "400 Bad Request"
                && response.body.contains("Last seen timestamps are required")
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-backfill-runtime").await;
        block_file(state.config.state_dir.join("pods.json"));
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/backfill/podcore-backfill-runtime/sync",
            r#"{"general":10}"#
        );
        record!(
            "POST",
            "/api/v0/podcore/backfill/{podId}/sync",
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response
                    .body
                    .contains("An error occurred while syncing backfill")
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-backfill-restart").await;
        prepare_feature_file(&state).await;
        let updated = request!(
            &state,
            "PUT",
            "/api/v0/podcore/backfill/podcore-backfill-restart/general/last-seen",
            "10"
        );
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/backfill/podcore-backfill-restart/sync",
            r#"{"general":10}"#
        );
        let loaded = crate::ControllerFeatureState::load(&state.config.state_dir)
            .expect("reload backfill state");
        record!(
            "POST",
            "/api/v0/podcore/backfill/{podId}/sync",
            "restart-persistence-or-reset",
            updated.status == "200 OK"
                && response.status == "200 OK"
                && loaded
                    .get("pod/backfill/podcore-backfill-restart/general")
                    .is_some()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-backfill-concurrent").await;
        let (left, right) = tokio::join!(
            crate::route_http_request(
                "POST",
                "/api/v0/podcore/backfill/podcore-backfill-concurrent/sync",
                None,
                r#"{"general":10}"#,
                &state
            ),
            crate::route_http_request(
                "POST",
                "/api/v0/podcore/backfill/podcore-backfill-concurrent/sync",
                None,
                r#"{"general":10}"#,
                &state
            )
        );
        record!(
            "POST",
            "/api/v0/podcore/backfill/{podId}/sync",
            "concurrency-and-idempotency",
            left.as_ref()
                .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
        );
    }

    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/backfill/sync-all",
            "not-json"
        );
        record!(
            "POST",
            "/api/v0/podcore/backfill/sync-all",
            "malformed-path-query-or-body",
            response.status == "200 OK" && json_body(&response).is_array()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let response = request!(&state, "POST", "/api/v0/podcore/backfill/sync-all", "");
        record!(
            "POST",
            "/api/v0/podcore/backfill/sync-all",
            "missing-empty-or-conflict-state",
            response.status == "200 OK" && json_body(&response).is_array()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-sync-all-runtime").await;
        block_file(state.config.state_dir.join("pods.json"));
        let response = request!(&state, "POST", "/api/v0/podcore/backfill/sync-all", "");
        record!(
            "POST",
            "/api/v0/podcore/backfill/sync-all",
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response
                    .body
                    .contains("An error occurred while syncing all pods")
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-sync-all-mutation").await;
        prepare_feature_file(&state).await;
        let updated = request!(
            &state,
            "PUT",
            "/api/v0/podcore/backfill/podcore-sync-all-mutation/general/last-seen",
            "10"
        );
        let response = request!(&state, "POST", "/api/v0/podcore/backfill/sync-all", "");
        record!(
            "POST",
            "/api/v0/podcore/backfill/sync-all",
            "mutation-side-effects-and-readback",
            updated.status == "200 OK"
                && response.status == "200 OK"
                && json_body(&response)
                    .as_array()
                    .is_some_and(|rows| !rows.is_empty())
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-sync-all-restart").await;
        prepare_feature_file(&state).await;
        let updated = request!(
            &state,
            "PUT",
            "/api/v0/podcore/backfill/podcore-sync-all-restart/general/last-seen",
            "10"
        );
        let loaded = crate::ControllerFeatureState::load(&state.config.state_dir)
            .expect("reload sync-all state");
        let response = request!(&state, "POST", "/api/v0/podcore/backfill/sync-all", "");
        record!(
            "POST",
            "/api/v0/podcore/backfill/sync-all",
            "restart-persistence-or-reset",
            updated.status == "200 OK"
                && loaded
                    .get("pod/backfill/podcore-sync-all-restart/general")
                    .is_some()
                && response.status == "200 OK"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let (left, right) = tokio::join!(
            crate::route_http_request(
                "POST",
                "/api/v0/podcore/backfill/sync-all",
                None,
                "",
                &state
            ),
            crate::route_http_request(
                "POST",
                "/api/v0/podcore/backfill/sync-all",
                None,
                "",
                &state
            )
        );
        record!(
            "POST",
            "/api/v0/podcore/backfill/sync-all",
            "concurrency-and-idempotency",
            left.as_ref()
                .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
        );
    }

    // Content validation is a pure projection; content-linked pod
    // creation is backed by the durable pod store.
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let response = request!(&state, "POST", "/api/v0/podcore/content/create-pod", "");
        record!(
            "POST",
            "/api/v0/podcore/content/create-pod",
            "missing-empty-or-conflict-state",
            response.status == "400 Bad Request"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        block_file(state.config.state_dir.join("pods.json"));
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/content/create-pod",
            &pod_body("podcore-content-create-runtime")
        );
        record!(
            "POST",
            "/api/v0/podcore/content/create-pod",
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response
                    .body
                    .contains("An error occurred while creating the pod")
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/content/create-pod",
            &pod_body("podcore-content-create-restart")
        );
        let loaded = crate::pods::PodStore::load(&state.config.state_dir)
            .expect("reload created content-linked pod");
        record!(
            "POST",
            "/api/v0/podcore/content/create-pod",
            "restart-persistence-or-reset",
            response.status == "201 Created"
                && loaded.get("podcore-content-create-restart").is_some()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let body = pod_body("podcore-content-create-concurrent");
        let (left, right) = tokio::join!(
            crate::route_http_request(
                "POST",
                "/api/v0/podcore/content/create-pod",
                None,
                &body,
                &state
            ),
            crate::route_http_request(
                "POST",
                "/api/v0/podcore/content/create-pod",
                None,
                &body,
                &state
            )
        );
        let loaded = crate::pods::PodStore::load(&state.config.state_dir)
            .expect("reload concurrent content-linked pod");
        let success_count = [left.as_ref(), right.as_ref()]
            .into_iter()
            .filter(|response| response.is_ok_and(|response| response.status == "201 Created"))
            .count();
        record!(
            "POST",
            "/api/v0/podcore/content/create-pod",
            "concurrency-and-idempotency",
            success_count == 1
                && [left.as_ref(), right.as_ref()]
                    .into_iter()
                    .all(|response| response.is_ok_and(|response| {
                        response.status == "201 Created" || response.status == "409 Conflict"
                    }))
                && loaded.get("podcore-content-create-concurrent").is_some()
        );
    }

    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let response = request!(&state, "POST", "/api/v0/podcore/content/validate", "");
        record!(
            "POST",
            "/api/v0/podcore/content/validate",
            "missing-empty-or-conflict-state",
            response.status == "400 Bad Request"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/content/validate",
            &serde_json::to_string(&content_id).expect("serialize content ID")
        );
        record!(
            "POST",
            "/api/v0/podcore/content/validate",
            "runtime-failure-and-timeout",
            response.status == "200 OK" && json_body(&response)["isValid"] == true
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let body = serde_json::to_string(&content_id).expect("serialize content ID");
        let first = request!(&state, "POST", "/api/v0/podcore/content/validate", &body);
        let second = request!(&state, "POST", "/api/v0/podcore/content/validate", &body);
        record!(
            "POST",
            "/api/v0/podcore/content/validate",
            "mutation-side-effects-and-readback",
            first.status == "200 OK"
                && second.status == "200 OK"
                && json_body(&second)["contentId"] == content_id
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let body = serde_json::to_string(&content_id).expect("serialize content ID");
        let first = request!(&state, "POST", "/api/v0/podcore/content/validate", &body);
        let loaded_body = body.clone();
        let second = request!(
            &state,
            "POST",
            "/api/v0/podcore/content/validate",
            &loaded_body
        );
        record!(
            "POST",
            "/api/v0/podcore/content/validate",
            "restart-persistence-or-reset",
            first.status == "200 OK" && second.status == "200 OK"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let body = serde_json::to_string(&content_id).expect("serialize content ID");
        let (left, right) = tokio::join!(
            crate::route_http_request(
                "POST",
                "/api/v0/podcore/content/validate",
                None,
                &body,
                &state
            ),
            crate::route_http_request(
                "POST",
                "/api/v0/podcore/content/validate",
                None,
                &body,
                &state
            )
        );
        record!(
            "POST",
            "/api/v0/podcore/content/validate",
            "concurrency-and-idempotency",
            left.as_ref()
                .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
        );
    }

    // Routing persistence has one shared seen-message registry.  The
    // route, explicit registration, and cleanup actions all preserve the
    // frozen error text and durable/idempotent behavior.
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/routing/cleanup",
            "not-json"
        );
        record!(
            "POST",
            "/api/v0/podcore/routing/cleanup",
            "malformed-path-query-or-body",
            response.status == "200 OK" && json_body(&response).is_object()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let response = request!(&state, "POST", "/api/v0/podcore/routing/cleanup", "");
        record!(
            "POST",
            "/api/v0/podcore/routing/cleanup",
            "missing-empty-or-conflict-state",
            response.status == "200 OK" && json_body(&response).is_object()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        prepare_feature_file(&state).await;
        {
            let mut features = state.controller_features.write_for_test().await;
            features
                .upsert(
                    "pod/routing-seen/podcore-cleanup-runtime/message".to_owned(),
                    serde_json::json!({
                        "messageId": "message",
                        "podId": "podcore-cleanup-runtime",
                        "seenAt": 0,
                    }),
                )
                .expect("seed expired routing entry");
        }
        block_feature_file(&state).await;
        let response = request!(&state, "POST", "/api/v0/podcore/routing/cleanup", "");
        record!(
            "POST",
            "/api/v0/podcore/routing/cleanup",
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("Failed to cleanup seen messages")
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        prepare_feature_file(&state).await;
        {
            let mut features = state.controller_features.write_for_test().await;
            features
                .upsert(
                    "pod/routing-seen/podcore-cleanup-restart/message".to_owned(),
                    serde_json::json!({
                        "messageId": "message",
                        "podId": "podcore-cleanup-restart",
                        "seenAt": 0,
                    }),
                )
                .expect("seed restart cleanup entry");
        }
        let response = request!(&state, "POST", "/api/v0/podcore/routing/cleanup", "");
        let loaded = crate::ControllerFeatureState::load(&state.config.state_dir)
            .expect("reload cleanup state");
        record!(
            "POST",
            "/api/v0/podcore/routing/cleanup",
            "restart-persistence-or-reset",
            response.status == "200 OK"
                && loaded
                    .get("pod/routing-seen/podcore-cleanup-restart/message")
                    .is_none()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        prepare_feature_file(&state).await;
        {
            let mut features = state.controller_features.write_for_test().await;
            features
                .upsert(
                    "pod/routing-seen/podcore-cleanup-concurrent/message".to_owned(),
                    serde_json::json!({
                        "messageId": "message",
                        "podId": "podcore-cleanup-concurrent",
                        "seenAt": 0,
                    }),
                )
                .expect("seed concurrent cleanup entry");
        }
        let (left, right) = tokio::join!(
            crate::route_http_request("POST", "/api/v0/podcore/routing/cleanup", None, "", &state),
            crate::route_http_request("POST", "/api/v0/podcore/routing/cleanup", None, "", &state)
        );
        record!(
            "POST",
            "/api/v0/podcore/routing/cleanup",
            "concurrency-and-idempotency",
            left.as_ref()
                .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
        );
    }

    {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-routing-route-missing").await;
        let response = request!(&state, "POST", "/api/v0/podcore/routing/route", "{}");
        record!(
            "POST",
            "/api/v0/podcore/routing/route",
            "missing-empty-or-conflict-state",
            response.status == "400 Bad Request"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-routing-route-restart").await;
        prepare_feature_file(&state).await;
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/routing/route",
            &message_body("route-restart", "podcore-routing-route-restart")
        );
        let loaded = crate::ControllerFeatureState::load(&state.config.state_dir)
            .expect("reload routed message state");
        record!(
            "POST",
            "/api/v0/podcore/routing/route",
            "restart-persistence-or-reset",
            response.status == "200 OK"
                && loaded
                    .get("pod/routing-seen/podcore-routing-route-restart/route-restart")
                    .is_some()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-routing-route-concurrent").await;
        prepare_feature_file(&state).await;
        let body = message_body("route-concurrent", "podcore-routing-route-concurrent");
        let (left, right) = tokio::join!(
            crate::route_http_request("POST", "/api/v0/podcore/routing/route", None, &body, &state),
            crate::route_http_request("POST", "/api/v0/podcore/routing/route", None, &body, &state)
        );
        let loaded = crate::ControllerFeatureState::load(&state.config.state_dir)
            .expect("reload concurrent routed message state");
        record!(
            "POST",
            "/api/v0/podcore/routing/route",
            "concurrency-and-idempotency",
            left.as_ref()
                .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
                && loaded
                    .get("pod/routing-seen/podcore-routing-route-concurrent/route-concurrent")
                    .is_some()
        );
    }

    let route_to_peers_body = |message_id: &str, pod_id: &str| {
        serde_json::json!({
            "message": serde_json::from_str::<serde_json::Value>(&message_body(message_id, pod_id))
                .expect("route-to-peers message fixture"),
            "targetPeerIds": ["missing-peer"],
        })
        .to_string()
    };
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/routing/route-to-peers",
            "{}"
        );
        record!(
            "POST",
            "/api/v0/podcore/routing/route-to-peers",
            "missing-empty-or-conflict-state",
            response.status == "400 Bad Request"
        );
    }
    for case in [
        "runtime-failure-and-timeout",
        "restart-persistence-or-reset",
    ] {
        let (state, _receiver) = test_state_with_env(pod_env());
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/routing/route-to-peers",
            &route_to_peers_body("route-to-peers", "podcore-route-to-peers")
        );
        let pass =
            response.status == "200 OK" && json_body(&response)["messageId"] == "route-to-peers";
        record!("POST", "/api/v0/podcore/routing/route-to-peers", case, pass);
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let body = route_to_peers_body("route-to-peers-concurrent", "podcore-route-to-peers");
        let (left, right) = tokio::join!(
            crate::route_http_request(
                "POST",
                "/api/v0/podcore/routing/route-to-peers",
                None,
                &body,
                &state
            ),
            crate::route_http_request(
                "POST",
                "/api/v0/podcore/routing/route-to-peers",
                None,
                &body,
                &state
            )
        );
        record!(
            "POST",
            "/api/v0/podcore/routing/route-to-peers",
            "concurrency-and-idempotency",
            left.as_ref()
                .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
        );
    }

    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/routing/seen/%20/podcore-seen-missing",
            ""
        );
        record!(
            "POST",
            "/api/v0/podcore/routing/seen/{messageId}/{podId}",
            "missing-empty-or-conflict-state",
            response.status == "400 Bad Request"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        prepare_feature_file(&state).await;
        block_feature_file(&state).await;
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/routing/seen/seen-runtime/podcore-seen-runtime",
            ""
        );
        record!(
            "POST",
            "/api/v0/podcore/routing/seen/{messageId}/{podId}",
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("Failed to register message as seen")
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        prepare_feature_file(&state).await;
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/routing/seen/seen-restart/podcore-seen-restart",
            ""
        );
        let loaded = crate::ControllerFeatureState::load(&state.config.state_dir)
            .expect("reload seen registration state");
        record!(
            "POST",
            "/api/v0/podcore/routing/seen/{messageId}/{podId}",
            "restart-persistence-or-reset",
            response.status == "200 OK"
                && loaded
                    .get("pod/routing-seen/podcore-seen-restart/seen-restart")
                    .is_some()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        prepare_feature_file(&state).await;
        let (left, right) = tokio::join!(
            crate::route_http_request(
                "POST",
                "/api/v0/podcore/routing/seen/seen-concurrent/podcore-seen-concurrent",
                None,
                "",
                &state
            ),
            crate::route_http_request(
                "POST",
                "/api/v0/podcore/routing/seen/seen-concurrent/podcore-seen-concurrent",
                None,
                "",
                &state
            )
        );
        let loaded = crate::ControllerFeatureState::load(&state.config.state_dir)
            .expect("reload concurrent seen state");
        record!(
            "POST",
            "/api/v0/podcore/routing/seen/{messageId}/{podId}",
            "concurrency-and-idempotency",
            left.as_ref()
                .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
                && loaded
                    .get("pod/routing-seen/podcore-seen-concurrent/seen-concurrent")
                    .is_some()
        );
    }

    // Signing and verification are process-local projections. Their
    // contracts are stable across restart and concurrent calls, with
    // malformed signing requests rejected before any crypto work.
    for case in [
        "malformed-path-query-or-body",
        "missing-empty-or-conflict-state",
        "runtime-failure-and-timeout",
        "restart-persistence-or-reset",
    ] {
        let (state, _receiver) = test_state_with_env(pod_env());
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/signing/generate-keypair",
            if case == "malformed-path-query-or-body" {
                "not-json"
            } else {
                ""
            }
        );
        let payload = json_body(&response);
        record!(
            "POST",
            "/api/v0/podcore/signing/generate-keypair",
            case,
            response.status == "200 OK"
                && payload["algorithm"] == "Ed25519"
                && payload["privateKey"]
                    .as_str()
                    .is_some_and(|value| !value.is_empty())
                && payload["publicKey"]
                    .as_str()
                    .is_some_and(|value| !value.is_empty())
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let (left, right) = tokio::join!(
            crate::route_http_request(
                "POST",
                "/api/v0/podcore/signing/generate-keypair",
                None,
                "",
                &state
            ),
            crate::route_http_request(
                "POST",
                "/api/v0/podcore/signing/generate-keypair",
                None,
                "",
                &state
            )
        );
        record!(
            "POST",
            "/api/v0/podcore/signing/generate-keypair",
            "concurrency-and-idempotency",
            left.as_ref().is_ok_and(|response| {
                response.status == "200 OK" && json_body(response)["privateKey"].is_string()
            }) && right.as_ref().is_ok_and(|response| {
                response.status == "200 OK" && json_body(response)["privateKey"].is_string()
            })
        );
    }

    let signing_message = message_body("signing-message", "podcore-signing");
    let signing_request = |message: &str| {
        serde_json::json!({
            "message": serde_json::from_str::<serde_json::Value>(message)
                .expect("signing message fixture"),
            "privateKey": signing_private_key,
        })
        .to_string()
    };
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let response = request!(&state, "POST", "/api/v0/podcore/signing/sign", "{}");
        record!(
            "POST",
            "/api/v0/podcore/signing/sign",
            "missing-empty-or-conflict-state",
            response.status == "400 Bad Request"
        );
    }
    for case in [
        "runtime-failure-and-timeout",
        "restart-persistence-or-reset",
    ] {
        let (state, _receiver) = test_state_with_env(pod_env());
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/signing/sign",
            &signing_request(&signing_message)
        );
        let payload = json_body(&response);
        record!(
            "POST",
            "/api/v0/podcore/signing/sign",
            case,
            response.status == "200 OK"
                && payload["signature"]
                    .as_str()
                    .is_some_and(|signature| signature.starts_with("ed25519:"))
                && payload["publicKey"].is_string()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let body = signing_request(&signing_message);
        let (left, right) = tokio::join!(
            crate::route_http_request("POST", "/api/v0/podcore/signing/sign", None, &body, &state),
            crate::route_http_request("POST", "/api/v0/podcore/signing/sign", None, &body, &state)
        );
        record!(
            "POST",
            "/api/v0/podcore/signing/sign",
            "concurrency-and-idempotency",
            left.as_ref()
                .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
        );
    }

    let verify_message = message_body("verify-message", "podcore-signing");
    for case in [
        "runtime-failure-and-timeout",
        "restart-persistence-or-reset",
    ] {
        let (state, _receiver) = test_state_with_env(pod_env());
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/signing/verify",
            &verify_message
        );
        record!(
            "POST",
            "/api/v0/podcore/signing/verify",
            case,
            response.status == "200 OK" && json_body(&response)["isValid"].is_boolean()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let (left, right) = tokio::join!(
            crate::route_http_request(
                "POST",
                "/api/v0/podcore/signing/verify",
                None,
                &verify_message,
                &state
            ),
            crate::route_http_request(
                "POST",
                "/api/v0/podcore/signing/verify",
                None,
                &verify_message,
                &state
            )
        );
        record!(
            "POST",
            "/api/v0/podcore/signing/verify",
            "concurrency-and-idempotency",
            left.as_ref()
                .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
        );
    }

    // The verification endpoint returns a structured, non-throwing
    // result for a syntactically valid but unknown member. This is a
    // process-local projection and remains stable across lifecycle calls.
    for case in [
        "runtime-failure-and-timeout",
        "restart-persistence-or-reset",
    ] {
        let (state, _receiver) = test_state_with_env(pod_env());
        let response = request!(
            &state,
            "POST",
            "/api/v0/podcore/verification/message",
            &message_body("verification-message", "podcore-verification-message")
        );
        let payload = json_body(&response);
        record!(
            "POST",
            "/api/v0/podcore/verification/message",
            case,
            response.status == "200 OK"
                && payload["isValid"].is_boolean()
                && payload["isFromValidMember"].is_boolean()
                && payload["hasValidSignature"].is_boolean()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let body = message_body(
            "verification-message-concurrent",
            "podcore-verification-message",
        );
        let (left, right) = tokio::join!(
            crate::route_http_request(
                "POST",
                "/api/v0/podcore/verification/message",
                None,
                &body,
                &state
            ),
            crate::route_http_request(
                "POST",
                "/api/v0/podcore/verification/message",
                None,
                &body,
                &state
            )
        );
        record!(
            "POST",
            "/api/v0/podcore/verification/message",
            "concurrency-and-idempotency",
            left.as_ref()
                .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
        );
    }

    // Last-seen updates are the one remaining backfill mutation.  The
    // fixture proves validation, durable write/readback, restart reload,
    // and concurrent overwrites of the same channel timestamp.
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        let response = request!(
            &state,
            "PUT",
            "/api/v0/podcore/backfill/podcore-last-seen-missing/general/last-seen",
            ""
        );
        record!(
            "PUT",
            "/api/v0/podcore/backfill/{podId}/{channelId}/last-seen",
            "missing-empty-or-conflict-state",
            response.status == "400 Bad Request"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        prepare_feature_file(&state).await;
        block_feature_file(&state).await;
        let response = request!(
            &state,
            "PUT",
            "/api/v0/podcore/backfill/podcore-last-seen-runtime/general/last-seen",
            "10"
        );
        record!(
            "PUT",
            "/api/v0/podcore/backfill/{podId}/{channelId}/last-seen",
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response
                    .body
                    .contains("An error occurred while updating last seen timestamp")
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        seed_pod(&state, "podcore-last-seen-mutation").await;
        prepare_feature_file(&state).await;
        let updated = request!(
            &state,
            "PUT",
            "/api/v0/podcore/backfill/podcore-last-seen-mutation/general/last-seen",
            "10"
        );
        let readback = request!(
            &state,
            "GET",
            "/api/v0/podcore/backfill/podcore-last-seen-mutation/last-seen",
            ""
        );
        record!(
            "PUT",
            "/api/v0/podcore/backfill/{podId}/{channelId}/last-seen",
            "mutation-side-effects-and-readback",
            updated.status == "200 OK"
                && readback.status == "200 OK"
                && json_body(&readback)["general"] == 10
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        prepare_feature_file(&state).await;
        let updated = request!(
            &state,
            "PUT",
            "/api/v0/podcore/backfill/podcore-last-seen-restart/general/last-seen",
            "10"
        );
        let loaded = crate::ControllerFeatureState::load(&state.config.state_dir)
            .expect("reload last-seen state");
        record!(
            "PUT",
            "/api/v0/podcore/backfill/{podId}/{channelId}/last-seen",
            "restart-persistence-or-reset",
            updated.status == "200 OK"
                && loaded
                    .get("pod/backfill/podcore-last-seen-restart/general")
                    .is_some()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(pod_env());
        prepare_feature_file(&state).await;
        let (left, right) = tokio::join!(
            crate::route_http_request(
                "PUT",
                "/api/v0/podcore/backfill/podcore-last-seen-concurrent/general/last-seen",
                None,
                "10",
                &state
            ),
            crate::route_http_request(
                "PUT",
                "/api/v0/podcore/backfill/podcore-last-seen-concurrent/general/last-seen",
                None,
                "20",
                &state
            )
        );
        let loaded = crate::ControllerFeatureState::load(&state.config.state_dir)
            .expect("reload concurrent last-seen state");
        record!(
            "PUT",
            "/api/v0/podcore/backfill/{podId}/{channelId}/last-seen",
            "concurrency-and-idempotency",
            left.as_ref()
                .is_ok_and(|response| response.status == "200 OK")
                && right
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
                && loaded
                    .get("pod/backfill/podcore-last-seen-concurrent/general")
                    .and_then(|value| value["lastSeen"].as_u64())
                    .is_some_and(|timestamp| timestamp == 10 || timestamp == 20)
        );
    }

    assert_eq!(ledger.len(), 98, "PodCore residual ledger size");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create PodCore evidence directory");
    fs::write(
        evidence_dir.join("podcore_controller_residuals.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize PodCore ledger"),
    )
    .expect("write PodCore ledger");
    assert!(
        mismatches.is_empty(),
        "{} PodCore residual mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
