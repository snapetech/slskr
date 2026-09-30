//! Controller full library contracts ownership.

use super::*;

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn library_health_patches_require_exact_paths_and_bound_repairs() {
    let (state, _receiver) = test_state();
    let created = crate::route_http_request(
        "POST",
        "/api/library/items",
        None,
        r#"{"artist":"","title":"Track","kind":"Audio"}"#,
        &state,
    )
    .await
    .unwrap();
    let item_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let issue_id = format!("{item_id}-missing-artist");

    crate::route_http_request(
        "PATCH",
        &format!("/api/library/health/issues/extra/{issue_id}"),
        None,
        r#"{"artist":"wrong"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(state.library.read().await.get(&item_id).unwrap().artist, "");
    assert_eq!(
        crate::library_health_issue_id(&format!("/api/library/health/issues/extra/{issue_id}")),
        None
    );

    let oversized_artist = "é".repeat(crate::MAX_LIST_ARTIST_BYTES);
    let repaired = crate::route_http_request(
        "PATCH",
        &format!("/api/v0/library/health/issues/{issue_id}"),
        None,
        &format!("{{\"artist\":\"{oversized_artist}\"}}"),
        &state,
    )
    .await
    .unwrap();
    assert_eq!(repaired.status, "204 No Content");
    let item = state.library.read().await.get(&item_id).unwrap();
    assert_eq!(item.artist.len(), crate::MAX_LIST_ARTIST_BYTES);
    assert_eq!(
        crate::library_health_issue_id(&format!("/api/library/health/issues/{issue_id}")),
        Some(issue_id.as_str())
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn collection_routes_roll_back_when_persistence_fails() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );
    let previous = state.collections.read().await.clone();
    db.close_for_test().await;
    let response = crate::route_http_request(
        "POST",
        "/api/collections",
        None,
        r#"{"name":"Collection"}"#,
        &state,
    )
    .await
    .expect("failed collection create response");
    assert_eq!(response.status, "503 Service Unavailable");
    assert!(response.body.contains("collection persistence failed"));
    assert_eq!(*state.collections.read().await, previous);

    for (method, path, body) in [
        ("PUT", "/api/collections/col-1", r#"{"name":"Changed"}"#),
        (
            "POST",
            "/api/collections/col-1/items",
            r#"{"title":"Added"}"#,
        ),
        (
            "PUT",
            "/api/collections/items/item-1",
            r#"{"title":"Changed"}"#,
        ),
        ("DELETE", "/api/collections/items/item-1", ""),
        (
            "PUT",
            "/api/collections/col-1/items/reorder",
            r#"{"item_ids":["item-2","item-1"]}"#,
        ),
    ] {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let mut collections = state.collections.write().await;
        collections
            .create(String::new(), "Collection".to_owned(), String::new())
            .unwrap();
        collections
            .add_item(
                "col-1",
                "one".to_owned(),
                String::new(),
                "One".to_owned(),
                "Audio".to_owned(),
            )
            .unwrap();
        collections
            .add_item(
                "col-1",
                "two".to_owned(),
                String::new(),
                "Two".to_owned(),
                "Audio".to_owned(),
            )
            .unwrap();
        let previous = collections.clone();
        drop(collections);
        db.close_for_test().await;

        let response = crate::route_http_request(method, path, None, body, &state)
            .await
            .expect("failed collection mutation response");
        assert_eq!(
            response.status, "503 Service Unavailable",
            "{method} {path}"
        );
        assert!(
            response.body.contains("collection persistence failed"),
            "{method} {path}"
        );
        assert_eq!(*state.collections.read().await, previous, "{method} {path}");
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn collection_item_order_is_persisted_as_one_snapshot() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );
    crate::route_http_request(
        "POST",
        "/api/collections",
        None,
        r#"{"name":"Collection"}"#,
        &state,
    )
    .await
    .unwrap();
    for title in ["One", "Two"] {
        crate::route_http_request(
            "POST",
            "/api/collections/col-1/items",
            None,
            &format!(r#"{{"title":"{title}"}}"#),
            &state,
        )
        .await
        .unwrap();
    }
    let response = crate::route_http_request(
        "PUT",
        "/api/collections/col-1/items/reorder",
        None,
        r#"{"item_ids":["item-2","item-1"]}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(response.status, "200 OK");

    let items = db.list_collection_items(10, 0).await.unwrap();
    assert_eq!(
        items
            .iter()
            .map(|item| (item.id.as_str(), item.position))
            .collect::<Vec<_>>(),
        vec![("item-2", 0), ("item-1", 1)]
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn collection_snapshot_database_write_is_atomic() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let original = crate::persistence::CollectionRecord {
        id: "col-1".to_owned(),
        owner_user_id: String::new(),
        name: "Original".to_owned(),
        description: String::new(),
        collection_type: "ShareList".to_owned(),
        created_at: 1,
        updated_at: 1,
    };
    let original_item = crate::persistence::CollectionItemRecord {
        id: "item-1".to_owned(),
        collection_id: "col-1".to_owned(),
        content_id: "original".to_owned(),
        artist: String::new(),
        title: "Original".to_owned(),
        kind: "Audio".to_owned(),
        file_name: String::new(),
        album: String::new(),
        content_hash: String::new(),
        added_at: 1,
        position: 0,
    };
    db.upsert_collection(&original).await.unwrap();
    db.upsert_collection_item(&original_item).await.unwrap();

    let changed = crate::persistence::CollectionRecord {
        name: "Changed".to_owned(),
        updated_at: 2,
        ..original.clone()
    };
    let duplicate_items = vec![
        crate::persistence::CollectionItemRecord {
            title: "Changed".to_owned(),
            ..original_item.clone()
        },
        crate::persistence::CollectionItemRecord {
            content_id: "duplicate".to_owned(),
            position: 1,
            ..original_item.clone()
        },
    ];
    assert!(db
        .replace_collection(&changed, &duplicate_items)
        .await
        .is_err());

    let collections = db.list_collections(10, 0).await.unwrap();
    assert_eq!(collections.len(), 1);
    assert_eq!(collections[0].name, "Original");
    let items = db.list_collection_items(10, 0).await.unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].title, "Original");
    assert_eq!(items[0].content_id, "original");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn library_routes_roll_back_when_persistence_fails() {
    for (path, body) in [
        (
            "/api/library/items",
            r#"{"artist":"Artist","title":"Track"}"#,
        ),
        (
            "/api/integrations/lidarr/manualimport",
            r#"{"artist":"Artist","album":"Release"}"#,
        ),
        (
            "/api/musicbrainz/targets",
            r#"{"artist":"Artist","title":"Release"}"#,
        ),
    ] {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        let previous = state.library.read().await.clone();
        db.close_for_test().await;

        let response = crate::route_http_request("POST", path, None, body, &state)
            .await
            .expect("failed library creation response");
        assert_eq!(response.status, "503 Service Unavailable", "{path}");
        assert!(
            response.body.contains("library persistence failed"),
            "{path}"
        );
        assert_eq!(*state.library.read().await, previous, "{path}");
    }

    for (method, path, body, expected_error) in [
        (
            "PATCH",
            "/api/v0/library/health/issues/lib-1-missing-title",
            r#"{"title":"Fixed"}"#,
            "library persistence failed",
        ),
        (
            "POST",
            "/api/v0/library/health/issues/fix",
            "",
            "library persistence failed",
        ),
        (
            "DELETE",
            "/api/library/items/lib-1",
            "",
            "library deletion persistence failed",
        ),
    ] {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        state
            .library
            .write()
            .await
            .create("Artist".to_owned(), String::new(), String::new())
            .unwrap();
        let previous = state.library.read().await.clone();
        db.close_for_test().await;

        let response = crate::route_http_request(method, path, None, body, &state)
            .await
            .expect("failed library mutation response");
        assert_eq!(
            response.status, "503 Service Unavailable",
            "{method} {path}"
        );
        assert!(response.body.contains(expected_error), "{method} {path}");
        assert_eq!(*state.library.read().await, previous, "{method} {path}");
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn collection_item_collection_routes_require_exact_paths() {
    let (state, _receiver) = test_state();
    let collection = crate::route_http_request(
        "POST",
        "/api/collections",
        None,
        r#"{"name":"Exact"}"#,
        &state,
    )
    .await
    .unwrap();
    let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    crate::route_http_request(
        "POST",
        &format!("/api/collections/{collection_id}/extra/items"),
        None,
        r#"{"content_id":"wrong","title":"Wrong"}"#,
        &state,
    )
    .await
    .unwrap();
    assert!(state
        .collections
        .read()
        .await
        .get(&collection_id)
        .unwrap()
        .items
        .is_empty());
    assert_eq!(
        crate::collection_items_id(&format!("/api/collections/{collection_id}/extra/items")),
        None
    );

    let exact = crate::route_http_request(
        "POST",
        &format!("/api/collections/{collection_id}/items"),
        None,
        r#"{"content_id":"right","title":"Right"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(exact.status, "201 Created");
    assert_eq!(
        crate::collection_items_id(&format!("/api/collections/{collection_id}/items")),
        Some(collection_id.as_str())
    );
    assert_eq!(
        state
            .collections
            .read()
            .await
            .get(&collection_id)
            .unwrap()
            .items
            .len(),
        1
    );
    let item_id = serde_json::from_str::<serde_json::Value>(&exact.body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(
        crate::collection_item_action_ids(&format!(
            "/api/collections/{collection_id}/items/{item_id}"
        )),
        Some((item_id.as_str(), Some(collection_id.as_str())))
    );
    assert_eq!(
        crate::collection_item_action_ids(&format!("/api/collections/items/{item_id}")),
        Some((item_id.as_str(), None))
    );
    assert_eq!(
        crate::collection_item_action_ids(&format!(
            "/api/collections/{collection_id}/extra/items/{item_id}"
        )),
        None
    );

    let nested_update = crate::route_http_request(
        "PUT",
        &format!("/api/collections/{collection_id}/items/{item_id}"),
        None,
        r#"{"title":"Nested update"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(nested_update.status, "200 OK");
    let nested_delete = crate::route_http_request(
        "DELETE",
        &format!("/api/collections/{collection_id}/items/{item_id}"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(nested_delete.status, "200 OK");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn collections_bound_nested_state_and_allocate_unique_item_ids() {
    let mut collections = crate::CollectionStore::with_limits(1, 2);
    let collection = collections
        .create(String::new(), "Road Trip".to_owned(), String::new())
        .unwrap();
    assert!(collections
        .create(String::new(), "Overflow".to_owned(), String::new())
        .is_none());

    let first = collections
        .add_item(
            &collection.id,
            "content-1".to_owned(),
            "Artist".to_owned(),
            "First".to_owned(),
            "Audio".to_owned(),
        )
        .unwrap()
        .unwrap();
    let second = collections
        .add_item(
            &collection.id,
            "content-2".to_owned(),
            "Artist".to_owned(),
            "Second".to_owned(),
            "Audio".to_owned(),
        )
        .unwrap()
        .unwrap();
    assert_ne!(first.id, second.id);
    assert!(collections
        .add_item(
            &collection.id,
            "content-3".to_owned(),
            String::new(),
            "Third".to_owned(),
            "Audio".to_owned(),
        )
        .is_err());
    assert!(collections
        .add_item(
            "missing",
            String::new(),
            String::new(),
            String::new(),
            "Audio".to_owned(),
        )
        .unwrap()
        .is_none());
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn collections_and_wishlist_bound_text_and_aggregate_items() {
    let mut collections = crate::CollectionStore::with_limits(6, crate::MAX_COLLECTION_ITEMS + 1);
    let first = collections
        .create(
            String::new(),
            "n".repeat(crate::MAX_LIST_NAME_BYTES + 1),
            "d".repeat(crate::MAX_LIST_DESCRIPTION_BYTES + 1),
        )
        .unwrap();
    assert_eq!(first.name.len(), crate::MAX_LIST_NAME_BYTES);
    assert_eq!(first.description.len(), crate::MAX_LIST_DESCRIPTION_BYTES);
    let item = collections
        .add_item(
            &first.id,
            "c".repeat(crate::MAX_LIST_CONTENT_ID_BYTES + 1),
            "a".repeat(crate::MAX_LIST_ARTIST_BYTES + 1),
            "t".repeat(crate::MAX_LIST_TITLE_BYTES + 1),
            "k".repeat(crate::MAX_LIST_KIND_BYTES + 1),
        )
        .unwrap()
        .unwrap();
    assert_eq!(item.content_id.len(), crate::MAX_LIST_CONTENT_ID_BYTES);
    assert_eq!(item.artist.len(), crate::MAX_LIST_ARTIST_BYTES);
    assert_eq!(item.title.len(), crate::MAX_LIST_TITLE_BYTES);
    assert_eq!(item.kind.len(), crate::MAX_LIST_KIND_BYTES);

    let template = crate::CollectionItem {
        id: "item".to_owned(),
        content_id: "content".to_owned(),
        artist: String::new(),
        title: String::new(),
        kind: String::new(),
        file_name: String::new(),
        album: String::new(),
        content_hash: String::new(),
        added_at: 0,
    };
    collections.records[0].items.clear();
    while collections.records.len() < 5 {
        collections
            .create(String::new(), "collection".to_owned(), String::new())
            .unwrap();
    }
    for record in &mut collections.records {
        record.items = vec![template.clone(); crate::MAX_COLLECTION_ITEMS];
    }
    let last = collections
        .create(String::new(), "last".to_owned(), String::new())
        .unwrap();
    assert!(collections
        .add_item(
            &last.id,
            "overflow".to_owned(),
            String::new(),
            String::new(),
            String::new(),
        )
        .is_err());
    assert_eq!(collections.total_items(), crate::MAX_TOTAL_COLLECTION_ITEMS);

    let mut wishlist = crate::WishlistStore::with_max_items(1);
    let wish = wishlist
        .add_item(
            "a".repeat(crate::MAX_LIST_ARTIST_BYTES + 1),
            "t".repeat(crate::MAX_LIST_TITLE_BYTES + 1),
            "k".repeat(crate::MAX_LIST_KIND_BYTES + 1),
        )
        .unwrap();
    assert_eq!(wish.artist.len(), crate::MAX_LIST_ARTIST_BYTES);
    assert_eq!(wish.title.len(), crate::MAX_LIST_TITLE_BYTES);
    assert_eq!(wish.kind.len(), crate::MAX_LIST_KIND_BYTES);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn collection_and_wishlist_ids_wrap_without_collisions() {
    let mut collections = crate::CollectionStore::with_limits(3, 3);
    collections.next_id = u64::MAX;
    let max_collection = collections
        .create(String::new(), "Max".to_owned(), String::new())
        .unwrap();
    let wrapped_collection = collections
        .create(String::new(), "Wrapped".to_owned(), String::new())
        .unwrap();
    assert_eq!(max_collection.id, format!("col-{}", u64::MAX));
    assert_eq!(wrapped_collection.id, "col-1");

    collections.next_item_id = u64::MAX;
    let max_item = collections
        .add_item(
            &max_collection.id,
            "max".to_owned(),
            String::new(),
            "Max".to_owned(),
            "Audio".to_owned(),
        )
        .unwrap()
        .unwrap();
    let wrapped_item = collections
        .add_item(
            &wrapped_collection.id,
            "wrapped".to_owned(),
            String::new(),
            "Wrapped".to_owned(),
            "Audio".to_owned(),
        )
        .unwrap()
        .unwrap();
    assert_eq!(max_item.id, format!("item-{}", u64::MAX));
    assert_eq!(wrapped_item.id, "item-1");

    let mut wishlist = crate::WishlistStore::with_max_items(3);
    wishlist.next_item_id = u64::MAX;
    assert!(wishlist.can_add_items(2));
    let max_wish = wishlist
        .add_item(String::new(), "Max".to_owned(), "Audio".to_owned())
        .unwrap();
    let wrapped_wish = wishlist
        .add_item(String::new(), "Wrapped".to_owned(), "Audio".to_owned())
        .unwrap();
    assert_eq!(max_wish.id, format!("wish-{}", u64::MAX));
    assert_eq!(wrapped_wish.id, "wish-1");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn library_health_scans_are_bounded_snapshots_with_unique_ids() {
    let mut library = crate::LibraryStore::new();
    library
        .create("Artist".to_owned(), "Title".to_owned(), String::new())
        .unwrap();
    let first = library.create_health_scan("/music".to_owned()).unwrap();
    let second = library.create_health_scan("/music".to_owned()).unwrap();
    assert_ne!(first.id, second.id);
    assert_eq!(first.issues.len(), 1);

    library.fix_health_issues();
    assert_eq!(library.health_scan(&first.id).unwrap().issues.len(), 1);
    for _ in 2..crate::MAX_LIBRARY_HEALTH_SCANS {
        library.create_health_scan("/music".to_owned()).unwrap();
    }
    assert_eq!(library.health_scans.len(), crate::MAX_LIBRARY_HEALTH_SCANS);
    library.create_health_scan("/music".to_owned()).unwrap();
    assert_eq!(library.health_scans.len(), crate::MAX_LIBRARY_HEALTH_SCANS);
    assert!(library.health_scan(&first.id).is_none());
    assert!(library.health_scan("scan-does-not-exist").is_none());
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn library_health_group_totals_match_the_bounded_groups() {
    let mut library = crate::LibraryStore::new();
    library
        .create("Artist A".to_owned(), "Release A".to_owned(), String::new())
        .unwrap();
    library
        .create("Artist B".to_owned(), "Release B".to_owned(), String::new())
        .unwrap();
    library
        .create("Artist C".to_owned(), String::new(), "Audio".to_owned())
        .unwrap();
    library
        .create("Artist B".to_owned(), "Release B".to_owned(), String::new())
        .unwrap();

    let artists =
        serde_json::from_str::<serde_json::Value>(&library.health_issues_by_artist_json(1))
            .unwrap();
    assert_eq!(artists["groups"].as_array().unwrap().len(), 1);
    assert_eq!(artists["totalArtists"], 1);

    let releases =
        serde_json::from_str::<serde_json::Value>(&library.health_issues_by_release_json(1))
            .unwrap();
    assert_eq!(releases["groups"].as_array().unwrap().len(), 1);
    assert_eq!(releases["totalReleases"], 1);
    assert_eq!(releases["groups"][0]["count"], 2);
    assert!(releases["groups"]
        .as_array()
        .unwrap()
        .iter()
        .all(|group| !group["album"].as_str().unwrap().is_empty()));
    let all_releases =
        serde_json::from_str::<serde_json::Value>(&library.health_issues_by_release_json(100))
            .unwrap();
    assert_eq!(all_releases["groups"].as_array().unwrap().len(), 2);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn collection_delete_rolls_back_grant_revocation_when_persistence_fails() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );
    let collection_id = state
        .collections
        .write()
        .await
        .create(String::new(), "Private".to_owned(), String::new())
        .expect("collection")
        .id;
    state
        .share_grants
        .write()
        .await
        .create_with_contract(None, collection_id.clone(), "friend".to_owned())
        .expect("share grant");
    db.close_for_test().await;

    let response = crate::route_http_request(
        "DELETE",
        &format!("/api/collections/{collection_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("failed persistence response");
    assert_eq!(response.status, "503 Service Unavailable");
    assert!(response
        .body
        .contains("collection deletion persistence failed"));
    assert!(state.collections.read().await.get(&collection_id).is_some());
    assert!(state.share_grants.read().await.get("grant-1").is_some());
}

#[cfg_attr(test, tokio::test)]
#[cfg(all(
    feature = "full-controller-tests",
    not(feature = "legacy-route-dispatch")
))]
pub(super) async fn collection_delete_releases_store_guards_during_sqlite_io() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let db_path = std::env::temp_dir().join(format!(
        "slskr-collection-delete-lock-test-{}-{unique}.db",
        std::process::id()
    ));
    let db = crate::persistence::DatabaseManager::new(
        db_path.to_str().expect("database path should be UTF-8"),
    )
    .await
    .expect("create collection deletion database");
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
    let grant_body = format!("{{\"collection_id\":\"{collection_id}\",\"username\":\"friend\"}}");
    let grant = crate::route_http_request("POST", "/api/share-grants", None, &grant_body, &state)
        .await
        .expect("create persisted share grant");
    assert_eq!(grant.status, "201 Created");

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
    let path = format!("/api/collections/{collection_id}");
    let deletion = tokio::spawn(async move {
        crate::route_http_request("DELETE", &path, None, "", &task_state).await
    });
    let stores_visible_during_sqlite_wait = tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            let collection_removed = state
                .collections
                .try_read()
                .is_ok_and(|collections| collections.get(&collection_id).is_none());
            let grants_removed = state
                .share_grants
                .try_read()
                .is_ok_and(|grants| grants.records.is_empty());
            if collection_removed && grants_removed {
                break true;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .is_ok();
    tokio::time::sleep(Duration::from_millis(50)).await;
    let request_waited_for_sqlite = !deletion.is_finished();
    let collection_reader_responsive = state.collections.try_read().is_ok();
    let grant_reader_responsive = state.share_grants.try_read().is_ok();

    sqlx_core::query::query("COMMIT")
        .execute(&mut *blocker)
        .await
        .expect("release SQLite write lock");
    drop(blocker);
    let response = tokio::time::timeout(Duration::from_secs(3), deletion)
        .await
        .expect("collection delete should finish after releasing SQLite")
        .expect("collection delete route task should join")
        .expect("collection delete response");

    assert!(
        stores_visible_during_sqlite_wait,
        "deleted collection and grants should be readable while SQLite is blocked"
    );
    assert!(
        request_waited_for_sqlite,
        "the request should wait for SQLite"
    );
    assert!(
        collection_reader_responsive,
        "collection readers should not wait for SQLite"
    );
    assert!(
        grant_reader_responsive,
        "share grant readers should not wait for SQLite"
    );
    assert_eq!(response.status, "200 OK");
    assert!(db
        .list_collections(10, 0)
        .await
        .expect("list persisted collections")
        .is_empty());
    assert!(db
        .list_share_grants(10, 0)
        .await
        .expect("list persisted share grants")
        .is_empty());

    blocker_pool.close().await;
    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
    let _ = fs::remove_file(&db_path);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn library_items_bound_growth_and_checked_ids() {
    let mut library = crate::LibraryStore::new();
    for index in 0..crate::MAX_LIBRARY_ITEMS {
        library
            .create(
                "Artist".to_owned(),
                format!("Track {index}"),
                "Audio".to_owned(),
            )
            .unwrap();
    }
    assert!(library
        .create(String::new(), "Overflow".to_owned(), "Audio".to_owned())
        .is_none());
    assert_eq!(library.records.len(), crate::MAX_LIBRARY_ITEMS);
    let mut exhausted = crate::LibraryStore::new();
    exhausted.next_id = u64::MAX;
    assert_eq!(
        exhausted
            .create(String::new(), "Track".to_owned(), "Audio".to_owned())
            .unwrap()
            .id,
        format!("lib-{}", u64::MAX)
    );

    let mut persisted = (1..=crate::MAX_LIBRARY_ITEMS + 1)
        .map(|index| crate::persistence::LibraryItemRecord {
            id: format!("lib-{index}"),
            artist: "Artist".to_owned(),
            title: format!("Track {index}"),
            kind: "Audio".to_owned(),
            created_at: 1,
        })
        .collect::<Vec<_>>();
    persisted.push(crate::persistence::LibraryItemRecord {
        id: "lib-1".to_owned(),
        artist: "Duplicate".to_owned(),
        title: "Duplicate".to_owned(),
        kind: "Audio".to_owned(),
        created_at: 2,
    });
    let hydrated = crate::LibraryStore::from_persisted(persisted);
    assert_eq!(hydrated.records.len(), crate::MAX_LIBRARY_ITEMS);
    assert_eq!(
        hydrated
            .records
            .iter()
            .filter(|item| item.id == "lib-1")
            .count(),
        1
    );
    assert_eq!(hydrated.next_id, crate::MAX_LIBRARY_ITEMS as u64 + 2);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn library_bloom_preview_reflects_real_hashdb_contents_not_an_empty_filter() {
    let (state, _receiver) = test_state();

    // An empty store must not be reported as containing anything, but
    // the filter parameters should still reflect the real (empty) item
    // count rather than a canned placeholder.
    let empty = crate::route_http_request(
        "POST",
        "/api/v0/musicbrainz/library-bloom/snapshots/preview",
        None,
        r#"{"saltId":"audit-empty"}"#,
        &state,
    )
    .await
    .unwrap();
    let empty = serde_json::from_str::<serde_json::Value>(&empty.body).unwrap();
    assert_eq!(empty["itemCount"], 0);
    assert_eq!(empty["fillRatio"], 0.0);
    assert_eq!(empty["namespaceItemCounts"], serde_json::json!({}));

    // Seed two real hashdb entries with MusicBrainz recording ids.
    {
        let mut discovery = state.content_discovery.write().await;
        discovery
            .merge_hash_entries(vec![
                crate::content_discovery::HashDbEntry {
                    flac_key: "bloom-key-1".to_owned(),
                    size: 111,
                    file_sha256: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                        .to_owned(),
                    music_brainz_id: "11111111-1111-1111-1111-111111111111".to_owned(),
                    ..Default::default()
                },
                crate::content_discovery::HashDbEntry {
                    flac_key: "bloom-key-2".to_owned(),
                    size: 222,
                    file_sha256: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                        .to_owned(),
                    music_brainz_id: "22222222-2222-2222-2222-222222222222".to_owned(),
                    ..Default::default()
                },
            ])
            .expect("seed hash entries");
    }

    let populated = crate::route_http_request(
        "POST",
        "/api/v0/musicbrainz/library-bloom/snapshots/preview",
        None,
        r#"{"saltId":"audit-populated"}"#,
        &state,
    )
    .await
    .unwrap();
    let populated = serde_json::from_str::<serde_json::Value>(&populated.body).unwrap();
    assert_eq!(populated["itemCount"], 2);
    assert!(
        populated["fillRatio"].as_f64().unwrap() > 0.0,
        "a populated store must not report an all-zero bitset: {populated}"
    );
    assert_eq!(
        populated["namespaceItemCounts"],
        serde_json::json!({"musicbrainz:recording": 2})
    );
    let bits = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        populated["bitsBase64"].as_str().unwrap(),
    )
    .unwrap();
    assert!(
        bits.iter().any(|byte| *byte != 0),
        "expected at least one set bit in the populated filter"
    );

    // Different salts for the same underlying data must not produce the
    // same bit pattern -- that's the entire point of salting.
    let differently_salted = crate::route_http_request(
        "POST",
        "/api/v0/musicbrainz/library-bloom/snapshots/preview",
        None,
        r#"{"saltId":"audit-different-salt"}"#,
        &state,
    )
    .await
    .unwrap();
    let differently_salted =
        serde_json::from_str::<serde_json::Value>(&differently_salted.body).unwrap();
    assert_ne!(populated["bitsBase64"], differently_salted["bitsBase64"]);

    let oversized = crate::route_http_request(
        "POST",
        "/api/v0/musicbrainz/library-bloom/snapshots/preview",
        None,
        r#"{"expectedItems":1000000,"falsePositiveRate":1e-300,"saltId":"audit-oversized"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(oversized.status, "400 Bad Request");
}
