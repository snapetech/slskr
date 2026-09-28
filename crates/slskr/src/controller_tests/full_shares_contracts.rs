//! Controller full shares contracts ownership.

use super::*;

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn share_filters_honor_startup_case_mode_against_original_paths() {
    let root =
        std::env::temp_dir().join(format!("slskr-share-filter-case-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("SECRET.flac"), b"test").unwrap();
    let shared = root.display().to_string();

    let insensitive = crate::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_SHARED_DIR", &shared)
            .with("SLSKD_SHARE_FILTER", "secret"),
    )
    .unwrap();
    assert!(crate::build_share_index(&insensitive).entries.is_empty());

    let sensitive = crate::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSKD_SHARED_DIR", &shared)
            .with("SLSKD_SHARE_FILTER", "secret")
            .with("SLSKD_CASE_SENSITIVE_REGEX", "true"),
    )
    .unwrap();
    assert_eq!(crate::build_share_index(&sensitive).entries.len(), 1);
    fs::remove_dir_all(root).unwrap();
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn removing_a_pod_channel_permanently_removes_its_message_history() {
    let (state, _receiver) = test_state();
    let created = crate::route_http_request(
        "POST",
        "/api/pods",
        None,
        r#"{"pod":{"podId":"pod-cleanup","name":"Cleanup","isPublic":true,"channels":[{"channelId":"general","kind":0,"name":"General"},{"channelId":"private","kind":0,"name":"Private"}]}}"#,
        &state,
    )
    .await
    .expect("create pod");
    assert_eq!(created.status, "201 Created");
    let message_path = "/api/pods/pod-cleanup/channels/private/messages";
    let sent = crate::route_http_request(
        "POST",
        message_path,
        None,
        r#"{"body":"must not resurface","senderPeerId":"tester"}"#,
        &state,
    )
    .await
    .expect("send channel message");
    assert_eq!(sent.status, "200 OK");

    let removed = crate::route_http_request(
        "PUT",
        "/api/pods/pod-cleanup",
        None,
        r#"{"pod":{"podId":"pod-cleanup","name":"Cleanup","isPublic":true,"channels":[{"channelId":"general","kind":0,"name":"General"}]}}"#,
        &state,
    )
    .await
    .expect("remove channel");
    assert_eq!(removed.status, "200 OK");
    assert!(state
        .pod_channels
        .read()
        .await
        .list("pod-cleanup", "private", None)
        .is_empty());

    let recreated = crate::route_http_request(
        "PUT",
        "/api/pods/pod-cleanup",
        None,
        r#"{"pod":{"podId":"pod-cleanup","name":"Cleanup","isPublic":true,"channels":[{"channelId":"general","kind":0,"name":"General"},{"channelId":"private","kind":0,"name":"Private"}]}}"#,
        &state,
    )
    .await
    .expect("recreate channel");
    assert_eq!(recreated.status, "200 OK");
    let history = crate::route_http_request("GET", message_path, None, "", &state)
        .await
        .expect("read recreated channel history");
    assert_eq!(history.status, "200 OK");
    assert_eq!(history.body, "[]");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn share_catalog_supports_filters_and_pagination() {
    let (state, _receiver) = test_state();
    {
        let mut shares = state.shares.write().await;
        shares.entries.push(FileEntry {
            filename_encoding: Default::default(),
            extension_encoding: Default::default(),
            code: 1,
            filename: "Virtual/Other.mp3".to_owned(),
            size: 12,
            extension: "mp3".to_owned(),
            attributes: Vec::new(),
        });
    }

    let response = crate::route_http_request(
        "GET",
        "/api/v0/shares/catalog?q=test&extension=flac&limit=1&offset=0",
        None,
        "",
        &state,
    )
    .await
    .expect("catalog response");

    assert_eq!(response.status, "200 OK");
    assert!(response.body.contains("\"count\":2"));
    assert!(response.body.contains("\"filtered_count\":1"));
    assert!(response.body.contains("\"total_bytes\":42"));
    assert!(response.body.contains("\"limit\":1"));
    assert!(response.body.contains("\"path\":\"Virtual/Test.flac\""));
    assert!(!response.body.contains("Other.mp3"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn share_rescan_route_rebuilds_snapshot() {
    let (state, _receiver) = test_state();

    {
        let mut shares = state.shares.write().await;
        shares.entries.clear();
    }

    let response = crate::route_http_request("POST", "/api/v0/shares/rescan", None, "", &state)
        .await
        .expect("route response");

    assert_eq!(response.status, "202 Accepted");
    assert!(response.body.contains("\"files\":1"));
    assert_eq!(state.shares.read().await.entries.len(), 1);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn share_rebuild_routes_reject_concurrent_scans() {
    let (state, _receiver) = test_state();
    let _permit = Arc::clone(&state.share_scans)
        .acquire_owned()
        .await
        .expect("share scan permit");

    for (method, path) in [("PUT", "/api/shares"), ("POST", "/api/v0/shares/rescan")] {
        let response = crate::route_http_request(method, path, None, "", &state)
            .await
            .expect("route response");
        assert_eq!(response.status, "503 Service Unavailable");
        assert_eq!(
            response.body,
            "{\"error\":\"share scan already in progress\"}"
        );
    }
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn share_rebuild_errors_redact_internal_details() {
    for error in [
        crate::SHARE_SCAN_WORKER_ERROR,
        "share index persistence failed: database path /private/slskr.db",
    ] {
        let response = crate::share_rebuild_error_response(error);
        assert_eq!(response.status, "503 Service Unavailable");
        assert_eq!(response.body, "{\"error\":\"share index unavailable\"}");
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn share_rebuild_routes_roll_back_when_persistence_fails() {
    for (method, path) in [("PUT", "/api/shares"), ("POST", "/api/v0/shares/rescan")] {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default()
                .with("SLSKR_CONTROLLER_PROFILE", "legacy")
                .with("SLSKR_PERSISTENCE_ENABLED", "true"),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        {
            let mut shares = state.shares.write().await;
            shares.entries.clear();
            shares.local_paths.clear();
            shares.roots.clear();
            shares.scan_errors.push("previous snapshot".to_owned());
        }
        let previous = state.shares.read().await.json();
        db.close_for_test().await;

        let response = crate::route_http_request(method, path, None, "", &state)
            .await
            .expect("failed share index persistence response");
        assert_eq!(
            response.status, "503 Service Unavailable",
            "{method} {path}"
        );
        assert_eq!(
            response.body, "{\"error\":\"share index unavailable\"}",
            "{method} {path}"
        );
        assert_eq!(
            state.shares.read().await.json(),
            previous,
            "{method} {path}"
        );
    }
}
#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn share_index_persists_and_rehydrates_records() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );

    let rescanned = crate::route_http_request("POST", "/api/v0/shares/rescan", None, "", &state)
        .await
        .expect("rescan shares");
    assert_eq!(rescanned.status, "202 Accepted");
    assert!(rescanned.body.contains("\"files\":1"));

    let persisted = db.list_share_files(10, 0).await.expect("list shares");
    assert_eq!(persisted.len(), 1);
    assert_eq!(persisted[0].filename, "Virtual/Test.flac");
    assert_eq!(persisted[0].size, 42);
    assert_eq!(persisted[0].extension, "flac");
    assert_eq!(persisted[0].root_label, "Virtual");

    let rehydrated = crate::ShareIndexSnapshot::from_persisted(&state.config, persisted);
    assert_eq!(rehydrated.entries.len(), 1);
    assert!(rehydrated.catalog_json(None).contains("Virtual/Test.flac"));
    assert_eq!(rehydrated.roots.len(), 1);
    assert_eq!(rehydrated.roots[0].label, "Virtual");

    let stats = crate::route_http_request("GET", "/api/admin/database/stats", None, "", &state)
        .await
        .expect("share database stats");
    assert_eq!(stats.status, "200 OK");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap();
    assert_eq!(stats_json["persisted"]["shares"], 1);
    assert_eq!(stats_json["projections"]["shares"], 1);
}

#[cfg(unix)]
#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn shared_file_confined_open_rejects_symlinked_parent() {
    use std::os::unix::fs::symlink;

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!(
        "slskr-share-parent-symlink-test-{}-{unique}",
        std::process::id()
    ));
    let outside = std::env::temp_dir().join(format!(
        "slskr-share-parent-symlink-outside-{}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("secret.flac"), b"secret").unwrap();
    symlink(&outside, root.join("album")).unwrap();

    let error = crate::preview_stream_controller::open_shared_local_file_unix(
        std::slice::from_ref(&root),
        &root.join("album/secret.flac"),
    )
    .expect_err("symlinked share parent must be rejected");
    assert!(error.contains("confined open failed"));

    let _ = std::fs::remove_dir_all(root);
    let _ = std::fs::remove_dir_all(outside);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn share_group_mutations_require_exact_paths_and_decode_members() {
    let (state, _receiver) = test_state();
    let created = crate::route_http_request(
        "POST",
        "/api/sharegroups",
        None,
        r#"{"name":"Trusted","description":"original"}"#,
        &state,
    )
    .await
    .unwrap();
    let group_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    crate::route_http_request(
        "POST",
        &format!("/api/sharegroups/{group_id}/members"),
        None,
        r#"{"username":"Peer One"}"#,
        &state,
    )
    .await
    .unwrap();

    crate::route_http_request(
        "PUT",
        &format!("/api/sharegroups/{group_id}/extra"),
        None,
        r#"{"name":"Corrupted","description":"wrong"}"#,
        &state,
    )
    .await
    .unwrap();
    crate::route_http_request(
        "POST",
        &format!("/api/sharegroups/{group_id}/extra/members"),
        None,
        r#"{"username":"intruder"}"#,
        &state,
    )
    .await
    .unwrap();
    crate::route_http_request(
        "DELETE",
        &format!("/api/sharegroups/{group_id}/members/Peer%20One/extra"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    crate::route_http_request(
        "DELETE",
        &format!("/api/sharegroups/{group_id}/extra"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();

    let group = state.sharegroups.read().await.get(&group_id).unwrap();
    assert_eq!(group.name, "Trusted");
    assert_eq!(group.description, "original");
    assert_eq!(group.members.len(), 1);
    assert_eq!(group.members[0].username, "Peer One");

    let deleted = crate::route_http_request(
        "DELETE",
        &format!("/api/sharegroups/{group_id}/members/Peer%20One"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(deleted.status, "200 OK");
    assert!(state
        .sharegroups
        .read()
        .await
        .get(&group_id)
        .unwrap()
        .members
        .is_empty());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn share_group_member_revocation_rolls_back_when_persistence_fails() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );
    let group_id = state
        .sharegroups
        .write()
        .await
        .create("Trusted".to_owned(), String::new())
        .expect("share group")
        .id;
    state
        .sharegroups
        .write()
        .await
        .add_member(&group_id, "friend".to_owned())
        .expect("member capacity")
        .expect("share group");
    db.close_for_test().await;

    let response = crate::route_http_request(
        "DELETE",
        &format!("/api/sharegroups/{group_id}/members/friend"),
        None,
        "",
        &state,
    )
    .await
    .expect("failed persistence response");
    assert_eq!(response.status, "503 Service Unavailable");
    assert!(response.body.contains("share group persistence failed"));
    let group = state
        .sharegroups
        .read()
        .await
        .get(&group_id)
        .expect("rolled-back group");
    assert_eq!(group.members.len(), 1);
    assert_eq!(group.members[0].username, "friend");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn share_group_revocation_rolls_back_when_persistence_fails() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );
    let group_id = state
        .sharegroups
        .write()
        .await
        .create("Trusted".to_owned(), String::new())
        .expect("share group")
        .id;
    state
        .sharegroups
        .write()
        .await
        .add_member(&group_id, "friend".to_owned())
        .expect("member capacity")
        .expect("share group");
    db.close_for_test().await;

    let response = crate::route_http_request(
        "DELETE",
        &format!("/api/sharegroups/{group_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("failed persistence response");
    assert_eq!(response.status, "503 Service Unavailable");
    assert!(response
        .body
        .contains("share group revocation persistence failed"));
    let group = state
        .sharegroups
        .read()
        .await
        .get(&group_id)
        .expect("rolled-back group");
    assert_eq!(group.members.len(), 1);
    assert_eq!(group.members[0].username, "friend");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn share_group_snapshot_database_write_is_atomic() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let original = crate::persistence::ShareGroupRecord {
        id: "group-1".to_owned(),
        name: "Original".to_owned(),
        description: String::new(),
        created_at: 1,
        updated_at: 1,
    };
    let member = crate::persistence::ShareGroupMemberRecord {
        group_id: original.id.clone(),
        username: "friend".to_owned(),
        added_at: 1,
    };
    db.upsert_share_group(&original).await.unwrap();
    db.upsert_share_group_member(&member).await.unwrap();

    let changed = crate::persistence::ShareGroupRecord {
        name: "Changed".to_owned(),
        updated_at: 2,
        ..original.clone()
    };
    assert!(db
        .replace_share_group(&changed, &[member.clone(), member.clone()])
        .await
        .is_err());

    let groups = db.list_share_groups(10, 0).await.unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].name, "Original");
    let members = db.list_share_group_members(10, 0).await.unwrap();
    assert_eq!(members.len(), 1);
    assert_eq!(members[0].username, "friend");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn share_group_writes_roll_back_when_persistence_fails() {
    let create_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (create_state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(create_db.clone()),
    );
    create_db.close_for_test().await;
    let create = crate::route_http_request(
        "POST",
        "/api/sharegroups",
        None,
        r#"{"name":"Transient"}"#,
        &create_state,
    )
    .await
    .expect("failed create response");
    assert_eq!(create.status, "503 Service Unavailable");
    assert!(create.body.contains("share group persistence failed"));
    assert!(create_state.sharegroups.read().await.records.is_empty());

    let update_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (update_state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(update_db.clone()),
    );
    let update_id = update_state
        .sharegroups
        .write()
        .await
        .create("Original".to_owned(), String::new())
        .expect("share group")
        .id;
    update_db.close_for_test().await;
    let update = crate::route_http_request(
        "PUT",
        &format!("/api/sharegroups/{update_id}"),
        None,
        r#"{"name":"Changed"}"#,
        &update_state,
    )
    .await
    .expect("failed update response");
    assert_eq!(update.status, "503 Service Unavailable");
    assert_eq!(
        update_state
            .sharegroups
            .read()
            .await
            .get(&update_id)
            .expect("rolled-back group")
            .name,
        "Original"
    );

    let member_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (member_state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(member_db.clone()),
    );
    let member_group_id = member_state
        .sharegroups
        .write()
        .await
        .create("Trusted".to_owned(), String::new())
        .expect("share group")
        .id;
    member_db.close_for_test().await;
    let member = crate::route_http_request(
        "POST",
        &format!("/api/sharegroups/{member_group_id}/members"),
        None,
        r#"{"username":"friend"}"#,
        &member_state,
    )
    .await
    .expect("failed member response");
    assert_eq!(member.status, "503 Service Unavailable");
    assert!(member.body.contains("share group persistence failed"));
    assert!(member_state
        .sharegroups
        .read()
        .await
        .get(&member_group_id)
        .expect("rolled-back group")
        .members
        .is_empty());
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn share_groups_bound_groups_and_case_insensitive_members() {
    let mut groups = crate::ShareGroupStore::with_limits(1, 1);
    let group = groups.create("Trusted".to_owned(), String::new()).unwrap();
    assert!(groups
        .create("Overflow".to_owned(), String::new())
        .is_none());

    let (_, added) = groups
        .add_member(&group.id, "Alice".to_owned())
        .unwrap()
        .unwrap();
    assert!(added);
    let (_, added) = groups
        .add_member(&group.id, "alice".to_owned())
        .unwrap()
        .unwrap();
    assert!(!added);
    assert!(groups.add_member(&group.id, "Bob".to_owned()).is_err());
    assert!(groups
        .add_member("missing", "Bob".to_owned())
        .unwrap()
        .is_none());
    assert!(groups.remove_member(&group.id, "ALICE").is_some());
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn share_grants_bound_and_deduplicate_collection_users() {
    let mut grants = crate::ShareGrantStore::new();
    assert!(grants
        .create_with_contract(None, "collection".to_owned(), "   ".to_owned())
        .is_none());
    let (first, created) = grants
        .create_with_contract(None, "collection".to_owned(), "  Alice  ".to_owned())
        .unwrap();
    assert!(created);
    assert_eq!(first.username, "Alice");
    let (duplicate, created) = grants
        .create_with_contract(None, "collection".to_owned(), "alice".to_owned())
        .unwrap();
    assert!(!created);
    assert_eq!(duplicate.id, first.id);
    for index in 1..crate::MAX_SHARE_GRANTS {
        grants
            .create_with_contract(None, format!("collection-{index}"), format!("user-{index}"))
            .unwrap();
    }
    assert!(grants
        .create_with_contract(None, "overflow".to_owned(), "user".to_owned())
        .is_none());
    assert_eq!(grants.records.len(), crate::MAX_SHARE_GRANTS);
    let mut exhausted = crate::ShareGrantStore::new();
    exhausted.next_id = u64::MAX;
    assert_eq!(
        exhausted
            .create_with_contract(None, "collection".to_owned(), "user".to_owned())
            .unwrap()
            .0
            .id,
        format!("grant-{}", u64::MAX)
    );

    let permissions = "p".repeat(crate::MAX_SHARE_GRANT_PERMISSIONS_BYTES + 1);
    let updated = exhausted
        .update(&format!("grant-{}", u64::MAX), permissions)
        .unwrap();
    assert_eq!(
        updated.permissions.len(),
        crate::MAX_SHARE_GRANT_PERMISSIONS_BYTES
    );
    assert_eq!(
        exhausted
            .update(&format!("grant-{}", u64::MAX), "   ".to_owned())
            .unwrap()
            .permissions,
        "read"
    );

    let hydrated = crate::ShareGrantStore::from_persisted(vec![
        crate::persistence::ShareGrantRecord {
            id: "grant-1".to_owned(),
            collection_id: "collection".to_owned(),
            username: format!("  {}  ", "é".repeat(crate::MAX_USER_USERNAME_BYTES)),
            shared_at: 1,
            permissions: "x".repeat(crate::MAX_SHARE_GRANT_PERMISSIONS_BYTES + 1),
        },
        crate::persistence::ShareGrantRecord {
            id: "grant-2".to_owned(),
            collection_id: "collection".to_owned(),
            username: "   ".to_owned(),
            shared_at: 2,
            permissions: "write".to_owned(),
        },
    ]);
    assert_eq!(hydrated.records.len(), 1);
    assert!(hydrated.records[0].username.len() <= crate::MAX_USER_USERNAME_BYTES);
    assert_eq!(
        hydrated.records[0].permissions.len(),
        crate::MAX_SHARE_GRANT_PERMISSIONS_BYTES
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn share_grants_require_live_collections_and_are_revoked_on_delete() {
    let (state, _receiver) = test_state();
    let missing = crate::route_http_request(
        "POST",
        "/api/share-grants",
        None,
        r#"{"collection_id":"col-1","username":"friend"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(missing.status, "404 Not Found");
    assert!(state.share_grants.read().await.records.is_empty());

    let collection = crate::route_http_request(
        "POST",
        "/api/collections",
        None,
        r#"{"name":"Private"}"#,
        &state,
    )
    .await
    .unwrap();
    let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let granted = crate::route_http_request(
        "POST",
        "/api/share-grants",
        None,
        &format!("{{\"collection_id\":\"{collection_id}\",\"username\":\"friend\"}}"),
        &state,
    )
    .await
    .unwrap();
    assert_eq!(granted.status, "201 Created");
    assert_eq!(state.share_grants.read().await.records.len(), 1);
    let grant_id = state.share_grants.read().await.records[0].id.clone();
    state
        .stream_tickets
        .write()
        .await
        .issue(
            "share",
            &format!("share:{grant_id}"),
            "content".to_owned(),
            "track.flac".to_owned(),
            Some("friend".to_owned()),
            1,
            "audio/flac".to_owned(),
            120,
        )
        .expect("issue grant stream ticket");

    let deleted = crate::route_http_request(
        "DELETE",
        &format!("/api/collections/{collection_id}"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(deleted.status, "200 OK");
    assert!(state.share_grants.read().await.records.is_empty());
    assert!(state.stream_tickets.read().await.records.is_empty());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn share_grant_revocation_rolls_back_when_persistence_fails() {
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
        .create_with_contract(None, "private-collection".to_owned(), "friend".to_owned())
        .expect("grant");
    db.close_for_test().await;

    let response =
        crate::route_http_request("DELETE", "/api/share-grants/grant-1", None, "", &state)
            .await
            .expect("failed persistence response");
    assert_eq!(response.status, "503 Service Unavailable");
    assert!(response
        .body
        .contains("share grant revocation persistence failed"));
    assert!(state.share_grants.read().await.get("grant-1").is_some());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn share_grant_create_and_update_roll_back_when_persistence_fails() {
    let create_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (create_state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(create_db.clone()),
    );
    create_state
        .collections
        .write()
        .await
        .create(String::new(), "Private".to_owned(), String::new())
        .expect("collection");
    create_db.close_for_test().await;

    let create = crate::route_http_request(
        "POST",
        "/api/share-grants",
        None,
        r#"{"collection_id":"col-1","username":"friend"}"#,
        &create_state,
    )
    .await
    .expect("failed persistence response");
    assert_eq!(create.status, "503 Service Unavailable");
    assert!(create.body.contains("share grant persistence failed"));
    assert!(create_state.share_grants.read().await.records.is_empty());

    let update_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (update_state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(update_db.clone()),
    );
    update_state
        .share_grants
        .write()
        .await
        .create_with_contract(None, "private-collection".to_owned(), "friend".to_owned())
        .expect("grant");
    update_db.close_for_test().await;

    let update = crate::route_http_request(
        "PUT",
        "/api/share-grants/grant-1",
        None,
        r#"{"permissions":"restricted"}"#,
        &update_state,
    )
    .await
    .expect("failed persistence response");
    assert_eq!(update.status, "503 Service Unavailable");
    assert!(update.body.contains("share grant persistence failed"));
    assert_eq!(
        update_state
            .share_grants
            .read()
            .await
            .get("grant-1")
            .expect("rolled-back grant")
            .permissions,
        "read"
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(all(
    feature = "full-controller-tests",
    not(feature = "legacy-route-dispatch")
))]
pub(super) async fn share_grant_create_releases_store_readers_during_sqlite_io() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let db_path = std::env::temp_dir().join(format!(
        "slskr-legacy-share-grant-lock-test-{}-{unique}.db",
        std::process::id()
    ));
    let db = crate::persistence::DatabaseManager::new(
        db_path.to_str().expect("database path should be UTF-8"),
    )
    .await
    .expect("create share grant database");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );
    let collection = crate::route_http_request(
        "POST",
        "/api/collections",
        None,
        r#"{"name":"Private"}"#,
        &state,
    )
    .await
    .expect("create persisted collection");
    assert_eq!(collection.status, "201 Created");
    let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body)
        .expect("collection JSON")["id"]
        .as_str()
        .expect("collection id")
        .to_owned();

    let blocker_pool = sqlx_sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            sqlx_sqlite::SqliteConnectOptions::new()
                .filename(&db_path)
                .busy_timeout(Duration::from_secs(30)),
        )
        .await
        .expect("open SQLite lock connection");
    let mut blocker = blocker_pool
        .acquire()
        .await
        .expect("acquire SQLite lock connection");
    sqlx_core::query::query("BEGIN IMMEDIATE")
        .execute(&mut *blocker)
        .await
        .expect("hold SQLite write lock");

    let task_state = Arc::clone(&state);
    let body = format!("{{\"collection_id\":\"{collection_id}\",\"username\":\"friend\"}}");
    let mutation = tokio::spawn(async move {
        crate::route_http_request("POST", "/api/share-grants", None, &body, &task_state).await
    });
    let grant_visible_during_sqlite_wait = tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            if state
                .share_grants
                .try_read()
                .is_ok_and(|grants| grants.get("grant-1").is_some())
            {
                break true;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .is_ok();
    tokio::time::sleep(Duration::from_millis(50)).await;
    let request_waited_for_sqlite = !mutation.is_finished();
    let grant_reader_responsive = state.share_grants.try_read().is_ok();

    sqlx_core::query::query("COMMIT")
        .execute(&mut *blocker)
        .await
        .expect("release SQLite write lock");
    drop(blocker);
    let response = tokio::time::timeout(Duration::from_secs(3), mutation)
        .await
        .expect("share grant create should finish after releasing SQLite")
        .expect("share grant route task should join")
        .expect("share grant response");

    assert!(
        grant_visible_during_sqlite_wait,
        "the candidate grant should be readable while SQLite is blocked"
    );
    assert!(
        request_waited_for_sqlite,
        "the request should wait for SQLite"
    );
    assert!(
        grant_reader_responsive,
        "share grant readers should not wait for SQLite"
    );
    assert_eq!(response.status, "201 Created");
    let persisted_grants = db
        .list_share_grants(10, 0)
        .await
        .expect("list persisted share grants");
    assert!(persisted_grants
        .iter()
        .any(|grant| grant.collection_id == collection_id && grant.username == "friend"));

    blocker_pool.close().await;
    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
    let _ = fs::remove_file(&db_path);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn share_grant_routes_bound_fields_and_require_exact_helper_paths() {
    let (state, _receiver) = test_state();
    let collection = crate::route_http_request(
        "POST",
        "/api/collections",
        None,
        r#"{"name":"Shared"}"#,
        &state,
    )
    .await
    .unwrap();
    let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let oversized_username = "é".repeat(crate::MAX_USER_USERNAME_BYTES);
    let grant = crate::route_http_request(
        "POST",
        "/api/share-grants",
        None,
        &format!(
            "{{\"collection_id\":\"{collection_id}\",\"username\":\"  {oversized_username}  \"}}"
        ),
        &state,
    )
    .await
    .unwrap();
    assert_eq!(grant.status, "201 Created");
    let grant_json = serde_json::from_str::<serde_json::Value>(&grant.body).unwrap();
    let grant_id = grant_json["id"].as_str().unwrap().to_owned();
    assert!(grant_json["username"].as_str().unwrap().len() <= crate::MAX_USER_USERNAME_BYTES);

    let oversized_permissions = "p".repeat(crate::MAX_SHARE_GRANT_PERMISSIONS_BYTES + 1);
    let updated = crate::route_http_request(
        "PUT",
        &format!("/api/share-grants/{grant_id}"),
        None,
        &format!("{{\"permissions\":\"{oversized_permissions}\"}}"),
        &state,
    )
    .await
    .unwrap();
    assert_eq!(updated.status, "200 OK");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&updated.body).unwrap()["permissions"]
            .as_str()
            .unwrap()
            .len(),
        crate::MAX_SHARE_GRANT_PERMISSIONS_BYTES
    );

    let reset = crate::route_http_request(
        "PUT",
        &format!("/api/share-grants/{grant_id}"),
        None,
        r#"{"permissions":"   "}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&reset.body).unwrap()["permissions"],
        "read"
    );
    crate::route_http_request(
        "PUT",
        &format!("/api/share-grants/{grant_id}/extra"),
        None,
        r#"{"permissions":"corrupted"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(
        state
            .share_grants
            .read()
            .await
            .get(&grant_id)
            .unwrap()
            .permissions,
        "read"
    );

    let token = crate::route_http_request(
        "POST",
        &format!("/api/share-grants/{grant_id}/token"),
        None,
        "{}",
        &state,
    )
    .await
    .unwrap();
    let token_json = serde_json::from_str::<serde_json::Value>(&token.body).unwrap();
    assert_eq!(token_json["created"], true);
    assert_eq!(token_json["persisted"], false);
    assert_eq!(token_json["status"], "ephemeral_compatibility_token");
    assert_eq!(token_json["expiresInSeconds"], 2_592_000);
    let token_value = token_json["token"].as_str().unwrap();
    assert_eq!(token_value.len(), "share-".len() + 64);
    assert!(!token_value.contains(&grant_id));
    let second_token = crate::route_http_request(
        "POST",
        &format!("/api/share-grants/{grant_id}/token"),
        None,
        "{}",
        &state,
    )
    .await
    .unwrap();
    let second_token_json = serde_json::from_str::<serde_json::Value>(&second_token.body).unwrap();
    assert_ne!(second_token_json["token"], token_json["token"]);
    assert!(!state
        .share_access_tokens
        .read()
        .await
        .records
        .contains_key(token_value));
    let malformed_token = crate::route_http_request(
        "POST",
        &format!("/api/share-grants/{grant_id}/extra/token"),
        None,
        "{}",
        &state,
    )
    .await
    .unwrap();
    assert_ne!(malformed_token.body, token.body);
    assert_eq!(
        crate::share_grant_helper_id(
            &format!("/api/share-grants/{grant_id}/backfill"),
            "backfill"
        ),
        Some(grant_id.as_str())
    );
    assert_eq!(
        crate::share_grant_helper_id(
            &format!("/api/share-grants/{grant_id}/extra/backfill"),
            "backfill"
        ),
        None
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn share_tokens_use_headers_and_content_bound_stream_tickets() {
    run_controller_future_on_large_stack("share-token-stream-tickets", || {
        share_tokens_use_headers_and_content_bound_stream_tickets_impl()
    });
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn share_access_tokens_persist_only_digests_and_rehydrate() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );
    let collection = crate::route_http_request(
        "POST",
        "/api/collections",
        None,
        r#"{"name":"Private"}"#,
        &state,
    )
    .await
    .expect("create collection");
    let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let grant = crate::route_http_request(
        "POST",
        "/api/share-grants",
        None,
        &format!(r#"{{"collection_id":"{collection_id}","username":"friend"}}"#),
        &state,
    )
    .await
    .expect("create grant");
    let grant_id = serde_json::from_str::<serde_json::Value>(&grant.body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let issued = crate::route_http_request(
        "POST",
        &format!("/api/share-grants/{grant_id}/token"),
        None,
        r#"{"expiresInSeconds":600}"#,
        &state,
    )
    .await
    .expect("issue token");
    assert_eq!(issued.status, "201 Created");
    let issued_json = serde_json::from_str::<serde_json::Value>(&issued.body).unwrap();
    assert_eq!(issued_json["persisted"], true);
    assert_eq!(issued_json["status"], "persistent_token");
    let raw_token = issued_json["token"].as_str().unwrap();
    let digest = crate::share_access_token_digest(raw_token);

    let persisted = db
        .list_share_access_tokens(0, 10, 0)
        .await
        .expect("list persisted token digests");
    assert_eq!(persisted.len(), 1);
    assert_eq!(persisted[0].token_digest, digest);
    assert_ne!(persisted[0].token_digest, raw_token);
    assert_eq!(persisted[0].grant_id, grant_id);
    let valid_grants = [grant_id.as_str()].into_iter().collect::<HashSet<_>>();
    let mut rehydrated = crate::ShareAccessTokenStore::from_persisted(persisted, &valid_grants);
    assert_eq!(
        rehydrated.validate(raw_token).map(|record| record.grant_id),
        Some(grant_id.clone())
    );

    let revoked = crate::route_http_request(
        "DELETE",
        &format!("/api/share-grants/{grant_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("revoke grant");
    assert_eq!(revoked.status, "200 OK");
    assert!(db
        .list_share_access_tokens(0, 10, 0)
        .await
        .expect("list revoked token digests")
        .is_empty());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn share_access_token_issue_rolls_back_when_persistence_fails() {
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
        .create(String::new(), "Private".to_owned(), String::new())
        .expect("collection");
    state
        .share_grants
        .write()
        .await
        .create_with_contract(None, "col-1".to_owned(), "friend".to_owned())
        .expect("grant");
    db.close_for_test().await;

    let response = crate::route_http_request(
        "POST",
        "/api/share-grants/grant-1/token",
        None,
        r#"{"expiresInSeconds":600}"#,
        &state,
    )
    .await
    .expect("failed persistence response");
    assert_eq!(response.status, "503 Service Unavailable");
    assert!(response.body.contains("share access token cleanup failed"));
    assert!(state.share_access_tokens.read().await.records.is_empty());
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn shared_file_list_payload_parses_to_browse_entries() {
    let entries = crate::config::parse_share_entries("Music/Artist - Song.flac=123;Loose.mp3=7")
        .expect("share fixture");
    let payload = crate::build_shared_file_list_payload(&entries).expect("payload");

    let parsed = crate::parse_shared_file_list_payload(&payload).expect("parsed payload");

    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0].filename, "Music/Artist - Song.flac");
    assert_eq!(parsed[0].size, 123);
    assert_eq!(parsed[0].extension, "flac");
    assert_eq!(parsed[1].filename, "Loose.mp3");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn shared_file_list_payload_retains_legacy_path_encoding() {
    let mut writer = crate::Writer::new();
    writer.write_u32_le(1);
    writer
        .write_string_with_encoding("Музыка", crate::ProtocolTextEncoding::Windows1251)
        .unwrap();
    writer.write_u32_le(1);
    writer.write_u8(1);
    writer
        .write_string_with_encoding("песня.flac", crate::ProtocolTextEncoding::Windows1251)
        .unwrap();
    writer.write_u64_le(123);
    writer.write_string("flac").unwrap();
    writer.write_u32_le(0);
    let payload = crate::compress_zlib_payload(&writer.into_inner()).unwrap();

    let parsed = crate::parse_shared_file_list_payload(&payload).unwrap();

    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].filename, "Музыка/песня.flac");
    assert_eq!(
        parsed[0].path_encoding,
        crate::ProtocolTextEncoding::Windows1251
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn shared_file_list_payload_rejects_excessive_entries() {
    let entry_count = crate::MAX_BROWSE_ENTRIES_PER_USER + 1;
    let mut writer = crate::Writer::new();
    writer.write_u32_le(1);
    writer.write_string("folder").unwrap();
    writer.write_u32_le(u32::try_from(entry_count).unwrap());
    for index in 0..entry_count {
        writer.write_u8(1);
        writer.write_string(&format!("file-{index}")).unwrap();
        writer.write_u64_le(1);
        writer.write_string("").unwrap();
        writer.write_u32_le(0);
    }
    let payload = crate::compress_zlib_payload(&writer.into_inner()).unwrap();

    let error = crate::parse_shared_file_list_payload(&payload).unwrap_err();
    assert!(error.contains("exceeds"), "{error}");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn shared_file_list_payload_rejects_untrusted_counts_without_looping() {
    let mut payload = Vec::new();
    payload.extend_from_slice(&u32::MAX.to_le_bytes());
    let compressed = crate::compress_zlib_payload(&payload).expect("compressed");

    let error = crate::parse_shared_file_list_payload(&compressed)
        .expect_err("untrusted folder count should be rejected");
    assert!(error.contains("shared folders"));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn shared_file_list_payload_rejects_untrusted_attribute_counts_without_looping() {
    let mut payload = Vec::new();
    payload.extend_from_slice(&1_u32.to_le_bytes());
    payload.extend_from_slice(&0_u32.to_le_bytes());
    payload.extend_from_slice(&1_u32.to_le_bytes());
    payload.push(1);
    payload.extend_from_slice(&4_u32.to_le_bytes());
    payload.extend_from_slice(b"song");
    payload.extend_from_slice(&123_u64.to_le_bytes());
    payload.extend_from_slice(&0_u32.to_le_bytes());
    payload.extend_from_slice(&u32::MAX.to_le_bytes());
    let compressed = crate::compress_zlib_payload(&payload).expect("compressed");

    let error = crate::parse_shared_file_list_payload(&compressed)
        .expect_err("untrusted attribute count should be rejected");
    assert!(error.contains("shared file attributes"));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn shared_file_list_payload_rejects_excessive_wire_records_without_entries() {
    let file_count = crate::MAX_BROWSE_WIRE_FILES_PER_RESPONSE + 1;
    let mut writer = crate::Writer::new();
    writer.write_u32_le(1);
    writer.write_string("folder").unwrap();
    writer.write_u32_le(u32::try_from(file_count).unwrap());
    for _ in 0..file_count {
        writer.write_u8(0);
        writer.write_string("").unwrap();
        writer.write_u64_le(0);
        writer.write_string("").unwrap();
        writer.write_u32_le(0);
    }
    let payload = crate::compress_zlib_payload(&writer.into_inner()).unwrap();

    let error = crate::parse_shared_file_list_payload(&payload)
        .expect_err("wire record cap should reject discarded records");
    assert!(error.contains("shared files"), "{error}");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn shared_file_list_payload_rejects_excessive_sections_without_files() {
    let mut raw = Vec::new();
    for _ in 0..=crate::browse_wire::MAX_BROWSE_WIRE_SECTIONS_PER_RESPONSE {
        raw.extend_from_slice(&0_u32.to_le_bytes());
    }
    let payload = crate::compress_zlib_payload(&raw).unwrap();

    let error = crate::parse_shared_file_list_payload(&payload)
        .expect_err("wire section cap should reject empty sections");
    assert!(error.contains("sections"), "{error}");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn share_fixture_entries_are_searchable() {
    let entries = crate::config::parse_share_entries("Music/Artist - Song.flac=123").unwrap();
    assert_eq!(entries[0].extension, "flac");
    assert_eq!(crate::search_shares(&entries, "artist song").len(), 1);
    assert!(crate::search_shares(&entries, "missing").is_empty());
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn share_scan_discovers_visible_files() {
    let root = std::env::temp_dir().join(format!("slskr-share-test-{}", std::process::id()));
    let artist = root.join("Artist");
    std::fs::create_dir_all(&artist).unwrap();
    std::fs::write(artist.join("Song.flac"), b"audio").unwrap();
    std::fs::create_dir_all(root.join(".hidden")).unwrap();
    std::fs::write(root.join(".hidden").join("Secret.mp3"), b"hidden").unwrap();

    let directory = crate::config::parse_share_directories(root.to_str().unwrap()).unwrap();
    let scan = crate::scan_share_dirs(&directory, false, false, 100, false, 1, &[]);

    assert_eq!(scan.entries.len(), 1);
    assert!(scan.entries[0].filename.ends_with("/Artist/Song.flac"));
    assert_eq!(scan.entries[0].size, 5);
    assert_eq!(scan.entries[0].extension, "flac");
    assert_eq!(scan.roots[0].files, 1);
    assert_eq!(scan.roots[0].bytes, 5);
    assert_eq!(scan.roots[0].extensions[0].extension, "flac");
    assert_eq!(scan.roots[0].extensions[0].files, 1);
    assert_eq!(scan.roots[0].extensions[0].bytes, 5);
    assert!(scan.roots[0].json().contains("\"bytes\":5"));

    std::fs::remove_dir_all(root).unwrap();
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn share_media_probe_emits_wav_protocol_attributes_only_when_enabled() {
    let root =
        std::env::temp_dir().join(format!("slskr-share-media-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let mut wav = Vec::new();
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&88_236_u32.to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16_u32.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&44_100_u32.to_le_bytes());
    wav.extend_from_slice(&88_200_u32.to_le_bytes());
    wav.extend_from_slice(&2_u16.to_le_bytes());
    wav.extend_from_slice(&16_u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&88_200_u32.to_le_bytes());
    wav.resize(88_244, 0);
    std::fs::write(root.join("tone.wav"), wav).unwrap();
    let directory = crate::config::parse_share_directories(root.to_str().unwrap()).unwrap();

    let probed = crate::scan_share_dirs(&directory, false, false, 100, true, 1, &[]);
    assert_eq!(
        probed.entries[0].attributes,
        vec![
            crate::FileAttribute {
                code: 0,
                value: 705
            },
            crate::FileAttribute { code: 1, value: 1 },
            crate::FileAttribute { code: 2, value: 0 },
            crate::FileAttribute {
                code: 4,
                value: 44_100,
            },
            crate::FileAttribute { code: 5, value: 16 },
        ]
    );
    let skipped = crate::scan_share_dirs(&directory, false, false, 100, false, 1, &[]);
    assert!(skipped.entries[0].attributes.is_empty());
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn share_cache_workers_scan_multiple_roots_deterministically() {
    let root =
        std::env::temp_dir().join(format!("slskr-share-workers-test-{}", uuid::Uuid::new_v4()));
    let first = root.join("first");
    let second = root.join("second");
    std::fs::create_dir_all(&first).unwrap();
    std::fs::create_dir_all(&second).unwrap();
    std::fs::write(first.join("a.flac"), b"a").unwrap();
    std::fs::write(second.join("b.flac"), b"b").unwrap();
    let directories = crate::config::parse_share_directories(&format!(
        "[First]{};[Second]{}",
        first.display(),
        second.display()
    ))
    .unwrap();
    let scan = crate::scan_share_dirs(&directories, false, false, 100, false, 2, &[]);
    assert_eq!(scan.entries.len(), 2);
    assert_eq!(scan.roots.len(), 2);
    assert_eq!(scan.roots.iter().map(|root| root.files).sum::<usize>(), 2);
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn share_scan_applies_alias_and_exclusion_before_indexing() {
    let root = std::env::temp_dir().join(format!(
        "slskr-share-exclusion-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    let excluded = root.join("excluded");
    std::fs::create_dir_all(&excluded).unwrap();
    std::fs::write(root.join("visible.flac"), b"visible").unwrap();
    std::fs::write(excluded.join("secret.flac"), b"secret").unwrap();
    let directories = crate::config::parse_share_directories(&format!(
        "[Library]{};!{}",
        root.display(),
        excluded.display()
    ))
    .unwrap();

    let scan = crate::scan_share_dirs(&directories, false, false, 100, false, 1, &[]);

    assert_eq!(scan.entries.len(), 1);
    assert_eq!(scan.entries[0].filename, "Library/visible.flac");
    assert_eq!(scan.roots.len(), 2);
    assert_eq!(scan.roots[0].label, "excluded");
    assert!(scan.roots[0].raw.starts_with('!'));
    assert_eq!(scan.roots[0].files, 0);
    assert_eq!(scan.roots[1].label, "Library");
    assert_eq!(scan.roots[1].files, 1);

    std::fs::remove_dir_all(root).unwrap();
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn share_scan_bounds_aggregate_directory_entries() {
    let mut scanned_entries = crate::MAX_SHARE_SCAN_ENTRIES - 1;
    assert!(crate::reserve_share_scan_entry(&mut scanned_entries));
    assert_eq!(scanned_entries, crate::MAX_SHARE_SCAN_ENTRIES);
    assert!(!crate::reserve_share_scan_entry(&mut scanned_entries));
}

#[cfg(unix)]
#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn share_scan_bounds_pending_directory_descriptors() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!(
        "slskr-share-scan-width-test-{}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).unwrap();
    for index in 0..=crate::MAX_SHARE_SCAN_PENDING_DIRECTORIES {
        std::fs::create_dir(root.join(format!("directory-{index:04}"))).unwrap();
    }

    let directory = crate::config::parse_share_directories(root.to_str().unwrap()).unwrap();
    let scan = crate::scan_share_dirs(&directory, false, false, 100, false, 1, &[]);

    assert!(scan.entries.is_empty());
    assert!(scan
        .errors
        .iter()
        .any(|error| error == crate::SHARE_SCAN_PENDING_DIRECTORY_LIMIT_ERROR));
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn share_scan_does_not_follow_symlinked_directory() {
    use std::os::unix::fs::symlink;

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!(
        "slskr-share-scan-symlink-test-{}-{unique}",
        std::process::id()
    ));
    let outside = std::env::temp_dir().join(format!(
        "slskr-share-scan-symlink-outside-{}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(root.join("visible.flac"), b"visible").unwrap();
    std::fs::write(outside.join("secret.flac"), b"secret").unwrap();
    symlink(&outside, root.join("linked")).unwrap();

    let directory = crate::config::parse_share_directories(root.to_str().unwrap()).unwrap();
    let scan = crate::scan_share_dirs(&directory, false, false, 100, false, 1, &[]);
    assert_eq!(scan.entries.len(), 1);
    assert!(scan.entries[0].filename.ends_with("/visible.flac"));
    assert!(!scan
        .entries
        .iter()
        .any(|entry| entry.filename.contains("secret.flac")));

    let _ = std::fs::remove_dir_all(root);
    let _ = std::fs::remove_dir_all(outside);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn share_cache_escapes_fields() {
    assert_eq!(crate::escape_cache_field("a\tb\\c\n"), "a\\tb\\\\c\\n");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn share_snapshot_errors_redact_internal_details() {
    let state_dir = std::env::temp_dir().join(format!(
        "slskr-share-error-redaction-test-{}",
        std::process::id()
    ));
    let env = MapEnv::default()
        .with("SLSKR_STATE_DIR", &state_dir.display().to_string())
        .with("SLSKR_SHARE_CACHE_TSV_ENABLED", "false");
    let config = crate::AppConfig::from_layers(None, FileConfig::default(), &env).expect("config");
    let mut snapshot = crate::build_share_index(&config);
    snapshot.cache_error =
        Some("share cache write failed: /private/share-index.tsv denied".to_owned());

    let json = snapshot.json();
    assert!(json.contains("\"cache_error\":\"share cache unavailable\""));
    assert!(!json.contains("/private"));
    assert!(!json.contains("denied"));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn share_cache_tsv_can_be_disabled_for_sqlite_primary_state() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let state_dir = std::env::temp_dir().join(format!("slskr-share-cache-disabled-{unique}"));
    std::fs::create_dir_all(&state_dir).unwrap();
    let env = MapEnv::default()
        .with("SLSKR_STATE_DIR", &state_dir.display().to_string())
        .with("SLSKR_SHARE_FIXTURE", "Virtual/Test.flac=42")
        .with("SLSKR_SHARE_CACHE_TSV_ENABLED", "false");
    let config = crate::AppConfig::from_layers(None, FileConfig::default(), &env).expect("config");

    let snapshot = crate::build_share_index(&config);

    assert_eq!(snapshot.entries.len(), 1);
    assert!(!snapshot.cache_enabled);
    assert!(snapshot.cache_written_at.is_none());
    assert!(snapshot.cache_error.is_none());
    assert!(!crate::share_cache_path(&state_dir).exists());
    assert!(snapshot.json().contains("\"cache_enabled\":false"));
    assert!(snapshot
        .json()
        .contains("\"cache_kind\":\"compatibility-debug\""));

    std::fs::remove_dir_all(state_dir).unwrap();
}
