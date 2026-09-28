//! Controller full compatibility contracts ownership.

use super::*;

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn compatibility_store_state_persists_and_rehydrates_records() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );

    let wishlist = crate::route_http_request(
        "POST",
        "/api/wishlist",
        None,
        r#"{"artist":"Alice","title":"Blue Track","kind":"Audio"}"#,
        &state,
    )
    .await
    .expect("create wishlist item");
    assert_eq!(wishlist.status, "201 Created");
    let wishlist_json = serde_json::from_str::<serde_json::Value>(&wishlist.body).unwrap();
    let wish_id = wishlist_json["id"].as_str().unwrap().to_owned();

    let aliased_wishlist_update = crate::route_http_request(
        "PUT",
        &format!("/api/wishlist/unrelated/{wish_id}"),
        None,
        r#"{"title":"Aliased Track"}"#,
        &state,
    )
    .await
    .expect("reject aliased wishlist update");
    assert_eq!(aliased_wishlist_update.status, "404 Not Found");
    assert_eq!(
        state.wishlist.read().await.records[0].items[0].title,
        "Blue Track",
        "nested route must not update the last path segment"
    );

    let updated_wishlist = crate::route_http_request(
        "PUT",
        &format!("/api/wishlist/{wish_id}"),
        None,
        r#"{"title":"Green Track"}"#,
        &state,
    )
    .await
    .expect("update wishlist item");
    assert_eq!(updated_wishlist.status, "200 OK");

    let contact = crate::route_http_request(
        "POST",
        "/api/contacts",
        None,
        r#"{"username":"friend"}"#,
        &state,
    )
    .await
    .expect("create contact");
    assert_eq!(contact.status, "201 Created");
    let contact_json = serde_json::from_str::<serde_json::Value>(&contact.body).unwrap();
    let contact_id = contact_json["id"].as_str().unwrap().to_owned();

    let updated_contact = crate::route_http_request(
        "PUT",
        &format!("/api/contacts/{contact_id}"),
        None,
        r#"{"online":true}"#,
        &state,
    )
    .await
    .expect("update contact");
    assert_eq!(updated_contact.status, "200 OK");

    let collection = crate::route_http_request(
        "POST",
        "/api/collections",
        None,
        r#"{"name":"Shared"}"#,
        &state,
    )
    .await
    .expect("create grant collection");
    let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let grant_body = format!("{{\"collection_id\":\"{collection_id}\",\"username\":\"friend\"}}");
    let grant = crate::route_http_request("POST", "/api/share-grants", None, &grant_body, &state)
        .await
        .expect("create share grant");
    assert_eq!(grant.status, "201 Created");
    let duplicate_grant_body =
        format!("{{\"collection_id\":\"{collection_id}\",\"username\":\"FRIEND\"}}");
    let duplicate_grant = crate::route_http_request(
        "POST",
        "/api/share-grants",
        None,
        &duplicate_grant_body,
        &state,
    )
    .await
    .expect("reuse share grant");
    assert_eq!(duplicate_grant.status, "200 OK");
    assert_eq!(duplicate_grant.body, grant.body);
    let grant_json = serde_json::from_str::<serde_json::Value>(&grant.body).unwrap();
    let grant_id = grant_json["id"].as_str().unwrap().to_owned();

    let updated_grant = crate::route_http_request(
        "PUT",
        &format!("/api/share-grants/{grant_id}"),
        None,
        r#"{"permissions":"read,download"}"#,
        &state,
    )
    .await
    .expect("update share grant");
    assert_eq!(updated_grant.status, "200 OK");

    let sharegroup = crate::route_http_request(
        "POST",
        "/api/sharegroups",
        None,
        r#"{"name":"Trusted peers","description":"sharing"}"#,
        &state,
    )
    .await
    .expect("create sharegroup");
    assert_eq!(sharegroup.status, "201 Created");
    let sharegroup_json = serde_json::from_str::<serde_json::Value>(&sharegroup.body).unwrap();
    let sharegroup_id = sharegroup_json["id"].as_str().unwrap().to_owned();

    let updated_sharegroup = crate::route_http_request(
        "PUT",
        &format!("/api/sharegroups/{sharegroup_id}"),
        None,
        r#"{"name":"Trusted peers updated","description":"sharing more"}"#,
        &state,
    )
    .await
    .expect("update sharegroup");
    assert_eq!(updated_sharegroup.status, "200 OK");

    let sharegroup_member = crate::route_http_request(
        "POST",
        &format!("/api/sharegroups/{sharegroup_id}/members"),
        None,
        r#"{"username":"friend"}"#,
        &state,
    )
    .await
    .expect("create sharegroup member");
    assert_eq!(sharegroup_member.status, "201 Created");

    db.upsert_destination(&crate::persistence::DestinationRecord {
        id: "archive".to_string(),
        name: "Archive".to_string(),
        path: "/srv/archive".to_string(),
        is_default: true,
        created_at: 10,
        updated_at: 11,
    })
    .await
    .expect("persist destination");

    let now_playing = crate::route_http_request(
        "POST",
        "/api/nowplaying",
        None,
        r#"{"username":"peer","artist":"Alice","title":"Currently Playing"}"#,
        &state,
    )
    .await
    .expect("create now playing");
    assert_eq!(now_playing.status, "200 OK");

    let persisted_wishlist = db.list_wishlist_items(10, 0).await.expect("list wishlist");
    assert_eq!(persisted_wishlist.len(), 1);
    assert_eq!(persisted_wishlist[0].title, "Green Track");
    let persisted_contacts = db.list_contacts(10, 0).await.expect("list contacts");
    assert_eq!(persisted_contacts.len(), 1);
    assert_eq!(persisted_contacts[0].username, "friend");
    assert!(persisted_contacts[0].online);
    let persisted_grants = db.list_share_grants(10, 0).await.expect("list grants");
    assert_eq!(persisted_grants.len(), 1);
    assert_eq!(persisted_grants[0].permissions, "read,download");
    let persisted_sharegroups = db.list_share_groups(10, 0).await.expect("list sharegroups");
    assert_eq!(persisted_sharegroups.len(), 1);
    assert_eq!(persisted_sharegroups[0].name, "Trusted peers updated");
    let persisted_sharegroup_members = db
        .list_share_group_members(10, 0)
        .await
        .expect("list sharegroup members");
    assert_eq!(persisted_sharegroup_members.len(), 1);
    assert_eq!(persisted_sharegroup_members[0].username, "friend");
    let persisted_destinations = db
        .list_destinations(10, 0)
        .await
        .expect("list destinations");
    assert_eq!(persisted_destinations.len(), 1);
    assert_eq!(persisted_destinations[0].name, "Archive");
    let persisted_now_playing = db.list_now_playing(10, 0).await.expect("list now playing");
    assert_eq!(persisted_now_playing.len(), 1);
    assert_eq!(persisted_now_playing[0].username, "peer");
    assert_eq!(persisted_now_playing[0].title, "Currently Playing");

    let mut rehydrated_wishlist =
        crate::WishlistStore::from_persisted_with_ignored(persisted_wishlist, Vec::new());
    let rehydrated_contacts = crate::ContactStore::from_persisted(persisted_contacts);
    let rehydrated_grants = crate::ShareGrantStore::from_persisted(persisted_grants);
    let rehydrated_sharegroups =
        crate::ShareGroupStore::from_persisted(persisted_sharegroups, persisted_sharegroup_members);
    let rehydrated_destinations = crate::DestinationStore::from_persisted(persisted_destinations);
    let rehydrated_now_playing = crate::NowPlayingStore::from_persisted(persisted_now_playing);
    assert!(rehydrated_wishlist
        .json_array()
        .contains("\"title\":\"Green Track\""));
    assert!(rehydrated_contacts
        .nearby_json(None)
        .contains("\"username\":\"friend\""));
    assert!(rehydrated_grants
        .json_array()
        .contains("\"permissions\":\"read,download\""));
    assert!(rehydrated_sharegroups
        .json_array(None)
        .contains("\"name\":\"Trusted peers updated\""));
    assert!(rehydrated_sharegroups
        .user_group_json("friend")
        .contains("\"group\":\"Trusted peers updated\""));
    assert!(rehydrated_destinations
        .list()
        .contains("\"name\":\"Archive\""));
    assert!(rehydrated_destinations
        .default()
        .contains("\"path\":\"/srv/archive\""));
    assert!(rehydrated_now_playing
        .json()
        .contains("\"title\":\"Currently Playing\""));

    let stats = crate::route_http_request("GET", "/api/admin/database/stats", None, "", &state)
        .await
        .expect("compat database stats");
    assert_eq!(stats.status, "200 OK");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap();
    assert_eq!(stats_json["wishlist"], 1);
    assert_eq!(stats_json["contacts"], 1);
    assert_eq!(stats_json["shareGrants"], 1);
    assert_eq!(stats_json["sharegroups"], 1);
    assert_eq!(stats_json["sharegroupMembers"], 1);
    assert_eq!(stats_json["destinations"], 1);
    assert_eq!(stats_json["nowPlaying"], 1);
    assert_eq!(stats_json["persisted"]["destinations"], 1);
    assert_eq!(stats_json["persisted"]["nowPlaying"], 1);
    assert_eq!(stats_json["projections"]["destinations"], 1);
    assert_eq!(stats_json["projections"]["nowPlaying"], 1);

    let clear_now_playing =
        crate::route_http_request("DELETE", "/api/nowplaying", None, "", &state)
            .await
            .expect("clear now playing");
    assert_eq!(clear_now_playing.status, "200 OK");
    assert!(db.list_now_playing(10, 0).await.unwrap().is_empty());

    let delete_wishlist = crate::route_http_request(
        "DELETE",
        &format!("/api/wishlist/{wish_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete wishlist item");
    assert_eq!(delete_wishlist.status, "200 OK");
    let delete_contact = crate::route_http_request(
        "DELETE",
        &format!("/api/contacts/{contact_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete contact");
    assert_eq!(delete_contact.status, "200 OK");
    let delete_grant = crate::route_http_request(
        "DELETE",
        &format!("/api/share-grants/{grant_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete share grant");
    assert_eq!(delete_grant.status, "200 OK");
    let delete_member = crate::route_http_request(
        "DELETE",
        &format!("/api/sharegroups/{sharegroup_id}/members/friend"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete sharegroup member");
    assert_eq!(delete_member.status, "200 OK");
    let delete_sharegroup = crate::route_http_request(
        "DELETE",
        &format!("/api/sharegroups/{sharegroup_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete sharegroup");
    assert_eq!(delete_sharegroup.status, "200 OK");
    assert!(db.list_wishlist_items(10, 0).await.unwrap().is_empty());
    assert!(db.list_contacts(10, 0).await.unwrap().is_empty());
    assert!(db.list_share_grants(10, 0).await.unwrap().is_empty());
    assert!(db.list_share_groups(10, 0).await.unwrap().is_empty());
    assert!(db.list_share_group_members(10, 0).await.unwrap().is_empty());
    db.delete_destination("archive")
        .await
        .expect("delete destination");
    assert!(db.list_destinations(10, 0).await.unwrap().is_empty());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn compatibility_projections_use_local_state_for_core_stores() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));

    {
        let mut shares = state.shares.write().await;
        shares.roots.push(crate::ShareRoot {
            label: "Virtual".to_owned(),
            local_path: PathBuf::from("Virtual"),
            raw: "Virtual".to_owned(),
            directories: 0,
            files: 2,
            bytes: 12,
            extensions: Vec::new(),
            statistics_ready: true,
        });
    }
    let shared = crate::route_http_request("GET", "/api/shared", None, "", &state)
        .await
        .expect("shared projection");
    let shared_json = serde_json::from_str::<serde_json::Value>(&shared.body).unwrap();
    assert_eq!(shared_json[0]["id"], crate::share_root_id("Virtual"));
    assert_eq!(shared_json[0]["files"], 2);
    let exact_share = crate::route_http_request(
        "GET",
        &format!("/api/shares/{}", crate::share_root_id("Virtual")),
        None,
        "",
        &state,
    )
    .await
    .expect("exact share resource");
    assert_eq!(exact_share.status, "200 OK");
    let aliased_contents = crate::route_http_request(
        "GET",
        "/api/shares/Virtual/extra/contents",
        None,
        "",
        &state,
    )
    .await
    .expect("reject aliased share contents");
    assert_eq!(aliased_contents.status, "404 Not Found");

    let contact = crate::route_http_request(
        "POST",
        "/api/contacts",
        None,
        r#"{"username":"nearby"}"#,
        &state,
    )
    .await
    .expect("create contact");
    let contact_json = serde_json::from_str::<serde_json::Value>(&contact.body).unwrap();
    let contact_id = contact_json["id"].as_str().unwrap();
    crate::route_http_request(
        "PUT",
        &format!("/api/contacts/{contact_id}"),
        None,
        r#"{"online":true}"#,
        &state,
    )
    .await
    .expect("update contact");
    let nearby = crate::route_http_request("GET", "/api/contacts/nearby", None, "", &state)
        .await
        .expect("nearby contacts");
    let nearby_json = serde_json::from_str::<serde_json::Value>(&nearby.body).unwrap();
    assert_eq!(nearby_json[0]["username"], "nearby");
    assert_eq!(nearby_json[0]["online"], true);

    let collection = crate::route_http_request(
        "POST",
        "/api/collections",
        None,
        r#"{"name":"Favorites"}"#,
        &state,
    )
    .await
    .expect("create collection");
    let collection_json = serde_json::from_str::<serde_json::Value>(&collection.body).unwrap();
    let collection_id = collection_json["id"].as_str().unwrap();
    let first = crate::route_http_request(
        "POST",
        &format!("/api/collections/{collection_id}/items"),
        None,
        r#"{"content_id":"1","artist":"A","title":"First"}"#,
        &state,
    )
    .await
    .expect("first item");
    let first_json = serde_json::from_str::<serde_json::Value>(&first.body).unwrap();
    let first_id = first_json["id"].as_str().unwrap();
    let second = crate::route_http_request(
        "POST",
        &format!("/api/collections/{collection_id}/items"),
        None,
        r#"{"content_id":"2","artist":"B","title":"Second"}"#,
        &state,
    )
    .await
    .expect("second item");
    let second_json = serde_json::from_str::<serde_json::Value>(&second.body).unwrap();
    let second_id = second_json["id"].as_str().unwrap();

    let updated_item = crate::route_http_request(
        "PUT",
        &format!("/api/collections/items/{first_id}"),
        None,
        r#"{"artist":"Updated","title":"Renamed","kind":"Video"}"#,
        &state,
    )
    .await
    .expect("update item");
    let updated_item_json = serde_json::from_str::<serde_json::Value>(&updated_item.body).unwrap();
    assert_eq!(updated_item_json["artist"], "Updated");
    assert_eq!(updated_item_json["kind"], "Video");

    let reordered = crate::route_http_request(
        "PUT",
        &format!("/api/collections/{collection_id}/items/reorder"),
        None,
        &format!(r#"{{"itemIds":["{second_id}","{first_id}"]}}"#),
        &state,
    )
    .await
    .expect("reorder items");
    let reordered_json = serde_json::from_str::<serde_json::Value>(&reordered.body).unwrap();
    assert_eq!(reordered_json["reordered"], true);
    assert_eq!(reordered_json["items"][0]["id"], second_id);

    let deleted_item = crate::route_http_request(
        "DELETE",
        &format!("/api/collections/items/{first_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete item");
    let deleted_item_json = serde_json::from_str::<serde_json::Value>(&deleted_item.body).unwrap();
    assert_eq!(deleted_item_json["deleted"], true);
    assert_eq!(deleted_item_json["item"]["id"], first_id);

    let wish = crate::route_http_request(
        "POST",
        "/api/wishlist",
        None,
        r#"{"artist":"Old","title":"Needle"}"#,
        &state,
    )
    .await
    .expect("create wishlist");
    let wish_json = serde_json::from_str::<serde_json::Value>(&wish.body).unwrap();
    let wish_id = wish_json["id"].as_str().unwrap();
    let wish_update = crate::route_http_request(
        "PUT",
        &format!("/api/wishlist/{wish_id}"),
        None,
        r#"{"artist":"New","title":"Needle"}"#,
        &state,
    )
    .await
    .expect("update wishlist");
    let wish_update_json = serde_json::from_str::<serde_json::Value>(&wish_update.body).unwrap();
    assert_eq!(wish_update_json["artist"], "New");
    let wish_delete = crate::route_http_request(
        "DELETE",
        &format!("/api/wishlist/{wish_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete wishlist");
    let wish_delete_json = serde_json::from_str::<serde_json::Value>(&wish_delete.body).unwrap();
    assert_eq!(wish_delete_json["deleted"], true);
    assert_eq!(wish_delete_json["item_id"], wish_id);

    crate::route_http_request(
        "POST",
        "/api/v0/transfers",
        None,
        r#"{"direction":0,"peer_username":"peer","filename":"Remote/Song.flac","size":100}"#,
        &state,
    )
    .await
    .expect("create transfer");
    crate::route_http_request(
        "POST",
        "/api/v0/transfers/1/progress",
        None,
        r#"{"bytes_transferred":40}"#,
        &state,
    )
    .await
    .expect("transfer progress");
    let bridge =
        crate::route_http_request("GET", "/api/bridge/transfer/1/progress", None, "", &state)
            .await
            .expect("bridge progress");
    let bridge_json = serde_json::from_str::<serde_json::Value>(&bridge.body).unwrap();
    assert_eq!(bridge_json["status"], "in_progress");
    assert_eq!(bridge_json["bytesTransferred"], 40);
    assert_eq!(bridge_json["progress"], 40.0);
    let aliased_bridge = crate::route_http_request(
        "GET",
        "/api/bridge/transfer/1/extra/progress",
        None,
        "",
        &state,
    )
    .await
    .expect("reject aliased bridge progress");
    assert_eq!(aliased_bridge.status, "404 Not Found");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn compatibility_projections_use_local_state_for_recommendations_and_activity() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));

    let liked = crate::route_http_request(
        "POST",
        "/api/soulseek/interests",
        None,
        r#"{"name":"ambient"}"#,
        &state,
    )
    .await
    .expect("add liked interest");
    assert_eq!(liked.status, "201 Created");

    crate::route_http_request(
        "POST",
        "/api/soulseek/hated-interests",
        None,
        r#"{"name":"low bitrate"}"#,
        &state,
    )
    .await
    .expect("add hated interest");

    let user_interests = crate::route_http_request(
        "GET",
        "/api/soulseek/users/peer/interests",
        None,
        "",
        &state,
    )
    .await
    .expect("user interests");
    let user_interests_json =
        serde_json::from_str::<serde_json::Value>(&user_interests.body).unwrap();
    assert_eq!(user_interests.status, "503 Service Unavailable");
    assert!(user_interests_json["error"]
        .as_str()
        .is_some_and(|error| error.contains("session")));

    let recommendations =
        crate::route_http_request("GET", "/api/soulseek/recommendations", None, "", &state)
            .await
            .expect("recommendations");
    let recommendations_json =
        serde_json::from_str::<serde_json::Value>(&recommendations.body).unwrap();
    assert_eq!(
        recommendations_json["recommendations"][0]["query"],
        "ambient"
    );

    let item_recommendations = crate::route_http_request(
        "GET",
        "/api/soulseek/items/lib-1/recommendations",
        None,
        "",
        &state,
    )
    .await
    .expect("item recommendations");
    let item_recommendations_json =
        serde_json::from_str::<serde_json::Value>(&item_recommendations.body).unwrap();
    assert_eq!(item_recommendations_json["item_id"], "lib-1");
    assert_eq!(
        item_recommendations_json["recommendations"][0]["interest"],
        "ambient"
    );

    {
        let mut users = state.users.write().await;
        users.watch("near-peer".to_owned()).unwrap();
        if let Some(record) = users
            .records
            .iter_mut()
            .find(|record| record.username == "near-peer")
        {
            record.status = Some("online".to_owned());
        }
    }
    let similar = crate::route_http_request(
        "GET",
        "/api/soulseek/items/lib-1/similar-users",
        None,
        "",
        &state,
    )
    .await
    .expect("similar users");
    let similar_json = serde_json::from_str::<serde_json::Value>(&similar.body).unwrap();
    assert_eq!(similar_json["similar_users"][0]["username"], "near-peer");

    crate::route_http_request(
        "POST",
        "/api/nowplaying",
        None,
        r#"{"username":"peer","artist":"A","title":"Track"}"#,
        &state,
    )
    .await
    .expect("now playing post");
    let now_playing = crate::route_http_request("GET", "/api/nowplaying", None, "", &state)
        .await
        .expect("now playing list");
    let now_playing_json = serde_json::from_str::<serde_json::Value>(&now_playing.body).unwrap();
    assert_eq!(now_playing_json["now_playing"][0]["username"], "peer");
    assert_eq!(now_playing_json["now_playing"][0]["title"], "Track");

    let source_preview = crate::route_http_request(
        "POST",
        "/api/v0/source-feed-imports/preview",
        None,
        r#"{"text":"Artist - One\nTwo"}"#,
        &state,
    )
    .await
    .expect("source preview");
    let source_preview_json =
        serde_json::from_str::<serde_json::Value>(&source_preview.body).unwrap();
    assert_eq!(source_preview_json["suggestionCount"], 2);
    assert_eq!(source_preview_json["suggestions"][0]["artist"], "Artist");
    assert_eq!(source_preview_json["suggestions"][1]["title"], "Two");

    crate::route_http_request(
        "POST",
        "/api/wishlist",
        None,
        r#"{"artist":"Wish","title":"Song"}"#,
        &state,
    )
    .await
    .expect("wishlist feed seed");
    let source_feeds = crate::route_http_request("GET", "/api/source-feeds", None, "", &state)
        .await
        .expect("source feeds");
    let source_feeds_json = serde_json::from_str::<serde_json::Value>(&source_feeds.body).unwrap();
    assert_eq!(source_feeds_json["feeds"][0]["provider"], "wishlist");
    let created_feed = crate::route_http_request(
        "POST",
        "/api/source-feeds",
        None,
        r#"{"name":"Manual feed","text":"Feed Artist - Feed Track"}"#,
        &state,
    )
    .await
    .expect("create source feed");
    let created_feed_json = serde_json::from_str::<serde_json::Value>(&created_feed.body).unwrap();
    assert_eq!(created_feed_json["provider"], "manual");
    assert_eq!(created_feed_json["count"], 1);
    let source_feeds = crate::route_http_request("GET", "/api/source-feeds", None, "", &state)
        .await
        .expect("source feeds after create");
    let source_feeds_json = serde_json::from_str::<serde_json::Value>(&source_feeds.body).unwrap();
    assert!(source_feeds_json["count"].as_u64().unwrap() >= 2);

    crate::route_http_request(
        "POST",
        "/api/v0/transfers",
        None,
        r#"{"direction":0,"peer_username":"peer","filename":"Remote/Song.flac","size":100}"#,
        &state,
    )
    .await
    .expect("create bridge transfer");
    crate::route_http_request(
        "POST",
        "/api/v0/transfers/1/progress",
        None,
        r#"{"bytes_transferred":25}"#,
        &state,
    )
    .await
    .expect("progress bridge transfer");
    let bridge_stats =
        crate::route_http_request("GET", "/api/bridge/admin/stats", None, "", &state)
            .await
            .expect("bridge stats");
    let bridge_stats_json = serde_json::from_str::<serde_json::Value>(&bridge_stats.body).unwrap();
    assert_eq!(bridge_stats_json["total_requests"], 1);
    assert_eq!(bridge_stats_json["total_bytes"], 25);
    // Matches the oracle's real BridgeStatistics contract: with no
    // embedded Soulfind bridge server, these must stay honest zeros
    // even though a real (unrelated) local transfer just happened --
    // they must never be backfilled from slskR's own transfer queue.
    assert_eq!(
        bridge_stats_json["totalConnections"], 0,
        "{bridge_stats_json}"
    );
    assert_eq!(
        bridge_stats_json["currentConnections"], 0,
        "{bridge_stats_json}"
    );
    assert_eq!(
        bridge_stats_json["totalDownloads"], 0,
        "{bridge_stats_json}"
    );
    assert_eq!(bridge_stats_json["totalSearches"], 0, "{bridge_stats_json}");
    assert_eq!(
        bridge_stats_json["totalRoomJoins"], 0,
        "{bridge_stats_json}"
    );
    assert_eq!(
        bridge_stats_json["totalBytesProxied"], 0,
        "{bridge_stats_json}"
    );

    let bridge_dashboard =
        crate::route_http_request("GET", "/api/bridge/admin/dashboard", None, "", &state)
            .await
            .expect("bridge dashboard");
    let bridge_dashboard_json =
        serde_json::from_str::<serde_json::Value>(&bridge_dashboard.body).unwrap();
    assert_eq!(
        bridge_dashboard_json["connectedClients"], 0,
        "{bridge_dashboard_json}"
    );
    assert_eq!(
        bridge_dashboard_json["stats"]["totalConnections"], 0,
        "{bridge_dashboard_json}"
    );
    assert_eq!(
        bridge_dashboard_json["stats"]["totalBytesProxied"], 0,
        "{bridge_dashboard_json}"
    );
}

#[cfg_attr(test, tokio::test(flavor = "multi_thread", worker_threads = 2))]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn compatibility_projections_use_local_state_for_system_mutation_shells() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));
    {
        let mut advanced = state.advanced_networking.write().await;
        advanced.mesh.enabled = true;
        advanced.mesh.enable_overlay = true;
    }

    crate::route_http_request(
        "POST",
        "/api/v0/transfers",
        None,
        r#"{"direction":0,"peer_username":"peer","filename":"Remote/System.flac","size":100}"#,
        &state,
    )
    .await
    .expect("transfer seed");
    crate::route_http_request(
        "POST",
        "/api/v0/transfers/1/progress",
        None,
        r#"{"bytes_transferred":55}"#,
        &state,
    )
    .await
    .expect("transfer progress");
    crate::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        r#"{"query":"system stats"}"#,
        &state,
    )
    .await
    .expect("search seed");

    let admin = crate::route_http_request("GET", "/api/admin/stats", None, "", &state)
        .await
        .expect("admin stats");
    let admin_json = serde_json::from_str::<serde_json::Value>(&admin.body).unwrap();
    assert_eq!(admin_json["total_transfers"], 1);
    assert_eq!(admin_json["total_bytes"], 55);
    assert_eq!(admin_json["searches"], 1);

    // These slskR-invented admin/{shutdown,restart,version} routes had
    // no oracle equivalent, no caller, and no test coverage, and always
    // faked a success response without doing anything -- a real 404 is
    // honest where a fake "requested" body was not. The real, working
    // equivalents are DELETE/PUT /api/application and GET /api/version.
    for (method, path) in [
        ("POST", "/api/admin/shutdown"),
        ("GET", "/api/admin/version"),
        ("POST", "/api/admin/restart"),
    ] {
        let response = crate::route_http_request(method, path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
        assert_eq!(response.status, "404 Not Found", "{method} {path}");
    }

    {
        let mut session = state.session.write().await;
        session.state = "connected";
        session.username = Some("local-user".to_owned());
        session.privileges_seconds = Some(60);
    }
    let updated_profile = crate::route_http_request(
        "PUT",
        "/api/profile/me",
        None,
        r#"{"username":"updated-user","privilegesSeconds":120,"connected":true}"#,
        &state,
    )
    .await
    .expect("profile update");
    let updated_profile_json =
        serde_json::from_str::<serde_json::Value>(&updated_profile.body).unwrap();
    assert_eq!(updated_profile_json["updated"], true);
    assert_eq!(updated_profile_json["persisted"], true);
    assert_eq!(updated_profile_json["profile"]["username"], "updated-user");
    assert_eq!(updated_profile_json["profile"]["privilegesSeconds"], 120);
    let profile = crate::route_http_request("GET", "/api/profile/me", None, "", &state)
        .await
        .expect("profile me");
    let profile_json = serde_json::from_str::<serde_json::Value>(&profile.body).unwrap();
    assert_eq!(profile_json["username"], "updated-user");
    assert_eq!(profile_json["user_type"], "privileged");

    let batch = crate::route_http_request(
        "POST",
        "/api/batch",
        None,
        r#"{"operations":[{"id":"stats","method":"GET","path":"/api/stats"},{"id":"caps","method":"GET","path":"/api/capabilities"}]}"#,
        &state,
    )
    .await
    .expect("batch execution");
    let batch_json = serde_json::from_str::<serde_json::Value>(&batch.body).unwrap();
    assert_eq!(batch_json["accepted"], true);
    assert_eq!(batch_json["executed"], 2);
    assert_eq!(batch_json["results"][0]["id"], "stats");
    assert_eq!(batch_json["results"][0]["status"], 200);

    let plugins = crate::route_http_request("GET", "/api/config/plugins", None, "", &state)
        .await
        .expect("plugins");
    let plugins_json = serde_json::from_str::<serde_json::Value>(&plugins.body).unwrap();
    assert_eq!(plugins_json["count"], 4);
    assert_eq!(plugins_json["plugins"][0]["id"], "spotify");

    let invite = crate::route_http_request("POST", "/api/profile/invite", None, "{}", &state)
        .await
        .expect("profile invite");
    let invite_json = serde_json::from_str::<serde_json::Value>(&invite.body).unwrap();
    assert_eq!(invite_json["count"], 1);
    assert_eq!(invite_json["persisted"], true);

    let warm_cache =
        crate::route_http_request("POST", "/api/slskdn/warm-cache", None, "{}", &state)
            .await
            .expect("warm cache");
    let warm_cache_json = serde_json::from_str::<serde_json::Value>(&warm_cache.body).unwrap();
    assert_eq!(warm_cache_json["runs"], 1);
    assert_eq!(warm_cache_json["persisted"], true);

    let bridge_config = crate::route_http_request(
        "PUT",
        "/api/v0/bridge/admin/config",
        None,
        r#"{"maxClients":4,"enabled":true}"#,
        &state,
    )
    .await
    .expect("bridge config update");
    let bridge_config_json =
        serde_json::from_str::<serde_json::Value>(&bridge_config.body).unwrap();
    assert_eq!(bridge_config_json["persisted"], true);
    assert_eq!(bridge_config_json["configUpdates"], 1);
    assert!(bridge_config_json["acceptedKeys"]
        .as_array()
        .unwrap()
        .iter()
        .any(|key| key == "enabled"));
    let bridge_start =
        crate::route_http_request("POST", "/api/v0/bridge/start", None, "{}", &state)
            .await
            .expect("bridge start");
    let bridge_start_json = serde_json::from_str::<serde_json::Value>(&bridge_start.body).unwrap();
    assert_eq!(bridge_start_json["persisted"], true);
    let bridge_status = crate::route_http_request("GET", "/api/bridge/status", None, "", &state)
        .await
        .expect("bridge status");
    let bridge_status_json =
        serde_json::from_str::<serde_json::Value>(&bridge_status.body).unwrap();
    assert_eq!(bridge_status_json["configUpdates"], 1);
    let bridge_stop = crate::route_http_request("POST", "/api/v0/bridge/stop", None, "{}", &state)
        .await
        .expect("bridge stop");
    let bridge_stop_json = serde_json::from_str::<serde_json::Value>(&bridge_stop.body).unwrap();
    assert_eq!(bridge_stop_json["stopped"], true);

    let application_restart =
        crate::route_http_request("PUT", "/api/application", None, "{}", &state)
            .await
            .expect("application restart request");
    assert_eq!(application_restart.status, "204 No Content");
    assert!(application_restart.body.is_empty());
    let application = crate::route_http_request("GET", "/api/application", None, "", &state)
        .await
        .expect("application state");
    let application_json = serde_json::from_str::<serde_json::Value>(&application.body).unwrap();
    assert_eq!(application_json["pendingRestart"], true);
    assert_eq!(application_json["bridge"]["configUpdates"], 1);
    assert_eq!(application_json["operations"]["profileInvitesCreated"], 1);
    assert_eq!(application_json["operations"]["cacheWarmRuns"], 1);
    let gc = crate::route_http_request("POST", "/api/application/gc", None, "", &state)
        .await
        .expect("application gc");
    let gc_json = serde_json::from_str::<serde_json::Value>(&gc.body).unwrap();
    assert_eq!(gc_json["collected"], true);
    assert_eq!(gc_json["gcRuns"], 1);
    let application_restart_clear =
        crate::route_http_request("DELETE", "/api/application", None, "", &state)
            .await
            .expect("application restart clear");
    assert_eq!(application_restart_clear.status, "204 No Content");
    assert!(application_restart_clear.body.is_empty());
    let application = crate::route_http_request("GET", "/api/application", None, "", &state)
        .await
        .expect("application state after shutdown request");
    let application_json = serde_json::from_str::<serde_json::Value>(&application.body).unwrap();
    assert_eq!(application_json["pendingRestart"], false);

    let autoreplace = crate::route_http_request("PUT", "/api/autoreplace/enable", None, "", &state)
        .await
        .expect("autoreplace enable");
    let autoreplace_json = serde_json::from_str::<serde_json::Value>(&autoreplace.body).unwrap();
    assert_eq!(autoreplace_json["enabled"], true);
    assert_eq!(autoreplace_json["persisted"], true);
    let preferences = crate::route_http_request(
        "PUT",
        "/api/config/preferences",
        None,
        r#"{"autoreplace_enabled":false}"#,
        &state,
    )
    .await
    .expect("preferences update");
    let preferences_json = serde_json::from_str::<serde_json::Value>(&preferences.body).unwrap();
    assert_eq!(preferences_json["persisted"], true);
    assert_eq!(preferences_json["autoreplace_enabled"], false);

    let relay = crate::route_http_request("PUT", "/api/relay", None, r#"{"enabled":true}"#, &state)
        .await
        .expect("relay enable");
    let relay_json = serde_json::from_str::<serde_json::Value>(&relay.body).unwrap();
    assert_eq!(relay_json["relay_enabled"], true);
    let relay_agent = crate::route_http_request(
        "PUT",
        "/api/relay/agent",
        None,
        r#"{"enabled":true}"#,
        &state,
    )
    .await
    .expect("relay agent enable");
    let relay_agent_json = serde_json::from_str::<serde_json::Value>(&relay_agent.body).unwrap();
    assert_eq!(relay_agent_json["relayAgentEnabled"], true);
    assert_eq!(relay_agent_json["persisted"], true);
    let relay_files = crate::route_http_request(
        "POST",
        "/api/relay/controller/files/token-1",
        None,
        "{}",
        &state,
    )
    .await
    .expect("relay files token");
    let relay_files_json = serde_json::from_str::<serde_json::Value>(&relay_files.body).unwrap();
    assert_eq!(relay_files_json["accepted"], true);
    assert_eq!(relay_files_json["token"], "token-1");
    assert_eq!(relay_files_json["relay_enabled"], true);
    let relay_shares = crate::route_http_request(
        "POST",
        "/api/relay/controller/shares/token-2",
        None,
        "{}",
        &state,
    )
    .await
    .expect("relay shares token");
    let relay_shares_json = serde_json::from_str::<serde_json::Value>(&relay_shares.body).unwrap();
    assert_eq!(relay_shares_json["accepted"], true);
    assert_eq!(relay_shares_json["token"], "token-2");
    assert!(relay_shares_json["shareCount"].as_u64().unwrap() >= 1);
    for (method, path) in [
        ("GET", "/api/relay/controller/downloads/nested/token-1"),
        ("POST", "/api/relay/controller/files/nested/token-1"),
        ("POST", "/api/relay/controller/shares/nested/token-2"),
    ] {
        let response = crate::route_http_request(method, path, None, "{}", &state)
            .await
            .expect("reject nested relay token route");
        assert_eq!(response.status, "404 Not Found", "{method} {path}");
    }
    let application = crate::route_http_request("GET", "/api/application", None, "", &state)
        .await
        .expect("application with relay");
    let application_json = serde_json::from_str::<serde_json::Value>(&application.body).unwrap();
    assert_eq!(application_json["relay"]["enabled"], true);
    assert_eq!(application_json["relay"]["agentEnabled"], true);
    let relay_agent_deleted =
        crate::route_http_request("DELETE", "/api/relay/agent", None, "", &state)
            .await
            .expect("relay agent disable");
    let relay_agent_deleted_json =
        serde_json::from_str::<serde_json::Value>(&relay_agent_deleted.body).unwrap();
    assert_eq!(relay_agent_deleted_json["relayAgentEnabled"], false);
    let relay_deleted = crate::route_http_request("DELETE", "/api/relay", None, "", &state)
        .await
        .expect("relay disable");
    let relay_deleted_json =
        serde_json::from_str::<serde_json::Value>(&relay_deleted.body).unwrap();
    assert_eq!(relay_deleted_json["relay_enabled"], false);

    let recorded_event = crate::route_http_request(
        "POST",
        "/api/events/parity",
        None,
        r#""compat event""#,
        &state,
    )
    .await
    .expect("record compat event");
    let recorded_event_json =
        serde_json::from_str::<serde_json::Value>(&recorded_event.body).unwrap();
    assert_eq!(recorded_event_json["recorded"], true);
    assert_eq!(recorded_event_json["event"]["type"], "compat.event");
    assert!(recorded_event_json["count"].as_u64().unwrap() >= 1);
    crate::record_daemon_log(
        &state,
        crate::logging::LogLevel::Info,
        "compat.event",
        "compat event",
    )
    .await;
    let logs = crate::route_http_request("GET", "/api/logs", None, "", &state)
        .await
        .expect("logs");
    let logs_json = serde_json::from_str::<serde_json::Value>(&logs.body).unwrap();
    assert_eq!(logs_json["entries"][0]["category"], "compat.event");
    assert_eq!(logs_json["entries"][0]["message"], "compat event");
    let compat_logs = crate::route_http_request("GET", "/api/v0/logs", None, "", &state)
        .await
        .expect("compat logs");
    let compat_logs_json = serde_json::from_str::<serde_json::Value>(&compat_logs.body).unwrap();
    assert_eq!(compat_logs_json[0]["category"], "compat.event");
    assert_eq!(compat_logs_json[0]["context"], "compat.event");
    assert_eq!(compat_logs_json[0]["message"], "compat event");

    {
        let mut users = state.users.write().await;
        users.watch("mesh-peer".to_owned());
    }
    let mesh_sync =
        crate::route_http_request("POST", "/api/mesh/sync/mesh-peer", None, "{}", &state)
            .await
            .expect("mesh sync");
    let mesh_sync_json = serde_json::from_str::<serde_json::Value>(&mesh_sync.body).unwrap();
    assert_eq!(mesh_sync_json["queued"], true);
    assert_eq!(mesh_sync_json["status"], "watched");

    let kpis = crate::route_http_request("GET", "/api/telemetry/metrics/kpis", None, "", &state)
        .await
        .expect("kpis");
    let kpis_json = serde_json::from_str::<serde_json::Value>(&kpis.body).unwrap();
    // Matches the oracle: TelemetryController.GetKpis and
    // MetricsController.GetKpis both call the same
    // Telemetry.Prometheus.GetMetricsAsObject with an identical KPI
    // regex list, so this sibling route returns the exact same
    // dictionary-of-PrometheusMetric shape as
    // /api/telemetry/prometheus/kpis, not an invented {kpis:[],count}
    // array.
    assert_eq!(kpis_json["slskr_transfers"]["type"], "gauge");
    assert_eq!(kpis_json["slskr_searches"]["type"], "gauge");

    let pod_created = crate::route_http_request(
        "POST",
        "/api/pods",
        None,
        r#"{"pod":{"podId":"pod:mesh-peer","name":"mesh-peer","isPublic":true,"channels":[]},"requestingPeerId":"mesh-peer"}"#,
        &state,
    )
    .await
    .expect("create compatibility pod");
    assert_eq!(pod_created.status, "201 Created");
    let pods = crate::route_http_request("GET", "/api/pods", None, "", &state)
        .await
        .expect("pods");
    let pods_json = serde_json::from_str::<serde_json::Value>(&pods.body).unwrap();
    assert!(pods_json
        .as_array()
        .unwrap()
        .iter()
        .any(|pod| pod["name"] == "mesh-peer"));

    let federation =
        crate::route_http_request("GET", "/api/v0/federation/diagnostics", None, "", &state)
            .await
            .expect("federation diagnostics");
    let federation_json = serde_json::from_str::<serde_json::Value>(&federation.body).unwrap();
    assert_eq!(federation_json["federation"]["enabled"], false);
    assert_eq!(federation_json["federation"]["mode"], "Hermit");
    assert_eq!(federation_json["federation"]["exposure"], "Hermit");
    assert_eq!(
        federation_json["publishing"]["publishableDomains"],
        serde_json::json!(["music"])
    );
    assert_eq!(federation_json["pods"]["joinSignatureMode"], "Off");
    assert_eq!(federation_json["mesh"]["selfPeerIdConfigured"], true);
    assert_eq!(
        federation_json["warnings"],
        serde_json::json!([
            "Pod join signatures are not enforced.",
            "Pod message signatures are not enforced."
        ])
    );

    let security = crate::route_http_request("GET", "/api/security/dashboard", None, "", &state)
        .await
        .expect("security dashboard");
    let security_json = serde_json::from_str::<serde_json::Value>(&security.body).unwrap();
    assert_eq!(security_json["status"], "local");
    assert!(
        security_json["stats"]["networkGuardStats"]["globalConnections"]
            .as_u64()
            .unwrap()
            >= 1
    );
    crate::route_http_request(
        "POST",
        "/api/security/bans/username",
        None,
        r#"{"username":"mesh-peer"}"#,
        &state,
    )
    .await
    .expect("security ban");
    let security_status =
        crate::route_http_request("GET", "/api/security/status", None, "", &state)
            .await
            .expect("security status");
    let security_status_json =
        serde_json::from_str::<serde_json::Value>(&security_status.body).unwrap();
    assert_eq!(security_status_json["activeBans"], 1);
    let security = crate::route_http_request("GET", "/api/security/dashboard", None, "", &state)
        .await
        .expect("security dashboard after ban");
    let security_json = serde_json::from_str::<serde_json::Value>(&security.body).unwrap();
    assert_eq!(security_json["stats"]["banStats"]["activeBans"], 1);

    let fairness = crate::route_http_request("GET", "/api/fairness", None, "", &state)
        .await
        .expect("fairness");
    let fairness_json = serde_json::from_str::<serde_json::Value>(&fairness.body).unwrap();
    assert_eq!(fairness_json["status"], "ready");
    assert_eq!(fairness_json["items"][0]["username"], "mesh-peer");
    let ranking = crate::route_http_request("GET", "/api/ranking", None, "", &state)
        .await
        .expect("ranking");
    let ranking_json = serde_json::from_str::<serde_json::Value>(&ranking.body).unwrap();
    assert_eq!(ranking_json["status"], "ready");
    assert!(ranking_json["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| { row["kind"] == "transfer" && row["score"].as_u64().unwrap_or(0) >= 55 }));
    let portforwarding =
        crate::route_http_request("GET", "/api/portforwarding/status", None, "", &state)
            .await
            .expect("portforwarding");
    let portforwarding_json =
        serde_json::from_str::<serde_json::Value>(&portforwarding.body).unwrap();
    assert!(portforwarding_json.as_array().unwrap().is_empty());

    let imported = crate::route_http_request(
        "POST",
        "/api/wishlist/import/csv",
        None,
        r#"{"csv":"artist,title\nA,One\nB,Two"}"#,
        &state,
    )
    .await
    .expect("wishlist csv import");
    let imported_json = serde_json::from_str::<serde_json::Value>(&imported.body).unwrap();
    assert_eq!(imported_json["imported"], 2);
    assert_eq!(imported_json["items"][0]["artist"], "A");
    let imported_item_id = imported_json["items"][0]["id"].as_str().unwrap();
    let wishlist_search = crate::route_http_request(
        "POST",
        &format!("/api/wishlist/{imported_item_id}/search"),
        None,
        "{}",
        &state,
    )
    .await
    .expect("wishlist item search");
    let wishlist_search_json =
        serde_json::from_str::<serde_json::Value>(&wishlist_search.body).unwrap();
    assert_eq!(wishlist_search_json["search_started"], true);
    assert_eq!(wishlist_search_json["target"], "wishlist");
    assert_eq!(wishlist_search_json["query"], "A One");

    let collection = crate::route_http_request(
        "POST",
        "/api/collections",
        None,
        r#"{"name":"Shared"}"#,
        &state,
    )
    .await
    .expect("collection");
    let collection_json = serde_json::from_str::<serde_json::Value>(&collection.body).unwrap();
    let collection_id = collection_json["id"].as_str().unwrap();
    let solid = crate::route_http_request("GET", "/api/solid/status", None, "", &state)
        .await
        .expect("solid status");
    let solid_json = serde_json::from_str::<serde_json::Value>(&solid.body).unwrap();
    assert_eq!(solid_json["enabled"], true);
    crate::route_http_request(
        "POST",
        &format!("/api/collections/{collection_id}/items"),
        None,
        r#"{"content_id":"lib-1","artist":"A","title":"One"}"#,
        &state,
    )
    .await
    .expect("collection item");
    let grant = crate::route_http_request(
        "POST",
        "/api/share-grants",
        None,
        &format!(r#"{{"collection_id":"{collection_id}","username":"peer"}}"#),
        &state,
    )
    .await
    .expect("share grant");
    let grant_json = serde_json::from_str::<serde_json::Value>(&grant.body).unwrap();
    let grant_id = grant_json["id"].as_str().unwrap();
    let token = crate::route_http_request(
        "POST",
        &format!("/api/share-grants/{grant_id}/token"),
        None,
        "{}",
        &state,
    )
    .await
    .expect("share grant token");
    let token_json = serde_json::from_str::<serde_json::Value>(&token.body).unwrap();
    assert_eq!(token_json["created"], true);
    assert_eq!(token_json["token"].as_str().unwrap().len(), 70);
    assert_eq!(token_json["persisted"], false);
    assert_eq!(token_json["status"], "ephemeral_compatibility_token");
    let backfill = crate::route_http_request(
        "POST",
        &format!("/api/share-grants/{grant_id}/backfill"),
        None,
        "{}",
        &state,
    )
    .await
    .expect("share grant backfill");
    let backfill_json = serde_json::from_str::<serde_json::Value>(&backfill.body).unwrap();
    assert_eq!(backfill_json["backfilled"], 1);
    assert_eq!(backfill_json["persisted"], true);
    assert_eq!(backfill_json["status"], "local");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn compatibility_noop_routes_advertise_supported_shape() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));

    let logs = crate::route_http_request("GET", "/api/logs", None, "", &state)
        .await
        .expect("logs");
    assert_eq!(logs.status, "200 OK");
    let logs_json = serde_json::from_str::<serde_json::Value>(&logs.body).unwrap();
    assert_eq!(logs_json["entries"].as_array().unwrap().len(), 0);
    assert_eq!(logs_json["level"], "Information");
    let compat_logs = crate::route_http_request("GET", "/api/v0/logs", None, "", &state)
        .await
        .expect("compat logs");
    let compat_logs_json = serde_json::from_str::<serde_json::Value>(&compat_logs.body).unwrap();
    assert_eq!(compat_logs_json.as_array().unwrap().len(), 0);

    let bridge =
        crate::route_http_request("PUT", "/api/v0/bridge/admin/config", None, "{}", &state)
            .await
            .expect("bridge config");
    assert_eq!(bridge.status, "200 OK");
    let bridge_json = serde_json::from_str::<serde_json::Value>(&bridge.body).unwrap();
    assert_eq!(bridge_json["persisted"], true);
    assert_eq!(bridge_json["restart_required"], true);
    assert_eq!(bridge_json["configUpdates"], 1);

    let username_ban = crate::route_http_request(
        "POST",
        "/api/bans/username",
        None,
        r#"{"username":"peer1"}"#,
        &state,
    )
    .await
    .expect("username ban");
    let username_ban_json = serde_json::from_str::<serde_json::Value>(&username_ban.body).unwrap();
    assert_eq!(username_ban_json["banned"], true);
    assert_eq!(username_ban_json["persisted"], false);
    assert_eq!(username_ban_json["activeBans"], 1);
    let bans = crate::route_http_request("GET", "/api/bans", None, "", &state)
        .await
        .expect("bans");
    let bans_json = serde_json::from_str::<serde_json::Value>(&bans.body).unwrap();
    assert_eq!(bans_json["count"], 1);
    assert_eq!(bans_json["bans"][0]["value"], "peer1");
    let unban = crate::route_http_request("DELETE", "/api/bans/username/peer1", None, "", &state)
        .await
        .expect("username unban");
    let unban_json = serde_json::from_str::<serde_json::Value>(&unban.body).unwrap();
    assert_eq!(unban_json["removed"], true);
    assert_eq!(unban_json["persisted"], false);
    assert_eq!(unban_json["activeBans"], 0);

    let share_token = crate::route_http_request(
        "POST",
        "/api/share-grants/grant-1/token",
        None,
        "{}",
        &state,
    )
    .await
    .expect("share grant token");
    let share_token_json = serde_json::from_str::<serde_json::Value>(&share_token.body).unwrap();
    assert_eq!(share_token_json["token"], serde_json::Value::Null);
    assert_eq!(share_token_json["created"], false);
    assert_eq!(share_token_json["persisted"], false);
    assert_eq!(share_token_json["status"], "compatibility_acknowledgement");

    let subscriptions = crate::route_http_request(
        "GET",
        "/api/musicbrainz/release-radar/subscriptions",
        None,
        "",
        &state,
    )
    .await
    .expect("subscriptions");
    let subscriptions_json =
        serde_json::from_str::<serde_json::Value>(&subscriptions.body).unwrap();
    assert_eq!(subscriptions_json.as_array().unwrap().len(), 0);

    let created_subscription = crate::route_http_request(
        "POST",
        "/api/musicbrainz/release-radar/subscriptions",
        None,
        "{}",
        &state,
    )
    .await
    .expect("subscription create");
    let created_subscription_json =
        serde_json::from_str::<serde_json::Value>(&created_subscription.body).unwrap();
    assert_eq!(created_subscription_json["created"], true);
    assert_eq!(created_subscription_json["persisted"], true);
    assert_eq!(created_subscription_json["status"], "local");
    assert_eq!(created_subscription_json["count"], 1);
    assert_eq!(
        created_subscription_json["subscriptions"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}
