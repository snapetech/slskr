//! Controller full persistence differential 02 ownership.

use super::*;

/// Bulk differential proof crediting `transaction-and-concurrency-
/// atomicity` for the 13 domains already touched by earlier
/// persistence-lifecycle batches, minus `Transfers` (its only
/// "creation" route, `POST /api/v0/transfers`, is a slskR-internal
/// compat shortcut with no registry entry in either frozen target,
/// same reason it was skipped for `update-delete-and-readback`).
/// `Events` IS included here, proven via the same internal
/// `record_event` call the existing `create-and-read-roundtrip`
/// differential already uses as its own real write path (no HTTP
/// route needed, matching that precedent).
///
/// Two proof shapes, chosen per domain by what's actually reachable
/// via real dispatch:
/// - Concurrent-create: fire N simultaneous creates of N distinct
///   rows through the real dispatcher (`route_http_request`) against
///   the SAME `DatabaseManager`/connection pool via
///   `futures_util::future::join_all`, then read back through the
///   real store and assert exactly N rows persisted with all N
///   expected distinct values present -- proves the real SQLite
///   pool's locking serializes concurrent writers without silently
///   dropping one.
/// - Concurrent-update: seed N distinct pre-existing rows, then fire
///   N simultaneous updates (one per row, each with its own distinct
///   new value) through the real dispatcher, then read back and
///   assert each row ended up with ITS OWN writer's value, not a
///   neighbor's -- proves real transactional isolation, not just "no
///   crash under load".
///
/// Confirmed against `/tmp/slskr-parity-evidence/persistence-
/// lifecycle/*.json` before writing: `transaction-and-concurrency-
/// atomicity` was 100% untouched for every domain in the whole
/// workstream's history.
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
    feature = "bounded-persistence-tests"
))]
pub(super) async fn persistence_lifecycle_differential_covered_domains_transaction_and_concurrency_atomicity(
) {
    let fanout = 6usize;
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($target:expr, $domain:expr, $pass:expr) => {
            let pass = $pass;
            if !pass {
                mismatches.push(format!(
                    "{} {} transaction-and-concurrency-atomicity",
                    $target, $domain
                ));
            }
            ledger.push(serde_json::json!({
                "target": $target,
                "domain": $domain,
                "case": "transaction-and-concurrency-atomicity",
                "pass": pass,
            }));
        };
    }

    // Collections: concurrent creates of distinct rows.
    {
        let target = "slskdn";
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
        let bodies: Vec<String> = (0..fanout)
            .map(|i| format!(r#"{{"name":"Differential Concurrent Collection {i}"}}"#))
            .collect();
        let responses = futures_util::future::join_all(bodies.iter().map(|body| {
            crate::route_http_request("POST", "/api/v0/collections", None, body, &state)
        }))
        .await;
        let all_ok = responses.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status.starts_with('2'))
        });
        let persisted = db.list_collections(20, 0).await.unwrap_or_default();
        let expected: std::collections::BTreeSet<String> = (0..fanout)
            .map(|i| format!("Differential Concurrent Collection {i}"))
            .collect();
        let persisted_names: std::collections::BTreeSet<String> =
            persisted.iter().map(|record| record.name.clone()).collect();
        record!(
            target,
            "Collections",
            all_ok && persisted.len() == fanout && persisted_names == expected
        );
    }

    // CollectionItems: one parent collection, concurrent item creates.
    {
        let target = "slskdn";
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
        let collection = crate::route_http_request(
            "POST",
            "/api/v0/collections",
            None,
            r#"{"name":"Differential Concurrency Parent"}"#,
            &state,
        )
        .await
        .expect("create parent collection");
        let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("parent collection id");
        let path = format!("/api/v0/collections/{collection_id}/items");
        let bodies: Vec<String> = (0..fanout)
            .map(|i| {
                format!(
                    r#"{{"content_id":"differential-concurrent-track-{i}","artist":"Differential Artist","title":"Track {i}","kind":"Audio"}}"#
                )
            })
            .collect();
        let responses = futures_util::future::join_all(
            bodies
                .iter()
                .map(|body| crate::route_http_request("POST", &path, None, body, &state)),
        )
        .await;
        let all_ok = responses.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status.starts_with('2'))
        });
        let persisted = db.list_collection_items(20, 0).await.unwrap_or_default();
        let expected: std::collections::BTreeSet<String> = (0..fanout)
            .map(|i| format!("differential-concurrent-track-{i}"))
            .collect();
        let persisted_ids: std::collections::BTreeSet<String> = persisted
            .iter()
            .map(|record| record.content_id.clone())
            .collect();
        record!(
            target,
            "CollectionItems",
            all_ok && persisted.len() == fanout && persisted_ids == expected
        );
    }

    // UserNotes: concurrent creates of distinct rows.
    {
        let target = "slskdn";
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
        let bodies: Vec<String> = (0..fanout)
            .map(|i| {
                format!(r#"{{"username":"differential-concurrent-friend-{i}","note":"note {i}"}}"#)
            })
            .collect();
        let responses = futures_util::future::join_all(bodies.iter().map(|body| {
            crate::route_http_request("POST", "/api/v0/users/notes", None, body, &state)
        }))
        .await;
        let all_ok = responses.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status.starts_with('2'))
        });
        let persisted = db.list_user_notes(20, 0).await.unwrap_or_default();
        let expected: std::collections::BTreeSet<String> = (0..fanout)
            .map(|i| format!("differential-concurrent-friend-{i}"))
            .collect();
        let persisted_usernames: std::collections::BTreeSet<String> = persisted
            .iter()
            .map(|record| record.username.clone())
            .collect();
        record!(
            target,
            "UserNotes",
            all_ok && persisted.len() == fanout && persisted_usernames == expected
        );
    }

    // WishlistItems: concurrent creates of distinct rows.
    {
        let target = "slskdn";
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
        let bodies: Vec<String> = (0..fanout)
            .map(|i| {
                format!(
                    r#"{{"artist":"Differential Artist","title":"Differential Concurrent Track {i}","kind":"Audio"}}"#
                )
            })
            .collect();
        let responses =
            futures_util::future::join_all(bodies.iter().map(|body| {
                crate::route_http_request("POST", "/api/v0/wishlist", None, body, &state)
            }))
            .await;
        let all_ok = responses.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status.starts_with('2'))
        });
        let persisted = db.list_wishlist_items(20, 0).await.unwrap_or_default();
        let expected: std::collections::BTreeSet<String> = (0..fanout)
            .map(|i| format!("Differential Concurrent Track {i}"))
            .collect();
        let persisted_titles: std::collections::BTreeSet<String> = persisted
            .iter()
            .map(|record| record.title.clone())
            .collect();
        record!(
            target,
            "WishlistItems",
            all_ok && persisted.len() == fanout && persisted_titles == expected
        );
    }

    // ShareGroupMembers: one parent group, concurrent member creates.
    {
        let target = "slskdn";
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
        let group = crate::route_http_request(
            "POST",
            "/api/v0/sharegroups",
            None,
            r#"{"name":"Differential Concurrency Group"}"#,
            &state,
        )
        .await
        .expect("create parent share group");
        let group_id = serde_json::from_str::<serde_json::Value>(&group.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("parent share group id");
        let path = format!("/api/v0/sharegroups/{group_id}/members");
        let bodies: Vec<String> = (0..fanout)
            .map(|i| format!(r#"{{"username":"differential-concurrent-member-{i}"}}"#))
            .collect();
        let responses = futures_util::future::join_all(
            bodies
                .iter()
                .map(|body| crate::route_http_request("POST", &path, None, body, &state)),
        )
        .await;
        let all_ok = responses.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status.starts_with('2'))
        });
        let persisted = db.list_share_group_members(20, 0).await.unwrap_or_default();
        let expected: std::collections::BTreeSet<String> = (0..fanout)
            .map(|i| format!("differential-concurrent-member-{i}"))
            .collect();
        let persisted_usernames: std::collections::BTreeSet<String> = persisted
            .iter()
            .map(|record| record.username.clone())
            .collect();
        record!(
            target,
            "ShareGroupMembers",
            all_ok && persisted.len() == fanout && persisted_usernames == expected
        );
    }

    // Contacts: seed N rows, then concurrent updates each row's own
    // distinct new username -- proves no cross-writer bleed.
    {
        let target = "slskdn";
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
        let mut ids = Vec::new();
        for i in 0..fanout {
            let created = crate::route_http_request(
                "POST",
                "/api/contacts",
                None,
                &format!(r#"{{"username":"differential-concurrent-contact-{i}"}}"#),
                &state,
            )
            .await
            .expect("seed differential contact");
            let id = serde_json::from_str::<serde_json::Value>(&created.body)
                .ok()
                .and_then(|json| json["id"].as_str().map(str::to_owned))
                .expect("seeded contact id");
            ids.push(id);
        }
        let update_paths: Vec<String> = ids
            .iter()
            .map(|id| format!("/api/v0/contacts/{id}"))
            .collect();
        let update_bodies: Vec<String> = (0..fanout)
            .map(|i| format!(r#"{{"username":"differential-concurrent-contact-{i}-updated"}}"#))
            .collect();
        let responses = futures_util::future::join_all(
            update_paths
                .iter()
                .zip(update_bodies.iter())
                .map(|(path, body)| crate::route_http_request("PUT", path, None, body, &state)),
        )
        .await;
        let all_ok = responses.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status.starts_with('2'))
        });
        let persisted = db.list_contacts(20, 0).await.unwrap_or_default();
        let expected: std::collections::BTreeMap<String, String> = ids
            .iter()
            .enumerate()
            .map(|(i, id)| {
                (
                    id.clone(),
                    format!("differential-concurrent-contact-{i}-updated"),
                )
            })
            .collect();
        let each_own_value = persisted.len() == fanout
            && persisted
                .iter()
                .all(|record| expected.get(&record.id) == Some(&record.username));
        record!(target, "Contacts", all_ok && each_own_value);
    }

    // ShareGrants: one parent collection, seed N grants, then
    // concurrent updates each grant's own distinct permissions value.
    {
        let target = "slskdn";
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
        let collection = crate::route_http_request(
            "POST",
            "/api/v0/collections",
            None,
            r#"{"name":"Differential Concurrency Grant Parent"}"#,
            &state,
        )
        .await
        .expect("create parent grant collection");
        let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("parent grant collection id");
        let mut ids = Vec::new();
        for i in 0..fanout {
            let created = crate::route_http_request(
                "POST",
                "/api/v0/share-grants",
                None,
                &format!(
                    r#"{{"collection_id":"{collection_id}","username":"differential-concurrent-grantee-{i}"}}"#
                ),
                &state,
            )
            .await
            .expect("seed differential share grant");
            let id = serde_json::from_str::<serde_json::Value>(&created.body)
                .ok()
                .and_then(|json| json["id"].as_str().map(str::to_owned))
                .expect("seeded share grant id");
            ids.push(id);
        }
        let update_paths: Vec<String> = ids
            .iter()
            .map(|id| format!("/api/v0/share-grants/{id}"))
            .collect();
        let update_bodies: Vec<String> = (0..fanout)
            .map(|i| format!(r#"{{"permissions":"differential-level-{i}"}}"#))
            .collect();
        let responses = futures_util::future::join_all(
            update_paths
                .iter()
                .zip(update_bodies.iter())
                .map(|(path, body)| crate::route_http_request("PUT", path, None, body, &state)),
        )
        .await;
        let all_ok = responses.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status.starts_with('2'))
        });
        let persisted = db.list_share_grants(20, 0).await.unwrap_or_default();
        let expected: std::collections::BTreeMap<String, String> = ids
            .iter()
            .enumerate()
            .map(|(i, id)| (id.clone(), format!("differential-level-{i}")))
            .collect();
        let each_own_value = persisted.len() == fanout
            && persisted
                .iter()
                .all(|record| expected.get(&record.id) == Some(&record.permissions));
        record!(target, "ShareGrants", all_ok && each_own_value);
    }

    // ShareGroups: seed N groups, then concurrent updates each
    // group's own distinct new name.
    {
        let target = "slskdn";
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
        let mut ids = Vec::new();
        for i in 0..fanout {
            let created = crate::route_http_request(
                "POST",
                "/api/v0/sharegroups",
                None,
                &format!(r#"{{"name":"differential-concurrent-group-{i}"}}"#),
                &state,
            )
            .await
            .expect("seed differential share group");
            let id = serde_json::from_str::<serde_json::Value>(&created.body)
                .ok()
                .and_then(|json| json["id"].as_str().map(str::to_owned))
                .expect("seeded share group id");
            ids.push(id);
        }
        let update_paths: Vec<String> = ids
            .iter()
            .map(|id| format!("/api/v0/sharegroups/{id}"))
            .collect();
        let update_bodies: Vec<String> = (0..fanout)
            .map(|i| format!(r#"{{"name":"differential-concurrent-group-{i}-renamed"}}"#))
            .collect();
        let responses = futures_util::future::join_all(
            update_paths
                .iter()
                .zip(update_bodies.iter())
                .map(|(path, body)| crate::route_http_request("PUT", path, None, body, &state)),
        )
        .await;
        let all_ok = responses.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status.starts_with('2'))
        });
        let persisted = db.list_share_groups(20, 0).await.unwrap_or_default();
        let expected: std::collections::BTreeMap<String, String> = ids
            .iter()
            .enumerate()
            .map(|(i, id)| {
                (
                    id.clone(),
                    format!("differential-concurrent-group-{i}-renamed"),
                )
            })
            .collect();
        let each_own_value = persisted.len() == fanout
            && persisted
                .iter()
                .all(|record| expected.get(&record.id) == Some(&record.name));
        record!(target, "ShareGroups", all_ok && each_own_value);
    }

    // Domains declared by both targets.
    for target in ["slskd", "slskdn"] {
        // Searches: concurrent creates of distinct rows.
        {
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
            state.session.write().await.state = "connected";
            let bodies: Vec<String> = (0..fanout)
                .map(|i| format!(r#"{{"query":"differential concurrent query {i}"}}"#))
                .collect();
            let responses = futures_util::future::join_all(bodies.iter().map(|body| {
                crate::route_http_request("POST", "/api/v0/searches", None, body, &state)
            }))
            .await;
            let all_ok = responses.iter().all(|response| {
                response
                    .as_ref()
                    .is_ok_and(|response| response.status.starts_with('2'))
            });
            let persisted = db.list_searches(20, 0).await.unwrap_or_default();
            let expected: std::collections::BTreeSet<String> = (0..fanout)
                .map(|i| format!("differential concurrent query {i}"))
                .collect();
            let persisted_queries: std::collections::BTreeSet<String> = persisted
                .iter()
                .map(|record| record.query.clone())
                .collect();
            record!(
                target,
                "Searches",
                all_ok && persisted.len() == fanout && persisted_queries == expected
            );
        }

        // Conversations / PrivateMessages: concurrent creates of
        // distinct messages to distinct peers, both frozen EF domain
        // names mapping to slskR's single consolidated `messages`
        // table/store.
        {
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
            let paths: Vec<String> = (0..fanout)
                .map(|i| format!("/api/conversations/differential-concurrent-peer-{i}"))
                .collect();
            let bodies: Vec<String> = (0..fanout)
                .map(|i| format!(r#"{{"body":"differential concurrent message {i}"}}"#))
                .collect();
            let responses =
                futures_util::future::join_all(paths.iter().zip(bodies.iter()).map(
                    |(path, body)| crate::route_http_request("POST", path, None, body, &state),
                ))
                .await;
            let all_ok = responses.iter().all(|response| {
                response
                    .as_ref()
                    .is_ok_and(|response| response.status.starts_with('2'))
            });
            let persisted = db.list_messages(20, 0).await.unwrap_or_default();
            let expected: std::collections::BTreeSet<String> = (0..fanout)
                .map(|i| format!("differential concurrent message {i}"))
                .collect();
            let persisted_bodies: std::collections::BTreeSet<String> = persisted
                .iter()
                .map(|record| record.content.clone())
                .collect();
            let pass = all_ok && persisted.len() == fanout && persisted_bodies == expected;
            for domain in ["Conversations", "PrivateMessages"] {
                record!(target, domain, pass);
            }
        }

        // Events: no HTTP route exists in either frozen registry
        // (confirmed: `/api/events` has zero registry entry), so
        // this proof uses the same internal `record_event` call the
        // existing `create-and-read-roundtrip` differential already
        // uses as its own real write path -- concurrent internal
        // calls against the same `AppState`/`DatabaseManager`.
        {
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
            let calls = (0..fanout).map(|i| {
                crate::record_event(
                    &state,
                    "differential.concurrent",
                    format!("differential-concurrent-resource-{i}"),
                    None,
                )
            });
            futures_util::future::join_all(calls).await;
            let persisted = db.list_events(20, 0).await.unwrap_or_default();
            let expected: std::collections::BTreeSet<String> = (0..fanout)
                .map(|i| format!("differential-concurrent-resource-{i}"))
                .collect();
            let persisted_resources: std::collections::BTreeSet<String> = persisted
                .iter()
                .map(|record| record.resource.clone())
                .collect();
            record!(
                target,
                "Events",
                persisted.len() == fanout && persisted_resources == expected
            );
        }
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("persistence-lifecycle");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("covered_domains_transaction_and_concurrency_atomicity.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize persistence-lifecycle ledger"),
    )
    .expect("write persistence-lifecycle ledger");

    assert!(
        mismatches.is_empty(),
        "{} persistence-lifecycle transaction-and-concurrency-atomicity mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting `corrupt-state-and-upgrade-
/// failure` for 10 of the 13 domains already touched by earlier
/// persistence-lifecycle batches. `ShareGroupMembers` is excluded:
/// its table has a composite primary key (`group_id`, `username`),
/// not a single `id` column, so the "corrupt exactly one row by id"
/// technique below doesn't directly apply. `Transfers` is excluded
/// for the same reason as every other case in this workstream --
/// its only "creation" route has no registry entry in either
/// frozen target.
///
/// Design: create one real row via the normal dispatch path, then
/// directly corrupt one of its `INTEGER NOT NULL` columns via a new
/// `DatabaseManager::execute_raw_for_test` raw-SQL escape hatch.
/// SQLite's weak column typing lets a value that could never come
/// from a real typed insert (a non-numeric string landing in an
/// INTEGER column) persist without SQLite itself rejecting it at
/// write time -- a realistic stand-in for the kind of row a botched
/// manual edit, an interrupted external tool, or a real
/// schema-upgrade bug could leave behind. Then call the REAL
/// `list_*` method `serve()` itself uses for startup rehydration
/// (the `collection_store`/`search_store`/etc. blocks
/// gated behind `.map_err(|error| format!("failed to load
/// persisted ...: {error}"))?` -- confirmed by reading that real
/// startup code before designing this proof) and assert it returns
/// a clean `Err`, not a silently wrong value and not a panic --
/// proving corrupted state fails the exact way production startup
/// already handles it: a typed, recoverable error the caller
/// already propagates, not an unhandled crash.
///
/// Confirmed against `/tmp/slskr-parity-evidence/persistence-
/// lifecycle/*.json` before writing: `corrupt-state-and-upgrade-
/// failure` was 100% untouched for every domain in the whole
/// workstream's history -- the last of the 6 case names to be
/// opened at all.
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
    feature = "bounded-persistence-tests"
))]
pub(super) async fn persistence_lifecycle_differential_covered_domains_corrupt_state_and_upgrade_failure(
) {
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($target:expr, $domain:expr, $pass:expr) => {
            let pass = $pass;
            if !pass {
                mismatches.push(format!(
                    "{} {} corrupt-state-and-upgrade-failure",
                    $target, $domain
                ));
            }
            ledger.push(serde_json::json!({
                "target": $target,
                "domain": $domain,
                "case": "corrupt-state-and-upgrade-failure",
                "pass": pass,
            }));
        };
    }

    // Collections.
    {
        let target = "slskdn";
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
        let created = crate::route_http_request(
            "POST",
            "/api/v0/collections",
            None,
            r#"{"name":"Differential Corrupt Collection"}"#,
            &state,
        )
        .await
        .expect("create differential collection");
        let id = serde_json::from_str::<serde_json::Value>(&created.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("differential collection id");
        db.execute_raw_for_test(&format!(
            "UPDATE collections SET created_at = 'not-a-number' WHERE id = '{id}'"
        ))
        .await
        .expect("corrupt collections row");
        record!(
            target,
            "Collections",
            db.list_collections(10, 0).await.is_err()
        );
    }

    // CollectionItems.
    {
        let target = "slskdn";
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
        let collection = crate::route_http_request(
            "POST",
            "/api/v0/collections",
            None,
            r#"{"name":"Differential Corrupt Item Parent"}"#,
            &state,
        )
        .await
        .expect("create parent collection");
        let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("parent collection id");
        let item = crate::route_http_request(
            "POST",
            &format!("/api/v0/collections/{collection_id}/items"),
            None,
            r#"{"content_id":"differential-corrupt-track","artist":"Differential Artist","title":"Track","kind":"Audio"}"#,
            &state,
        )
        .await
        .expect("create differential collection item");
        let item_id = serde_json::from_str::<serde_json::Value>(&item.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("differential collection item id");
        db.execute_raw_for_test(&format!(
            "UPDATE collection_items SET added_at = 'not-a-number' WHERE id = '{item_id}'"
        ))
        .await
        .expect("corrupt collection_items row");
        record!(
            target,
            "CollectionItems",
            db.list_collection_items(10, 0).await.is_err()
        );
    }

    // UserNotes.
    {
        let target = "slskdn";
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
        let created = crate::route_http_request(
            "POST",
            "/api/users/notes",
            None,
            r#"{"username":"differential-corrupt-friend","note":"note"}"#,
            &state,
        )
        .await
        .expect("create differential user note");
        let id = serde_json::from_str::<serde_json::Value>(&created.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("differential note id");
        db.execute_raw_for_test(&format!(
            "UPDATE user_notes SET created_at = 'not-a-number' WHERE id = '{id}'"
        ))
        .await
        .expect("corrupt user_notes row");
        record!(
            target,
            "UserNotes",
            db.list_user_notes(10, 0).await.is_err()
        );
    }

    // WishlistItems.
    {
        let target = "slskdn";
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
        let created = crate::route_http_request(
            "POST",
            "/api/v0/wishlist",
            None,
            r#"{"artist":"Differential Artist","title":"Differential Corrupt Track","kind":"Audio"}"#,
            &state,
        )
        .await
        .expect("create differential wishlist item");
        let id = serde_json::from_str::<serde_json::Value>(&created.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("differential wishlist id");
        db.execute_raw_for_test(&format!(
            "UPDATE wishlist_items SET added_at = 'not-a-number' WHERE id = '{id}'"
        ))
        .await
        .expect("corrupt wishlist_items row");
        record!(
            target,
            "WishlistItems",
            db.list_wishlist_items(10, 0).await.is_err()
        );
    }

    // Contacts.
    {
        let target = "slskdn";
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
        let created = crate::route_http_request(
            "POST",
            "/api/contacts",
            None,
            r#"{"username":"differential-corrupt-contact"}"#,
            &state,
        )
        .await
        .expect("create differential contact");
        let id = serde_json::from_str::<serde_json::Value>(&created.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("differential contact id");
        db.execute_raw_for_test(&format!(
            "UPDATE contacts SET created_at = 'not-a-number' WHERE id = '{id}'"
        ))
        .await
        .expect("corrupt contacts row");
        record!(target, "Contacts", db.list_contacts(10, 0).await.is_err());
    }

    // ShareGrants.
    {
        let target = "slskdn";
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
        let collection = crate::route_http_request(
            "POST",
            "/api/v0/collections",
            None,
            r#"{"name":"Differential Corrupt Grant Parent"}"#,
            &state,
        )
        .await
        .expect("create parent grant collection");
        let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("parent grant collection id");
        let created = crate::route_http_request(
            "POST",
            "/api/v0/share-grants",
            None,
            &format!(
                r#"{{"collection_id":"{collection_id}","username":"differential-corrupt-grantee"}}"#
            ),
            &state,
        )
        .await
        .expect("create differential share grant");
        let id = serde_json::from_str::<serde_json::Value>(&created.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("differential grant id");
        db.execute_raw_for_test(&format!(
            "UPDATE share_grants SET shared_at = 'not-a-number' WHERE id = '{id}'"
        ))
        .await
        .expect("corrupt share_grants row");
        record!(
            target,
            "ShareGrants",
            db.list_share_grants(10, 0).await.is_err()
        );
    }

    // ShareGroups.
    {
        let target = "slskdn";
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
        let created = crate::route_http_request(
            "POST",
            "/api/v0/sharegroups",
            None,
            r#"{"name":"Differential Corrupt Group"}"#,
            &state,
        )
        .await
        .expect("create differential share group");
        let id = serde_json::from_str::<serde_json::Value>(&created.body)
            .ok()
            .and_then(|json| json["id"].as_str().map(str::to_owned))
            .expect("differential share group id");
        db.execute_raw_for_test(&format!(
            "UPDATE share_groups SET created_at = 'not-a-number' WHERE id = '{id}'"
        ))
        .await
        .expect("corrupt share_groups row");
        record!(
            target,
            "ShareGroups",
            db.list_share_groups(10, 0).await.is_err()
        );
    }

    // ShareGroupMembers has a composite primary key, so corrupt the
    // typed timestamp while addressing the row by both key columns.
    // This is the same real table/loader path as the other corruption
    // checks, without pretending the member table has a synthetic id.
    {
        let target = "slskdn";
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let group = crate::persistence::ShareGroupRecord {
            id: "differential-corrupt-member-group".to_owned(),
            name: "Differential Corrupt Member Group".to_owned(),
            description: String::new(),
            created_at: crate::unix_timestamp() as i64,
            updated_at: crate::unix_timestamp() as i64,
        };
        db.upsert_share_group(&group)
            .await
            .expect("create differential member group");
        db.upsert_share_group_member(&crate::persistence::ShareGroupMemberRecord {
            group_id: group.id.clone(),
            username: "differential-corrupt-member".to_owned(),
            added_at: crate::unix_timestamp() as i64,
        })
        .await
        .expect("create differential share group member");
        db.execute_raw_for_test(
            "UPDATE share_group_members SET added_at = 'not-a-number' \
             WHERE group_id = 'differential-corrupt-member-group' \
             AND username = 'differential-corrupt-member'",
        )
        .await
        .expect("corrupt share_group_members row");
        record!(
            target,
            "ShareGroupMembers",
            db.list_share_group_members(10, 0).await.is_err()
        );
    }

    // Domains declared by both targets.
    for target in ["slskd", "slskdn"] {
        // Searches.
        {
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
            let created = crate::route_http_request(
                "POST",
                "/api/v0/searches",
                None,
                r#"{"query":"differential corrupt query"}"#,
                &state,
            )
            .await
            .expect("create differential search");
            let id = serde_json::from_str::<serde_json::Value>(&created.body)
                .ok()
                .and_then(|json| json["id"].as_str().map(str::to_owned));
            let id = match id {
                Some(id) => id,
                None => state
                    .searches
                    .read()
                    .await
                    .records
                    .first()
                    .map(|record| record.id.clone())
                    .expect("differential search id"),
            };
            db.execute_raw_for_test(&format!(
                "UPDATE searches SET created_at = 'not-a-number' WHERE id = '{id}'"
            ))
            .await
            .expect("corrupt searches row");
            record!(target, "Searches", db.list_searches(10, 0).await.is_err());
        }

        // Conversations / PrivateMessages: both frozen EF domain
        // names map to slskR's single consolidated `messages`
        // table/store.
        {
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
            crate::route_http_request(
                "POST",
                "/api/conversations/differential-corrupt-peer",
                None,
                r#"{"body":"differential corrupt message"}"#,
                &state,
            )
            .await
            .expect("create differential message");
            let id = state
                .messages
                .read()
                .await
                .records
                .first()
                .map(|record| record.id)
                .expect("differential message id");
            db.execute_raw_for_test(&format!(
                "UPDATE messages SET created_at = 'not-a-number' WHERE id = '{id}'"
            ))
            .await
            .expect("corrupt messages row");
            let pass = db.list_messages(10, 0).await.is_err();
            for domain in ["Conversations", "PrivateMessages"] {
                record!(target, domain, pass);
            }
        }

        // Events: no HTTP route exists in either frozen registry, so
        // this proof uses the same internal `record_event` call the
        // existing `create-and-read-roundtrip` differential already
        // uses as its own real write path. `events.id` is a real
        // `INTEGER PRIMARY KEY`, not a UUID string, so the raw SQL
        // targets it unquoted.
        {
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
            crate::record_event(
                &state,
                "differential.corrupt",
                "differential-corrupt-resource",
                None,
            )
            .await;
            let id = db
                .list_events(10, 0)
                .await
                .expect("list events before corruption")
                .first()
                .map(|record| record.id)
                .expect("differential event id");
            db.execute_raw_for_test(&format!(
                "UPDATE events SET created_at = 'not-a-number' WHERE id = {id}"
            ))
            .await
            .expect("corrupt events row");
            record!(target, "Events", db.list_events(10, 0).await.is_err());
        }
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("persistence-lifecycle");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("covered_domains_corrupt_state_and_upgrade_failure.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize persistence-lifecycle ledger"),
    )
    .expect("write persistence-lifecycle ledger");

    assert!(
        mismatches.is_empty(),
        "{} persistence-lifecycle corrupt-state-and-upgrade-failure mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
/// Differential lifecycle proof for the frozen slskd Transfers database
/// `Batches` domain.  This deliberately exercises the dedicated SQLite
/// table rather than the generic controller-feature JSON fallback.
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
    feature = "bounded-persistence-tests"
))]
pub(super) async fn persistence_lifecycle_differential_controller_batches_domain() {
    let target = "slskd";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!("{target} Batches {}", $case));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "domain": "Batches",
                "case": $case,
                "pass": pass,
            }));
        }};
    }

    let schema_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("batches schema database");
    record!(
        "schema-create-and-migrate",
        schema_db
            .get_transfer_batch("aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa")
            .await
            .is_ok_and(|record| record.is_none())
    );

    let create_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("batches controller database");
    let env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKR_PERSISTENCE_ENABLED", "true")
        .with(
            "SLSKR_TEST_USER_ENDPOINT_OVERRIDES",
            "batches-roundtrip-peer=127.0.0.1:2234",
        );
    let (state, _receiver) =
        test_state_with_env_parts(env, crate::SearchStore::new(), Some(create_db.clone()));
    let batch_id = "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb";
    let created = crate::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/batches",
        None,
        &format!(
            r#"{{"id":"{batch_id}","username":"batches-roundtrip-peer","searchId":"cccccccc-cccc-4ccc-8ccc-cccccccccccc","files":[{{"filename":"Batches/Track.flac","size":42}}],"options":{{"destination":"Albums"}}}}"#
        ),
        &state,
    )
    .await
    .expect("create durable transfer batch");
    let stored = create_db
        .get_transfer_batch(batch_id)
        .await
        .expect("read durable batch")
        .expect("durable batch row");
    let fetched = crate::route_http_request(
        "GET",
        &format!("/api/v0/transfers/downloads/batches/{batch_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("read durable batch through controller");
    let fetched_json = serde_json::from_str::<serde_json::Value>(&fetched.body).ok();
    record!(
        "create-and-read-roundtrip",
        created.status == "201 Created"
            && stored.username == "batches-roundtrip-peer"
            && stored.search_id.as_deref() == Some("cccccccc-cccc-4ccc-8ccc-cccccccccccc")
            && stored.options_json.as_deref() == Some(r#"{"destination":"Albums"}"#)
            && fetched.status == "200 OK"
            && fetched_json.as_ref().is_some_and(|json| {
                json["id"] == batch_id
                    && json["searchId"] == "cccccccc-cccc-4ccc-8ccc-cccccccccccc"
                    && json["options"]["destination"] == "Albums"
                    && json["transfers"]
                        .as_array()
                        .is_some_and(|rows| rows.len() == 1)
            })
    );

    let mut updated = stored.clone();
    updated.search_id = None;
    updated.username = "batches-updated-peer".to_owned();
    updated.options_json = Some(r#"{"destination":"Updated"}"#.to_owned());
    let update_result = create_db.update_transfer_batch(&updated).await;
    let after_update = create_db
        .get_transfer_batch(batch_id)
        .await
        .expect("read updated durable batch");
    let delete_result = create_db
        .delete_transfer_batch(batch_id)
        .await
        .expect("delete durable batch");
    let after_delete = create_db
        .get_transfer_batch(batch_id)
        .await
        .expect("read deleted durable batch");
    record!(
        "update-delete-and-readback",
        update_result.is_ok()
            && after_update.as_ref().is_some_and(|record| {
                record.username == "batches-updated-peer"
                    && record.search_id.is_none()
                    && record.options_json.as_deref() == Some(r#"{"destination":"Updated"}"#)
            })
            && delete_result
            && after_delete.is_none()
    );

    let restart_path = std::env::temp_dir().join(format!(
        "slskr-batches-restart-{}-{}.sqlite",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    let restart_id = "dddddddd-dddd-4ddd-8ddd-dddddddddddd";
    {
        let restart_db = crate::persistence::DatabaseManager::new(
            restart_path.to_str().expect("restart database path"),
        )
        .await
        .expect("create file-backed batches database");
        restart_db
            .insert_transfer_batch(&crate::persistence::TransferBatchRecord {
                id: restart_id.to_owned(),
                search_id: None,
                username: "batches-restart-peer".to_owned(),
                direction: 0,
                created_at: "2026-08-12T00:00:00Z".to_owned(),
                options_json: Some(r#"{"destination":"Restarted"}"#.to_owned()),
            })
            .await
            .expect("persist restart batch");
    }
    let reopened = crate::persistence::DatabaseManager::new(
        restart_path.to_str().expect("reopen database path"),
    )
    .await
    .expect("reopen file-backed batches database");
    let rehydrated = reopened
        .get_transfer_batch(restart_id)
        .await
        .expect("read rehydrated batch");
    record!(
        "restart-rehydration",
        rehydrated.is_some_and(|record| {
            record.username == "batches-restart-peer"
                && record.options_json.as_deref() == Some(r#"{"destination":"Restarted"}"#)
        })
    );
    drop(reopened);
    let _ = fs::remove_file(&restart_path);

    let concurrent_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("batches concurrency database");
    let concurrent_record = crate::persistence::TransferBatchRecord {
        id: "eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee".to_owned(),
        search_id: None,
        username: "batches-concurrent-peer".to_owned(),
        direction: 0,
        created_at: "2026-08-12T00:00:00Z".to_owned(),
        options_json: Some("{}".to_owned()),
    };
    let concurrent_results = futures_util::future::join_all(
        (0..2).map(|_| concurrent_db.insert_transfer_batch(&concurrent_record)),
    )
    .await;
    let concurrent_read = concurrent_db
        .get_transfer_batch(&concurrent_record.id)
        .await
        .expect("read concurrent batch");
    record!(
        "transaction-and-concurrency-atomicity",
        concurrent_results
            .iter()
            .filter(|result| result.is_ok())
            .count()
            == 1
            && concurrent_results
                .iter()
                .filter(|result| result.is_err())
                .count()
                == 1
            && concurrent_read.is_some()
    );

    let corrupt_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("batches corrupt-state database");
    corrupt_db
        .execute_raw_for_test(
            "INSERT INTO Batches (Id, SearchId, Username, Direction, CreatedAt, Options) VALUES ('ffffffff-ffff-4fff-8fff-ffffffffffff', NULL, 'corrupt-peer', 0, '2026-08-12T00:00:00Z', 'not-json')",
        )
        .await
        .expect("insert corrupt batch options");
    let (corrupt_state, _receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(corrupt_db),
    );
    let corrupt_response = crate::route_http_request(
        "GET",
        "/api/v0/transfers/downloads/batches/ffffffff-ffff-4fff-8fff-ffffffffffff",
        None,
        "",
        &corrupt_state,
    )
    .await
    .expect("corrupt batch response");
    record!(
        "corrupt-state-and-upgrade-failure",
        corrupt_response.status == "500 Internal Server Error"
            && corrupt_response.body.contains("options")
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("persistence-lifecycle");
    fs::create_dir_all(&evidence_dir).expect("create persistence evidence directory");
    fs::write(
        evidence_dir.join("batches_domain_lifecycle.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize batches persistence ledger"),
    )
    .expect("write batches persistence ledger");
    assert!(
        mismatches.is_empty(),
        "{} Batches persistence mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential lifecycle proof for the frozen slskd Shares `files`
/// domain.  slskR keeps the durable share projection in its own
/// `share_files` table; this exercises that production snapshot,
/// rehydration, replacement, restart, transaction, and failure paths
/// without claiming parity for the frozen repository's derived directory,
/// FTS, or scan-history tables.
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
    feature = "bounded-persistence-tests"
))]
pub(super) async fn persistence_lifecycle_differential_controller_share_files_domain() {
    let target = "slskd";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!("{target} files {}", $case));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "domain": "files",
                "case": $case,
                "pass": pass,
            }));
        }};
    }

    let schema_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("share files schema database");
    record!(
        "schema-create-and-migrate",
        schema_db
            .list_share_files(10, 0)
            .await
            .is_ok_and(|records| records.is_empty())
    );

    let base = crate::persistence::ShareFileRecord {
        filename: "Virtual/Track.flac".to_owned(),
        size: 42,
        extension: "flac".to_owned(),
        root_label: "Virtual".to_owned(),
        local_path: Some("/library/Track.flac".to_owned()),
        attributes_json: "1:42".to_owned(),
        updated_at: 1,
    };
    let stale = crate::persistence::ShareFileRecord {
        filename: "Virtual/Stale.flac".to_owned(),
        size: 7,
        extension: "flac".to_owned(),
        root_label: "Virtual".to_owned(),
        local_path: Some("/library/Stale.flac".to_owned()),
        attributes_json: "".to_owned(),
        updated_at: 1,
    };
    let initial = vec![base.clone(), stale.clone()];
    let create_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("share files lifecycle database");
    create_db
        .replace_share_files(&initial)
        .await
        .expect("persist initial share files snapshot");
    let stored = create_db
        .list_share_files(10, 0)
        .await
        .expect("read initial share files snapshot");
    record!(
        "create-and-read-roundtrip",
        stored.len() == 2
            && stored.iter().any(|record| {
                record.filename == base.filename
                    && record.size == base.size
                    && record.local_path == base.local_path
                    && record.attributes_json == base.attributes_json
            })
    );

    let updated = crate::persistence::ShareFileRecord {
        size: 84,
        local_path: Some("/library/Track-remastered.flac".to_owned()),
        attributes_json: "1:84,2:320".to_owned(),
        updated_at: 2,
        ..base.clone()
    };
    create_db
        .replace_share_files(std::slice::from_ref(&updated))
        .await
        .expect("replace share files snapshot");
    let after_update = create_db
        .list_share_files(10, 0)
        .await
        .expect("read replaced share files snapshot");
    record!(
        "update-delete-and-readback",
        after_update.len() == 1
            && after_update[0].filename == updated.filename
            && after_update[0].size == 84
            && after_update[0].local_path == updated.local_path
            && after_update[0].attributes_json == updated.attributes_json
            && !after_update
                .iter()
                .any(|record| record.filename == stale.filename)
    );

    let restart_path = std::env::temp_dir().join(format!(
        "slskr-share-files-restart-{}-{}.sqlite",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    {
        let restart_db = crate::persistence::DatabaseManager::new(
            restart_path.to_str().expect("share files restart path"),
        )
        .await
        .expect("create file-backed share files database");
        restart_db
            .replace_share_files(std::slice::from_ref(&base))
            .await
            .expect("persist share file for restart");
    }
    let reopened = crate::persistence::DatabaseManager::new(
        restart_path
            .to_str()
            .expect("reopen share files restart path"),
    )
    .await
    .expect("reopen file-backed share files database");
    let rehydrated = reopened
        .list_share_files(10, 0)
        .await
        .expect("read rehydrated share file");
    record!(
        "restart-rehydration",
        rehydrated.len() == 1
            && rehydrated[0].filename == base.filename
            && rehydrated[0].size == base.size
            && rehydrated[0].root_label == base.root_label
    );
    drop(reopened);
    let _ = fs::remove_file(&restart_path);

    let concurrent_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("share files concurrency database");
    let concurrent_a = crate::persistence::ShareFileRecord {
        filename: "Virtual/Concurrent-A.flac".to_owned(),
        ..base.clone()
    };
    let concurrent_b = crate::persistence::ShareFileRecord {
        filename: "Virtual/Concurrent-B.flac".to_owned(),
        ..base.clone()
    };
    let concurrent_results = futures_util::future::join_all(
        vec![
            (concurrent_db.clone(), vec![concurrent_a.clone()]),
            (concurrent_db.clone(), vec![concurrent_b.clone()]),
        ]
        .into_iter()
        .map(|(db, records)| async move { db.replace_share_files(&records).await }),
    )
    .await;
    let concurrent_rows = concurrent_db
        .list_share_files(10, 0)
        .await
        .expect("read concurrent share files snapshot");
    record!(
        "transaction-and-concurrency-atomicity",
        concurrent_results.iter().all(Result::is_ok)
            && concurrent_rows.len() == 1
            && matches!(
                concurrent_rows[0].filename.as_str(),
                "Virtual/Concurrent-A.flac" | "Virtual/Concurrent-B.flac"
            )
    );

    let corrupt_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("share files corrupt-state database");
    corrupt_db
        .replace_share_files(std::slice::from_ref(&base))
        .await
        .expect("persist share file before corruption");
    corrupt_db
        .execute_raw_for_test("UPDATE share_files SET size = 'not-a-number'")
        .await
        .expect("corrupt share file size");
    record!(
        "corrupt-state-and-upgrade-failure",
        corrupt_db.list_share_files(10, 0).await.is_err()
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("persistence-lifecycle");
    fs::create_dir_all(&evidence_dir).expect("create persistence evidence directory");
    fs::write(
        evidence_dir.join("share_files_domain_lifecycle.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize share files persistence ledger"),
    )
    .expect("write share files persistence ledger");
    assert!(
        mismatches.is_empty(),
        "{} files persistence mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential lifecycle proof for the frozen `Transfers` domain.  The
/// transfer controller creates rows through search/download workflows in
/// production, so this proof uses the same durable `TransferRecord` write
/// path and its real read/update/delete/reopen methods instead of inventing
/// a REST creation endpoint that neither frozen registry declares.
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
    feature = "bounded-persistence-tests"
))]
pub(super) async fn persistence_lifecycle_differential_transfers_domain_full_lifecycle() {
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($target:expr, $case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!("{} Transfers {}", $target, $case));
            }
            ledger.push(serde_json::json!({
                "target": $target,
                "domain": "Transfers",
                "case": $case,
                "pass": pass,
            }));
        }};
    }

    for target in ["slskd", "slskdn"] {
        let schema_db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("transfer schema database");
        record!(
            target,
            "schema-create-and-migrate",
            schema_db
                .list_transfers(None, 10, 0)
                .await
                .is_ok_and(|records| records.is_empty())
        );

        let base = crate::persistence::TransferRecord {
            id: format!("transfer-roundtrip-{target}"),
            direction: "download".to_owned(),
            filename: "Persistence/Transfer.flac".to_owned(),
            peer_username: "transfer-peer".to_owned(),
            filesize: 4096,
            progress: 512,
            status: "queued".to_owned(),
            started_at: 1_700_000_000,
            completed_at: None,
            request_id: Some("request-1".to_owned()),
            wishlist_item_id: None,
            request_name: Some("transfer-proof".to_owned()),
            destination_directory: Some("Albums".to_owned()),
            local_path: None,
            batch_id: None,
            reason: None,
            bit_rate: Some(320),
            sample_rate: Some(44_100),
            bit_depth: Some(16),
            length_seconds: Some(12),
            artist: Some("Parity".to_owned()),
            album: Some("Lifecycle".to_owned()),
            title: Some("Transfer".to_owned()),
            track_number: Some(1),
            year: Some(2026),
            attempts: 1,
            auto_replace_attempts: 0,
            next_attempt_at: None,
            updated_at_ms: 1_700_000_000_000,
        };

        let create_db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("transfer lifecycle database");
        create_db
            .insert_transfer(&base)
            .await
            .expect("insert transfer lifecycle row");
        let stored = create_db
            .get_transfer(&base.id)
            .await
            .expect("read transfer lifecycle row");
        record!(
            target,
            "create-and-read-roundtrip",
            stored.as_ref().is_some_and(|row| {
                row.filename == base.filename
                    && row.progress == 512
                    && row.request_name.as_deref() == Some("transfer-proof")
            })
        );

        create_db
            .update_transfer_progress(&base.id, 2048, base.updated_at_ms as u64 + 1)
            .await
            .expect("update transfer progress");
        let updated = create_db
            .get_transfer(&base.id)
            .await
            .expect("read updated transfer");
        create_db
            .delete_transfer(&base.id)
            .await
            .expect("delete transfer lifecycle row");
        let deleted = create_db
            .get_transfer(&base.id)
            .await
            .expect("read deleted transfer");
        record!(
            target,
            "update-delete-and-readback",
            updated.is_some_and(|row| row.progress == 2048) && deleted.is_none()
        );

        let restart_path = std::env::temp_dir().join(format!(
            "slskr-transfer-lifecycle-{target}-{}-{}.sqlite",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        {
            let restart_db = crate::persistence::DatabaseManager::new(
                restart_path.to_str().expect("transfer restart path"),
            )
            .await
            .expect("create transfer restart database");
            restart_db
                .insert_transfer(&base)
                .await
                .expect("persist transfer for restart");
        }
        let reopened = crate::persistence::DatabaseManager::new(
            restart_path.to_str().expect("reopen transfer restart path"),
        )
        .await
        .expect("reopen transfer restart database");
        let rehydrated = reopened
            .get_transfer(&base.id)
            .await
            .expect("read rehydrated transfer");
        record!(
            target,
            "restart-rehydration",
            rehydrated.is_some_and(|row| row.peer_username == "transfer-peer")
        );
        drop(reopened);
        let _ = std::fs::remove_file(&restart_path);

        let concurrent_db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("transfer concurrency database");
        let concurrent_writes = futures_util::future::join_all((0..8).map(|index| {
            let mut row = base.clone();
            row.id = format!("transfer-concurrent-{target}-{index}");
            let db = concurrent_db.clone();
            async move { db.insert_transfer(&row).await }
        }))
        .await;
        let concurrent_rows = concurrent_db
            .list_transfers(None, 32, 0)
            .await
            .expect("read concurrent transfers");
        record!(
            target,
            "transaction-and-concurrency-atomicity",
            concurrent_writes.iter().all(Result::is_ok)
                && concurrent_rows.len() == 8
                && concurrent_rows
                    .iter()
                    .all(|row| row.filename == "Persistence/Transfer.flac")
        );

        let corrupt_db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("transfer corrupt-state database");
        corrupt_db
            .insert_transfer(&base)
            .await
            .expect("insert transfer before corruption");
        corrupt_db
            .execute_raw_for_test(&format!(
                "UPDATE transfers SET progress = 'not-a-number' WHERE id = '{}'",
                base.id
            ))
            .await
            .expect("corrupt transfer progress");
        record!(
            target,
            "corrupt-state-and-upgrade-failure",
            corrupt_db.list_transfers(None, 10, 0).await.is_err()
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("persistence-lifecycle");
    fs::create_dir_all(&evidence_dir).expect("create persistence evidence directory");
    fs::write(
        evidence_dir.join("transfers_domain_full_lifecycle.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize transfer lifecycle evidence"),
    )
    .expect("write transfer lifecycle evidence");
    assert!(
        mismatches.is_empty(),
        "{} transfer persistence mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential lifecycle proof for the frozen slskdN HashDb and
/// HashDbState tables.  Controller writes use the same SQLite snapshot
/// transaction that startup rehydrates, while state parsing failures are
/// exercised through the backfill route rather than a raw-only assertion.
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
    feature = "bounded-persistence-tests"
))]
pub(super) async fn persistence_lifecycle_differential_native_hashdb_domains() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($domain:expr, $case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!("{target} {} {}", $domain, $case));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "domain": $domain,
                "case": $case,
                "pass": pass,
            }));
        }};
    }

    let schema_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("HashDb schema database");
    record!(
        "HashDb",
        "schema-create-and-migrate",
        schema_db
            .get_hash_db_entry("schema-missing")
            .await
            .is_ok_and(|entry| entry.is_none())
    );
    record!(
        "HashDbState",
        "schema-create-and-migrate",
        schema_db
            .get_hash_db_state("schema-missing")
            .await
            .is_ok_and(|state| state.is_none())
    );

    let create_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("HashDb controller database");
    let env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKR_PERSISTENCE_ENABLED", "true");
    let (state, _receiver) =
        test_state_with_env_parts(env, crate::SearchStore::new(), Some(create_db.clone()));
    let filename = "Persistence/HashDb.flac";
    let hash_key = crate::content_discovery::generate_flac_key(filename, 4096);
    let byte_hash = "a".repeat(64);
    let stored = crate::route_http_request(
        "POST",
        "/api/v0/hashdb/hash",
        None,
        &serde_json::json!({
            "filename": filename,
            "byteHash": byte_hash,
            "size": 4096,
        })
        .to_string(),
        &state,
    )
    .await
    .expect("store HashDb entry through controller");
    let persisted_hash = create_db
        .get_hash_db_entry(&hash_key)
        .await
        .expect("read controller-persisted HashDb entry");
    let persisted_state = create_db
        .get_hash_db_state("latest_seq")
        .await
        .expect("read controller-persisted HashDb state");
    let read_back = crate::route_http_request(
        "GET",
        &format!("/api/v0/hashdb/hash/{hash_key}"),
        None,
        "",
        &state,
    )
    .await
    .expect("read HashDb entry through controller");
    record!(
        "HashDb",
        "create-and-read-roundtrip",
        stored.status == "200 OK"
            && persisted_hash.as_ref().is_some_and(|entry| {
                entry.flac_key == hash_key
                    && entry.byte_hash == byte_hash
                    && entry.size == 4096
                    && entry.seq_id == 1
            })
            && read_back.status == "200 OK"
            && read_back.body.contains(&hash_key)
    );
    record!(
        "HashDbState",
        "create-and-read-roundtrip",
        persisted_state.is_some_and(|state| state.value.as_deref() == Some("1"))
    );

    let mut updated_hash = persisted_hash.expect("HashDb row for update");
    updated_hash.byte_hash = "b".repeat(64);
    updated_hash.use_count = 4;
    let hash_update = create_db.upsert_hash_db_entry(&updated_hash).await;
    let hash_after_update = create_db
        .get_hash_db_entry(&hash_key)
        .await
        .expect("read updated HashDb entry");
    let hash_deleted = create_db
        .delete_hash_db_entry(&hash_key)
        .await
        .expect("delete HashDb entry");
    let hash_after_delete = create_db
        .get_hash_db_entry(&hash_key)
        .await
        .expect("read deleted HashDb entry");
    record!(
        "HashDb",
        "update-delete-and-readback",
        hash_update.is_ok()
            && hash_after_update
                .as_ref()
                .is_some_and(|entry| { entry.byte_hash == "b".repeat(64) && entry.use_count == 4 })
            && hash_deleted
            && hash_after_delete.is_none()
    );

    let state_key = "backfill_progress";
    let state_update = create_db
        .upsert_hash_db_state(&crate::persistence::HashDbStateRecord {
            key: state_key.to_owned(),
            value: Some(r#"{"lastProcessedAt":12}"#.to_owned()),
        })
        .await;
    let state_after_update = create_db
        .get_hash_db_state(state_key)
        .await
        .expect("read updated HashDb state");
    let state_deleted = create_db
        .delete_hash_db_state(state_key)
        .await
        .expect("delete HashDb state");
    let state_after_delete = create_db
        .get_hash_db_state(state_key)
        .await
        .expect("read deleted HashDb state");
    record!(
        "HashDbState",
        "update-delete-and-readback",
        state_update.is_ok()
            && state_after_update.is_some_and(|state| {
                state.value.as_deref() == Some(r#"{"lastProcessedAt":12}"#)
            })
            && state_deleted
            && state_after_delete.is_none()
    );

    let restart_path = std::env::temp_dir().join(format!(
        "slskr-hashdb-restart-{}-{}.sqlite",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    {
        let restart_db = crate::persistence::DatabaseManager::new(
            restart_path.to_str().expect("HashDb restart path"),
        )
        .await
        .expect("create file-backed HashDb");
        restart_db
            .replace_hash_db_snapshot(
                &[crate::persistence::HashDbRecord {
                    flac_key: "hashdb-restart-key".to_owned(),
                    byte_hash: "c".repeat(64),
                    size: 8192,
                    first_seen_at: 1,
                    last_updated_at: 2,
                    seq_id: 7,
                    use_count: 2,
                    full_file_hash: String::new(),
                    musicbrainz_id: String::new(),
                    file_sha256: String::new(),
                }],
                7,
            )
            .await
            .expect("persist restart HashDb snapshot");
        restart_db
            .upsert_hash_db_state(&crate::persistence::HashDbStateRecord {
                key: "backfill_progress".to_owned(),
                value: Some(r#"{"lastProcessedAt":7}"#.to_owned()),
            })
            .await
            .expect("persist restart HashDb state");
    }
    let reopened = crate::persistence::DatabaseManager::new(
        restart_path.to_str().expect("reopen HashDb path"),
    )
    .await
    .expect("reopen file-backed HashDb");
    let restart_hash = reopened
        .get_hash_db_entry("hashdb-restart-key")
        .await
        .expect("read rehydrated HashDb");
    let restart_state = reopened
        .get_hash_db_state("backfill_progress")
        .await
        .expect("read rehydrated HashDb state");
    record!(
        "HashDb",
        "restart-rehydration",
        restart_hash.is_some_and(|entry| entry.seq_id == 7 && entry.size == 8192)
    );
    record!(
        "HashDbState",
        "restart-rehydration",
        restart_state
            .is_some_and(|state| { state.value.as_deref() == Some(r#"{"lastProcessedAt":7}"#) })
    );
    drop(reopened);
    let _ = fs::remove_file(&restart_path);

    let concurrent_hash_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("HashDb concurrency database");
    let snapshot_a = vec![crate::persistence::HashDbRecord {
        flac_key: "hashdb-concurrent-a".to_owned(),
        byte_hash: "d".repeat(64),
        size: 100,
        first_seen_at: 1,
        last_updated_at: 1,
        seq_id: 1,
        use_count: 1,
        full_file_hash: String::new(),
        musicbrainz_id: String::new(),
        file_sha256: String::new(),
    }];
    let snapshot_b = vec![crate::persistence::HashDbRecord {
        flac_key: "hashdb-concurrent-b".to_owned(),
        byte_hash: "e".repeat(64),
        size: 200,
        first_seen_at: 2,
        last_updated_at: 2,
        seq_id: 2,
        use_count: 1,
        full_file_hash: String::new(),
        musicbrainz_id: String::new(),
        file_sha256: String::new(),
    }];
    let snapshots = [(&snapshot_a, 1_i64), (&snapshot_b, 2_i64)];
    let snapshot_results =
        futures_util::future::join_all(snapshots.iter().map(|(records, latest_seq)| {
            concurrent_hash_db.replace_hash_db_snapshot(records, *latest_seq)
        }))
        .await;
    let final_hash_rows = concurrent_hash_db
        .list_hash_db_entries()
        .await
        .expect("read concurrent HashDb snapshot");
    let final_seq = concurrent_hash_db
        .get_hash_db_state("latest_seq")
        .await
        .expect("read concurrent HashDb cursor")
        .and_then(|state| state.value)
        .and_then(|value| value.parse::<i64>().ok());
    let snapshot_is_atomic = final_hash_rows.len() == 1
        && final_seq == final_hash_rows.first().map(|entry| entry.seq_id)
        && (final_hash_rows[0].flac_key == "hashdb-concurrent-a"
            || final_hash_rows[0].flac_key == "hashdb-concurrent-b");
    record!(
        "HashDb",
        "transaction-and-concurrency-atomicity",
        snapshot_results.iter().all(Result::is_ok) && snapshot_is_atomic
    );

    let concurrent_state_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("HashDb state concurrency database");
    let concurrent_states = (0..4)
        .map(|index| crate::persistence::HashDbStateRecord {
            key: format!("concurrent-{index}"),
            value: Some(index.to_string()),
        })
        .collect::<Vec<_>>();
    let state_results = futures_util::future::join_all(
        concurrent_states
            .iter()
            .map(|state| concurrent_state_db.upsert_hash_db_state(state)),
    )
    .await;
    let states_read = futures_util::future::join_all(
        concurrent_states
            .iter()
            .map(|state| concurrent_state_db.get_hash_db_state(&state.key)),
    )
    .await;
    record!(
        "HashDbState",
        "transaction-and-concurrency-atomicity",
        state_results.iter().all(Result::is_ok)
            && states_read
                .iter()
                .all(|result| { result.as_ref().is_ok_and(|state| state.is_some()) })
    );

    let corrupt_hash_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("corrupt HashDb database");
    corrupt_hash_db
        .execute_raw_for_test(
            "INSERT INTO HashDb (flac_key, byte_hash, size, first_seen_at, last_updated_at, seq_id, use_count) VALUES ('corrupt-hash', 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', 'not-a-number', 1, 1, 1, 1)",
        )
        .await
        .expect("insert corrupt HashDb row");
    record!(
        "HashDb",
        "corrupt-state-and-upgrade-failure",
        corrupt_hash_db
            .get_hash_db_entry("corrupt-hash")
            .await
            .is_err()
    );

    let corrupt_state_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("corrupt HashDb state database");
    corrupt_state_db
        .execute_raw_for_test(
            "INSERT INTO HashDbState (key, value) VALUES ('hashdb/backfill/progress', 'not-json')",
        )
        .await
        .expect("insert corrupt HashDb state");
    let (corrupt_state, _receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(corrupt_state_db),
    );
    let corrupt_response = crate::route_http_request(
        "POST",
        "/api/v0/hashdb/backfill/from-history",
        None,
        "",
        &corrupt_state,
    )
    .await
    .expect("corrupt HashDb state response");
    record!(
        "HashDbState",
        "corrupt-state-and-upgrade-failure",
        corrupt_response.status == "500 Internal Server Error"
            && corrupt_response.body.contains("HashDbState")
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("persistence-lifecycle");
    fs::create_dir_all(&evidence_dir).expect("create persistence evidence directory");
    fs::write(
        evidence_dir.join("hashdb_domains_lifecycle.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize HashDb persistence ledger"),
    )
    .expect("write HashDb persistence ledger");
    assert!(
        mismatches.is_empty(),
        "{} HashDb persistence mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
