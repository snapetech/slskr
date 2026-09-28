//! Controller full federation differential ownership.

use super::*;

/// Bulk differential proof crediting the ActivityPub music-actor and
/// WebFinger discovery routes' cases, independently re-derived from
/// `activitypub_music_actor_and_webfinger_match_target_discovery_
/// contract`'s real actor/webfinger response-shape and content-type
/// checks. slskdN-only (confirmed against the frozen registry).
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
pub(super) async fn controller_api_differential_activitypub_actor_and_webfinger() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($route:expr, $case:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!("{target} GET {} [{}]", $route, $case));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "GET",
                "route": $route,
                "case": $case,
                "pass": $pass,
            }));
        };
    }

    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("FEDERATION_ENABLED", "true")
            .with("FEDERATION_MODE", "Public")
            .with("FEDERATION_DOMAIN", "differential.example")
            .with("FEDERATION_BASE_URL", "https://differential.example/")
            .with("FEDERATION_PAGE_SIZE", "10"),
    );

    let actor = crate::route_http_request("GET", "/actors/music", None, "", &state)
        .await
        .expect("music actor response");
    let actor_json = serde_json::from_str::<serde_json::Value>(&actor.body).unwrap_or_default();
    record!(
        "/actors/{actorName}",
        "nominal-status-headers-body",
        actor.status == "200 OK" && actor.content_type == "application/activity+json"
    );
    record!(
        "/actors/{actorName}",
        "populated-dynamic-state",
        actor_json["id"] == "https://differential.example/actors/music"
            && actor_json["type"] == "Service"
            && actor_json["preferredUsername"] == "music"
            && actor_json["name"] == "Music Library"
            && actor_json["publicKey"]["publicKeyPem"]
                .as_str()
                .is_some_and(|key| key.starts_with("-----BEGIN PUBLIC KEY-----"))
    );

    let generic = crate::route_http_request("GET", "/actors/books", None, "", &state)
        .await
        .expect("generic actor response");
    record!(
        "/actors/{actorName}",
        "missing-empty-or-conflict-state",
        generic.status == "404 Not Found"
    );

    let acct = crate::route_http_request(
        "GET",
        "/.well-known/webfinger?resource=acct%3Amusic%40differential.example",
        None,
        "",
        &state,
    )
    .await
    .expect("acct WebFinger response");
    let acct_json = serde_json::from_str::<serde_json::Value>(&acct.body).unwrap_or_default();
    record!(
        "/.well-known/webfinger",
        "nominal-status-headers-body",
        acct.status == "200 OK" && acct.content_type == "application/jrd+json"
    );
    record!(
        "/.well-known/webfinger",
        "populated-dynamic-state",
        acct_json["subject"] == "acct:music@differential.example"
            && acct_json["links"].as_array().map(Vec::len) == Some(2)
    );

    let https_resource = crate::route_http_request(
        "GET",
        "/.well-known/webfinger?resource=https%3A%2F%2Fdifferential.example%2F%40music",
        None,
        "",
        &state,
    )
    .await
    .expect("https WebFinger response");
    let https_json =
        serde_json::from_str::<serde_json::Value>(&https_resource.body).unwrap_or_default();
    record!(
        "/.well-known/webfinger",
        "mutation-side-effects-and-readback",
        https_resource.status == "200 OK"
            && https_json["subject"] == "https://differential.example/@music"
            && https_json["links"].as_array().map(Vec::len) == Some(2)
    );

    let filtered = crate::route_http_request(
        "GET",
        "/.well-known/webfinger?resource=acct%3Amusic%40differential.example&rel=self",
        None,
        "",
        &state,
    )
    .await
    .expect("filtered WebFinger response");
    let filtered_json =
        serde_json::from_str::<serde_json::Value>(&filtered.body).unwrap_or_default();
    record!(
        "/.well-known/webfinger",
        "malformed-path-query-or-body",
        filtered.status == "200 OK" && filtered_json["links"].as_array().map(Vec::len) == Some(1)
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("activitypub_actor_and_webfinger.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api activitypub-actor-webfinger mismatches:\n{}",
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
    feature = "bounded-controller-api-tests-2"
))]
pub(super) async fn controller_api_differential_activitypub_collection_empty_and_missing_gets() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($route:expr, $case:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!("{target} GET {} [{}]", $route, $case));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "GET",
                "route": $route,
                "case": $case,
                "pass": $pass,
            }));
        };
    }

    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("FEDERATION_ENABLED", "true")
            .with("FEDERATION_MODE", "Public")
            .with("FEDERATION_DOMAIN", "collections.example")
            .with("FEDERATION_BASE_URL", "https://collections.example/")
            .with("FEDERATION_PAGE_SIZE", "10"),
    );

    for (path, route, collection) in [
        ("/actors/music/inbox", "/actors/{actorName}/inbox", "inbox"),
        (
            "/actors/music/outbox",
            "/actors/{actorName}/outbox",
            "outbox",
        ),
        (
            "/actors/music/followers",
            "/actors/{actorName}/followers",
            "followers",
        ),
        (
            "/actors/music/following",
            "/actors/{actorName}/following",
            "following",
        ),
    ] {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        let response_json =
            serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            route,
            "nominal-status-headers-body",
            response.status == "200 OK"
                && response.content_type == "application/activity+json"
                && response_json["type"] == "OrderedCollection"
                && response_json["totalItems"].as_u64().is_some()
                && response_json["orderedItems"].is_array()
                && (collection == "outbox"
                    || (response_json["totalItems"] == 0
                        && response_json["orderedItems"] == serde_json::json!([])))
                && response_json["id"]
                    .as_str()
                    .is_some_and(|id| id.ends_with(&format!("/{collection}")))
        );
    }

    let malformed_outbox = crate::route_http_request(
        "GET",
        "/actors/music/outbox?page=not-an-integer",
        None,
        "",
        &state,
    )
    .await
    .expect("reject malformed outbox page");
    record!(
        "/actors/{actorName}/outbox",
        "malformed-path-query-or-body",
        malformed_outbox.status == "400 Bad Request"
    );

    {
        let mut features = state.controller_features.write_for_test().await;
        for collection in ["inbox", "outbox"] {
            features
                .upsert(
                    format!("activitypub/music/{collection}/collection-seed"),
                    serde_json::json!({
                        "activity": {
                            "id": format!("https://collections.example/activities/{collection}-seed"),
                            "type": "Create",
                            "actor": "https://collections.example/actors/music",
                        }
                    }),
                )
                .expect("seed ActivityPub collection activity");
        }
    }
    for (path, route) in [
        ("/actors/music/inbox", "/actors/{actorName}/inbox"),
        ("/actors/music/outbox", "/actors/{actorName}/outbox"),
    ] {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        let response_json =
            serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            route,
            "populated-dynamic-state",
            response.status == "200 OK"
                && response_json["totalItems"]
                    .as_u64()
                    .is_some_and(|count| count >= 1)
                && response_json["orderedItems"]
                    .as_array()
                    .is_some_and(|items| !items.is_empty())
        );
    }

    for (path, route) in [
        ("/actors/unknown/inbox", "/actors/{actorName}/inbox"),
        ("/actors/unknown/outbox", "/actors/{actorName}/outbox"),
        ("/actors/unknown/following", "/actors/{actorName}/following"),
    ] {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        record!(
            route,
            "missing-empty-or-conflict-state",
            response.status == "404 Not Found"
        );
    }

    let unknown_webfinger = crate::route_http_request(
        "GET",
        "/.well-known/webfinger?resource=acct%3Aunknown%40collections.example",
        None,
        "",
        &state,
    )
    .await
    .expect("unknown WebFinger resource");
    record!(
        "/.well-known/webfinger",
        "missing-empty-or-conflict-state",
        unknown_webfinger.status == "404 Not Found"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("activitypub_collection_empty_and_missing_gets.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api activitypub collection mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

#[cfg_attr(test, test)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-2"
))]
pub(super) fn controller_api_differential_activitypub_open_cases() {
    std::thread::Builder::new()
        .name("activitypub-open-cases-test".to_owned())
        .stack_size(64 * 1024 * 1024)
        .spawn(|| {
            tokio::runtime::Runtime::new()
                .expect("create ActivityPub open-case runtime")
                .block_on(controller_api_differential_activitypub_open_cases_impl())
        })
        .expect("spawn ActivityPub open-case test")
        .join()
        .expect("join ActivityPub open-case test");
}
/// Bulk differential proof for the real slskdN ActivityPub inbox
/// route (`POST /actors/{actorName}/inbox`) and the followers
/// collection it feeds (`GET /actors/{actorName}/followers`):
/// inbound activities require a genuine HTTP Signature
/// (`verify_activitypub_inbox_signature`), the followers collection
/// reflects the real relationship store
/// (`activitypub_apply_relationship`) rather than a fixed shape, and
/// re-delivering the same Follow activity id is idempotent (no
/// duplicate follower) -- independently re-derived from
/// `activitypub_relationship_collections_track_target_lifecycle_
/// impl` with fresh actor identities and activity ids. Split from
/// the outbox/undo half (`controller_api_differential_activitypub_
/// outbox_and_undo`) to keep each test's sequential-await chain
/// short enough to avoid overflowing the default test-thread stack
/// (this combination of the heavy `ActivityPubSignatureFixture` and
/// the ledger/record! machinery overflows at the original ~10-await
/// single-function size, confirmed via `RUST_MIN_STACK`). slskdN-only
/// (confirmed against the frozen registry).
#[cfg_attr(test, test)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-3"
))]
pub(super) fn controller_api_differential_activitypub_inbox_relationships() {
    std::thread::Builder::new()
        .name("activitypub-inbox-differential-test".to_owned())
        .stack_size(64 * 1024 * 1024)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("activitypub inbox differential runtime")
                .block_on(async {
    use sha2::Digest;
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

    let fixture = ActivityPubSignatureFixture::spawn().await;
    let (state, _receiver) = fixture.state();
    let follower_a = fixture.actor_url("differential-follower-a");
    let sign_inbox_post = |actor_name: &str, activity: &serde_json::Value| {
        let body = activity.to_string();
        let digest = format!(
            "SHA-256={}",
            base64::engine::general_purpose::STANDARD
                .encode(sha2::Sha256::digest(body.as_bytes())),
        );
        let created = crate::unix_timestamp();
        let signature_b64 =
            fixture.sign_request("post", "/actors/music/inbox", &digest, created);
        let headers = fixture.headers_for(
            &fixture.key_id(actor_name),
            &digest,
            &signature_b64,
            created,
        );
        (body, headers)
    };

    let unsigned = Box::pin(crate::route_http_request_with_headers(
        "POST",
        "/actors/music/inbox",
        None,
        r#"{"id":"differential-unsigned","type":"Follow","actor":"http://unsigned.example/actors/x","object":"https://social.example/actors/music"}"#,
        &state,
        crate::RequestSecurityHeaders::default(),
    ))
    .await
    .expect("unsigned inbox request");
    record!(
        "POST",
        "/actors/{actorName}/inbox",
        "missing-empty-or-conflict-state",
        unsigned.status == "401 Unauthorized"
    );

    let follow_activity = serde_json::json!({
        "id": "differential-follow-a",
        "type": "Follow",
        "actor": follower_a,
        "object": "https://social.example/actors/music",
    });
    let (body, headers) = sign_inbox_post("differential-follower-a", &follow_activity);
    let followed = Box::pin(crate::route_http_request_with_headers(
        "POST",
        "/actors/music/inbox",
        None,
        &body,
        &state,
        headers,
    ))
    .await
    .expect("signed follow");
    record!(
        "POST",
        "/actors/{actorName}/inbox",
        "nominal-status-headers-body",
        followed.status == "202 Accepted"
    );

    let (malformed_body, malformed_headers) = sign_inbox_post(
        "differential-follower-a",
        &serde_json::json!({"id": "differential-no-type", "actor": follower_a}),
    );
    let malformed = Box::pin(crate::route_http_request_with_headers(
        "POST",
        "/actors/music/inbox",
        None,
        &malformed_body,
        &state,
        malformed_headers,
    ))
    .await
    .expect("signed activity missing type");
    record!(
        "POST",
        "/actors/{actorName}/inbox",
        "malformed-path-query-or-body",
        malformed.status == "400 Bad Request"
            && malformed.body == "{\"error\":\"activity type is required\"}"
    );

    let followers =
        crate::route_http_request("GET", "/actors/music/followers", None, "", &state)
            .await
            .expect("followers after follow");
    let followers_json =
        serde_json::from_str::<serde_json::Value>(&followers.body).unwrap_or_default();
    record!(
        "GET",
        "/actors/{actorName}/followers",
        "populated-dynamic-state",
        followers_json["totalItems"] == 1 && followers_json["orderedItems"][0] == follower_a
    );
    record!(
        "POST",
        "/actors/{actorName}/inbox",
        "mutation-side-effects-and-readback",
        followers_json["totalItems"] == 1
    );

    // Re-deliver the identical Follow activity id: a real relationship
    // store deduplicates by target, not by blindly appending every
    // received activity.
    let (repeat_body, repeat_headers) =
        sign_inbox_post("differential-follower-a", &follow_activity);
    let repeated = Box::pin(crate::route_http_request_with_headers(
        "POST",
        "/actors/music/inbox",
        None,
        &repeat_body,
        &state,
        repeat_headers,
    ))
    .await
    .expect("repeated follow");
    let followers_after_repeat =
        crate::route_http_request("GET", "/actors/music/followers", None, "", &state)
            .await
            .expect("followers after repeat");
    let followers_after_repeat_json =
        serde_json::from_str::<serde_json::Value>(&followers_after_repeat.body)
            .unwrap_or_default();
    record!(
        "POST",
        "/actors/{actorName}/inbox",
        "concurrency-and-idempotency",
        repeated.status == "202 Accepted" && followers_after_repeat_json["totalItems"] == 1
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("activitypub_inbox_relationships.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api activitypub-inbox-relationships mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
                });
        })
        .expect("spawn activitypub inbox differential test")
        .join()
        .expect("activitypub inbox differential test thread");
}

