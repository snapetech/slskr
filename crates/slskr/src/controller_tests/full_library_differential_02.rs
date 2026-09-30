//! Controller full library differential 02 ownership.

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
pub(super) async fn controller_api_differential_collections_persistence_and_concurrency() {
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

    // Collection creation survives a state rebuild from the real rows.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("collection create restart database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let created = crate::route_http_request(
            "POST",
            "/api/v0/collections",
            None,
            r#"{"title":"Restart Collection","description":"persisted"}"#,
            &state,
        )
        .await
        .expect("create collection for restart");
        let created_json = serde_json::from_str::<serde_json::Value>(&created.body).unwrap();
        let collection_id = created_json["id"].as_str().unwrap().to_owned();
        let persisted = db.list_collections(10, 0).await.unwrap();
        let (restarted_state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        *restarted_state.collections.write().await =
            crate::CollectionStore::from_persisted(persisted.clone(), Vec::new());
        let listed =
            crate::route_http_request("GET", "/api/v0/collections", None, "", &restarted_state)
                .await
                .unwrap();
        let listed_json = serde_json::from_str::<serde_json::Value>(&listed.body).unwrap();
        record!(
            "POST",
            "/api/v0/collections",
            "restart-persistence-or-reset",
            created.status == "201 Created"
                && persisted.len() == 1
                && persisted[0].id == collection_id
                && listed.status == "200 OK"
                && listed_json.as_array().is_some_and(|collections| {
                    collections
                        .iter()
                        .any(|collection| collection["id"] == collection_id)
                })
        );
    }

    // Distinct collection creates through the same route are all durable.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("collection create concurrency database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let bodies: Vec<String> = (0..4)
            .map(|index| format!(r#"{{"title":"Concurrent Collection {index}"}}"#))
            .collect();
        let responses = futures_util::future::join_all(bodies.iter().map(|body| {
            crate::route_http_request("POST", "/api/v0/collections", None, body, &state)
        }))
        .await;
        let persisted = db.list_collections(10, 0).await.unwrap_or_default();
        let expected: std::collections::BTreeSet<String> = (0..4)
            .map(|index| format!("Concurrent Collection {index}"))
            .collect();
        let persisted_names: std::collections::BTreeSet<String> =
            persisted.iter().map(|record| record.name.clone()).collect();
        record!(
            "POST",
            "/api/v0/collections",
            "concurrency-and-idempotency",
            responses.iter().all(|response| {
                response
                    .as_ref()
                    .is_ok_and(|response| response.status == "201 Created")
            }) && persisted.len() == 4
                && persisted_names == expected
        );
    }

    // An item create is present after collection and item rehydration.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("collection item create restart database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let collection = crate::route_http_request(
            "POST",
            "/api/v0/collections",
            None,
            r#"{"title":"Item Restart Collection"}"#,
            &state,
        )
        .await
        .unwrap();
        let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body).unwrap()
            ["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let item = crate::route_http_request(
            "POST",
            &format!("/api/v0/collections/{collection_id}/items"),
            None,
            r#"{"contentId":"content:music:recording:restart-item","title":"Restart Item","artist":"Artist"}"#,
            &state,
        )
        .await
        .unwrap();
        let item_id = serde_json::from_str::<serde_json::Value>(&item.body).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let persisted_collections = db.list_collections(10, 0).await.unwrap();
        let persisted_items = db.list_collection_items(10, 0).await.unwrap();
        let (restarted_state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        *restarted_state.collections.write().await =
            crate::CollectionStore::from_persisted(persisted_collections, persisted_items.clone());
        let listed = crate::route_http_request(
            "GET",
            &format!("/api/v0/collections/{collection_id}/items"),
            None,
            "",
            &restarted_state,
        )
        .await
        .unwrap();
        let listed_json = serde_json::from_str::<serde_json::Value>(&listed.body).unwrap();
        record!(
            "POST",
            "/api/v0/collections/{id}/items",
            "restart-persistence-or-reset",
            collection.status == "201 Created"
                && item.status == "201 Created"
                && persisted_items.len() == 1
                && persisted_items[0].id == item_id
                && listed.status == "200 OK"
                && listed_json
                    .as_array()
                    .is_some_and(|items| { items.iter().any(|item| item["id"] == item_id) })
        );
    }

    // Distinct item creates through one real collection are all durable.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("collection item create concurrency database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let collection = crate::route_http_request(
            "POST",
            "/api/v0/collections",
            None,
            r#"{"title":"Item Concurrent Collection"}"#,
            &state,
        )
        .await
        .unwrap();
        let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body).unwrap()
            ["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let bodies: Vec<String> = (0..4)
            .map(|index| {
                format!(
                    r#"{{"contentId":"content:music:recording:concurrent-item-{index}","title":"Concurrent Item {index}"}}"#
                )
            })
            .collect();
        let item_path = format!("/api/v0/collections/{collection_id}/items");
        let responses = futures_util::future::join_all(
            bodies
                .iter()
                .map(|body| crate::route_http_request("POST", &item_path, None, body, &state)),
        )
        .await;
        let persisted = db.list_collection_items(10, 0).await.unwrap_or_default();
        let expected: std::collections::BTreeSet<String> = (0..4)
            .map(|index| format!("content:music:recording:concurrent-item-{index}"))
            .collect();
        let persisted_ids: std::collections::BTreeSet<String> = persisted
            .iter()
            .map(|record| record.content_id.clone())
            .collect();
        record!(
            "POST",
            "/api/v0/collections/{id}/items",
            "concurrency-and-idempotency",
            responses.iter().all(|response| {
                response
                    .as_ref()
                    .is_ok_and(|response| response.status == "201 Created")
            }) && persisted.len() == 4
                && persisted_ids == expected
        );
    }

    // A collection update survives a real persisted-state rebuild.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("collection update restart database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let collection = crate::route_http_request(
            "POST",
            "/api/v0/collections",
            None,
            r#"{"title":"Update Restart Before"}"#,
            &state,
        )
        .await
        .unwrap();
        let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body).unwrap()
            ["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let updated = crate::route_http_request(
            "PUT",
            &format!("/api/v0/collections/{collection_id}"),
            None,
            r#"{"title":"Update Restart After"}"#,
            &state,
        )
        .await
        .unwrap();
        let persisted = db.list_collections(10, 0).await.unwrap();
        let (restarted_state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        *restarted_state.collections.write().await =
            crate::CollectionStore::from_persisted(persisted.clone(), Vec::new());
        let fetched = crate::route_http_request(
            "GET",
            &format!("/api/v0/collections/{collection_id}"),
            None,
            "",
            &restarted_state,
        )
        .await
        .unwrap();
        let fetched_json = serde_json::from_str::<serde_json::Value>(&fetched.body).unwrap();
        record!(
            "PUT",
            "/api/v0/collections/{id}",
            "restart-persistence-or-reset",
            updated.status == "200 OK"
                && persisted.len() == 1
                && persisted[0].name == "Update Restart After"
                && fetched.status == "200 OK"
                && fetched_json["title"] == "Update Restart After"
        );
    }

    // Concurrent collection updates retain each writer's own value.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("collection update concurrency database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let mut collection_ids = Vec::new();
        for index in 0..4 {
            let created = crate::route_http_request(
                "POST",
                "/api/v0/collections",
                None,
                &format!(r#"{{"title":"Update Concurrent Before {index}"}}"#),
                &state,
            )
            .await
            .unwrap();
            collection_ids.push(
                serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
                    .as_str()
                    .unwrap()
                    .to_owned(),
            );
        }
        let responses = futures_util::future::join_all(collection_ids.iter().enumerate().map(
            |(index, collection_id)| {
                let path = format!("/api/v0/collections/{collection_id}");
                let body = format!(r#"{{"title":"Update Concurrent After {index}"}}"#);
                let state = Arc::clone(&state);
                async move { crate::route_http_request("PUT", &path, None, &body, &state).await }
            },
        ))
        .await;
        let persisted = db.list_collections(10, 0).await.unwrap_or_default();
        let expected: std::collections::BTreeSet<String> = (0..4)
            .map(|index| format!("Update Concurrent After {index}"))
            .collect();
        let persisted_names: std::collections::BTreeSet<String> =
            persisted.iter().map(|record| record.name.clone()).collect();
        record!(
            "PUT",
            "/api/v0/collections/{id}",
            "concurrency-and-idempotency",
            responses.iter().all(|response| {
                response
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
            }) && persisted.len() == 4
                && persisted_names == expected
        );
    }

    // An item update survives collection/item rehydration.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("collection item update restart database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let collection = crate::route_http_request(
            "POST",
            "/api/v0/collections",
            None,
            r#"{"title":"Item Update Restart Collection"}"#,
            &state,
        )
        .await
        .unwrap();
        let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body).unwrap()
            ["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let item = crate::route_http_request(
            "POST",
            &format!("/api/v0/collections/{collection_id}/items"),
            None,
            r#"{"contentId":"content:music:recording:update-item","title":"Before"}"#,
            &state,
        )
        .await
        .unwrap();
        let item_id = serde_json::from_str::<serde_json::Value>(&item.body).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let updated = crate::route_http_request(
            "PUT",
            &format!("/api/v0/collections/{collection_id}/items/{item_id}"),
            None,
            r#"{"title":"After"}"#,
            &state,
        )
        .await
        .unwrap();
        let persisted_collections = db.list_collections(10, 0).await.unwrap();
        let persisted_items = db.list_collection_items(10, 0).await.unwrap();
        let (restarted_state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        *restarted_state.collections.write().await =
            crate::CollectionStore::from_persisted(persisted_collections, persisted_items);
        let listed = crate::route_http_request(
            "GET",
            &format!("/api/v0/collections/{collection_id}/items"),
            None,
            "",
            &restarted_state,
        )
        .await
        .unwrap();
        let listed_json = serde_json::from_str::<serde_json::Value>(&listed.body).unwrap();
        record!(
            "PUT",
            "/api/v0/collections/{id}/items/{itemId}",
            "restart-persistence-or-reset",
            updated.status == "200 OK"
                && listed.status == "200 OK"
                && listed_json.as_array().is_some_and(|items| {
                    items
                        .iter()
                        .any(|item| item["id"] == item_id && item["title"] == "After")
                })
        );
    }

    // Concurrent item updates retain every writer's own value.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("collection item update concurrency database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let collection = crate::route_http_request(
            "POST",
            "/api/v0/collections",
            None,
            r#"{"title":"Item Update Concurrent Collection"}"#,
            &state,
        )
        .await
        .unwrap();
        let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body).unwrap()
            ["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let mut item_ids = Vec::new();
        for index in 0..4 {
            let item = crate::route_http_request(
                "POST",
                &format!("/api/v0/collections/{collection_id}/items"),
                None,
                &format!(
                    r#"{{"contentId":"content:music:recording:update-concurrent-{index}","title":"Before {index}"}}"#
                ),
                &state,
            )
            .await
            .unwrap();
            item_ids.push(
                serde_json::from_str::<serde_json::Value>(&item.body).unwrap()["id"]
                    .as_str()
                    .unwrap()
                    .to_owned(),
            );
        }
        let responses =
            futures_util::future::join_all(item_ids.iter().enumerate().map(|(index, item_id)| {
                let path = format!("/api/v0/collections/{collection_id}/items/{item_id}");
                let body = format!(r#"{{"title":"After {index}"}}"#);
                let state = Arc::clone(&state);
                async move { crate::route_http_request("PUT", &path, None, &body, &state).await }
            }))
            .await;
        let persisted = db.list_collection_items(10, 0).await.unwrap_or_default();
        let expected: std::collections::BTreeSet<String> =
            (0..4).map(|index| format!("After {index}")).collect();
        let persisted_titles: std::collections::BTreeSet<String> = persisted
            .iter()
            .map(|record| record.title.clone())
            .collect();
        record!(
            "PUT",
            "/api/v0/collections/{id}/items/{itemId}",
            "concurrency-and-idempotency",
            responses.iter().all(|response| {
                response
                    .as_ref()
                    .is_ok_and(|response| response.status == "200 OK")
            }) && persisted.len() == 4
                && persisted_titles == expected
        );
    }

    // Collection deletion removes its persisted record and remains deleted after rebuild.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("collection delete restart database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let created = crate::route_http_request(
            "POST",
            "/api/v0/collections",
            None,
            r#"{"title":"Delete Restart Collection"}"#,
            &state,
        )
        .await
        .unwrap();
        let collection_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let deleted = crate::route_http_request(
            "DELETE",
            &format!("/api/v0/collections/{collection_id}"),
            None,
            "",
            &state,
        )
        .await
        .unwrap();
        let persisted_collections = db.list_collections(10, 0).await.unwrap();
        let persisted_items = db.list_collection_items(10, 0).await.unwrap();
        let (restarted_state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        *restarted_state.collections.write().await =
            crate::CollectionStore::from_persisted(persisted_collections.clone(), persisted_items);
        let fetched = crate::route_http_request(
            "GET",
            &format!("/api/v0/collections/{collection_id}"),
            None,
            "",
            &restarted_state,
        )
        .await
        .unwrap();
        record!(
            "DELETE",
            "/api/v0/collections/{id}",
            "restart-persistence-or-reset",
            created.status == "201 Created"
                && deleted.status == "204 No Content"
                && persisted_collections.is_empty()
                && fetched.status == "404 Not Found"
        );
    }

    // Distinct collection deletions complete concurrently and clear persisted rows.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("collection delete concurrency database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let mut collection_ids = Vec::new();
        for index in 0..4 {
            let created = crate::route_http_request(
                "POST",
                "/api/v0/collections",
                None,
                &format!(r#"{{"title":"Delete Concurrent Collection {index}"}}"#),
                &state,
            )
            .await
            .unwrap();
            collection_ids.push(
                serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
                    .as_str()
                    .unwrap()
                    .to_owned(),
            );
        }
        let responses = futures_util::future::join_all(collection_ids.iter().map(|id| {
            let path = format!("/api/v0/collections/{id}");
            let state = Arc::clone(&state);
            async move { crate::route_http_request("DELETE", &path, None, "", &state).await }
        }))
        .await;
        let persisted = db.list_collections(10, 0).await.unwrap_or_default();
        let pass = responses.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "204 No Content")
        }) && persisted.is_empty();
        record!(
            "DELETE",
            "/api/v0/collections/{id}",
            "concurrency-and-idempotency",
            pass
        );
    }

    // Item deletion is durable and leaves its parent collection after rebuild.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("collection item delete restart database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let collection = crate::route_http_request(
            "POST",
            "/api/v0/collections",
            None,
            r#"{"title":"Item Delete Restart Collection"}"#,
            &state,
        )
        .await
        .unwrap();
        let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body).unwrap()
            ["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let item = crate::route_http_request(
            "POST",
            &format!("/api/v0/collections/{collection_id}/items"),
            None,
            r#"{"contentId":"content:music:recording:delete-restart-item","title":"Delete Me"}"#,
            &state,
        )
        .await
        .unwrap();
        let item_id = serde_json::from_str::<serde_json::Value>(&item.body).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let deleted = crate::route_http_request(
            "DELETE",
            &format!("/api/v0/collections/{collection_id}/items/{item_id}"),
            None,
            "",
            &state,
        )
        .await
        .unwrap();
        let persisted_collections = db.list_collections(10, 0).await.unwrap();
        let persisted_items = db.list_collection_items(10, 0).await.unwrap();
        let (restarted_state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        *restarted_state.collections.write().await =
            crate::CollectionStore::from_persisted(persisted_collections, persisted_items.clone());
        let listed = crate::route_http_request(
            "GET",
            &format!("/api/v0/collections/{collection_id}/items"),
            None,
            "",
            &restarted_state,
        )
        .await
        .unwrap();
        record!(
            "DELETE",
            "/api/v0/collections/{id}/items/{itemId}",
            "restart-persistence-or-reset",
            collection.status == "201 Created"
                && item.status == "201 Created"
                && deleted.status == "204 No Content"
                && persisted_items.is_empty()
                && listed.status == "200 OK"
                && listed.body == "[]"
        );
    }

    // Distinct item deletions complete concurrently without leaving rows.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("collection item delete concurrency database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let collection = crate::route_http_request(
            "POST",
            "/api/v0/collections",
            None,
            r#"{"title":"Item Delete Concurrent Collection"}"#,
            &state,
        )
        .await
        .unwrap();
        let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body).unwrap()
            ["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let mut item_ids = Vec::new();
        for index in 0..4 {
            let item = crate::route_http_request(
                "POST",
                &format!("/api/v0/collections/{collection_id}/items"),
                None,
                &format!(
                    r#"{{"contentId":"content:music:recording:delete-concurrent-{index}","title":"Delete {index}"}}"#
                ),
                &state,
            )
            .await
            .unwrap();
            item_ids.push(
                serde_json::from_str::<serde_json::Value>(&item.body).unwrap()["id"]
                    .as_str()
                    .unwrap()
                    .to_owned(),
            );
        }
        let responses = futures_util::future::join_all(item_ids.iter().map(|item_id| {
            let path = format!("/api/v0/collections/{collection_id}/items/{item_id}");
            let state = Arc::clone(&state);
            async move { crate::route_http_request("DELETE", &path, None, "", &state).await }
        }))
        .await;
        let persisted = db.list_collection_items(10, 0).await.unwrap_or_default();
        let pass = responses.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "204 No Content")
        }) && persisted.is_empty();
        record!(
            "DELETE",
            "/api/v0/collections/{id}/items/{itemId}",
            "concurrency-and-idempotency",
            pass
        );
    }

    // Reordering rolls back the in-memory order when persistence fails.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("collection reorder runtime database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let collection = crate::route_http_request(
            "POST",
            "/api/v0/collections",
            None,
            r#"{"title":"Reorder Runtime Collection"}"#,
            &state,
        )
        .await
        .unwrap();
        let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body).unwrap()
            ["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let mut item_ids = Vec::new();
        for index in 0..2 {
            let item = crate::route_http_request(
                "POST",
                &format!("/api/v0/collections/{collection_id}/items"),
                None,
                &format!(
                    r#"{{"contentId":"content:music:recording:reorder-runtime-{index}","title":"Runtime {index}"}}"#
                ),
                &state,
            )
            .await
            .unwrap();
            item_ids.push(
                serde_json::from_str::<serde_json::Value>(&item.body).unwrap()["id"]
                    .as_str()
                    .unwrap()
                    .to_owned(),
            );
        }
        db.close_for_test().await;
        let response = crate::route_http_request(
            "POST",
            &format!("/api/v0/collections/{collection_id}/items/reorder"),
            None,
            &format!(r#"{{"itemIds":["{}","{}"]}}"#, item_ids[1], item_ids[0]),
            &state,
        )
        .await
        .unwrap();
        let order = state
            .collections
            .read()
            .await
            .get(&collection_id)
            .map(|record| {
                record
                    .items
                    .iter()
                    .map(|item| item.id.clone())
                    .collect::<Vec<_>>()
            });
        let pass = response.status == "503 Service Unavailable"
            && order == Some(vec![item_ids[0].clone(), item_ids[1].clone()]);
        record!(
            "POST",
            "/api/v0/collections/{id}/items/reorder",
            "runtime-failure-and-timeout",
            pass
        );
    }

    // Reordering survives a real collection/item state rebuild.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("collection reorder restart database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let collection = crate::route_http_request(
            "POST",
            "/api/v0/collections",
            None,
            r#"{"title":"Reorder Restart Collection"}"#,
            &state,
        )
        .await
        .unwrap();
        let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body).unwrap()
            ["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let mut item_ids = Vec::new();
        for index in 0..2 {
            let item = crate::route_http_request(
                "POST",
                &format!("/api/v0/collections/{collection_id}/items"),
                None,
                &format!(
                    r#"{{"contentId":"content:music:recording:reorder-restart-{index}","title":"Restart {index}"}}"#
                ),
                &state,
            )
            .await
            .unwrap();
            item_ids.push(
                serde_json::from_str::<serde_json::Value>(&item.body).unwrap()["id"]
                    .as_str()
                    .unwrap()
                    .to_owned(),
            );
        }
        let reordered = crate::route_http_request(
            "POST",
            &format!("/api/v0/collections/{collection_id}/items/reorder"),
            None,
            &format!(r#"{{"itemIds":["{}","{}"]}}"#, item_ids[1], item_ids[0]),
            &state,
        )
        .await
        .unwrap();
        let persisted_collections = db.list_collections(10, 0).await.unwrap();
        let persisted_items = db.list_collection_items(10, 0).await.unwrap();
        let (restarted_state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        *restarted_state.collections.write().await =
            crate::CollectionStore::from_persisted(persisted_collections, persisted_items.clone());
        let listed = crate::route_http_request(
            "GET",
            &format!("/api/v0/collections/{collection_id}/items"),
            None,
            "",
            &restarted_state,
        )
        .await
        .unwrap();
        let listed_json = serde_json::from_str::<serde_json::Value>(&listed.body).unwrap();
        record!(
            "POST",
            "/api/v0/collections/{id}/items/reorder",
            "restart-persistence-or-reset",
            reordered.status == "204 No Content"
                && persisted_items.len() == 2
                && persisted_items
                    .iter()
                    .find(|item| item.id == item_ids[1])
                    .is_some_and(|item| item.position == 0)
                && listed.status == "200 OK"
                && listed_json.as_array().is_some_and(|items| {
                    items.len() == 2
                        && items[0]["id"] == item_ids[1]
                        && items[1]["id"] == item_ids[0]
                })
        );
    }

    // Reorders on distinct collections complete concurrently and persist each order.
    {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("collection reorder concurrency database");
        let (state, _receiver) = test_state_with_env_parts(
            persistence_env(),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let mut jobs = Vec::new();
        for index in 0..4 {
            let collection = crate::route_http_request(
                "POST",
                "/api/v0/collections",
                None,
                &format!(r#"{{"title":"Reorder Concurrent Collection {index}"}}"#),
                &state,
            )
            .await
            .unwrap();
            let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body)
                .unwrap()["id"]
                .as_str()
                .unwrap()
                .to_owned();
            let mut item_ids = Vec::new();
            for item_index in 0..2 {
                let item = crate::route_http_request(
                    "POST",
                    &format!("/api/v0/collections/{collection_id}/items"),
                    None,
                    &format!(
                        r#"{{"contentId":"content:music:recording:reorder-concurrent-{index}-{item_index}","title":"Concurrent {index}-{item_index}"}}"#
                    ),
                    &state,
                )
                .await
                .unwrap();
                item_ids.push(
                    serde_json::from_str::<serde_json::Value>(&item.body).unwrap()["id"]
                        .as_str()
                        .unwrap()
                        .to_owned(),
                );
            }
            jobs.push((collection_id, item_ids[0].clone(), item_ids[1].clone()));
        }
        let responses = futures_util::future::join_all(jobs.iter().map(
            |(collection_id, first_item_id, second_item_id)| {
                let path = format!("/api/v0/collections/{collection_id}/items/reorder");
                let body = format!(r#"{{"itemIds":["{second_item_id}","{first_item_id}"]}}"#);
                let state = Arc::clone(&state);
                async move { crate::route_http_request("POST", &path, None, &body, &state).await }
            },
        ))
        .await;
        let persisted = db.list_collection_items(32, 0).await.unwrap_or_default();
        let orders_ok = jobs.iter().all(|(_, first_item_id, second_item_id)| {
            persisted
                .iter()
                .find(|item| item.id == *first_item_id)
                .is_some_and(|item| item.position == 1)
                && persisted
                    .iter()
                    .find(|item| item.id == *second_item_id)
                    .is_some_and(|item| item.position == 0)
        });
        let pass = responses.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "204 No Content")
        }) && persisted.len() == 8
            && orders_ok;
        record!(
            "POST",
            "/api/v0/collections/{id}/items/reorder",
            "concurrency-and-idempotency",
            pass
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("collections_persistence_and_concurrency.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        mismatches.is_empty(),
        "{} controller-api collection persistence mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting the Collections routes' real
/// per-caller ownership enforcement, independently re-derived from
/// `collections_are_scoped_to_the_real_authenticated_caller_identity`'s
/// checks that a collection created by one authenticated identity is
/// invisible to, and immutable by, a different one. Credits
/// `missing-empty-or-conflict-state` (cross-user 404) for the mutating
/// routes already covered by other cases, plus first-time coverage of
/// the single-collection GET and list-collections GET. slskdN-only
/// (confirmed against the frozen registry: slskd has no Collections
/// routes at all).
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
pub(super) async fn controller_api_differential_collections_ownership_scoping() {
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($target:expr, $method:expr, $route:expr, $case:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!("{} {} {} [{}]", $target, $method, $route, $case));
            }
            ledger.push(serde_json::json!({
                "target": $target,
                "method": $method,
                "route": $route,
                "case": $case,
                "pass": $pass,
            }));
        };
    }

    for target in ["slskdn"] {
        let keys = serde_json::json!({
            "alice": {"key": "alice-key-0123456789", "role": "readwrite", "cidr": ""},
            "bob": {"key": "bob-key-00123456789ab", "role": "readwrite", "cidr": ""},
        });
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_AUTH_DISABLED", "false")
                .with("SLSKD_API_KEYS_JSON", &keys.to_string()),
        );
        let alice = Some("ApiKey alice-key-0123456789");
        let bob = Some("ApiKey bob-key-00123456789ab");

        let created = crate::route_http_request(
            "POST",
            "/api/v0/collections",
            alice,
            r#"{"title":"Ownership Differential"}"#,
            &state,
        )
        .await
        .expect("alice creates a collection");
        let created_json =
            serde_json::from_str::<serde_json::Value>(&created.body).unwrap_or_default();
        let collection_id = created_json["id"].as_str().unwrap_or_default().to_owned();
        record!(
            target,
            "POST",
            "/api/v0/collections",
            "mutation-side-effects-and-readback",
            created.status == "201 Created" && created_json["ownerUserId"] == "alice"
        );

        let mut cross_user_pass = true;
        for (method, path, body) in [
            ("GET", format!("/api/v0/collections/{collection_id}"), ""),
            (
                "PUT",
                format!("/api/v0/collections/{collection_id}"),
                r#"{"title":"Hijacked"}"#,
            ),
            (
                "GET",
                format!("/api/v0/collections/{collection_id}/items"),
                "",
            ),
            (
                "POST",
                format!("/api/v0/collections/{collection_id}/items"),
                r#"{"contentId":"track-1"}"#,
            ),
        ] {
            let response = crate::route_http_request(method, &path, bob, body, &state)
                .await
                .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
            cross_user_pass &= response.status == "404 Not Found";
        }
        record!(
            target,
            "GET",
            "/api/v0/collections/{id}",
            "missing-empty-or-conflict-state",
            cross_user_pass
        );
        record!(
            target,
            "PUT",
            "/api/v0/collections/{id}",
            "missing-empty-or-conflict-state",
            cross_user_pass
        );
        record!(
            target,
            "GET",
            "/api/v0/collections/{id}/items",
            "missing-empty-or-conflict-state",
            cross_user_pass
        );
        record!(
            target,
            "POST",
            "/api/v0/collections/{id}/items",
            "missing-empty-or-conflict-state",
            cross_user_pass
        );

        let alice_get = crate::route_http_request(
            "GET",
            &format!("/api/v0/collections/{collection_id}"),
            alice,
            "",
            &state,
        )
        .await
        .expect("alice reads her own collection");
        record!(
            target,
            "GET",
            "/api/v0/collections/{id}",
            "nominal-status-headers-body",
            alice_get.status == "200 OK"
        );

        let bob_created = crate::route_http_request(
            "POST",
            "/api/v0/collections",
            bob,
            r#"{"title":"Bob Ownership Differential"}"#,
            &state,
        )
        .await
        .expect("bob creates his own collection");
        let bob_created_json =
            serde_json::from_str::<serde_json::Value>(&bob_created.body).unwrap_or_default();
        let bob_owned =
            bob_created.status == "201 Created" && bob_created_json["ownerUserId"] == "bob";

        let alice_list = crate::route_http_request("GET", "/api/v0/collections", alice, "", &state)
            .await
            .expect("alice lists collections");
        let alice_titles = serde_json::from_str::<serde_json::Value>(&alice_list.body)
            .unwrap_or_default()
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .map(|record| record["title"].as_str().unwrap_or_default().to_owned())
            .collect::<Vec<_>>();
        let bob_list = crate::route_http_request("GET", "/api/v0/collections", bob, "", &state)
            .await
            .expect("bob lists collections");
        let bob_titles = serde_json::from_str::<serde_json::Value>(&bob_list.body)
            .unwrap_or_default()
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .map(|record| record["title"].as_str().unwrap_or_default().to_owned())
            .collect::<Vec<_>>();
        record!(
            target,
            "GET",
            "/api/v0/collections",
            "nominal-status-headers-body",
            alice_list.status == "200 OK" && bob_list.status == "200 OK"
        );
        record!(
            target,
            "GET",
            "/api/v0/collections",
            "populated-dynamic-state",
            bob_owned
                && alice_titles == vec!["Ownership Differential".to_owned()]
                && bob_titles == vec!["Bob Ownership Differential".to_owned()]
        );

        let bob_delete = crate::route_http_request(
            "DELETE",
            &format!("/api/v0/collections/{collection_id}"),
            bob,
            "",
            &state,
        )
        .await
        .expect("bob attempts to delete alice's collection");
        let still_there = crate::route_http_request(
            "GET",
            &format!("/api/v0/collections/{collection_id}"),
            alice,
            "",
            &state,
        )
        .await
        .expect("alice's collection still exists");
        record!(
            target,
            "DELETE",
            "/api/v0/collections/{id}",
            "missing-empty-or-conflict-state",
            bob_delete.status == "404 Not Found" && still_there.status == "200 OK"
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("collections_ownership_scoping.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "controller-api collections-ownership-scoping mismatches:\n{}",
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting the library-health portion of
/// `compatibility_projections_use_local_state_for_library_jobs_and_
/// discovery` (10 registered routes; the source test's remaining
/// lidarr/musicbrainz/wishlist/discovery-graph/listening-party
/// sections mostly hit slskR-internal bare `/api/...` compat-shell
/// routes with no registry entry in either target, or routes already
/// credited via other differentials -- not revisited here).
/// Independently re-derived from the same real seeded-library-item
/// health/dashboard/scan/remediation checks. slskdN-only (confirmed
/// against the frozen registry route-by-route; `/api/library/items`
/// itself has no registry entry and is used only as fixture setup).
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
pub(super) async fn controller_api_differential_library_health() {
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

    crate::route_http_request(
        "POST",
        "/api/library/items",
        None,
        r#"{"artist":"","title":"Untitled","kind":"Audio"}"#,
        &state,
    )
    .await
    .expect("create incomplete library item fixture");
    crate::route_http_request(
        "POST",
        "/api/library/items",
        None,
        r#"{"artist":"Known","title":"Release","kind":"Audio"}"#,
        &state,
    )
    .await
    .expect("create complete library item fixture");

    let health = crate::route_http_request("GET", "/api/library/health/issues", None, "", &state)
        .await
        .expect("library issues");
    let health_json = serde_json::from_str::<serde_json::Value>(&health.body).unwrap_or_default();
    record!(
        "GET",
        "/api/library/health/issues",
        "nominal-status-headers-body",
        health.status == "200 OK"
    );
    record!(
        "GET",
        "/api/library/health/issues",
        "populated-dynamic-state",
        health_json["totalCount"] == 1
            && health_json["issues"][0]["type"] == "MissingMetadata"
            && health_json["issues"][0]["metadata"]["missingField"] == "missing_artist"
            && health_json["filter"]["limit"] == 100
            && health_json["filter"]["offset"] == 0
    );

    let by_artist = crate::route_http_request(
        "GET",
        "/api/library/health/issues/by-artist",
        None,
        "",
        &state,
    )
    .await
    .expect("issues by artist");
    let by_artist_json =
        serde_json::from_str::<serde_json::Value>(&by_artist.body).unwrap_or_default();
    record!(
        "GET",
        "/api/library/health/issues/by-artist",
        "nominal-status-headers-body",
        by_artist.status == "200 OK"
    );
    record!(
        "GET",
        "/api/library/health/issues/by-artist",
        "populated-dynamic-state",
        by_artist_json["groups"]
            .as_array()
            .is_some_and(Vec::is_empty)
            && by_artist_json["totalArtists"] == 0
    );

    let summary = crate::route_http_request(
        "GET",
        "/api/library/health/summary?LibraryPath=%2Fmusic",
        None,
        "",
        &state,
    )
    .await
    .expect("library health summary");
    let summary_json = serde_json::from_str::<serde_json::Value>(&summary.body).unwrap_or_default();
    record!(
        "GET",
        "/api/library/health/summary",
        "nominal-status-headers-body",
        summary.status == "200 OK"
    );
    record!(
        "GET",
        "/api/library/health/summary",
        "populated-dynamic-state",
        summary_json["libraryPath"] == "/music"
            && summary_json["totalIssues"] == 1
            && summary_json["issuesOpen"] == 1
    );

    let dashboard = crate::route_http_request(
        "GET",
        "/api/library/health/dashboard?libraryPath=%2Fmusic&artistLimit=1&issueLimit=1",
        None,
        "",
        &state,
    )
    .await
    .expect("library health dashboard");
    let dashboard_json =
        serde_json::from_str::<serde_json::Value>(&dashboard.body).unwrap_or_default();
    record!(
        "GET",
        "/api/library/health/dashboard",
        "nominal-status-headers-body",
        dashboard.status == "200 OK"
    );
    record!(
        "GET",
        "/api/library/health/dashboard",
        "populated-dynamic-state",
        dashboard_json["summary"]["libraryPath"] == "/music"
            && dashboard_json["issuesByType"][0]["type"] == "MissingMetadata"
            && dashboard_json["issuesByArtist"]
                .as_array()
                .is_some_and(Vec::is_empty)
            && dashboard_json["issues"].as_array().map(Vec::len) == Some(1)
            && dashboard_json["totalIssues"] == 1
    );

    let by_codec = crate::route_http_request(
        "GET",
        "/api/library/health/issues/by-codec",
        None,
        "",
        &state,
    )
    .await
    .expect("issues by codec");
    let by_codec_json =
        serde_json::from_str::<serde_json::Value>(&by_codec.body).unwrap_or_default();
    record!(
        "GET",
        "/api/library/health/issues/by-codec",
        "nominal-status-headers-body",
        by_codec.status == "200 OK"
    );
    record!(
        "GET",
        "/api/library/health/issues/by-codec",
        "populated-dynamic-state",
        by_codec_json["groups"].as_array().map(Vec::len) == Some(1)
            && by_codec_json["groups"][0]["codec"] == "UNKNOWN"
            && by_codec_json["groups"][0]["count"] == 1
            && by_codec_json["groups"][0]["transcodeSuspect"] == 0
            && by_codec_json["totalIssues"] == 1
    );

    let filtered = crate::route_http_request(
        "GET",
        "/api/library/health/issues?LibraryPath=%2Fmusic&types=CorruptedFile&severities=Medium&statuses=Detected&Limit=2&Offset=999999",
        None,
        "",
        &state,
    )
    .await
    .expect("filtered library issues");
    let filtered_json =
        serde_json::from_str::<serde_json::Value>(&filtered.body).unwrap_or_default();
    record!(
        "GET",
        "/api/library/health/issues",
        "malformed-path-query-or-body",
        filtered.status == "200 OK"
            && filtered_json["totalCount"] == 0
            && filtered_json["filter"]["libraryPath"] == "/music"
            && filtered_json["filter"]["types"][0] == "CorruptedFile"
            && filtered_json["filter"]["severities"][0] == "Medium"
            && filtered_json["filter"]["statuses"][0] == "Detected"
            && filtered_json["filter"]["limit"] == 2
            && filtered_json["filter"]["offset"] == 999999
    );

    let by_type = crate::route_http_request(
        "GET",
        "/api/library/health/issues/by-type",
        None,
        "",
        &state,
    )
    .await
    .expect("issues by type");
    let by_type_json = serde_json::from_str::<serde_json::Value>(&by_type.body).unwrap_or_default();
    record!(
        "GET",
        "/api/library/health/issues/by-type",
        "nominal-status-headers-body",
        by_type.status == "200 OK"
    );
    record!(
        "GET",
        "/api/library/health/issues/by-type",
        "populated-dynamic-state",
        by_type_json["groups"][0]["type"] == "MissingMetadata" && by_type_json["totalIssues"] == 1
    );

    // The versioned controller exposes the same library-health actions
    // through the frozen v0 route table.  Exercise those actual aliases
    // as separate requests: route-presence alone does not prove that the
    // versioned dispatch reaches the state-backed implementation.
    for (path, route, populated) in [
        (
            "/api/v0/library/health/issues",
            "/api/v0/library/health/issues",
            true,
        ),
        (
            "/api/v0/library/health/issues/by-artist",
            "/api/v0/library/health/issues/by-artist",
            true,
        ),
        (
            "/api/v0/library/health/issues/by-release",
            "/api/v0/library/health/issues/by-release",
            true,
        ),
        (
            "/api/v0/library/health/issues/by-codec",
            "/api/v0/library/health/issues/by-codec",
            true,
        ),
        (
            "/api/v0/library/health/issues/by-type?libraryPath=%2Fmusic",
            "/api/v0/library/health/issues/by-type",
            true,
        ),
        (
            "/api/v0/library/health/summary?LibraryPath=%2Fmusic",
            "/api/v0/library/health/summary",
            true,
        ),
        (
            "/api/v0/library/health/dashboard?libraryPath=%2Fmusic&artistLimit=1&issueLimit=1",
            "/api/v0/library/health/dashboard",
            true,
        ),
    ] {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "GET",
            route,
            "nominal-status-headers-body",
            response.status == "200 OK"
        );
        let populated_pass = match route {
            "/api/v0/library/health/issues" => {
                value["totalCount"] == 1
                    && value["issues"][0]["metadata"]["missingField"] == "missing_artist"
            }
            "/api/v0/library/health/issues/by-artist" => {
                value["groups"].as_array().is_some() && value["totalArtists"].as_u64().is_some()
            }
            "/api/v0/library/health/issues/by-release" => {
                value["groups"].as_array().is_some() && value["totalReleases"].as_u64().is_some()
            }
            "/api/v0/library/health/issues/by-codec" => {
                value["groups"].as_array().is_some() && value["totalIssues"] == 1
            }
            "/api/v0/library/health/issues/by-type" => {
                value["groups"].as_array().is_some() && value["totalIssues"] == 1
            }
            "/api/v0/library/health/summary" => {
                value["libraryPath"] == "/music"
                    && value["totalIssues"] == 1
                    && value["issuesOpen"] == 1
            }
            "/api/v0/library/health/dashboard" => {
                value["summary"]["libraryPath"] == "/music"
                    && value["issues"]
                        .as_array()
                        .is_some_and(|issues| issues.len() == 1)
                    && value["totalIssues"] == 1
            }
            _ => false,
        };
        if populated {
            record!("GET", route, "populated-dynamic-state", populated_pass);
        }
    }

    let by_release = crate::route_http_request(
        "GET",
        "/api/library/health/issues/by-release",
        None,
        "",
        &state,
    )
    .await
    .expect("issues by release");
    let by_release_json =
        serde_json::from_str::<serde_json::Value>(&by_release.body).unwrap_or_default();
    record!(
        "GET",
        "/api/library/health/issues/by-release",
        "nominal-status-headers-body",
        by_release.status == "200 OK"
    );
    record!(
        "GET",
        "/api/library/health/issues/by-release",
        "populated-dynamic-state",
        by_release_json["groups"].as_array().is_some()
            && by_release_json["totalReleases"].as_u64().is_some()
    );

    let mut bad_query_pass = true;
    for path in [
        "/api/library/health/summary",
        "/api/library/health/dashboard?libraryPath=%2Fmusic&artistLimit=0",
        "/api/library/health/dashboard?libraryPath=%2Fmusic&issueLimit=251",
        "/api/library/health/issues?limit=0",
        "/api/library/health/issues?limit=251",
        "/api/library/health/issues?offset=-1",
        "/api/library/health/issues?types=NotAnIssueType",
        "/api/library/health/issues?severities=Urgent",
        "/api/library/health/issues?statuses=Open",
        "/api/library/health/issues/by-artist?limit=101",
        "/api/library/health/issues/by-release?limit=0",
    ] {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        bad_query_pass &= response.status == "400 Bad Request";
    }
    record!(
        "GET",
        "/api/library/health/summary",
        "malformed-path-query-or-body",
        bad_query_pass
    );
    record!(
        "GET",
        "/api/library/health/dashboard",
        "malformed-path-query-or-body",
        bad_query_pass
    );
    record!(
        "GET",
        "/api/library/health/issues/by-artist",
        "malformed-path-query-or-body",
        bad_query_pass
    );
    record!(
        "GET",
        "/api/library/health/issues/by-release",
        "malformed-path-query-or-body",
        bad_query_pass
    );

    let patched_route = "/api/v0/library/health/issues/lib-1-missing-artist";
    let patched_issue = crate::route_http_request(
        "PATCH",
        patched_route,
        None,
        r#"{"artist":"Recovered Artist"}"#,
        &state,
    )
    .await
    .unwrap_or_else(|error| panic!("{patched_route}: {error}"));
    record!(
        "PATCH",
        "/api/v0/library/health/issues/{issueId}",
        "nominal-status-headers-body",
        patched_issue.status == "204 No Content"
    );
    record!(
        "PATCH",
        "/api/v0/library/health/issues/{issueId}",
        "mutation-side-effects-and-readback",
        patched_issue.status == "204 No Content"
            && state
                .library
                .read()
                .await
                .get("lib-1")
                .is_some_and(|record| { record.artist == "Recovered Artist" })
    );

    crate::route_http_request(
        "POST",
        "/api/library/items",
        None,
        r#"{"artist":"Fixable","title":"Kindless","kind":""}"#,
        &state,
    )
    .await
    .expect("create fixable library item fixture");

    let scan = crate::route_http_request(
        "POST",
        "/api/v0/library/health/scans",
        None,
        r#"{"libraryPath":"/music"}"#,
        &state,
    )
    .await
    .expect("library scan");
    let scan_json = serde_json::from_str::<serde_json::Value>(&scan.body).unwrap_or_default();
    let active_id = scan_json["scanId"].as_str().unwrap_or_default().to_owned();
    record!(
        "POST",
        "/api/v0/library/health/scans",
        "nominal-status-headers-body",
        scan.status == "200 OK"
            && scan_json["scanId"].as_str().is_some()
            && scan_json["message"] == "Scan started successfully"
    );

    let second_scan = crate::route_http_request(
        "POST",
        "/api/v0/library/health/scans",
        None,
        r#"{"libraryPath":"/music"}"#,
        &state,
    )
    .await
    .expect("second concurrent library scan");
    record!(
        "POST",
        "/api/v0/library/health/scans",
        "missing-empty-or-conflict-state",
        second_scan.status == "409 Conflict" && second_scan.body.contains(&active_id)
    );

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let completed_route = format!("/api/library/health/scans/{active_id}");
    let completed = crate::route_http_request("GET", &completed_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{completed_route}: {error}"));
    record!(
        "GET",
        "/api/library/health/scans/{scanId}",
        "nominal-status-headers-body",
        serde_json::from_str::<serde_json::Value>(&completed.body).unwrap_or_default()["status"]
            == "completed"
    );

    let completed_versioned_route = format!("/api/v0/library/health/scans/{active_id}");
    let completed_versioned =
        crate::route_http_request("GET", &completed_versioned_route, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{completed_versioned_route}: {error}"));
    let completed_versioned_json =
        serde_json::from_str::<serde_json::Value>(&completed_versioned.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/library/health/scans/{scanId}",
        "nominal-status-headers-body",
        completed_versioned.status == "200 OK" && completed_versioned_json["status"] == "completed"
    );
    record!(
        "GET",
        "/api/v0/library/health/scans/{scanId}",
        "populated-dynamic-state",
        completed_versioned_json["status"] == "completed"
            && completed_versioned_json["issues_found"].as_u64().is_some()
    );
    record!(
        "POST",
        "/api/v0/library/health/scans",
        "mutation-side-effects-and-readback",
        scan.status == "200 OK"
            && completed_versioned_json["status"] == "completed"
            && completed_versioned_json["issues_found"].as_u64().is_some()
    );

    let missing_scan = crate::route_http_request(
        "GET",
        "/api/library/health/scans/scan-does-not-exist-differential",
        None,
        "",
        &state,
    )
    .await
    .expect("missing library scan");
    record!(
        "GET",
        "/api/library/health/scans/{scanId}",
        "missing-empty-or-conflict-state",
        missing_scan.status == "404 Not Found"
    );

    let fixed = crate::route_http_request(
        "POST",
        "/api/v0/slskdn/library/remediate",
        None,
        r#"{"issue_ids":["lib-3-missing-kind"]}"#,
        &state,
    )
    .await
    .expect("fix library issues");
    let fixed_json = serde_json::from_str::<serde_json::Value>(&fixed.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/slskdn/library/remediate",
        "nominal-status-headers-body",
        fixed.status == "200 OK" && fixed_json["id"].as_str().is_some()
    );
    record!(
        "POST",
        "/api/v0/slskdn/library/remediate",
        "mutation-side-effects-and-readback",
        fixed_json["kind"] == "library_remediation"
            && fixed_json["status"] == "completed"
            && fixed_json["fixedCount"] == 1
            && fixed_json["issueIds"] == serde_json::json!(["lib-3-missing-kind"])
            && fixed_json["remaining"] == serde_json::Value::Null
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("library_health.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api library-health mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// The library-health differential above already covers populated and
/// nominal projections.  This companion closes the remaining deterministic
/// empty/error branches for the same real handlers, including the frozen
/// `/api/v0/` aliases and the native `/api/v0/slskdn/` alias.  No database
/// fault or external scan is used here; runtime-failure cases remain open.
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
pub(super) async fn controller_api_differential_library_health_versioned_edge_states() {
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

    macro_rules! get_case {
        ($state:expr, $path:expr, $route:expr, $case:expr, $check:expr) => {{
            let response = crate::route_http_request("GET", $path, None, "", $state)
                .await
                .unwrap_or_else(|error| panic!("GET {}: {error}", $path));
            let value = serde_json::from_str::<serde_json::Value>(&response.body)
                .unwrap_or(serde_json::Value::Null);
            let check: fn(&crate::routing::HttpResponse, &serde_json::Value) -> bool = $check;
            let pass = check(&response, &value);
            record!("GET", $route, $case, pass);
        }};
    }

    let (empty_state, _receiver) = test_state();

    // The unversioned compatibility routes have real empty-state
    // contracts already implemented; exercise the branches that the
    // populated library-health test intentionally leaves open.
    get_case!(
        &empty_state,
        "/api/library/health/summary",
        "/api/library/health/summary",
        "missing-empty-or-conflict-state",
        |response, _| response.status == "400 Bad Request"
    );
    get_case!(
        &empty_state,
        "/api/library/health/dashboard",
        "/api/library/health/dashboard",
        "missing-empty-or-conflict-state",
        |response, _| response.status == "400 Bad Request"
    );
    get_case!(
        &empty_state,
        "/api/library/health/issues/by-artist",
        "/api/library/health/issues/by-artist",
        "missing-empty-or-conflict-state",
        |response, value| {
            response.status == "200 OK"
                && value["groups"].as_array().is_some_and(Vec::is_empty)
                && value["totalArtists"] == 0
        }
    );
    get_case!(
        &empty_state,
        "/api/library/health/issues/by-codec",
        "/api/library/health/issues/by-codec",
        "missing-empty-or-conflict-state",
        |response, value| {
            response.status == "200 OK"
                && value["groups"].as_array().is_some_and(Vec::is_empty)
                && value["totalIssues"] == 0
        }
    );
    get_case!(
        &empty_state,
        "/api/library/health/issues/by-release",
        "/api/library/health/issues/by-release",
        "missing-empty-or-conflict-state",
        |response, value| {
            response.status == "200 OK"
                && value["groups"].as_array().is_some_and(Vec::is_empty)
                && value["totalReleases"] == 0
        }
    );
    get_case!(
        &empty_state,
        "/api/library/health/issues/by-type",
        "/api/library/health/issues/by-type",
        "missing-empty-or-conflict-state",
        |response, value| {
            response.status == "200 OK"
                && value["groups"].as_array().is_some_and(Vec::is_empty)
                && value["totalIssues"] == 0
        }
    );
    get_case!(
        &empty_state,
        "/api/slskdn/library/health",
        "/api/slskdn/library/health",
        "missing-empty-or-conflict-state",
        |response, value| { response.status == "200 OK" && value["summary"]["total_issues"] == 0 }
    );
    get_case!(
        &empty_state,
        "/api/library/health/scans/",
        "/api/library/health/scans/{scanId}",
        "malformed-path-query-or-body",
        |response, _| response.status == "404 Not Found"
    );

    // Versioned library-health projections: empty results are real
    // state-backed responses, while missing required values and malformed
    // values follow the controller's BadRequest/NotFound contracts.
    get_case!(
        &empty_state,
        "/api/v0/library/health/issues?limit=0",
        "/api/v0/library/health/issues",
        "malformed-path-query-or-body",
        |response, _| response.status == "400 Bad Request"
    );
    get_case!(
        &empty_state,
        "/api/v0/library/health/issues",
        "/api/v0/library/health/issues",
        "missing-empty-or-conflict-state",
        |response, value| {
            response.status == "200 OK"
                && value["issues"].as_array().is_some_and(Vec::is_empty)
                && value["totalCount"] == 0
        }
    );
    get_case!(
        &empty_state,
        "/api/v0/library/health/issues/by-artist?limit=101",
        "/api/v0/library/health/issues/by-artist",
        "malformed-path-query-or-body",
        |response, _| response.status == "400 Bad Request"
    );
    get_case!(
        &empty_state,
        "/api/v0/library/health/issues/by-artist",
        "/api/v0/library/health/issues/by-artist",
        "missing-empty-or-conflict-state",
        |response, value| {
            response.status == "200 OK"
                && value["groups"].as_array().is_some_and(Vec::is_empty)
                && value["totalArtists"] == 0
        }
    );
    get_case!(
        &empty_state,
        "/api/v0/library/health/issues/by-codec/",
        "/api/v0/library/health/issues/by-codec",
        "malformed-path-query-or-body",
        |response, _| response.status == "404 Not Found"
    );
    get_case!(
        &empty_state,
        "/api/v0/library/health/issues/by-codec",
        "/api/v0/library/health/issues/by-codec",
        "missing-empty-or-conflict-state",
        |response, value| {
            response.status == "200 OK"
                && value["groups"].as_array().is_some_and(Vec::is_empty)
                && value["totalIssues"] == 0
        }
    );
    get_case!(
        &empty_state,
        "/api/v0/library/health/issues/by-release?limit=0",
        "/api/v0/library/health/issues/by-release",
        "malformed-path-query-or-body",
        |response, _| response.status == "400 Bad Request"
    );
    get_case!(
        &empty_state,
        "/api/v0/library/health/issues/by-release",
        "/api/v0/library/health/issues/by-release",
        "missing-empty-or-conflict-state",
        |response, value| {
            response.status == "200 OK"
                && value["groups"].as_array().is_some_and(Vec::is_empty)
                && value["totalReleases"] == 0
        }
    );
    get_case!(
        &empty_state,
        "/api/v0/library/health/issues/by-type",
        "/api/v0/library/health/issues/by-type",
        "missing-empty-or-conflict-state",
        |response, _| response.status == "400 Bad Request"
    );
    get_case!(
        &empty_state,
        "/api/v0/library/health/summary?libraryPath=%20",
        "/api/v0/library/health/summary",
        "malformed-path-query-or-body",
        |response, _| response.status == "400 Bad Request"
    );
    get_case!(
        &empty_state,
        "/api/v0/library/health/summary",
        "/api/v0/library/health/summary",
        "missing-empty-or-conflict-state",
        |response, _| response.status == "400 Bad Request"
    );
    get_case!(
        &empty_state,
        "/api/v0/library/health/dashboard?libraryPath=%2Fmusic&artistLimit=0",
        "/api/v0/library/health/dashboard",
        "malformed-path-query-or-body",
        |response, _| response.status == "400 Bad Request"
    );
    get_case!(
        &empty_state,
        "/api/v0/library/health/dashboard",
        "/api/v0/library/health/dashboard",
        "missing-empty-or-conflict-state",
        |response, _| response.status == "400 Bad Request"
    );
    get_case!(
        &empty_state,
        "/api/v0/library/health/scans/",
        "/api/v0/library/health/scans/{scanId}",
        "malformed-path-query-or-body",
        |response, _| response.status == "404 Not Found"
    );
    get_case!(
        &empty_state,
        "/api/v0/library/health/scans/no-such-scan",
        "/api/v0/library/health/scans/{scanId}",
        "missing-empty-or-conflict-state",
        |response, _| response.status == "404 Not Found"
    );

    // The native versioned alias also has a populated branch backed by the
    // same library store, not a hardcoded fixture response.
    get_case!(
        &empty_state,
        "/api/v0/slskdn/library/health?limit=1",
        "/api/v0/slskdn/library/health",
        "nominal-status-headers-body",
        |response, value| {
            response.status == "200 OK"
                && value["path"] == "(all)"
                && value["summary"]["total_issues"] == 0
        }
    );
    get_case!(
        &empty_state,
        "/api/v0/slskdn/library/health?limit=251",
        "/api/v0/slskdn/library/health",
        "malformed-path-query-or-body",
        |response, _| response.status == "400 Bad Request"
    );

    let (populated_state, _receiver) = test_state();
    let seed = crate::route_http_request(
        "POST",
        "/api/library/items",
        None,
        r#"{"artist":"","title":"Versioned health","kind":"Audio"}"#,
        &populated_state,
    )
    .await
    .expect("seed versioned library-health issue");
    assert_eq!(seed.status, "201 Created", "{}", seed.body);
    get_case!(
        &populated_state,
        "/api/v0/slskdn/library/health?limit=1",
        "/api/v0/slskdn/library/health",
        "populated-dynamic-state",
        |response, value| {
            response.status == "200 OK"
                && value["summary"]["total_issues"]
                    .as_u64()
                    .is_some_and(|count| count >= 1)
                && value["issues"]
                    .as_array()
                    .is_some_and(|issues| !issues.is_empty())
        }
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("library_health_versioned_edge_states.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api versioned library-health mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
