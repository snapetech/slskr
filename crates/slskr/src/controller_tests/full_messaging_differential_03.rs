//! Controller full messaging differential 03 ownership.

use super::*;

/// Differential proof for the residual slskdn ConversationsController
/// cases. The frozen controller trims required route values, validates
/// model-bound ids and booleans before service work, returns empty 200/201
/// bodies for mutations, and evaluates its durable conversation store
/// before producing read results.
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
pub(super) async fn controller_api_differential_conversations_open_cases() {
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

    // DELETE /conversations/{username}
    {
        let (state, _receiver) = test_state_with_env(target_env());
        conversation_add(&state, "delete-peer", "close me").await;
        let response =
            conversation_request(&state, "DELETE", "/api/v0/conversations/delete-peer", "").await;
        record!(
            "DELETE",
            "/api/v0/conversations/{username}",
            "nominal-status-headers-body",
            response.status == "204 No Content" && response.body.is_empty()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let response = conversation_request(
            &state,
            "DELETE",
            "/api/v0/conversations/delete-peer/extra",
            "",
        )
        .await;
        record!(
            "DELETE",
            "/api/v0/conversations/{username}",
            "malformed-path-query-or-body",
            response.status == "404 Not Found"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        conversation_add(&state, "delete-peer", "close me").await;
        let response =
            conversation_request(&state, "DELETE", "/api/v0/conversations/delete-peer", "").await;
        let remaining = state
            .messages
            .read()
            .await
            .records
            .iter()
            .any(|record| record.username == "delete-peer");
        record!(
            "DELETE",
            "/api/v0/conversations/{username}",
            "mutation-side-effects-and-readback",
            response.status == "204 No Content" && !remaining
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let response =
            conversation_request(&state, "DELETE", "/api/v0/conversations/restarted-peer", "")
                .await;
        record!(
            "DELETE",
            "/api/v0/conversations/{username}",
            "restart-persistence-or-reset",
            response.status == "404 Not Found"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        conversation_add(&state, "delete-peer", "close me").await;
        let responses = futures_util::future::join_all([
            conversation_request(&state, "DELETE", "/api/v0/conversations/delete-peer", ""),
            conversation_request(&state, "DELETE", "/api/v0/conversations/delete-peer", ""),
        ])
        .await;
        let statuses = responses
            .iter()
            .map(|response| response.status)
            .collect::<Vec<_>>();
        record!(
            "DELETE",
            "/api/v0/conversations/{username}",
            "concurrency-and-idempotency",
            statuses.contains(&"204 No Content") && statuses.contains(&"404 Not Found")
        );
    }

    // GET /conversations
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let malformed = conversation_request(
            &state,
            "GET",
            "/api/v0/conversations?includeInactive=maybe",
            "",
        )
        .await;
        record!(
            "GET",
            "/api/v0/conversations",
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
        let empty = conversation_request(&state, "GET", "/api/v0/conversations", "").await;
        record!(
            "GET",
            "/api/v0/conversations",
            "missing-empty-or-conflict-state",
            empty.status == "200 OK" && json_value(&empty).as_array().is_some_and(Vec::is_empty)
        );
    }
    {
        let state = conversation_runtime_state(target_env(), None).await;
        let response = conversation_request(&state, "GET", "/api/v0/conversations", "").await;
        record!(
            "GET",
            "/api/v0/conversations",
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("conversation storage unavailable")
        );
    }

    // GET /conversations/{username}
    {
        let (state, _receiver) = test_state_with_env(target_env());
        conversation_add(&state, "conversation-peer", "hello").await;
        let nominal =
            conversation_request(&state, "GET", "/api/v0/conversations/conversation-peer", "")
                .await;
        let value = json_value(&nominal);
        record!(
            "GET",
            "/api/v0/conversations/{username}",
            "nominal-status-headers-body",
            nominal.status == "200 OK"
                && value["username"] == "conversation-peer"
                && value["messages"].is_array()
        );
        let malformed = conversation_request(
            &state,
            "GET",
            "/api/v0/conversations/conversation-peer?since=-1",
            "",
        )
        .await;
        record!(
            "GET",
            "/api/v0/conversations/{username}",
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
    }
    {
        let state = conversation_runtime_state(target_env(), Some("conversation-peer")).await;
        let response =
            conversation_request(&state, "GET", "/api/v0/conversations/conversation-peer", "")
                .await;
        record!(
            "GET",
            "/api/v0/conversations/{username}",
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("conversation storage unavailable")
        );
    }

    // GET /conversations/{username}/messages
    {
        let (state, _receiver) = test_state_with_env(target_env());
        conversation_add(&state, "messages-peer", "one").await;
        let nominal = conversation_request(
            &state,
            "GET",
            "/api/v0/conversations/messages-peer/messages",
            "",
        )
        .await;
        record!(
            "GET",
            "/api/v0/conversations/{username}/messages",
            "nominal-status-headers-body",
            nominal.status == "200 OK" && json_value(&nominal).is_array()
        );
        let malformed = conversation_request(
            &state,
            "GET",
            "/api/v0/conversations/messages-peer/messages?unAcknowledgedOnly=maybe",
            "",
        )
        .await;
        record!(
            "GET",
            "/api/v0/conversations/{username}/messages",
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
        let acknowledged_id = conversation_add(&state, "messages-peer", "two").await;
        state.messages.write().await.ack(acknowledged_id);
        let populated = conversation_request(
            &state,
            "GET",
            "/api/v0/conversations/messages-peer/messages?unAcknowledgedOnly=true",
            "",
        )
        .await;
        record!(
            "GET",
            "/api/v0/conversations/{username}/messages",
            "populated-dynamic-state",
            populated.status == "200 OK"
                && json_value(&populated)
                    .as_array()
                    .is_some_and(|messages| messages.len() == 1)
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let response = conversation_request(
            &state,
            "GET",
            "/api/v0/conversations/missing-messages/messages",
            "",
        )
        .await;
        record!(
            "GET",
            "/api/v0/conversations/{username}/messages",
            "missing-empty-or-conflict-state",
            response.status == "404 Not Found"
        );
    }
    {
        let state = conversation_runtime_state(target_env(), Some("messages-peer")).await;
        let response = conversation_request(
            &state,
            "GET",
            "/api/v0/conversations/messages-peer/messages",
            "",
        )
        .await;
        record!(
            "GET",
            "/api/v0/conversations/{username}/messages",
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("conversation storage unavailable")
        );
    }

    // GET /conversations/activity/unacknowledged
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let malformed = conversation_request(
            &state,
            "GET",
            "/api/v0/conversations/activity/unacknowledged/extra",
            "",
        )
        .await;
        record!(
            "GET",
            "/api/v0/conversations/activity/unacknowledged",
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );
        let missing = conversation_request(
            &state,
            "GET",
            "/api/v0/conversations/activity/unacknowledged",
            "",
        )
        .await;
        record!(
            "GET",
            "/api/v0/conversations/activity/unacknowledged",
            "missing-empty-or-conflict-state",
            missing.status == "200 OK" && json_value(&missing) == serde_json::json!(false)
        );
    }
    {
        let state = conversation_runtime_state(target_env(), None).await;
        let response = conversation_request(
            &state,
            "GET",
            "/api/v0/conversations/activity/unacknowledged",
            "",
        )
        .await;
        record!(
            "GET",
            "/api/v0/conversations/activity/unacknowledged",
            "runtime-failure-and-timeout",
            response.status == "500 Internal Server Error"
                && response.body.contains("conversation storage unavailable")
        );
    }

    // POST /conversations/{username}
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let malformed = conversation_request(
            &state,
            "POST",
            "/api/v0/conversations/post-peer",
            r#"{"message":""}"#,
        )
        .await;
        record!(
            "POST",
            "/api/v0/conversations/{username}",
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
        let missing =
            conversation_request(&state, "POST", "/api/v0/conversations/post-peer", "").await;
        record!(
            "POST",
            "/api/v0/conversations/{username}",
            "missing-empty-or-conflict-state",
            missing.status == "400 Bad Request"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        conversation_connect(&state).await;
        let response = conversation_request(
            &state,
            "POST",
            "/api/v0/conversations/restarted-post-peer",
            r#""restart""#,
        )
        .await;
        record!(
            "POST",
            "/api/v0/conversations/{username}",
            "restart-persistence-or-reset",
            response.status == "201 Created" && response.body.is_empty()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        conversation_connect(&state).await;
        let responses = futures_util::future::join_all([
            conversation_request(
                &state,
                "POST",
                "/api/v0/conversations/concurrent-post-peer",
                r#""one""#,
            ),
            conversation_request(
                &state,
                "POST",
                "/api/v0/conversations/concurrent-post-peer",
                r#""two""#,
            ),
        ])
        .await;
        record!(
            "POST",
            "/api/v0/conversations/{username}",
            "concurrency-and-idempotency",
            responses
                .iter()
                .all(|response| { response.status == "201 Created" && response.body.is_empty() })
        );
    }

    // POST /conversations/batch
    {
        let (state, mut receiver) = test_state_with_env(target_env());
        let nominal = conversation_request(
            &state,
            "POST",
            "/api/v0/conversations/batch",
            r#"{"usernames":["batch-a","batch-b"],"message":"hello"}"#,
        )
        .await;
        let command = receiver.try_recv().ok();
        record!(
            "POST",
            "/api/v0/conversations/batch",
            "nominal-status-headers-body",
            nominal.status == "201 Created"
                && nominal.body.is_empty()
                && matches!(command, Some(crate::SessionCommand::MessageUsers { .. }))
        );
        let malformed = conversation_request(
            &state,
            "POST",
            "/api/v0/conversations/batch",
            r#"{"usernames":"not-an-array","message":"hello"}"#,
        )
        .await;
        record!(
            "POST",
            "/api/v0/conversations/batch",
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
        let missing =
            conversation_request(&state, "POST", "/api/v0/conversations/batch", "{}").await;
        record!(
            "POST",
            "/api/v0/conversations/batch",
            "missing-empty-or-conflict-state",
            missing.status == "400 Bad Request"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let response = conversation_request(
            &state,
            "POST",
            "/api/v0/conversations/batch",
            r#"{"usernames":["restart-batch"],"message":"hello"}"#,
        )
        .await;
        record!(
            "POST",
            "/api/v0/conversations/batch",
            "restart-persistence-or-reset",
            response.status == "201 Created" && response.body.is_empty()
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let responses = futures_util::future::join_all([
            conversation_request(
                &state,
                "POST",
                "/api/v0/conversations/batch",
                r#"{"usernames":["concurrent-batch"],"message":"one"}"#,
            ),
            conversation_request(
                &state,
                "POST",
                "/api/v0/conversations/batch",
                r#"{"usernames":["concurrent-batch"],"message":"two"}"#,
            ),
        ])
        .await;
        record!(
            "POST",
            "/api/v0/conversations/batch",
            "concurrency-and-idempotency",
            responses
                .iter()
                .all(|response| { response.status == "201 Created" && response.body.is_empty() })
        );
    }

    // PUT /conversations/{username}
    {
        let (state, _receiver) = test_state_with_env(target_env());
        conversation_add(&state, "ack-all-peer", "ack me").await;
        conversation_connect(&state).await;
        let nominal =
            conversation_request(&state, "PUT", "/api/v0/conversations/ack-all-peer", "").await;
        record!(
            "PUT",
            "/api/v0/conversations/{username}",
            "nominal-status-headers-body",
            nominal.status == "200 OK" && nominal.body.is_empty()
        );
        let malformed = conversation_request(&state, "PUT", "/api/v0/conversations/%20", "").await;
        record!(
            "PUT",
            "/api/v0/conversations/{username}",
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        conversation_connect(&state).await;
        let response =
            conversation_request(&state, "PUT", "/api/v0/conversations/missing-ack-all", "").await;
        record!(
            "PUT",
            "/api/v0/conversations/{username}",
            "missing-empty-or-conflict-state",
            response.status == "404 Not Found"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let id = conversation_add(&state, "mutation-ack-all", "ack me").await;
        conversation_connect(&state).await;
        let response =
            conversation_request(&state, "PUT", "/api/v0/conversations/mutation-ack-all", "").await;
        let acknowledged = state
            .messages
            .read()
            .await
            .records
            .iter()
            .find(|record| record.id == id)
            .is_some_and(|record| record.acknowledged);
        record!(
            "PUT",
            "/api/v0/conversations/{username}",
            "mutation-side-effects-and-readback",
            response.status == "200 OK" && acknowledged
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        conversation_connect(&state).await;
        let response =
            conversation_request(&state, "PUT", "/api/v0/conversations/restarted-ack-all", "")
                .await;
        record!(
            "PUT",
            "/api/v0/conversations/{username}",
            "restart-persistence-or-reset",
            response.status == "404 Not Found"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        conversation_add(&state, "concurrent-ack-all", "ack me").await;
        conversation_connect(&state).await;
        let responses = futures_util::future::join_all([
            conversation_request(
                &state,
                "PUT",
                "/api/v0/conversations/concurrent-ack-all",
                "",
            ),
            conversation_request(
                &state,
                "PUT",
                "/api/v0/conversations/concurrent-ack-all",
                "",
            ),
        ])
        .await;
        record!(
            "PUT",
            "/api/v0/conversations/{username}",
            "concurrency-and-idempotency",
            responses
                .iter()
                .all(|response| { response.status == "200 OK" && response.body.is_empty() })
        );
    }

    // PUT /conversations/{username}/{id}
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let id = conversation_add(&state, "ack-one-peer", "ack me").await;
        conversation_connect(&state).await;
        let path = format!("/api/v0/conversations/ack-one-peer/{id}");
        let nominal = conversation_request(&state, "PUT", &path, "").await;
        record!(
            "PUT",
            "/api/v0/conversations/{username}/{id}",
            "nominal-status-headers-body",
            nominal.status == "200 OK" && nominal.body.is_empty()
        );
        let malformed = conversation_request(
            &state,
            "PUT",
            "/api/v0/conversations/ack-one-peer/not-an-int",
            "",
        )
        .await;
        record!(
            "PUT",
            "/api/v0/conversations/{username}/{id}",
            "malformed-path-query-or-body",
            malformed.status == "400 Bad Request"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        conversation_connect(&state).await;
        let response =
            conversation_request(&state, "PUT", "/api/v0/conversations/missing-ack-one/1", "")
                .await;
        record!(
            "PUT",
            "/api/v0/conversations/{username}/{id}",
            "missing-empty-or-conflict-state",
            response.status == "404 Not Found"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let id = conversation_add(&state, "mutation-ack-one", "ack me").await;
        conversation_connect(&state).await;
        let path = format!("/api/v0/conversations/mutation-ack-one/{id}");
        let response = conversation_request(&state, "PUT", &path, "").await;
        let acknowledged = state
            .messages
            .read()
            .await
            .records
            .iter()
            .find(|record| record.id == id)
            .is_some_and(|record| record.acknowledged);
        record!(
            "PUT",
            "/api/v0/conversations/{username}/{id}",
            "mutation-side-effects-and-readback",
            response.status == "200 OK" && acknowledged
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        conversation_connect(&state).await;
        let response = conversation_request(
            &state,
            "PUT",
            "/api/v0/conversations/restarted-ack-one/1",
            "",
        )
        .await;
        record!(
            "PUT",
            "/api/v0/conversations/{username}/{id}",
            "restart-persistence-or-reset",
            response.status == "404 Not Found"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let id = conversation_add(&state, "concurrent-ack-one", "ack me").await;
        conversation_connect(&state).await;
        let path = format!("/api/v0/conversations/concurrent-ack-one/{id}");
        let responses = futures_util::future::join_all([
            conversation_request(&state, "PUT", &path, ""),
            conversation_request(&state, "PUT", &path, ""),
        ])
        .await;
        record!(
            "PUT",
            "/api/v0/conversations/{username}/{id}",
            "concurrency-and-idempotency",
            responses
                .iter()
                .all(|response| { response.status == "200 OK" && response.body.is_empty() })
        );
    }

    assert_eq!(ledger.len(), 40, "conversations residual ledger size");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create conversations evidence directory");
    fs::write(
        evidence_dir.join("conversations_open_cases.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize conversations ledger"),
    )
    .expect("write conversations ledger");
    assert!(
        mismatches.is_empty(),
        "{} controller-api conversations mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