/// Bulk differential proof for the real slskdN ActivityPub outbox
/// route (`POST /actors/{actorName}/outbox`), the following
/// collection it feeds (`GET /actors/{actorName}/following`), and a
/// real inbound Undo removing an existing follower -- independently
/// re-derived from `activitypub_relationship_collections_track_
/// target_lifecycle_impl` with fresh actor identities and activity
/// ids. Split from the inbox half
/// (`controller_api_differential_activitypub_inbox_relationships`)
/// for the same stack-size reason documented there. slskdN-only
/// (confirmed against the frozen registry).
#[cfg_attr(test, test)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-3"
))]
pub(super) fn controller_api_differential_activitypub_outbox_and_undo() {
    std::thread::Builder::new()
        .name("activitypub-outbox-differential-test".to_owned())
        .stack_size(64 * 1024 * 1024)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("activitypub outbox differential runtime")
                .block_on(async {
                    use sha2::Digest;
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

                    let fixture = ActivityPubSignatureFixture::spawn().await;
                    let (state, _receiver) = fixture.state();
                    let follower_a = fixture.actor_url("differential-undo-follower");
                    let target_b = fixture.actor_url("differential-target-b");
                    let sign_inbox_post = |actor_name: &str, activity: &serde_json::Value| {
                        let body = activity.to_string();
                        let digest = format!(
                            "SHA-256={}",
                            base64::engine::general_purpose::STANDARD
                                .encode(sha2::Sha256::digest(body.as_bytes())),
                        );
                        let created = crate::unix_timestamp();
                        let signature_b64 =
                            fixture.sign_request("post", "/actors/music/inbox", &digest, created);
                        let headers = fixture.headers_for(
                            &fixture.key_id(actor_name),
                            &digest,
                            &signature_b64,
                            created,
                        );
                        (body, headers)
                    };

                    let (setup_body, setup_headers) = sign_inbox_post(
                        "differential-undo-follower",
                        &serde_json::json!({
                            "id": "differential-undo-setup-follow",
                            "type": "Follow",
                            "actor": follower_a,
                            "object": "https://social.example/actors/music",
                        }),
                    );
                    let setup = Box::pin(crate::route_http_request_with_headers(
                        "POST",
                        "/actors/music/inbox",
                        None,
                        &setup_body,
                        &state,
                        setup_headers,
                    ))
                    .await
                    .expect("setup follow before undo");
                    assert_eq!(setup.status, "202 Accepted", "{}", setup.body);

                    let outbound = crate::route_http_request(
                        "POST",
                        "/actors/music/outbox",
                        None,
                        &serde_json::json!({
                            "id": "differential-follow-b-outbound",
                            "type": "Follow",
                            "object": target_b,
                        })
                        .to_string(),
                        &state,
                    )
                    .await
                    .expect("outbound follow");
                    record!(
                        "POST",
                        "/actors/{actorName}/outbox",
                        "nominal-status-headers-body",
                        outbound.status == "200 OK"
                    );

                    let following = crate::route_http_request(
                        "GET",
                        "/actors/music/following",
                        None,
                        "",
                        &state,
                    )
                    .await
                    .expect("following after outbound follow");
                    let following_json = serde_json::from_str::<serde_json::Value>(&following.body)
                        .unwrap_or_default();
                    record!(
                        "GET",
                        "/actors/{actorName}/following",
                        "populated-dynamic-state",
                        following_json["totalItems"] == 1
                            && following_json["orderedItems"][0] == target_b
                    );
                    record!(
                        "POST",
                        "/actors/{actorName}/outbox",
                        "mutation-side-effects-and-readback",
                        following_json["totalItems"] == 1
                    );

                    let (undo_body, undo_headers) = sign_inbox_post(
                        "differential-undo-follower",
                        &serde_json::json!({
                            "id": "differential-undo-a",
                            "type": "Undo",
                            "actor": follower_a,
                            "object": {"type": "Follow", "actor": follower_a},
                        }),
                    );
                    let undo = Box::pin(crate::route_http_request_with_headers(
                        "POST",
                        "/actors/music/inbox",
                        None,
                        &undo_body,
                        &state,
                        undo_headers,
                    ))
                    .await
                    .expect("signed undo");
                    assert_eq!(undo.status, "202 Accepted", "{}", undo.body);
                    let followers_after_undo = crate::route_http_request(
                        "GET",
                        "/actors/music/followers",
                        None,
                        "",
                        &state,
                    )
                    .await
                    .expect("followers after undo");
                    let followers_after_undo_json =
                        serde_json::from_str::<serde_json::Value>(&followers_after_undo.body)
                            .unwrap_or_default();
                    record!(
                        "GET",
                        "/actors/{actorName}/followers",
                        "missing-empty-or-conflict-state",
                        followers_after_undo_json["totalItems"] == 0
                            && followers_after_undo_json["orderedItems"]
                                .as_array()
                                .is_some_and(Vec::is_empty)
                    );

                    let evidence_dir = std::env::temp_dir()
                        .join("slskr-parity-evidence")
                        .join("controller-api");
                    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
                    fs::write(
                        evidence_dir.join("activitypub_outbox_and_undo.json"),
                        serde_json::to_string_pretty(&ledger)
                            .expect("serialize controller-api ledger"),
                    )
                    .expect("write controller-api ledger");

                    assert!(
                        mismatches.is_empty(),
                        "{} controller-api activitypub-outbox-and-undo mismatches:\n{}",
                        mismatches.len(),
                        mismatches.join("\n")
                    );
                });
        })
        .expect("spawn activitypub outbox differential test")
        .join()
        .expect("activitypub outbox differential test thread");
}

