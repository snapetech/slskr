use super::{test_state, test_state_with_env, MapEnv};

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn collections_are_scoped_to_the_real_authenticated_caller_identity() {
    // Matches the oracle's real AuthenticatedWebUserId-based
    // ownership (CollectionsController.cs): a collection created by
    // one authenticated identity must not be visible to, or mutable
    // by, a different one. Previously `owner_user_id` was always a
    // hardcoded "Anonymous"/empty placeholder, never checked on any
    // read or write -- any caller could view/edit/delete any other
    // caller's collections.
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
        r#"{"title":"Alice Collection"}"#,
        &state,
    )
    .await
    .expect("alice creates a collection");
    assert_eq!(created.status, "201 Created", "{}", created.body);
    let created_json = serde_json::from_str::<serde_json::Value>(&created.body).unwrap();
    assert_eq!(created_json["ownerUserId"], "alice");
    let collection_id = created_json["id"].as_str().unwrap().to_owned();

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
        assert_eq!(response.status, "404 Not Found", "{method} {path}");
    }

    // Alice can still read/mutate her own collection.
    let alice_get = crate::route_http_request(
        "GET",
        &format!("/api/v0/collections/{collection_id}"),
        alice,
        "",
        &state,
    )
    .await
    .expect("alice reads her own collection");
    assert_eq!(alice_get.status, "200 OK");

    // Bob's own collection list never includes Alice's collection,
    // and vice versa.
    let bob_created = crate::route_http_request(
        "POST",
        "/api/v0/collections",
        bob,
        r#"{"title":"Bob Collection"}"#,
        &state,
    )
    .await
    .expect("bob creates his own collection");
    assert_eq!(bob_created.status, "201 Created");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&bob_created.body).unwrap()["ownerUserId"],
        "bob"
    );

    let alice_list = crate::route_http_request("GET", "/api/v0/collections", alice, "", &state)
        .await
        .expect("alice lists collections");
    let alice_list_json = serde_json::from_str::<serde_json::Value>(&alice_list.body).unwrap();
    let alice_titles = alice_list_json
        .as_array()
        .unwrap()
        .iter()
        .map(|record| record["title"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(alice_titles, vec!["Alice Collection"]);

    let bob_list = crate::route_http_request("GET", "/api/v0/collections", bob, "", &state)
        .await
        .expect("bob lists collections");
    let bob_list_json = serde_json::from_str::<serde_json::Value>(&bob_list.body).unwrap();
    let bob_titles = bob_list_json
        .as_array()
        .unwrap()
        .iter()
        .map(|record| record["title"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(bob_titles, vec!["Bob Collection"]);

    // Bob cannot delete Alice's collection.
    let bob_delete = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/collections/{collection_id}"),
        bob,
        "",
        &state,
    )
    .await
    .expect("bob attempts to delete alice's collection");
    assert_eq!(bob_delete.status, "404 Not Found");
    let still_there = crate::route_http_request(
        "GET",
        &format!("/api/v0/collections/{collection_id}"),
        alice,
        "",
        &state,
    )
    .await
    .expect("alice's collection still exists");
    assert_eq!(still_there.status, "200 OK");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn share_grants_are_scoped_to_the_real_collection_owner() {
    // Matches the oracle's real Share-Grants ownership gate
    // (SharesController.cs): a grant is owned transitively through
    // its collection, so every action 404s unless
    // `collection.OwnerUserId == currentUserId`. Previously there was
    // no ownership check anywhere -- any authenticated caller could
    // view, mutate, delete, or mint access tokens for any other
    // caller's share grants.
    let keys = serde_json::json!({
        "alice": {"key": "alice-key-0123456789", "role": "administrator", "cidr": ""},
        "bob": {"key": "bob-key-00123456789ab", "role": "administrator", "cidr": ""},
    });
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "false")
            .with("SLSKD_API_KEYS_JSON", &keys.to_string()),
    );
    let alice = Some("ApiKey alice-key-0123456789");
    let bob = Some("ApiKey bob-key-00123456789ab");

    let collection = crate::route_http_request(
        "POST",
        "/api/v0/collections",
        alice,
        r#"{"title":"Alice Private"}"#,
        &state,
    )
    .await
    .expect("alice creates a collection");
    assert_eq!(collection.status, "201 Created");
    let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    // Bob cannot create a grant against a collection he doesn't own.
    let bob_create = crate::route_http_request(
        "POST",
        "/api/v0/share-grants",
        bob,
        &format!(r#"{{"collection_id":"{collection_id}","username":"recipient"}}"#),
        &state,
    )
    .await
    .expect("bob attempts to grant alice's collection");
    assert_eq!(bob_create.status, "404 Not Found");

    let granted = crate::route_http_request(
        "POST",
        "/api/v0/share-grants",
        alice,
        &format!(r#"{{"collection_id":"{collection_id}","username":"recipient"}}"#),
        &state,
    )
    .await
    .expect("alice grants her own collection");
    assert_eq!(granted.status, "201 Created", "{}", granted.body);
    let grant_id = serde_json::from_str::<serde_json::Value>(&granted.body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    for (method, path, body) in [
        ("GET", format!("/api/share-grants/{grant_id}"), ""),
        (
            "PUT",
            format!("/api/v0/share-grants/{grant_id}"),
            r#"{"permissions":"read,download"}"#,
        ),
        (
            "GET",
            format!("/api/v0/share-grants/by-collection/{collection_id}"),
            "",
        ),
        (
            "POST",
            format!("/api/v0/share-grants/{grant_id}/token"),
            "{}",
        ),
    ] {
        let response = crate::route_http_request(method, &path, bob, body, &state)
            .await
            .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
        assert_eq!(response.status, "404 Not Found", "{method} {path}");
    }

    // Bob's own grant list never includes Alice's grant.
    let bob_list = crate::route_http_request("GET", "/api/v0/share-grants", bob, "", &state)
        .await
        .expect("bob lists share grants");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&bob_list.body)
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        0
    );

    // Alice can still manage her own grant.
    let alice_get = crate::route_http_request(
        "GET",
        &format!("/api/share-grants/{grant_id}"),
        alice,
        "",
        &state,
    )
    .await
    .expect("alice reads her own grant");
    assert_eq!(alice_get.status, "200 OK");

    let alice_token = crate::route_http_request(
        "POST",
        &format!("/api/v0/share-grants/{grant_id}/token"),
        alice,
        "{}",
        &state,
    )
    .await
    .expect("alice mints a token for her own grant");
    assert_eq!(alice_token.status, "201 Created", "{}", alice_token.body);

    // Bob cannot delete Alice's grant, either.
    let bob_delete = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/share-grants/{grant_id}"),
        bob,
        "",
        &state,
    )
    .await
    .expect("bob attempts to delete alice's grant");
    assert_eq!(bob_delete.status, "404 Not Found");
    let still_there = crate::route_http_request(
        "GET",
        &format!("/api/share-grants/{grant_id}"),
        alice,
        "",
        &state,
    )
    .await
    .expect("alice's grant still exists");
    assert_eq!(still_there.status, "200 OK");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn v0_share_grants_get_real_uuid_ids_usable_on_versioned_routes() {
    // Matches the oracle's real ShareGrant.Id (a Guid), and the
    // existing `versioned_get_failure_contract` UUID-format guard
    // that v0 share-grant routes already enforced -- previously
    // ShareGrantStore always minted sequential "grant-N" ids
    // regardless of API version (unlike CollectionStore, which
    // already mints a real UUID for v0 requests), so any v0
    // GET/PUT/DELETE by id always 400'd against that same guard for
    // a real grant, no matter what id was passed.
    let (state, _receiver) = test_state();
    let collection = crate::route_http_request(
        "POST",
        "/api/v0/collections",
        None,
        r#"{"title":"Versioned Grants"}"#,
        &state,
    )
    .await
    .expect("create collection");
    let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    let created = crate::route_http_request(
        "POST",
        "/api/v0/share-grants",
        None,
        &format!(r#"{{"collection_id":"{collection_id}","username":"friend"}}"#),
        &state,
    )
    .await
    .expect("create share grant via the v0 route");
    assert_eq!(created.status, "201 Created", "{}", created.body);
    let grant_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(
        uuid::Uuid::parse_str(&grant_id).is_ok(),
        "v0-created share grant id must be a real UUID: {grant_id}"
    );

    let get = crate::route_http_request(
        "GET",
        &format!("/api/v0/share-grants/{grant_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("get share grant via the v0 route");
    assert_eq!(get.status, "200 OK", "{}", get.body);

    let update = crate::route_http_request(
        "PUT",
        &format!("/api/v0/share-grants/{grant_id}"),
        None,
        r#"{"permissions":"read,download"}"#,
        &state,
    )
    .await
    .expect("update share grant via the v0 route");
    assert_eq!(update.status, "200 OK", "{}", update.body);

    let delete = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/share-grants/{grant_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("delete share grant via the v0 route");
    assert_eq!(delete.status, "200 OK", "{}", delete.body);
}
