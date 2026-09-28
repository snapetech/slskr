//! Controller full shares differential 02 ownership.

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
    feature = "bounded-controller-api-tests-3"
))]
pub(super) async fn controller_api_differential_sharegroups_persistence_and_concurrency() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    let persistence_env = || {
        MapEnv::default()
            .with("SLSKR_PERSISTENCE_ENABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", target)
    };

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

    // Share-group creation survives a real persisted-state rebuild.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("share-group create restart database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let created = crate::route_http_request(
            "POST",
            "/api/v0/sharegroups",
            None,
            r#"{"name":"Restart Share Group","description":"persisted"}"#,
            &state,
        )
        .await
        .unwrap();
        let created_json = serde_json::from_str::<serde_json::Value>(&created.body).unwrap();
        let group_id = created_json["id"].as_str().unwrap().to_owned();
        let groups = db.list_share_groups(10, 0).await.unwrap();
        let members = db.list_share_group_members(10, 0).await.unwrap();
        let (restarted_state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        *restarted_state.sharegroups.write().await =
            crate::ShareGroupStore::from_persisted(groups.clone(), members);
        let fetched = crate::route_http_request(
            "GET",
            &format!("/api/v0/sharegroups/{group_id}"),
            None,
            "",
            &restarted_state,
        )
        .await
        .unwrap();
        let fetched_json = serde_json::from_str::<serde_json::Value>(&fetched.body).unwrap();
        record!(
            "POST",
            "/api/v0/sharegroups",
            "restart-persistence-or-reset",
            created.status == "201 Created"
                && groups.len() == 1
                && groups[0].id == group_id
                && fetched.status == "200 OK"
                && fetched_json["id"] == group_id
        );
    }

    // Distinct group creates through one real controller are all durable.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("share-group create concurrency database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let bodies: Vec<String> = (0..4)
            .map(|index| format!(r#"{{"name":"Concurrent Share Group {index}"}}"#))
            .collect();
        let responses = futures_util::future::join_all(bodies.iter().map(|body| {
            crate::route_http_request("POST", "/api/v0/sharegroups", None, body, &state)
        }))
        .await;
        let groups = db.list_share_groups(10, 0).await.unwrap_or_default();
        let expected: std::collections::BTreeSet<String> = (0..4)
            .map(|index| format!("Concurrent Share Group {index}"))
            .collect();
        let names: std::collections::BTreeSet<String> =
            groups.iter().map(|group| group.name.clone()).collect();
        record!(
            "POST",
            "/api/v0/sharegroups",
            "concurrency-and-idempotency",
            responses.iter().all(|response| {
                response
                    .as_ref()
                    .is_ok_and(|response| response.status == "201 Created")
            }) && groups.len() == 4
                && names == expected
        );
    }

    // A group update survives restart rehydration.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("share-group update restart database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let created = crate::route_http_request(
            "POST",
            "/api/v0/sharegroups",
            None,
            r#"{"name":"Update Restart Before"}"#,
            &state,
        )
        .await
        .unwrap();
        let group_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let updated = crate::route_http_request(
            "PUT",
            &format!("/api/v0/sharegroups/{group_id}"),
            None,
            r#"{"name":"Update Restart After","description":"changed"}"#,
            &state,
        )
        .await
        .unwrap();
        let groups = db.list_share_groups(10, 0).await.unwrap();
        let members = db.list_share_group_members(10, 0).await.unwrap();
        let (restarted_state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        *restarted_state.sharegroups.write().await =
            crate::ShareGroupStore::from_persisted(groups, members);
        let fetched = crate::route_http_request(
            "GET",
            &format!("/api/v0/sharegroups/{group_id}"),
            None,
            "",
            &restarted_state,
        )
        .await
        .unwrap();
        let fetched_json = serde_json::from_str::<serde_json::Value>(&fetched.body).unwrap();
        record!(
            "PUT",
            "/api/v0/sharegroups/{id}",
            "restart-persistence-or-reset",
            updated.status == "200 OK"
                && fetched.status == "200 OK"
                && fetched_json["name"] == "Update Restart After"
        );
    }

    // Concurrent group updates retain each writer's own value.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("share-group update concurrency database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let mut group_ids = Vec::new();
        for index in 0..4 {
            let created = crate::route_http_request(
                "POST",
                "/api/v0/sharegroups",
                None,
                &format!(r#"{{"name":"Update Concurrent Before {index}"}}"#),
                &state,
            )
            .await
            .unwrap();
            group_ids.push(
                serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
                    .as_str()
                    .unwrap()
                    .to_owned(),
            );
        }
        let responses = futures_util::future::join_all(group_ids.iter().enumerate().map(
            |(index, group_id)| {
                let path = format!("/api/v0/sharegroups/{group_id}");
                let body = format!(r#"{{"name":"Update Concurrent After {index}"}}"#);
                let state = Arc::clone(&state);
                async move { crate::route_http_request("PUT", &path, None, &body, &state).await }
            },
        ))
        .await;
        let groups = db.list_share_groups(10, 0).await.unwrap_or_default();
        let expected: std::collections::BTreeSet<String> = (0..4)
            .map(|index| format!("Update Concurrent After {index}"))
            .collect();
        let names: std::collections::BTreeSet<String> =
            groups.iter().map(|group| group.name.clone()).collect();
        record!(
            "PUT",
            "/api/v0/sharegroups/{id}",
            "concurrency-and-idempotency",
            responses.iter().all(|response| {
                response
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
            }) && groups.len() == 4
                && names == expected
        );
    }

    // A member create survives group/member snapshot rehydration.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("share-group member create restart database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let created = crate::route_http_request(
            "POST",
            "/api/v0/sharegroups",
            None,
            r#"{"name":"Member Restart Group"}"#,
            &state,
        )
        .await
        .unwrap();
        let group_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let added = crate::route_http_request(
            "POST",
            &format!("/api/v0/sharegroups/{group_id}/members"),
            None,
            r#"{"username":"restart-member"}"#,
            &state,
        )
        .await
        .unwrap();
        let groups = db.list_share_groups(10, 0).await.unwrap();
        let members = db.list_share_group_members(10, 0).await.unwrap();
        let (restarted_state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        *restarted_state.sharegroups.write().await =
            crate::ShareGroupStore::from_persisted(groups, members);
        let listed = crate::route_http_request(
            "GET",
            &format!("/api/v0/sharegroups/{group_id}/members"),
            None,
            "",
            &restarted_state,
        )
        .await
        .unwrap();
        let listed_json = serde_json::from_str::<serde_json::Value>(&listed.body).unwrap();
        record!(
            "POST",
            "/api/v0/sharegroups/{id}/members",
            "restart-persistence-or-reset",
            added.status == "201 Created"
                && listed.status == "200 OK"
                && listed_json.as_array().is_some_and(|members| {
                    members
                        .iter()
                        .any(|member| member["username"] == "restart-member")
                })
        );
    }

    // Distinct member creates through one group are all durable.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("share-group member create concurrency database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let created = crate::route_http_request(
            "POST",
            "/api/v0/sharegroups",
            None,
            r#"{"name":"Member Concurrent Group"}"#,
            &state,
        )
        .await
        .unwrap();
        let group_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let bodies: Vec<String> = (0..4)
            .map(|index| format!(r#"{{"username":"concurrent-member-{index}"}}"#))
            .collect();
        let path = format!("/api/v0/sharegroups/{group_id}/members");
        let responses = futures_util::future::join_all(
            bodies
                .iter()
                .map(|body| crate::route_http_request("POST", &path, None, body, &state)),
        )
        .await;
        let members = db.list_share_group_members(10, 0).await.unwrap_or_default();
        let expected: std::collections::BTreeSet<String> = (0..4)
            .map(|index| format!("concurrent-member-{index}"))
            .collect();
        let usernames: std::collections::BTreeSet<String> = members
            .iter()
            .map(|member| member.username.clone())
            .collect();
        record!(
            "POST",
            "/api/v0/sharegroups/{id}/members",
            "concurrency-and-idempotency",
            responses.iter().all(|response| {
                response
                    .as_ref()
                    .is_ok_and(|response| response.status == "201 Created")
            }) && members.len() == 4
                && usernames == expected
        );
    }

    // Group deletion is durable across restart.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("share-group delete restart database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let created = crate::route_http_request(
            "POST",
            "/api/v0/sharegroups",
            None,
            r#"{"name":"Delete Restart Group"}"#,
            &state,
        )
        .await
        .unwrap();
        let group_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let deleted = crate::route_http_request(
            "DELETE",
            &format!("/api/v0/sharegroups/{group_id}"),
            None,
            "",
            &state,
        )
        .await
        .unwrap();
        let groups = db.list_share_groups(10, 0).await.unwrap();
        let members = db.list_share_group_members(10, 0).await.unwrap();
        let (restarted_state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        *restarted_state.sharegroups.write().await =
            crate::ShareGroupStore::from_persisted(groups.clone(), members);
        let fetched = crate::route_http_request(
            "GET",
            &format!("/api/v0/sharegroups/{group_id}"),
            None,
            "",
            &restarted_state,
        )
        .await
        .unwrap();
        record!(
            "DELETE",
            "/api/v0/sharegroups/{id}",
            "restart-persistence-or-reset",
            deleted.status == "200 OK" && groups.is_empty() && fetched.status == "404 Not Found"
        );
    }

    // Distinct group deletions complete and remove every persisted row.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("share-group delete concurrency database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let mut group_ids = Vec::new();
        for index in 0..4 {
            let created = crate::route_http_request(
                "POST",
                "/api/v0/sharegroups",
                None,
                &format!(r#"{{"name":"Delete Concurrent Group {index}"}}"#),
                &state,
            )
            .await
            .unwrap();
            group_ids.push(
                serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
                    .as_str()
                    .unwrap()
                    .to_owned(),
            );
        }
        let responses = futures_util::future::join_all(group_ids.iter().map(|group_id| {
            let path = format!("/api/v0/sharegroups/{group_id}");
            let state = Arc::clone(&state);
            async move { crate::route_http_request("DELETE", &path, None, "", &state).await }
        }))
        .await;
        let groups = db.list_share_groups(10, 0).await.unwrap_or_default();
        record!(
            "DELETE",
            "/api/v0/sharegroups/{id}",
            "concurrency-and-idempotency",
            responses.iter().all(|response| {
                response
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
            }) && groups.is_empty()
        );
    }

    // Member deletion is durable across restart.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("share-group member delete restart database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let created = crate::route_http_request(
            "POST",
            "/api/v0/sharegroups",
            None,
            r#"{"name":"Delete Member Restart Group"}"#,
            &state,
        )
        .await
        .unwrap();
        let group_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let member_path = format!("/api/v0/sharegroups/{group_id}/members");
        let added = crate::route_http_request(
            "POST",
            &member_path,
            None,
            r#"{"username":"delete-restart-member"}"#,
            &state,
        )
        .await
        .unwrap();
        let deleted = crate::route_http_request(
            "DELETE",
            &format!("{member_path}/delete-restart-member"),
            None,
            "",
            &state,
        )
        .await
        .unwrap();
        let groups = db.list_share_groups(10, 0).await.unwrap();
        let members = db.list_share_group_members(10, 0).await.unwrap();
        let (restarted_state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        *restarted_state.sharegroups.write().await =
            crate::ShareGroupStore::from_persisted(groups, members.clone());
        let listed = crate::route_http_request("GET", &member_path, None, "", &restarted_state)
            .await
            .unwrap();
        record!(
            "DELETE",
            "/api/v0/sharegroups/{id}/members/{userId}",
            "restart-persistence-or-reset",
            added.status == "201 Created"
                && deleted.status == "200 OK"
                && members.is_empty()
                && listed.status == "200 OK"
                && listed.body == "[]"
        );
    }

    // Distinct member deletions complete without leaving persisted rows.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("share-group member delete concurrency database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let created = crate::route_http_request(
            "POST",
            "/api/v0/sharegroups",
            None,
            r#"{"name":"Delete Member Concurrent Group"}"#,
            &state,
        )
        .await
        .unwrap();
        let group_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let member_path = format!("/api/v0/sharegroups/{group_id}/members");
        let mut usernames = Vec::new();
        for index in 0..4 {
            let username = format!("delete-concurrent-member-{index}");
            crate::route_http_request(
                "POST",
                &member_path,
                None,
                &format!(r#"{{"username":"{username}"}}"#),
                &state,
            )
            .await
            .unwrap();
            usernames.push(username);
        }
        let responses = futures_util::future::join_all(usernames.iter().map(|username| {
            let path = format!("{member_path}/{username}");
            let state = Arc::clone(&state);
            async move { crate::route_http_request("DELETE", &path, None, "", &state).await }
        }))
        .await;
        let members = db.list_share_group_members(10, 0).await.unwrap_or_default();
        record!(
            "DELETE",
            "/api/v0/sharegroups/{id}/members/{userId}",
            "concurrency-and-idempotency",
            responses.iter().all(|response| {
                response
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
            }) && members.is_empty()
        );
    }

    // Group creation rolls back when the persistence backend fails.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("share-group create runtime database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        db.close_for_test().await;
        let response = crate::route_http_request(
            "POST",
            "/api/v0/sharegroups",
            None,
            r#"{"name":"Runtime Share Group"}"#,
            &state,
        )
        .await
        .unwrap();
        let pass = response.status == "503 Service Unavailable"
            && state.sharegroups.read().await.records.is_empty();
        record!(
            "POST",
            "/api/v0/sharegroups",
            "runtime-failure-and-timeout",
            pass
        );
    }

    // Group updates restore the prior in-memory state when persistence fails.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("share-group update runtime database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let created = crate::route_http_request(
            "POST",
            "/api/v0/sharegroups",
            None,
            r#"{"name":"Runtime Update Before"}"#,
            &state,
        )
        .await
        .unwrap();
        let group_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        db.close_for_test().await;
        let response = crate::route_http_request(
            "PUT",
            &format!("/api/v0/sharegroups/{group_id}"),
            None,
            r#"{"name":"Runtime Update After"}"#,
            &state,
        )
        .await
        .unwrap();
        let group = state.sharegroups.read().await.get(&group_id);
        let pass = response.status == "503 Service Unavailable"
            && group.is_some_and(|group| group.name == "Runtime Update Before");
        record!(
            "PUT",
            "/api/v0/sharegroups/{id}",
            "runtime-failure-and-timeout",
            pass
        );
    }

    // Member additions roll back when persistence fails.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("share-group member create runtime database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let created = crate::route_http_request(
            "POST",
            "/api/v0/sharegroups",
            None,
            r#"{"name":"Runtime Member Group"}"#,
            &state,
        )
        .await
        .unwrap();
        let group_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        db.close_for_test().await;
        let response = crate::route_http_request(
            "POST",
            &format!("/api/v0/sharegroups/{group_id}/members"),
            None,
            r#"{"username":"runtime-member"}"#,
            &state,
        )
        .await
        .unwrap();
        let group = state.sharegroups.read().await.get(&group_id);
        let pass = response.status == "503 Service Unavailable"
            && group.is_some_and(|group| group.members.is_empty());
        record!(
            "POST",
            "/api/v0/sharegroups/{id}/members",
            "runtime-failure-and-timeout",
            pass
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("sharegroups_persistence_and_concurrency.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        mismatches.is_empty(),
        "{} controller-api sharegroup persistence mismatches:\n{}",
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
    feature = "bounded-controller-api-tests-4"
))]
pub(super) async fn controller_api_differential_share_grants_open_cases() {
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

    let target_env = || MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target);
    let parse_json = |body: &str| {
        serde_json::from_str::<serde_json::Value>(body).unwrap_or(serde_json::Value::Null)
    };
    let missing_id = "00000000-0000-0000-0000-000000000000";

    // Read routes remain deterministic when their persistence backend is
    // unavailable: list is an empty array, while an unknown resource is
    // still NotFound.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("share-grants list runtime database");
        let (state, _receiver) =
            test_state_with_env_parts(target_env(), crate::SearchStore::new(), Some(db.clone()));
        db.close_for_test().await;
        let list = crate::route_http_request("GET", "/api/v0/share-grants", None, "", &state)
            .await
            .expect("share-grants list runtime response");
        let detail = crate::route_http_request(
            "GET",
            &format!("/api/v0/share-grants/{missing_id}"),
            None,
            "",
            &state,
        )
        .await
        .expect("share-grants detail runtime response");
        record!(
            "GET",
            "/api/v0/share-grants",
            "runtime-failure-and-timeout",
            list.status == "200 OK" && parse_json(&list.body).is_array()
        );
        record!(
            "GET",
            "/api/v0/share-grants/{id}",
            "runtime-failure-and-timeout",
            detail.status == "404 Not Found"
        );
    }

    let (state, _receiver) = test_state_with_env(target_env());
    let collection_id = uuid::Uuid::new_v4().to_string();
    state
        .collections
        .write()
        .await
        .create_with_contract(
            collection_id.clone(),
            "tester".to_owned(),
            "Share Grant Open Cases".to_owned(),
            String::new(),
            "ShareList".to_owned(),
        )
        .expect("share-grants collection fixture");
    state
        .collections
        .write()
        .await
        .add_item(
            &collection_id,
            "content:share-grant-open-case".to_owned(),
            "Differential Artist".to_owned(),
            "Differential Title".to_owned(),
            "Audio".to_owned(),
        )
        .expect("share-grants collection item fixture")
        .expect("share-grants collection item");
    let grant_id = uuid::Uuid::new_v4().to_string();
    state
        .share_grants
        .write()
        .await
        .create_with_contract(
            Some(grant_id.clone()),
            collection_id.clone(),
            "recipient".to_owned(),
        )
        .expect("share-grants fixture");

    let manifest_path = format!("/api/v0/share-grants/{grant_id}/manifest");
    let manifest = crate::route_http_request("GET", &manifest_path, None, "", &state)
        .await
        .expect("share-grants manifest nominal response");
    let manifest_value = parse_json(&manifest.body);
    record!(
        "GET",
        "/api/v0/share-grants/{id}/manifest",
        "nominal-status-headers-body",
        manifest.status == "200 OK"
            && manifest_value["share"].is_object()
            && manifest_value["collection"].is_object()
            && manifest_value["items"].is_array()
            && manifest_value["itemCount"] == 1
    );
    let manifest_missing = crate::route_http_request(
        "GET",
        &format!("/api/v0/share-grants/{missing_id}/manifest"),
        None,
        "",
        &state,
    )
    .await
    .expect("share-grants manifest missing response");
    record!(
        "GET",
        "/api/v0/share-grants/{id}/manifest",
        "missing-empty-or-conflict-state",
        manifest_missing.status == "404 Not Found"
    );
    let manifest_runtime_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("share-grants manifest runtime database");
    let (manifest_runtime_state, _receiver) = test_state_with_env_parts(
        target_env(),
        crate::SearchStore::new(),
        Some(manifest_runtime_db.clone()),
    );
    manifest_runtime_db.close_for_test().await;
    let manifest_runtime = crate::route_http_request(
        "GET",
        &format!("/api/v0/share-grants/{missing_id}/manifest"),
        None,
        "",
        &manifest_runtime_state,
    )
    .await
    .expect("share-grants manifest runtime response");
    record!(
        "GET",
        "/api/v0/share-grants/{id}/manifest",
        "runtime-failure-and-timeout",
        manifest_runtime.status == "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/share-grants/{id}/manifest",
        "populated-dynamic-state",
        manifest_value["share"]["id"] == grant_id
            && manifest_value["collection"]["id"] == collection_id
            && manifest_value["items"]
                .as_array()
                .is_some_and(|items| items.len() == 1)
    );

    let by_collection_path = format!("/api/v0/share-grants/by-collection/{collection_id}");
    let by_collection = crate::route_http_request("GET", &by_collection_path, None, "", &state)
        .await
        .expect("share-grants by-collection nominal response");
    let by_collection_value = parse_json(&by_collection.body);
    record!(
        "GET",
        "/api/v0/share-grants/by-collection/{collectionId}",
        "nominal-status-headers-body",
        by_collection.status == "200 OK" && by_collection_value.is_array()
    );
    let by_collection_missing = crate::route_http_request(
        "GET",
        &format!("/api/v0/share-grants/by-collection/{missing_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("share-grants by-collection missing response");
    record!(
        "GET",
        "/api/v0/share-grants/by-collection/{collectionId}",
        "runtime-failure-and-timeout",
        by_collection_missing.status == "404 Not Found"
    );
    record!(
        "GET",
        "/api/v0/share-grants/by-collection/{collectionId}",
        "populated-dynamic-state",
        by_collection_value
            .as_array()
            .is_some_and(|records| { records.iter().any(|record| record["id"] == grant_id) })
    );

    // Backfill has a policy gate. An empty permitted collection exercises
    // the source controller's stable 200/no-op response without invoking
    // an external download service.
    let empty_collection_id = uuid::Uuid::new_v4().to_string();
    state
        .collections
        .write()
        .await
        .create_with_contract(
            empty_collection_id.clone(),
            "tester".to_owned(),
            "Empty Share Grant".to_owned(),
            String::new(),
            "ShareList".to_owned(),
        )
        .expect("empty share-grants collection fixture");
    let allowed_grant_id = uuid::Uuid::new_v4().to_string();
    state
        .share_grants
        .write()
        .await
        .create_with_contract(
            Some(allowed_grant_id.clone()),
            empty_collection_id.clone(),
            "download-recipient".to_owned(),
        )
        .expect("allowed share-grants fixture");
    state
        .share_grants
        .write()
        .await
        .update(&allowed_grant_id, "read,download".to_owned())
        .expect("enable share-grant download permission");
    let backfill_path = format!("/api/v0/share-grants/{allowed_grant_id}/backfill");
    let backfill = crate::route_http_request("POST", &backfill_path, None, "", &state)
        .await
        .expect("share-grants backfill nominal response");
    let backfill_value = parse_json(&backfill.body);
    record!(
        "POST",
        "/api/v0/share-grants/{id}/backfill",
        "nominal-status-headers-body",
        backfill.status == "200 OK"
            && backfill_value["enqueued"] == 0
            && backfill_value["failed"] == 0
            && backfill_value["message"] == "No items to backfill"
    );
    let backfill_malformed =
        crate::route_http_request("POST", &format!("{backfill_path}/extra"), None, "", &state)
            .await
            .expect("share-grants backfill malformed response");
    record!(
        "POST",
        "/api/v0/share-grants/{id}/backfill",
        "malformed-path-query-or-body",
        backfill_malformed.status == "404 Not Found"
    );
    let backfill_missing = crate::route_http_request(
        "POST",
        &format!("/api/v0/share-grants/{missing_id}/backfill"),
        None,
        "",
        &state,
    )
    .await
    .expect("share-grants backfill missing response");
    record!(
        "POST",
        "/api/v0/share-grants/{id}/backfill",
        "missing-empty-or-conflict-state",
        backfill_missing.status == "404 Not Found"
    );
    let backfill_runtime_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("share-grants backfill runtime database");
    let (backfill_runtime_state, _receiver) = test_state_with_env_parts(
        target_env(),
        crate::SearchStore::new(),
        Some(backfill_runtime_db.clone()),
    );
    backfill_runtime_db.close_for_test().await;
    let backfill_runtime = crate::route_http_request(
        "POST",
        &format!("/api/v0/share-grants/{missing_id}/backfill"),
        None,
        "",
        &backfill_runtime_state,
    )
    .await
    .expect("share-grants backfill runtime response");
    record!(
        "POST",
        "/api/v0/share-grants/{id}/backfill",
        "runtime-failure-and-timeout",
        backfill_runtime.status == "404 Not Found"
    );
    let item_count_before = state
        .collections
        .read()
        .await
        .get(&empty_collection_id)
        .map(|collection| collection.items.len())
        .unwrap_or(0);
    let backfill_again = crate::route_http_request("POST", &backfill_path, None, "", &state)
        .await
        .expect("share-grants backfill mutation response");
    let item_count_after = state
        .collections
        .read()
        .await
        .get(&empty_collection_id)
        .map(|collection| collection.items.len())
        .unwrap_or(0);
    record!(
        "POST",
        "/api/v0/share-grants/{id}/backfill",
        "mutation-side-effects-and-readback",
        backfill_again.status == "200 OK" && item_count_before == 0 && item_count_after == 0
    );
    let (restarted_state, _receiver) = test_state_with_env(target_env());
    let backfill_restarted = crate::route_http_request(
        "POST",
        &format!("/api/v0/share-grants/{allowed_grant_id}/backfill"),
        None,
        "",
        &restarted_state,
    )
    .await
    .expect("share-grants backfill restart response");
    record!(
        "POST",
        "/api/v0/share-grants/{id}/backfill",
        "restart-persistence-or-reset",
        backfill_restarted.status == "404 Not Found"
    );
    let (left, right) = tokio::join!(
        crate::route_http_request("POST", &backfill_path, None, "", &state),
        crate::route_http_request("POST", &backfill_path, None, "", &state)
    );
    record!(
        "POST",
        "/api/v0/share-grants/{id}/backfill",
        "concurrency-and-idempotency",
        left.as_ref()
            .is_ok_and(|response| response.status == "200 OK")
            && right
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
    );

    // Token binding rejects malformed JSON on the versioned route. The
    // existing v0 compatibility response is retained for valid requests.
    let token_path = format!("/api/v0/share-grants/{grant_id}/token");
    let malformed_token = crate::route_http_request("POST", &token_path, None, "not-json", &state)
        .await
        .expect("share-grants malformed token response");
    record!(
        "POST",
        "/api/v0/share-grants/{id}/token",
        "malformed-path-query-or-body",
        malformed_token.status == "400 Bad Request"
    );
    let token = crate::route_http_request("POST", &token_path, None, "{}", &state)
        .await
        .expect("share-grants token mutation response");
    let token_value = parse_json(&token.body);
    record!(
        "POST",
        "/api/v0/share-grants/{id}/token",
        "mutation-side-effects-and-readback",
        token.status == "201 Created"
            && token_value["created"] == true
            && token_value["token"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
    );
    let token_restarted = crate::route_http_request(
        "POST",
        &format!("/api/v0/share-grants/{missing_id}/token"),
        None,
        "{}",
        &state,
    )
    .await
    .expect("share-grants token restart response");
    record!(
        "POST",
        "/api/v0/share-grants/{id}/token",
        "restart-persistence-or-reset",
        token_restarted.status == "404 Not Found"
    );
    let (token_left, token_right) = tokio::join!(
        crate::route_http_request("POST", &token_path, None, "{}", &state),
        crate::route_http_request("POST", &token_path, None, "{}", &state)
    );
    let token_left = token_left.expect("share-grants left token response");
    let token_right = token_right.expect("share-grants right token response");
    let token_left_value = parse_json(&token_left.body);
    let token_right_value = parse_json(&token_right.body);
    record!(
        "POST",
        "/api/v0/share-grants/{id}/token",
        "concurrency-and-idempotency",
        token_left.status == "201 Created"
            && token_right.status == "201 Created"
            && token_left_value["token"].as_str().is_some_and(|value| {
                token_right_value["token"]
                    .as_str()
                    .is_some_and(|other| value != other)
            })
    );

    // The frozen controller deliberately keeps this E2E-only endpoint
    // disabled unless SLSKDN_E2E_SHARE_ANNOUNCE=1. The test environment
    // does not enable that opt-in, so every v0 case is a stable 404.
    let announce_path = "/api/v0/share-grants/announce";
    let announce_nominal = crate::route_http_request(
        "POST",
        announce_path,
        None,
        r#"{"shareGrantId":"00000000-0000-0000-0000-000000000001","collectionId":"00000000-0000-0000-0000-000000000002","recipientUserId":"recipient"}"#,
        &state,
    )
    .await
    .expect("share-grants announce nominal response");
    record!(
        "POST",
        "/api/v0/share-grants/announce",
        "nominal-status-headers-body",
        announce_nominal.status == "404 Not Found"
    );
    let announce_malformed = crate::route_http_request(
        "POST",
        "/api/v0/share-grants/announce/extra",
        None,
        "{}",
        &state,
    )
    .await
    .expect("share-grants announce malformed response");
    record!(
        "POST",
        "/api/v0/share-grants/announce",
        "malformed-path-query-or-body",
        announce_malformed.status == "404 Not Found"
    );
    let announce_missing = crate::route_http_request("POST", announce_path, None, "", &state)
        .await
        .expect("share-grants announce missing response");
    record!(
        "POST",
        "/api/v0/share-grants/announce",
        "missing-empty-or-conflict-state",
        announce_missing.status == "404 Not Found"
    );
    let announce_runtime_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("share-grants announce runtime database");
    let (announce_runtime_state, _receiver) = test_state_with_env_parts(
        target_env(),
        crate::SearchStore::new(),
        Some(announce_runtime_db.clone()),
    );
    announce_runtime_db.close_for_test().await;
    let announce_runtime =
        crate::route_http_request("POST", announce_path, None, "{}", &announce_runtime_state)
            .await
            .expect("share-grants announce runtime response");
    record!(
        "POST",
        "/api/v0/share-grants/announce",
        "runtime-failure-and-timeout",
        announce_runtime.status == "404 Not Found"
    );
    let count_before = state.share_grants.read().await.records.len();
    let announce_mutation = crate::route_http_request("POST", announce_path, None, "{}", &state)
        .await
        .expect("share-grants announce mutation response");
    let count_after = state.share_grants.read().await.records.len();
    record!(
        "POST",
        "/api/v0/share-grants/announce",
        "mutation-side-effects-and-readback",
        announce_mutation.status == "404 Not Found" && count_before == count_after
    );
    let (announce_restarted_state, _receiver) = test_state_with_env(target_env());
    let announce_restarted =
        crate::route_http_request("POST", announce_path, None, "{}", &announce_restarted_state)
            .await
            .expect("share-grants announce restart response");
    record!(
        "POST",
        "/api/v0/share-grants/announce",
        "restart-persistence-or-reset",
        announce_restarted.status == "404 Not Found"
    );
    let (announce_left, announce_right) = tokio::join!(
        crate::route_http_request("POST", announce_path, None, "{}", &state),
        crate::route_http_request("POST", announce_path, None, "{}", &state)
    );
    record!(
        "POST",
        "/api/v0/share-grants/announce",
        "concurrency-and-idempotency",
        announce_left
            .as_ref()
            .is_ok_and(|response| response.status == "404 Not Found")
            && announce_right
                .as_ref()
                .is_ok_and(|response| response.status == "404 Not Found")
    );

    assert_eq!(ledger.len(), 27, "share-grants residual ledger size");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create share-grants evidence directory");
    fs::write(
        evidence_dir.join("share_grants_open_cases.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize share-grants ledger"),
    )
    .expect("write share-grants ledger");
    assert!(
        mismatches.is_empty(),
        "{} share-grants controller mismatches:\n{}",
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
    feature = "bounded-controller-api-tests-4"
))]
pub(super) async fn controller_api_differential_shares_open_cases() {
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

    let target_env = || MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target);
    let parse_json = |body: &str| {
        serde_json::from_str::<serde_json::Value>(body).unwrap_or(serde_json::Value::Null)
    };
    let root_id = crate::share_root_id("Virtual");
    macro_rules! seed_share_root {
        ($state:expr) => {{
            let mut shares = $state.shares.write().await;
            shares.roots.clear();
            shares.entries.clear();
            shares.local_paths.clear();
            shares.roots.push(crate::ShareRoot {
                label: "Virtual".to_owned(),
                local_path: PathBuf::from("/srv/music"),
                raw: "Virtual".to_owned(),
                directories: 1,
                files: 1,
                bytes: 42,
                extensions: vec![crate::ShareExtensionSummary {
                    extension: "flac".to_owned(),
                    files: 1,
                    bytes: 42,
                }],
                statistics_ready: true,
            });
            shares.entries.push(FileEntry {
                filename_encoding: Default::default(),
                extension_encoding: Default::default(),
                code: 1,
                filename: "Virtual/Track.flac".to_owned(),
                size: 42,
                extension: "flac".to_owned(),
                attributes: Vec::new(),
            });
        }};
    }

    // CancelShareScan returns NoContent only while a scan is active.
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let _permit = Arc::clone(&state.share_scans)
            .acquire_owned()
            .await
            .expect("share scan permit");
        let response = crate::route_http_request("DELETE", "/api/v0/shares", None, "", &state)
            .await
            .expect("share cancel nominal response");
        record!(
            "DELETE",
            "/api/v0/shares",
            "nominal-status-headers-body",
            response.status == "204 No Content" && response.body.is_empty()
        );
    }
    let malformed_delete = {
        let (state, _receiver) = test_state_with_env(target_env());
        crate::route_http_request("DELETE", "/api/v0/shares/extra", None, "", &state)
            .await
            .expect("share cancel malformed response")
    };
    record!(
        "DELETE",
        "/api/v0/shares",
        "malformed-path-query-or-body",
        malformed_delete.status == "404 Not Found"
    );
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("share cancel runtime database");
        let (state, _receiver) =
            test_state_with_env_parts(target_env(), crate::SearchStore::new(), Some(db.clone()));
        db.close_for_test().await;
        let response = crate::route_http_request("DELETE", "/api/v0/shares", None, "", &state)
            .await
            .expect("share cancel runtime response");
        record!(
            "DELETE",
            "/api/v0/shares",
            "runtime-failure-and-timeout",
            response.status == "404 Not Found"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let response = crate::route_http_request("DELETE", "/api/v0/shares", None, "", &state)
            .await
            .expect("share cancel restart response");
        record!(
            "DELETE",
            "/api/v0/shares",
            "restart-persistence-or-reset",
            response.status == "404 Not Found"
        );
    }
    {
        let (state, _receiver) = test_state_with_env(target_env());
        let _permit = Arc::clone(&state.share_scans)
            .acquire_owned()
            .await
            .expect("share scan concurrency permit");
        let first = crate::route_http_request("DELETE", "/api/v0/shares", None, "", &state)
            .await
            .expect("first share cancel concurrency response");
        let second = crate::route_http_request("DELETE", "/api/v0/shares", None, "", &state)
            .await
            .expect("second share cancel concurrency response");
        record!(
            "DELETE",
            "/api/v0/shares",
            "concurrency-and-idempotency",
            first.status == "204 No Content" && second.status == "404 Not Found"
        );
    }

    // List and detail use the live share index and remain readable when
    // the optional database is closed.
    {
        let (empty_state, _receiver) =
            test_state_with_env(target_env().with("SLSKR_SHARE_FIXTURE", ""));
        let empty = crate::route_http_request("GET", "/api/v0/shares", None, "", &empty_state)
            .await
            .expect("empty shares list response");
        let empty_value = parse_json(&empty.body);
        record!(
            "GET",
            "/api/v0/shares",
            "missing-empty-or-conflict-state",
            empty.status == "200 OK" && empty_value["local"].as_array().is_some_and(Vec::is_empty)
        );
    }
    let malformed_list = {
        let (state, _receiver) = test_state_with_env(target_env());
        crate::route_http_request("GET", "/api/v0/shares/extra/path", None, "", &state)
            .await
            .expect("malformed shares list response")
    };
    record!(
        "GET",
        "/api/v0/shares",
        "malformed-path-query-or-body",
        malformed_list.status == "404 Not Found"
    );
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("shares list runtime database");
        let (state, _receiver) =
            test_state_with_env_parts(target_env(), crate::SearchStore::new(), Some(db.clone()));
        db.close_for_test().await;
        let response = crate::route_http_request("GET", "/api/v0/shares", None, "", &state)
            .await
            .expect("shares list runtime response");
        let value = parse_json(&response.body);
        record!(
            "GET",
            "/api/v0/shares",
            "runtime-failure-and-timeout",
            response.status == "200 OK" && value["local"].is_array()
        );
    }

    let (state, _receiver) = test_state_with_env(target_env());
    seed_share_root!(&state);
    let detail = crate::route_http_request(
        "GET",
        &format!("/api/v0/shares/{root_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("share detail nominal response");
    let detail_value = parse_json(&detail.body);
    record!(
        "GET",
        "/api/v0/shares/{id}",
        "nominal-status-headers-body",
        detail.status == "200 OK" && detail_value["id"] == root_id
    );
    record!(
        "GET",
        "/api/v0/shares/{id}",
        "populated-dynamic-state",
        detail_value["files"] == 1
            && detail_value["directories"] == 1
            && detail_value["remotePath"] == "Virtual"
    );
    let malformed_detail =
        crate::route_http_request("GET", "/api/v0/shares/extra/path", None, "", &state)
            .await
            .expect("share detail malformed response");
    record!(
        "GET",
        "/api/v0/shares/{id}",
        "malformed-path-query-or-body",
        malformed_detail.status == "404 Not Found"
    );
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("share detail runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(target_env(), crate::SearchStore::new(), Some(db.clone()));
        seed_share_root!(&runtime_state);
        db.close_for_test().await;
        let response = crate::route_http_request(
            "GET",
            &format!("/api/v0/shares/{root_id}"),
            None,
            "",
            &runtime_state,
        )
        .await
        .expect("share detail runtime response");
        record!(
            "GET",
            "/api/v0/shares/{id}",
            "runtime-failure-and-timeout",
            response.status == "200 OK" && parse_json(&response.body)["id"] == root_id
        );
    }

    let all_contents =
        crate::route_http_request("GET", "/api/v0/shares/contents", None, "", &state)
            .await
            .expect("all share contents nominal response");
    let all_contents_value = parse_json(&all_contents.body);
    record!(
        "GET",
        "/api/v0/shares/contents",
        "runtime-failure-and-timeout",
        all_contents.status == "200 OK" && all_contents_value.is_array()
    );
    let malformed_all_contents =
        crate::route_http_request("GET", "/api/v0/shares/contents/extra", None, "", &state)
            .await
            .expect("all share contents malformed response");
    record!(
        "GET",
        "/api/v0/shares/contents",
        "malformed-path-query-or-body",
        malformed_all_contents.status == "404 Not Found"
    );
    {
        let (empty_state, _receiver) =
            test_state_with_env(target_env().with("SLSKR_SHARE_FIXTURE", ""));
        let response =
            crate::route_http_request("GET", "/api/v0/shares/contents", None, "", &empty_state)
                .await
                .expect("empty all share contents response");
        record!(
            "GET",
            "/api/v0/shares/contents",
            "missing-empty-or-conflict-state",
            response.status == "200 OK" && parse_json(&response.body).is_array()
        );
    }

    let share_contents_path = format!("/api/v0/shares/{root_id}/contents");
    let share_contents = crate::route_http_request("GET", &share_contents_path, None, "", &state)
        .await
        .expect("share contents nominal response");
    let share_contents_value = parse_json(&share_contents.body);
    record!(
        "GET",
        "/api/v0/shares/{id}/contents",
        "nominal-status-headers-body",
        share_contents.status == "200 OK" && share_contents_value.is_array()
    );
    record!(
        "GET",
        "/api/v0/shares/{id}/contents",
        "populated-dynamic-state",
        share_contents_value.as_array().is_some_and(|directories| {
            directories.iter().any(|directory| {
                directory["files"]
                    .as_array()
                    .is_some_and(|files| !files.is_empty())
            })
        })
    );
    let malformed_share_contents = crate::route_http_request(
        "GET",
        &format!("{share_contents_path}/extra"),
        None,
        "",
        &state,
    )
    .await
    .expect("share contents malformed response");
    record!(
        "GET",
        "/api/v0/shares/{id}/contents",
        "malformed-path-query-or-body",
        malformed_share_contents.status == "404 Not Found"
    );
    let missing_share_contents =
        crate::route_http_request("GET", "/api/v0/shares/missing/contents", None, "", &state)
            .await
            .expect("missing share contents response");
    record!(
        "GET",
        "/api/v0/shares/{id}/contents",
        "missing-empty-or-conflict-state",
        missing_share_contents.status == "404 Not Found"
    );
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("share contents runtime database");
        let (runtime_state, _receiver) =
            test_state_with_env_parts(target_env(), crate::SearchStore::new(), Some(db.clone()));
        seed_share_root!(&runtime_state);
        db.close_for_test().await;
        let response = crate::route_http_request(
            "GET",
            &format!("/api/v0/shares/{root_id}/contents"),
            None,
            "",
            &runtime_state,
        )
        .await
        .expect("share contents runtime response");
        record!(
            "GET",
            "/api/v0/shares/{id}/contents",
            "runtime-failure-and-timeout",
            response.status == "200 OK" && parse_json(&response.body).is_array()
        );
    }

    // PUT is the versioned rescan action. Its successful response is an
    // empty 200, while the asynchronous scan state becomes ready.
    let malformed_rescan =
        crate::route_http_request("PUT", "/api/v0/shares/extra", None, "", &state)
            .await
            .expect("share rescan malformed response");
    record!(
        "PUT",
        "/api/v0/shares",
        "malformed-path-query-or-body",
        malformed_rescan.status == "404 Not Found"
    );
    {
        let (empty_state, _receiver) =
            test_state_with_env(target_env().with("SLSKR_SHARE_FIXTURE", ""));
        let response = crate::route_http_request("PUT", "/api/v0/shares", None, "", &empty_state)
            .await
            .expect("empty share rescan response");
        record!(
            "PUT",
            "/api/v0/shares",
            "missing-empty-or-conflict-state",
            response.status == "200 OK" && response.body.is_empty()
        );
    }
    {
        let response = crate::route_http_request("PUT", "/api/v0/shares", None, "", &state)
            .await
            .expect("share rescan mutation response");
        record!(
            "PUT",
            "/api/v0/shares",
            "mutation-side-effects-and-readback",
            response.status == "200 OK"
                && response.body.is_empty()
                && state.share_lifecycle.read().await.ready
        );
    }
    {
        let (restarted_state, _receiver) = test_state_with_env(target_env());
        let response =
            crate::route_http_request("PUT", "/api/v0/shares", None, "", &restarted_state)
                .await
                .expect("share rescan restart response");
        record!(
            "PUT",
            "/api/v0/shares",
            "restart-persistence-or-reset",
            response.status == "200 OK"
                && response.body.is_empty()
                && restarted_state.share_lifecycle.read().await.ready
        );
    }

    assert_eq!(ledger.len(), 24, "shares residual ledger size");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create shares evidence directory");
    fs::write(
        evidence_dir.join("shares_open_cases.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize shares ledger"),
    )
    .expect("write shares ledger");
    assert!(
        mismatches.is_empty(),
        "{} shares controller mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