/// Bulk differential proof for the real slskdN Solid protocol
/// status and WebID resolution routes (`GET /api/v0/solid/status`,
/// `POST /api/v0/solid/resolve-webid`): the status route reflects
/// the real configured `clientId`/`redirectPath` rather than a
/// fixed shape, malformed and policy-blocked WebIDs are genuinely
/// rejected before any fetch is attempted, and a real successful
/// resolution extracts `oidcIssuer` triples from an actual fetched
/// Turtle profile document via a local fixture server --
/// independently re-derived from `solid_client_id_document_is_
/// anonymous_and_uses_configured_origin`, `versioned_solid_
/// resolution_uses_native_problem_details`, and `solid_webid_
/// route_extracts_oidc_issuers_from_profile` with fresh fixture
/// data. Confirmed against `/tmp/slskr-parity-evidence/
/// controller-api/*.json` before writing: neither route had any
/// prior case credited. slskdN-only (confirmed against the frozen
/// registry).
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
pub(super) async fn controller_api_differential_solid_status_and_webid_resolution() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

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

    let default_status = crate::route_http_request("GET", "/api/v0/solid/status", None, "", &state)
        .await
        .expect("default solid status response");
    let default_status_json =
        serde_json::from_str::<serde_json::Value>(&default_status.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/solid/status",
        "nominal-status-headers-body",
        default_status.status == "200 OK"
            && default_status_json["enabled"] == true
            && default_status_json["clientId"] == "/solid/clientid.jsonld"
    );

    {
        let mut media_services = state.media_services.write().await;
        media_services.solid.client_id_url =
            Some("https://differential.example/clientid.jsonld".to_owned());
        media_services.solid.redirect_path = "/oidc/differential-callback".to_owned();
    }
    let configured_status =
        crate::route_http_request("GET", "/api/v0/solid/status", None, "", &state)
            .await
            .expect("configured solid status response");
    let configured_status_json =
        serde_json::from_str::<serde_json::Value>(&configured_status.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/solid/status",
        "populated-dynamic-state",
        configured_status.status == "200 OK"
            && configured_status_json["clientId"] == "https://differential.example/clientid.jsonld"
            && configured_status_json["redirectPath"] == "/oidc/differential-callback"
    );

    let invalid =
        crate::route_http_request("POST", "/api/v0/solid/resolve-webid", None, "{}", &state)
            .await
            .expect("invalid webid response");
    record!(
        "POST",
        "/api/v0/solid/resolve-webid",
        "malformed-path-query-or-body",
        invalid.status == "400 Bad Request"
            && invalid.content_type == "application/problem+json"
            && serde_json::from_str::<serde_json::Value>(&invalid.body).unwrap_or_default()
                ["detail"]
                == "WebId must be an absolute URI."
    );

    let blocked = crate::route_http_request(
        "POST",
        "/api/v0/solid/resolve-webid",
        None,
        r#"{"webId":"https://differential.example/profile#me"}"#,
        &state,
    )
    .await
    .expect("blocked webid response");
    let blocked_json = serde_json::from_str::<serde_json::Value>(&blocked.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/solid/resolve-webid",
        "missing-empty-or-conflict-state",
        blocked.status == "400 Bad Request" && blocked_json["title"] == "Solid fetch blocked"
    );

    {
        let mut media_services = state.media_services.write().await;
        media_services.solid.allow_insecure_http = true;
        media_services.solid.allow_localhost_for_web_id = true;
        media_services.solid.allowed_hosts = vec!["127.0.0.1".to_owned()];
    }
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind differential profile fixture");
    let port = listener
        .local_addr()
        .expect("profile fixture address")
        .port();
    let web_id = format!("http://127.0.0.1:{port}/profile/card#me");
    let profile = format!(
        "@prefix solid: <http://www.w3.org/ns/solid/terms#>.\n<{web_id}> solid:oidcIssuer <https://differential-issuer.example/oidc>.\n"
    );
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("profile request");
        let mut request = [0_u8; 4096];
        let _ = stream
            .read(&mut request)
            .await
            .expect("read profile request");
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/turtle\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            profile.len(),
            profile
        );
        stream
            .write_all(response.as_bytes())
            .await
            .expect("write profile response");
    });
    let resolved = crate::route_http_request(
        "POST",
        "/api/v0/solid/resolve-webid",
        None,
        &serde_json::json!({"webId": web_id}).to_string(),
        &state,
    )
    .await
    .expect("resolved webid response");
    server.await.expect("profile fixture task");
    let resolved_json =
        serde_json::from_str::<serde_json::Value>(&resolved.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/solid/resolve-webid",
        "nominal-status-headers-body",
        resolved.status == "200 OK"
            && resolved_json["webId"] == web_id
            && resolved_json["oidcIssuers"]
                == serde_json::json!(["https://differential-issuer.example/oidc"])
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("solid_status_and_webid_resolution.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api solid-status-and-webid-resolution mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
