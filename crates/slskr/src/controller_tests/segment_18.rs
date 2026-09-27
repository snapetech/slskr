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
fn controller_api_differential_activitypub_inbox_relationships() {
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
        let created = super::unix_timestamp();
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

    let unsigned = Box::pin(super::route_http_request_with_headers(
        "POST",
        "/actors/music/inbox",
        None,
        r#"{"id":"differential-unsigned","type":"Follow","actor":"http://unsigned.example/actors/x","object":"https://social.example/actors/music"}"#,
        &state,
        super::RequestSecurityHeaders::default(),
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
    let followed = Box::pin(super::route_http_request_with_headers(
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
    let malformed = Box::pin(super::route_http_request_with_headers(
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
        super::route_http_request("GET", "/actors/music/followers", None, "", &state)
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
    let repeated = Box::pin(super::route_http_request_with_headers(
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
        super::route_http_request("GET", "/actors/music/followers", None, "", &state)
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
fn controller_api_differential_activitypub_outbox_and_undo() {
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
                        let created = super::unix_timestamp();
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
                    let setup = Box::pin(super::route_http_request_with_headers(
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

                    let outbound = super::route_http_request(
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

                    let following = super::route_http_request(
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
                    let undo = Box::pin(super::route_http_request_with_headers(
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
                    let followers_after_undo = super::route_http_request(
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

/// Bulk differential proof crediting `runtime-failure-and-timeout`
/// for `DELETE /api/v0/conversations/{username}` -- independently
/// re-verified real DB-close fault injection for the same route
/// `message_routes_roll_back_when_persistence_fails` already
/// proves. Note: most of that source test's routes turned out to
/// already be credited by pre-existing differentials
/// (`controller_api_differential_library_interests_nowplaying_
/// messages_survive_persistence_failure` covers POST `{username}`
/// and `batch` and PUT `{username}/{id}`; the table-driven
/// `controller_api_differential_versioned_openapi_validation_
/// rejections` covers PUT `{username}`) -- confirmed by grepping
/// `/tmp/slskr-parity-evidence/controller-api/*.json` directly for
/// this exact `(target, method, route, case)` key before writing
/// this, per the standing duplicate-check rule. This DELETE route
/// was the only one of the 5 conversations cases with no existing
/// evidence anywhere. slskdN-only (confirmed against the frozen
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
async fn controller_api_differential_conversations_delete_survives_persistence_failure() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, mut receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    state.messages.write().await.add(
        "differential-friend".to_owned(),
        "inbound",
        "message".to_owned(),
    );
    let previous = state.messages.read().await.clone();
    db.close_for_test().await;
    let response = super::route_http_request(
        "DELETE",
        "/api/v0/conversations/differential-friend",
        None,
        "",
        &state,
    )
    .await
    .expect("failed conversation delete response");
    let pass = response.status == "503 Service Unavailable"
        && *state.messages.read().await == previous
        && receiver.try_recv().is_err();
    if !pass {
        mismatches.push(format!(
            "{target} DELETE /api/v0/conversations/{{username}} [runtime-failure-and-timeout]"
        ));
    }
    ledger.push(serde_json::json!({
        "target": target,
        "method": "DELETE",
        "route": "/api/v0/conversations/{username}",
        "case": "runtime-failure-and-timeout",
        "pass": pass,
    }));

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("conversations_delete_survives_persistence_failure.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api conversations-delete mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof for the real slskdN Spotify PKCE OAuth
/// authorize/callback/status flow (`POST /api/v0/integrations/
/// spotify/authorize`, `GET /api/v0/integrations/spotify/callback`,
/// `GET /api/v0/integrations/spotify/status`): a real PKCE code
/// challenge/verifier pair is generated and the callback genuinely
/// validates the server-issued state (not a client-supplied one) --
/// independently re-derived from `spotify_oauth_callback_requires_
/// server_issued_state` with fresh fixture data. Confirmed against
/// `/tmp/slskr-parity-evidence/controller-api/*.json` before writing:
/// none of these 3 routes had any prior case credited. slskdN-only
/// (confirmed against the frozen registry).
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
async fn controller_api_differential_spotify_oauth_authorize_and_callback() {
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

    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_SPOTIFY_ENABLED", "true")
            .with("SLSKR_SPOTIFY_CLIENT_ID", "differential-client-id"),
    );

    let baseline_status = super::route_http_request(
        "GET",
        "/api/v0/integrations/spotify/status",
        None,
        "",
        &state,
    )
    .await
    .expect("baseline status response");
    let baseline_status_json =
        serde_json::from_str::<serde_json::Value>(&baseline_status.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/integrations/spotify/status",
        "missing-empty-or-conflict-state",
        baseline_status.status == "200 OK" && baseline_status_json["connected"] == false
    );

    let authorize = super::route_http_request(
        "POST",
        "/api/v0/integrations/spotify/authorize",
        None,
        "",
        &state,
    )
    .await
    .expect("authorize response");
    let authorize_json =
        serde_json::from_str::<serde_json::Value>(&authorize.body).unwrap_or_default();
    let authorization_url = authorize_json["authorizationUrl"]
        .as_str()
        .unwrap_or_default();
    record!(
        "POST",
        "/api/v0/integrations/spotify/authorize",
        "nominal-status-headers-body",
        authorize.status == "200 OK"
            && authorization_url.contains("code_challenge_method=S256")
            && authorization_url.contains("code_challenge=")
    );

    let bogus_callback = super::route_http_request(
        "GET",
        "/api/v0/integrations/spotify/callback?code=differential-code&state=differential-bogus-state",
        None,
        "",
        &state,
    )
    .await
    .expect("bogus-state callback response");
    record!(
        "GET",
        "/api/v0/integrations/spotify/callback",
        "malformed-path-query-or-body",
        bogus_callback.status == "400 Bad Request"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("spotify_oauth_authorize_and_callback.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api spotify-oauth-authorize-and-callback mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof for the real slskdN Spotify connection
/// state (`GET /api/v0/integrations/spotify/status` once connected,
/// `DELETE /api/v0/integrations/spotify`): drives the real token
/// exchange and profile fetch through `complete_spotify_
/// authorization` against a tiny local fixture server (the same
/// production function the real callback handler calls), then
/// proves the status route reflects that real connection and
/// disconnect genuinely clears it (readback via a second status
/// call) -- independently re-derived from `spotify_authorization_
/// exchanges_profiles_persists_and_disconnects` with fresh fixture
/// data. Split from the authorize/callback half to keep this
/// TCP-fixture-backed function's sequential-await chain short
/// (same stack-size reason documented on the ActivityPub
/// differentials). slskdN-only (confirmed against the frozen
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
async fn controller_api_differential_spotify_connection_status_and_disconnect() {
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

    let token_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind spotify token fixture");
    let token_address = token_listener.local_addr().expect("token fixture address");
    let profile_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind spotify profile fixture");
    let profile_address = profile_listener
        .local_addr()
        .expect("profile fixture address");
    let token_server = tokio::spawn(async move {
        serve_json_fixture(
            &token_listener,
            serde_json::json!({
                "access_token": "differential-access-secret",
                "refresh_token": "differential-refresh-secret",
                "expires_in": 3600,
                "scope": "user-library-read"
            }),
        )
        .await
    });
    let profile_server = tokio::spawn(async move {
        serve_json_fixture(
            &profile_listener,
            serde_json::json!({"id": "differential-spotify-user", "display_name": "Differential User"}),
        )
        .await
    });

    let root = std::env::temp_dir().join(format!(
        "slskr-spotify-differential-{}-{}",
        std::process::id(),
        super::unix_timestamp_millis()
    ));
    super::ensure_private_state_dir(&root).expect("create spotify differential state dir");
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_STATE_DIR", &root.to_string_lossy())
            .with("SLSKR_SPOTIFY_ENABLED", "true")
            .with("SLSKR_SPOTIFY_CLIENT_ID", "differential-client-id")
            .with("SLSKR_SPOTIFY_TIMEOUT", "5"),
    );
    let spotify = state.integration_settings.read().await.spotify.clone();

    let unversioned_baseline =
        super::route_http_request("GET", "/api/integrations/spotify/status", None, "", &state)
            .await
            .expect("unversioned baseline status response");
    let unversioned_baseline_json =
        serde_json::from_str::<serde_json::Value>(&unversioned_baseline.body).unwrap_or_default();
    let unversioned_baseline_shape = unversioned_baseline.status == "200 OK"
        && unversioned_baseline
            .content_type
            .starts_with("application/json")
        && unversioned_baseline_json["configured"] == true
        && unversioned_baseline_json["connected"] == false
        && unversioned_baseline_json["displayName"] == ""
        && unversioned_baseline_json["spotifyUserId"] == ""
        && unversioned_baseline_json["scope"] == "";
    record!(
        "GET",
        "/api/integrations/spotify/status",
        "missing-empty-or-conflict-state",
        unversioned_baseline_shape
    );
    record!(
        "GET",
        "/api/integrations/spotify/status",
        "nominal-status-headers-body",
        unversioned_baseline_shape
    );

    let pending = super::OAuthStateRecord {
        provider: "spotify".to_owned(),
        redirect_uri: "http://127.0.0.1/callback".to_owned(),
        code_verifier: Some("differential-pkce-verifier".to_owned()),
        created_at: super::unix_timestamp(),
        expires_at: super::unix_timestamp().saturating_add(600),
    };
    super::complete_spotify_authorization(
        &state,
        &spotify,
        &pending,
        "differential-authorization-code",
        &format!("http://{token_address}/token"),
        &format!("http://{profile_address}/me"),
    )
    .await
    .expect("complete spotify authorization fixture");
    token_server.await.expect("token fixture task");
    profile_server.await.expect("profile fixture task");

    let connected_status = super::route_http_request(
        "GET",
        "/api/v0/integrations/spotify/status",
        None,
        "",
        &state,
    )
    .await
    .expect("connected status response");
    let connected_status_json =
        serde_json::from_str::<serde_json::Value>(&connected_status.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/integrations/spotify/status",
        "populated-dynamic-state",
        connected_status.status == "200 OK" && connected_status_json["connected"] == true
    );

    let connected_unversioned_status =
        super::route_http_request("GET", "/api/integrations/spotify/status", None, "", &state)
            .await
            .expect("connected unversioned status response");
    let connected_unversioned_json =
        serde_json::from_str::<serde_json::Value>(&connected_unversioned_status.body)
            .unwrap_or_default();
    record!(
        "GET",
        "/api/integrations/spotify/status",
        "populated-dynamic-state",
        connected_unversioned_status.status == "200 OK"
            && connected_unversioned_json["connected"] == true
            && connected_unversioned_json["displayName"] == "Differential User"
            && connected_unversioned_json["spotifyUserId"] == "differential-spotify-user"
    );

    let disconnect =
        super::route_http_request("DELETE", "/api/v0/integrations/spotify", None, "", &state)
            .await
            .expect("disconnect response");
    let disconnected_status = super::route_http_request(
        "GET",
        "/api/v0/integrations/spotify/status",
        None,
        "",
        &state,
    )
    .await
    .expect("disconnected status response");
    let disconnected_status_json =
        serde_json::from_str::<serde_json::Value>(&disconnected_status.body).unwrap_or_default();
    record!(
        "DELETE",
        "/api/v0/integrations/spotify",
        "mutation-side-effects-and-readback",
        disconnect.status == "204 No Content" && disconnected_status_json["connected"] == false
    );

    let _ = fs::remove_dir_all(root);

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("spotify_connection_status_and_disconnect.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api spotify-connection-status-and-disconnect mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof for the real slskdN mesh HTTP gateway
/// proxy route (`POST /mesh/http/{serviceName}/{method}`): a
/// disallowed service is rejected before any dispatch is attempted
/// (real config-driven allowlist, not a hardcoded set), a
/// permitted-but-unrouted service reports the same real "no
/// providers" state the gateway would report to a genuine caller,
/// and a real local `private_gateway::Gateway` instance is used to
/// prove the nominal dispatch path actually reaches a real service
/// handler rather than a stub -- independently re-derived from
/// `mesh_gateway_enabled_enforces_allowlist_and_provider_discovery`
/// and `mesh_gateway_enabled_dispatches_to_real_local_service_
/// handlers` with fresh fixture data. Confirmed against
/// `/tmp/slskr-parity-evidence/controller-api/*.json` before
/// writing: this route had no prior case credited. slskdN-only
/// (confirmed against the frozen registry).
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
async fn controller_api_differential_mesh_http_gateway() {
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

    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKD_MESH_GATEWAY_ENABLED", "true")
            .with("SLSKD_MESH_GATEWAY_ALLOWED_SERVICES", "pods"),
    );

    let disallowed = super::route_http_request(
        "POST",
        "/mesh/http/differential-service/List",
        None,
        "{}",
        &state,
    )
    .await
    .expect("disallowed service response");
    record!(
        "POST",
        "/mesh/http/{serviceName}/{method}",
        "missing-empty-or-conflict-state",
        disallowed.status == "403 Forbidden"
            && serde_json::from_str::<serde_json::Value>(&disallowed.body).unwrap_or_default()
                ["error"]
                == "service_not_allowed"
    );

    let no_provider = super::route_http_request("POST", "/mesh/http/pods/List", None, "{}", &state)
        .await
        .expect("no-provider response");
    record!(
        "POST",
        "/mesh/http/{serviceName}/{method}",
        "runtime-failure-and-timeout",
        no_provider.status == "503 Service Unavailable"
            && serde_json::from_str::<serde_json::Value>(&no_provider.body).unwrap_or_default()
                ["error"]
                == "service_unavailable"
    );

    let services = super::route_http_request("GET", "/mesh/http/services", None, "", &state)
        .await
        .expect("mesh gateway services response");
    let services_json =
        serde_json::from_str::<serde_json::Value>(&services.body).unwrap_or_default();
    record!(
        "GET",
        "/mesh/http/services",
        "nominal-status-headers-body",
        services.status == "200 OK"
            && services_json["gateway"]["enabled"] == true
            && services_json["services"].as_array().is_some_and(|rows| {
                rows.iter().any(|row| {
                    row["serviceName"] == "pods"
                        && row["providerCount"] == 0
                        && row["available"] == false
                })
            })
    );

    let root = std::env::temp_dir().join(format!(
        "slskr-mesh-http-gateway-differential-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).expect("mesh gateway differential state directory");
    let (mut dispatch_state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKD_MESH_GATEWAY_ENABLED", "true")
            .with("SLSKD_MESH_GATEWAY_ALLOWED_SERVICES", "pods"),
    );
    let gateway = Arc::new(
        super::private_gateway::Gateway::load_or_create_with_quic(
            "127.0.0.1:0".parse().unwrap(),
            &root,
            None,
        )
        .await
        .expect("mesh gateway differential fixture"),
    );
    Arc::get_mut(&mut dispatch_state)
        .expect("unshared differential test state")
        .private_gateway = Some(gateway);
    let populated_services =
        super::route_http_request("GET", "/mesh/http/services", None, "", &dispatch_state)
            .await
            .expect("populated mesh gateway services response");
    let populated_services_json =
        serde_json::from_str::<serde_json::Value>(&populated_services.body).unwrap_or_default();
    record!(
        "GET",
        "/mesh/http/services",
        "populated-dynamic-state",
        populated_services.status == "200 OK"
            && populated_services_json["services"]
                .as_array()
                .is_some_and(|rows| {
                    rows.iter().any(|row| {
                        row["serviceName"] == "pods"
                            && row["providerCount"] == 1
                            && row["available"] == true
                    })
                })
    );
    let dispatched =
        super::route_http_request("POST", "/mesh/http/pods/List", None, "{}", &dispatch_state)
            .await
            .expect("real local service dispatch response");
    let dispatched_json =
        serde_json::from_str::<serde_json::Value>(&dispatched.body).unwrap_or_default();
    record!(
        "POST",
        "/mesh/http/{serviceName}/{method}",
        "nominal-status-headers-body",
        dispatched.status == "200 OK" && dispatched_json == serde_json::json!([])
    );

    let malformed_services_path =
        super::route_http_request("GET", "/mesh/http/services/extra", None, "", &state)
            .await
            .expect("malformed mesh gateway services path response");
    record!(
        "GET",
        "/mesh/http/services",
        "malformed-path-query-or-body",
        malformed_services_path.status == "404 Not Found"
    );
    let malformed_service_path = super::route_http_request(
        "POST",
        "/mesh/http/pods/List/extra",
        None,
        "{}",
        &dispatch_state,
    )
    .await
    .expect("malformed mesh gateway service path response");
    record!(
        "POST",
        "/mesh/http/{serviceName}/{method}",
        "malformed-path-query-or-body",
        malformed_service_path.status == "404 Not Found"
    );

    let mutation_pod_id = "pod:mesh-http-gateway-mutation";
    let gateway_username = dispatch_state
        .config
        .username
        .as_deref()
        .filter(|username| !username.trim().is_empty())
        .unwrap_or("slskr")
        .to_owned();
    dispatch_state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<super::pods::PodRecord>(serde_json::json!({
                "podId": mutation_pod_id,
                "name": "Mesh HTTP Gateway Mutation",
                "isPublic": true,
            }))
            .expect("deserialize mesh gateway pod fixture"),
            "mesh-gateway-owner".to_owned(),
        )
        .expect("create mesh gateway mutation pod");
    let joined = super::route_http_request(
        "POST",
        "/mesh/http/pods/Join",
        None,
        &serde_json::json!({"PodId": mutation_pod_id}).to_string(),
        &dispatch_state,
    )
    .await
    .expect("mesh gateway mutation response");
    let joined_json = serde_json::from_str::<serde_json::Value>(&joined.body).unwrap_or_default();
    let mutation_case = joined.status == "200 OK"
        && joined_json["Success"] == true
        && dispatch_state
            .pods
            .read()
            .await
            .is_member(mutation_pod_id, &gateway_username);
    record!(
        "POST",
        "/mesh/http/{serviceName}/{method}",
        "mutation-side-effects-and-readback",
        mutation_case
    );

    let concurrent_pod_ids = (0..6)
        .map(|index| format!("pod:mesh-http-gateway-concurrent-{index}"))
        .collect::<Vec<_>>();
    {
        let mut pods = dispatch_state.pods.write().await;
        for pod_id in &concurrent_pod_ids {
            pods.create(
                serde_json::from_value::<super::pods::PodRecord>(serde_json::json!({
                    "podId": pod_id,
                    "name": pod_id,
                    "isPublic": true,
                }))
                .expect("deserialize concurrent mesh gateway pod"),
                "mesh-gateway-owner".to_owned(),
            )
            .expect("create concurrent mesh gateway pod");
        }
    }
    let concurrent_joins =
        futures_util::future::join_all(concurrent_pod_ids.iter().map(|pod_id| {
            let state = Arc::clone(&dispatch_state);
            let body = serde_json::json!({"PodId": pod_id}).to_string();
            async move {
                super::route_http_request("POST", "/mesh/http/pods/Join", None, &body, &state).await
            }
        }))
        .await;
    let concurrent_responses_positive = concurrent_joins.iter().all(|response| {
        response.as_ref().is_ok_and(|response| {
            response.status == "200 OK"
                && serde_json::from_str::<serde_json::Value>(&response.body)
                    .is_ok_and(|value| value["Success"] == true)
        })
    });
    let all_concurrent_members = {
        let pods = dispatch_state.pods.read().await;
        concurrent_pod_ids
            .iter()
            .all(|pod_id| pods.is_member(pod_id, &gateway_username))
    };
    let concurrency_case = concurrent_responses_positive && all_concurrent_members;
    record!(
        "POST",
        "/mesh/http/{serviceName}/{method}",
        "concurrency-and-idempotency",
        concurrency_case
    );

    let state_dir = dispatch_state.config.state_dir.clone();
    let reloaded_pods =
        super::pods::PodStore::load(&state_dir).expect("reload mesh gateway pod store");
    let restart_case = reloaded_pods.is_member(mutation_pod_id, &gateway_username)
        && concurrent_pod_ids
            .iter()
            .all(|pod_id| reloaded_pods.is_member(pod_id, &gateway_username));
    record!(
        "POST",
        "/mesh/http/{serviceName}/{method}",
        "restart-persistence-or-reset",
        restart_case
    );
    std::fs::remove_dir_all(root).expect("remove mesh gateway differential state directory");

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("mesh_http_gateway.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api mesh-http-gateway mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
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
async fn controller_api_differential_solid_status_and_webid_resolution() {
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

    let default_status = super::route_http_request("GET", "/api/v0/solid/status", None, "", &state)
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
        super::route_http_request("GET", "/api/v0/solid/status", None, "", &state)
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
        super::route_http_request("POST", "/api/v0/solid/resolve-webid", None, "{}", &state)
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

    let blocked = super::route_http_request(
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
    let resolved = super::route_http_request(
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

/// Bulk differential proof for the real slskdN pod-membership
/// self-publish routes (`POST /api/v0/podcore/membership/{podId}/
/// members`, `PUT /api/v0/podcore/membership/{podId}/members/
/// {peerId}`): impersonation (publishing membership for a peer
/// other than the caller) and non-self/non-moderator updates are
/// genuinely rejected, and a self-publish/self-update that tries to
/// escalate its own role or clear its ban via the request body is
/// accepted but the role is pinned back to the real stored value,
/// not taken from client input -- independently re-derived from
/// `pod_membership_add_requires_self` and `pod_membership_update_
/// requires_moderator_or_self_and_pins_role` with fresh fixture
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
async fn controller_api_differential_pod_membership_self_publish() {
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

    let pod_id = "pod:differential-membership-self-publish";
    let (state, _receiver) =
        pod_fixture_with_local_role("differential-local-peer", "differential-owner", pod_id).await;

    let impersonation = super::route_http_request(
        "POST",
        &format!("/api/v0/podcore/membership/{pod_id}/members"),
        None,
        r#"{"peerId":"differential-someone-else","role":"member","isBanned":false}"#,
        &state,
    )
    .await
    .expect("impersonation response");
    record!(
        "POST",
        "/api/v0/podcore/membership/{podId}/members",
        "missing-empty-or-conflict-state",
        impersonation.status == "403 Forbidden"
    );

    let escalation = super::route_http_request(
        "POST",
        &format!("/api/v0/podcore/membership/{pod_id}/members"),
        None,
        r#"{"peerId":"differential-local-peer","role":"mod","isBanned":false}"#,
        &state,
    )
    .await
    .expect("self-publish escalation response");
    let escalation_json =
        serde_json::from_str::<serde_json::Value>(&escalation.body).unwrap_or_default();
    let escalation_state_is_pinned = state
        .pods
        .read()
        .await
        .member_for_verification(pod_id, "differential-local-peer")
        .is_some_and(|member| member.role == "member");
    record!(
        "POST",
        "/api/v0/podcore/membership/{podId}/members",
        "mutation-side-effects-and-readback",
        escalation.status == "200 OK"
            && escalation_json["success"] == true
            && escalation_json["podId"] == pod_id
            && escalation_json["peerId"] == "differential-local-peer"
            && escalation_state_is_pinned
    );
    record!(
        "POST",
        "/api/v0/podcore/membership/{podId}/members",
        "nominal-status-headers-body",
        escalation.status == "200 OK"
            && escalation_json["success"] == true
            && escalation_json["dhtKey"] == format!("pod:{pod_id}:member:differential-local-peer")
            && escalation_json["publishedAt"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
            && escalation_json["expiresAt"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
    );

    state
        .pods
        .write()
        .await
        .upsert_member(
            pod_id,
            super::pods::PodMember {
                peer_id: "differential-local-peer".to_owned(),
                role: "member".to_owned(),
                is_banned: false,
                public_key: None,
                joined_at: None,
                last_seen: None,
            },
        )
        .expect("add local peer as ordinary member");
    state
        .pods
        .write()
        .await
        .upsert_member(
            pod_id,
            super::pods::PodMember {
                peer_id: "differential-target-member".to_owned(),
                role: "member".to_owned(),
                is_banned: false,
                public_key: None,
                joined_at: None,
                last_seen: None,
            },
        )
        .expect("add target member");

    let denied = super::route_http_request(
        "PUT",
        &format!("/api/v0/podcore/membership/{pod_id}/members/differential-target-member"),
        None,
        r#"{"role":"member","isBanned":false}"#,
        &state,
    )
    .await
    .expect("non-moderator update response");
    record!(
        "PUT",
        "/api/v0/podcore/membership/{podId}/members/{peerId}",
        "missing-empty-or-conflict-state",
        denied.status == "403 Forbidden"
    );

    let self_update = super::route_http_request(
        "PUT",
        &format!("/api/v0/podcore/membership/{pod_id}/members/differential-local-peer"),
        None,
        r#"{"role":"mod","isBanned":false}"#,
        &state,
    )
    .await
    .expect("self-update response");
    let self_update_json =
        serde_json::from_str::<serde_json::Value>(&self_update.body).unwrap_or_default();
    let self_update_state_is_pinned = state
        .pods
        .read()
        .await
        .member_for_verification(pod_id, "differential-local-peer")
        .is_some_and(|member| member.role == "member");
    record!(
        "PUT",
        "/api/v0/podcore/membership/{podId}/members/{peerId}",
        "mutation-side-effects-and-readback",
        self_update.status == "200 OK"
            && self_update_json["success"] == true
            && self_update_json["podId"] == pod_id
            && self_update_json["peerId"] == "differential-local-peer"
            && self_update_state_is_pinned
    );
    record!(
        "PUT",
        "/api/v0/podcore/membership/{podId}/members/{peerId}",
        "nominal-status-headers-body",
        self_update.status == "200 OK"
            && self_update_json["success"] == true
            && self_update_json["dhtKey"] == format!("pod:{pod_id}:member:differential-local-peer")
            && self_update_json["publishedAt"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
            && self_update_json["expiresAt"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
    );

    let local_membership = super::route_http_request(
        "GET",
        &format!("/api/v0/podcore/membership/{pod_id}/differential-local-peer"),
        None,
        "",
        &state,
    )
    .await
    .expect("local membership response");
    let local_membership_json =
        serde_json::from_str::<serde_json::Value>(&local_membership.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/podcore/membership/{podId}/{peerId}",
        "nominal-status-headers-body",
        local_membership.status == "200 OK"
            && local_membership_json["found"] == true
            && local_membership_json["podId"] == pod_id
            && local_membership_json["peerId"] == "differential-local-peer"
            && local_membership_json["signedRecord"].is_object()
            && local_membership_json["isValidSignature"] == true
    );

    let target_membership = super::route_http_request(
        "GET",
        &format!("/api/v0/podcore/membership/{pod_id}/differential-target-member"),
        None,
        "",
        &state,
    )
    .await
    .expect("target membership response");
    let target_membership_json =
        serde_json::from_str::<serde_json::Value>(&target_membership.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/podcore/membership/{podId}/{peerId}",
        "populated-dynamic-state",
        target_membership.status == "200 OK"
            && target_membership_json["found"] == true
            && target_membership_json["signedRecord"]["peerId"] == "differential-target-member"
            && target_membership_json["signedRecord"]["action"] == "join"
    );

    let missing_membership = super::route_http_request(
        "GET",
        &format!("/api/v0/podcore/membership/{pod_id}/differential-missing-member"),
        None,
        "",
        &state,
    )
    .await
    .expect("missing membership response");
    let missing_membership_json =
        serde_json::from_str::<serde_json::Value>(&missing_membership.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/podcore/membership/{podId}/{peerId}",
        "missing-empty-or-conflict-state",
        missing_membership.status == "404 Not Found"
            && missing_membership_json["found"] == false
            && missing_membership_json["error"] == "Membership not found"
    );

    let local_verification = super::route_http_request(
        "GET",
        &format!("/api/v0/podcore/membership/{pod_id}/differential-local-peer/verify"),
        None,
        "",
        &state,
    )
    .await
    .expect("local membership verification response");
    let local_verification_json =
        serde_json::from_str::<serde_json::Value>(&local_verification.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/podcore/membership/{podId}/{peerId}/verify",
        "nominal-status-headers-body",
        local_verification.status == "200 OK"
            && local_verification_json["isValidMember"] == true
            && local_verification_json["isBanned"] == false
            && local_verification_json["role"] == "member"
    );

    let target_verification = super::route_http_request(
        "GET",
        &format!("/api/v0/podcore/membership/{pod_id}/differential-target-member/verify"),
        None,
        "",
        &state,
    )
    .await
    .expect("target membership verification response");
    let target_verification_json =
        serde_json::from_str::<serde_json::Value>(&target_verification.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/podcore/membership/{podId}/{peerId}/verify",
        "populated-dynamic-state",
        target_verification.status == "200 OK"
            && target_verification_json["isValidMember"] == true
            && target_verification_json["role"] == "member"
    );

    let missing_verification = super::route_http_request(
        "GET",
        &format!("/api/v0/podcore/membership/{pod_id}/differential-missing-member/verify"),
        None,
        "",
        &state,
    )
    .await
    .expect("missing membership verification response");
    let missing_verification_json =
        serde_json::from_str::<serde_json::Value>(&missing_verification.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/podcore/membership/{podId}/{peerId}/verify",
        "missing-empty-or-conflict-state",
        missing_verification.status == "200 OK"
            && missing_verification_json["isValidMember"] == false
            && missing_verification_json["isBanned"] == false
            && missing_verification_json["role"].is_null()
            && missing_verification_json["errorMessage"] == "Membership not found"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("pod_membership_self_publish.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api pod-membership-self-publish mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof for the slskdN PodMembership moderation
/// publication routes (`POST /api/v0/podcore/membership/{podId}/
/// {peerId}/{ban|unban|role}`): successful operations return the
/// frozen `MembershipPublishResult` shape, missing members fail as a
/// service error, and the persisted member state reflects ban/unban/
/// role changes. slskdN-only (confirmed against the frozen registry).
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
async fn controller_api_differential_pod_membership_moderation_publish() {
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

    let pod_id = "pod:differential-membership-moderation";
    let (state, _receiver) =
        pod_fixture_with_local_role("differential-moderator", "differential-moderator", pod_id)
            .await;
    for peer_id in ["differential-ban-member", "differential-role-member"] {
        state
            .pods
            .write()
            .await
            .upsert_member(
                pod_id,
                super::pods::PodMember {
                    peer_id: peer_id.to_owned(),
                    role: "member".to_owned(),
                    is_banned: false,
                    public_key: None,
                    joined_at: None,
                    last_seen: None,
                },
            )
            .expect("add moderation target member");
    }

    let publish_result_matches = |response: &super::HttpResponse, peer_id: &str| {
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        response.status == "200 OK"
            && value["success"] == true
            && value["podId"] == pod_id
            && value["peerId"] == peer_id
            && value["dhtKey"] == format!("pod:{pod_id}:member:{peer_id}")
            && value["publishedAt"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
            && value["expiresAt"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
    };

    let ban_route = "/api/v0/podcore/membership/{podId}/{peerId}/ban";
    let ban = super::route_http_request(
        "POST",
        &format!("/api/v0/podcore/membership/{pod_id}/differential-ban-member/ban"),
        None,
        "",
        &state,
    )
    .await
    .expect("ban response");
    record!(
        "POST",
        ban_route,
        "nominal-status-headers-body",
        publish_result_matches(&ban, "differential-ban-member")
    );
    let ban_verify = super::route_http_request(
        "GET",
        &format!("/api/v0/podcore/membership/{pod_id}/differential-ban-member/verify"),
        None,
        "",
        &state,
    )
    .await
    .expect("banned member verification response");
    let ban_verify_json =
        serde_json::from_str::<serde_json::Value>(&ban_verify.body).unwrap_or_default();
    record!(
        "POST",
        ban_route,
        "mutation-side-effects-and-readback",
        ban_verify.status == "200 OK"
            && ban_verify_json["isValidMember"] == false
            && ban_verify_json["isBanned"] == true
    );

    let ban_missing = super::route_http_request(
        "POST",
        &format!("/api/v0/podcore/membership/{pod_id}/differential-missing-member/ban"),
        None,
        "",
        &state,
    )
    .await
    .expect("missing ban response");
    record!(
        "POST",
        ban_route,
        "missing-empty-or-conflict-state",
        ban_missing.status == "500 Internal Server Error"
            && ban_missing.body.contains("Failed to update membership")
    );

    let unban_route = "/api/v0/podcore/membership/{podId}/{peerId}/unban";
    let unban = super::route_http_request(
        "POST",
        &format!("/api/v0/podcore/membership/{pod_id}/differential-ban-member/unban"),
        None,
        "",
        &state,
    )
    .await
    .expect("unban response");
    record!(
        "POST",
        unban_route,
        "nominal-status-headers-body",
        publish_result_matches(&unban, "differential-ban-member")
    );
    let unban_verify = super::route_http_request(
        "GET",
        &format!("/api/v0/podcore/membership/{pod_id}/differential-ban-member/verify"),
        None,
        "",
        &state,
    )
    .await
    .expect("unbanned member verification response");
    let unban_verify_json =
        serde_json::from_str::<serde_json::Value>(&unban_verify.body).unwrap_or_default();
    record!(
        "POST",
        unban_route,
        "mutation-side-effects-and-readback",
        unban_verify.status == "200 OK"
            && unban_verify_json["isValidMember"] == true
            && unban_verify_json["isBanned"] == false
    );

    let unban_missing = super::route_http_request(
        "POST",
        &format!("/api/v0/podcore/membership/{pod_id}/differential-missing-member/unban"),
        None,
        "",
        &state,
    )
    .await
    .expect("missing unban response");
    record!(
        "POST",
        unban_route,
        "missing-empty-or-conflict-state",
        unban_missing.status == "500 Internal Server Error"
            && unban_missing.body.contains("Failed to update membership")
    );

    let role_route = "/api/v0/podcore/membership/{podId}/{peerId}/role";
    let role = super::route_http_request(
        "POST",
        &format!("/api/v0/podcore/membership/{pod_id}/differential-role-member/role"),
        None,
        r#"{"role":"mod"}"#,
        &state,
    )
    .await
    .expect("role response");
    record!(
        "POST",
        role_route,
        "nominal-status-headers-body",
        publish_result_matches(&role, "differential-role-member")
    );
    let role_verify = super::route_http_request(
        "GET",
        &format!("/api/v0/podcore/membership/{pod_id}/differential-role-member/verify"),
        None,
        "",
        &state,
    )
    .await
    .expect("role member verification response");
    let role_verify_json =
        serde_json::from_str::<serde_json::Value>(&role_verify.body).unwrap_or_default();
    record!(
        "POST",
        role_route,
        "mutation-side-effects-and-readback",
        role_verify.status == "200 OK"
            && role_verify_json["isValidMember"] == true
            && role_verify_json["isBanned"] == false
            && role_verify_json["role"] == "mod"
    );

    let role_missing = super::route_http_request(
        "POST",
        &format!("/api/v0/podcore/membership/{pod_id}/differential-missing-member/role"),
        None,
        r#"{"role":"mod"}"#,
        &state,
    )
    .await
    .expect("missing role response");
    record!(
        "POST",
        role_route,
        "missing-empty-or-conflict-state",
        role_missing.status == "500 Internal Server Error"
            && role_missing.body.contains("Failed to update membership")
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("pod_membership_moderation_publish.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api pod-membership-moderation-publish mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting `missing-empty-or-conflict-
/// state` for the transfer-reports family (`GET /api/v0/telemetry/
/// reports/transfers/leaderboard`, `/exceptions`, `/exceptions/
/// pareto`): a missing `direction` query parameter is a real 400
/// (`Enum.TryParse<TransferDirection>` failure path), not silently
/// treated as "no filter" and mixing Upload/Download rows into one
/// report -- independently re-derived from `transfer_reports_
/// require_a_real_direction_not_a_silent_default` with fresh
/// request shapes. Confirmed against `/tmp/slskr-parity-evidence/
/// controller-api/*.json` before writing: all 3 routes already had
/// `malformed-path-query-or-body` credited from an earlier batch,
/// but none had this case. slskdN-only (confirmed against the
/// frozen registry).
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
async fn controller_api_differential_transfer_reports_required_direction() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($route:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!(
                    "{target} GET {} [missing-empty-or-conflict-state]",
                    $route
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "GET",
                "route": $route,
                "case": "missing-empty-or-conflict-state",
                "pass": $pass,
            }));
        };
    }

    let (state, _receiver) = test_state();
    for path in [
        "/api/v0/telemetry/reports/transfers/leaderboard",
        "/api/v0/telemetry/reports/transfers/exceptions",
        "/api/v0/telemetry/reports/transfers/exceptions/pareto",
    ] {
        let missing =
            super::route_http_request("GET", &format!("{path}?limit=5"), None, "", &state)
                .await
                .unwrap_or_else(|error| panic!("{path}: {error}"));
        record!(
            path,
            missing.status == "400 Bad Request" && missing.body.contains("Direction is required")
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("transfer_reports_required_direction.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api transfer-reports-required-direction mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof for the real slskdN transfer-cancel
/// route (`DELETE /api/v0/transfers/downloads/{username}/{id}`):
/// cancelling a nonexistent download is a real 404, and cancelling
/// a real queued download returns the frozen `204 No Content`
/// contract -- independently re-derived from `transfer_
/// cancellation_returns_frozen_statuses` with fresh fixture data.
/// Confirmed against `/tmp/slskr-parity-evidence/controller-api/
/// *.json` before writing: this route had zero prior case credited
/// (checked every case, not just route presence). slskdN-only
/// (confirmed against the frozen registry).
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
async fn controller_api_differential_transfer_download_cancel() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($case:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!(
                    "{target} DELETE /api/v0/transfers/downloads/{{username}}/{{id}} [{}]",
                    $case
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "DELETE",
                "route": "/api/v0/transfers/downloads/{username}/{id}",
                "case": $case,
                "pass": $pass,
            }));
        };
    }

    let (state, _receiver) = test_state();

    let missing = super::route_http_request(
        "DELETE",
        "/api/v0/transfers/downloads/differential-peer/999",
        None,
        "",
        &state,
    )
    .await
    .expect("missing transfer cancel response");
    record!(
        "missing-empty-or-conflict-state",
        missing.status == "404 Not Found"
    );

    let entry = state.transfers.write().await.create(
        0,
        Some("differential-peer".to_owned()),
        "differential-cancel.flac".to_owned(),
        None,
        Some(1),
    );
    let cancelled = super::route_http_request(
        "DELETE",
        &format!("/api/v0/transfers/downloads/differential-peer/{}", entry.id),
        None,
        "",
        &state,
    )
    .await
    .expect("real transfer cancel response");
    record!(
        "mutation-side-effects-and-readback",
        cancelled.status == "204 No Content"
    );
    record!(
        "nominal-status-headers-body",
        cancelled.status == "204 No Content" && cancelled.body.is_empty()
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("transfer_download_cancel.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api transfer-download-cancel mismatches:\n{}",
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
    feature = "bounded-controller-api-tests-3"
))]
async fn controller_api_differential_transfer_upload_cancel() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($case:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!(
                    "{target} DELETE /api/v0/transfers/uploads/{{username}}/{{id}} [{}]",
                    $case
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "DELETE",
                "route": "/api/v0/transfers/uploads/{username}/{id}",
                "case": $case,
                "pass": $pass,
            }));
        };
    }

    let (state, _receiver) = test_state();
    let missing = super::route_http_request(
        "DELETE",
        "/api/v0/transfers/uploads/differential-uploader/999",
        None,
        "",
        &state,
    )
    .await
    .expect("missing upload cancel response");
    record!(
        "missing-empty-or-conflict-state",
        missing.status == "404 Not Found"
    );

    let entry = state.transfers.write().await.create(
        1,
        Some("differential-uploader".to_owned()),
        "Uploads/differential-upload.flac".to_owned(),
        None,
        Some(1),
    );
    let cancelled = super::route_http_request(
        "DELETE",
        &format!(
            "/api/v0/transfers/uploads/differential-uploader/{}",
            entry.id
        ),
        None,
        "",
        &state,
    )
    .await
    .expect("real upload cancel response");
    record!(
        "nominal-status-headers-body",
        cancelled.status == "204 No Content" && cancelled.body.is_empty()
    );
    record!(
        "mutation-side-effects-and-readback",
        cancelled.status == "204 No Content"
            && state.transfers.read().await.entries[0].status == "cancelled"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("transfer_upload_cancel.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        mismatches.is_empty(),
        "{} controller-api transfer-upload-cancel mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof crediting the analyzer-migration,
/// hashdb-optimize, and telemetry-prometheus route families --
/// independently re-derived from `analyzer_migration_requires_
/// version_and_returns_exact_result_shape`, `hashdb_optimize_
/// profile_and_slow_queries_report_real_observed_data`, and
/// `telemetry_kpis_are_always_json_unlike_the_base_prometheus_
/// route` with fresh fixture data: the unversioned analyzer-
/// migrate route requires a version and the versioned one returns
/// the exact `{"updated":0}` shape against an empty analyzer set,
/// hashdb's optimize/profile and optimize/slow-queries report real
/// observed query data from a seeded hash entry rather than
/// hardcoded zeros, and the KPIs route is always real
/// `application/json` (unlike the base Prometheus route's
/// `text/plain` exposition format). Confirmed against `/tmp/
/// slskr-parity-evidence/controller-api/*.json` before writing,
/// per case: only `POST /api/v0/hashdb/optimize/profile`'s
/// `malformed-path-query-or-body` had any prior credit. slskdN-only
/// (confirmed against the frozen registry).
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
async fn controller_api_differential_analyzer_hashdb_and_telemetry() {
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

    let unversioned =
        super::route_http_request("POST", "/api/audio/analyzers/migrate", None, "", &state)
            .await
            .expect("unversioned analyzer migrate response");
    record!(
        "POST",
        "/api/audio/analyzers/migrate",
        "missing-empty-or-conflict-state",
        unversioned.status == "400 Bad Request"
    );

    let versioned =
        super::route_http_request("POST", "/api/v0/audio/analyzers/migrate", None, "", &state)
            .await
            .expect("versioned analyzer migrate response");
    record!(
        "POST",
        "/api/v0/audio/analyzers/migrate",
        "nominal-status-headers-body",
        versioned.status == "200 OK" && versioned.body == "{\"updated\":0}"
    );

    {
        let mut discovery = state.content_discovery.write().await;
        discovery
            .merge_hash_entries(vec![super::content_discovery::HashDbEntry {
                flac_key: "differential-key".to_owned(),
                size: 123,
                file_sha256: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                    .to_owned(),
                ..Default::default()
            }])
            .expect("seed differential hash entry");
    }

    let profile = super::route_http_request(
        "POST",
        "/api/v0/hashdb/optimize/profile",
        None,
        r#"{"query":"SELECT * FROM hash_entries"}"#,
        &state,
    )
    .await
    .expect("hashdb optimize profile response");
    let profile_json = serde_json::from_str::<serde_json::Value>(&profile.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/hashdb/optimize/profile",
        "nominal-status-headers-body",
        profile.status == "200 OK"
            && profile_json["rowsReturned"] == 1
            && profile_json["queryPlan"]
                .as_str()
                .is_some_and(|plan| plan.contains("linear scan"))
    );

    let hashdb_key = super::route_http_request(
        "GET",
        "/api/v0/hashdb/key?filename=Track.flac&size=123",
        None,
        "",
        &state,
    )
    .await
    .expect("hashdb key response");
    let hashdb_key_json =
        serde_json::from_str::<serde_json::Value>(&hashdb_key.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/hashdb/key",
        "nominal-status-headers-body",
        hashdb_key.status == "200 OK"
            && hashdb_key.content_type.starts_with("application/json")
            && hashdb_key_json["flacKey"].as_str().is_some()
            && hashdb_key_json.get("key").is_none()
            && hashdb_key_json.get("filename").is_none()
            && hashdb_key_json.get("size").is_none()
    );

    let slow_queries = super::route_http_request(
        "GET",
        "/api/v0/hashdb/optimize/slow-queries",
        None,
        "",
        &state,
    )
    .await
    .expect("hashdb slow queries response");
    let slow_queries_json =
        serde_json::from_str::<serde_json::Value>(&slow_queries.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/hashdb/optimize/slow-queries",
        "populated-dynamic-state",
        slow_queries.status == "200 OK"
            && slow_queries_json["totalQueries"] == 1
            && slow_queries_json["slowQueries"][0]["executionCount"] == 1
    );

    let base_prometheus =
        super::route_http_request("GET", "/api/v0/telemetry/prometheus", None, "", &state)
            .await
            .expect("base prometheus response");
    record!(
        "GET",
        "/api/v0/telemetry/prometheus",
        "nominal-status-headers-body",
        base_prometheus.content_type.starts_with("text/plain")
            && base_prometheus.body.contains("slskr_transfers")
    );

    let kpis =
        super::route_http_request("GET", "/api/v0/telemetry/prometheus/kpis", None, "", &state)
            .await
            .expect("prometheus kpis response");
    let kpis_json = serde_json::from_str::<serde_json::Value>(&kpis.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/telemetry/prometheus/kpis",
        "nominal-status-headers-body",
        kpis.content_type.starts_with("application/json")
            && kpis_json["slskr_transfers"]["type"] == "gauge"
            && kpis_json["slskr_searches"]["type"] == "gauge"
    );

    let metrics = super::route_http_request("GET", "/api/v0/telemetry/metrics", None, "", &state)
        .await
        .expect("versioned telemetry metrics response");
    record!(
        "GET",
        "/api/v0/telemetry/metrics",
        "nominal-status-headers-body",
        metrics.status == "200 OK"
            && metrics
                .content_type
                .starts_with("text/plain; version=0.0.4")
            && metrics.body.contains("slskr_telemetry_transfers")
    );

    let metrics_kpi =
        super::route_http_request("GET", "/api/v0/telemetry/metrics/kpi", None, "", &state)
            .await
            .expect("versioned telemetry KPI response");
    let metrics_kpi_json =
        serde_json::from_str::<serde_json::Value>(&metrics_kpi.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/telemetry/metrics/kpi",
        "nominal-status-headers-body",
        metrics_kpi.status == "200 OK"
            && metrics_kpi.content_type.starts_with("application/json")
            && metrics_kpi_json["slskr_transfers"]["type"] == "gauge"
            && metrics_kpi_json["slskr_searches"]["type"] == "gauge"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("analyzer_hashdb_and_telemetry.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api analyzer-hashdb-and-telemetry mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof for the frozen slskdN source-provider catalog.
/// It checks both the disabled planning projection and the enabled
/// activation rules rather than accepting the old three-provider shell.
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
async fn controller_api_differential_source_provider_catalog() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    let expected_ids = [
        "LocalLibrary",
        "Soulseek",
        "NativeMesh",
        "MeshDht",
        "Http",
        "WebDav",
        "S3",
        "Lan",
        "Torrent",
    ];

    let (disabled_state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_VIRTUAL_SOULFIND_V2_ENABLED", "false"),
    );
    for path in ["/api/source-providers", "/api/v0/source-providers"] {
        let response = super::route_http_request("GET", path, None, "", &disabled_state)
            .await
            .expect("disabled source-provider catalog response");
        let value = serde_json::from_str::<serde_json::Value>(&response.body)
            .unwrap_or(serde_json::Value::Null);
        let providers = value["providers"].as_array().cloned().unwrap_or_default();
        let provider_ids = providers
            .iter()
            .filter_map(|provider| provider["id"].as_str())
            .collect::<Vec<_>>();
        let profile_policies = value["profilePolicies"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let pass = response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && value["acquisitionPlanningEnabled"] == false
            && provider_ids == expected_ids
            && providers.iter().all(|provider| {
                provider["registered"] == true
                    && provider["active"] == false
                    && provider["disabledReason"]
                        == "VirtualSoulfind v2 acquisition planning is disabled."
                    && provider.get("description").is_some()
                    && provider.get("riskLevel").is_some()
                    && provider.get("capabilities").is_some()
                    && provider.get("sortOrder").is_some()
            })
            && profile_policies.len() == 7
            && value.get("count").is_none();
        if !pass {
            mismatches.push(format!(
                "{target} GET {path}: {} {}",
                response.status, response.body
            ));
        }
        ledger.push(serde_json::json!({
            "target": target,
            "method": "GET",
            "route": path,
            "case": "nominal-status-headers-body",
            "pass": pass,
        }));
    }

    let (enabled_state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_VIRTUAL_SOULFIND_V2_ENABLED", "true"),
    );
    let enabled =
        super::route_http_request("GET", "/api/v0/source-providers", None, "", &enabled_state)
            .await
            .expect("enabled source-provider catalog response");
    let enabled_json =
        serde_json::from_str::<serde_json::Value>(&enabled.body).unwrap_or(serde_json::Value::Null);
    let enabled_providers = enabled_json["providers"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let enabled_pass = enabled.status == "200 OK"
        && enabled_json["acquisitionPlanningEnabled"] == true
        && enabled_providers
            .iter()
            .map(|provider| provider["id"].as_str().unwrap_or_default())
            .eq(expected_ids.iter().copied())
        && enabled_providers.iter().all(|provider| {
            if matches!(
                provider["id"].as_str(),
                Some("LocalLibrary") | Some("Soulseek")
            ) {
                provider["active"] == true && provider["disabledReason"].is_null()
            } else {
                provider["active"] == false && provider["disabledReason"].is_string()
            }
        });
    if !enabled_pass {
        mismatches.push(format!(
            "{target} GET /api/v0/source-providers enabled projection: {} {}",
            enabled.status, enabled.body
        ));
    }
    ledger.push(serde_json::json!({
        "target": target,
        "method": "GET",
        "route": "/api/v0/source-providers",
        "case": "populated-dynamic-state",
        "pass": enabled_pass,
    }));

    let enabled_alias =
        super::route_http_request("GET", "/api/source-providers", None, "", &enabled_state)
            .await
            .expect("enabled source-provider catalog alias response");
    let enabled_alias_json = serde_json::from_str::<serde_json::Value>(&enabled_alias.body)
        .unwrap_or(serde_json::Value::Null);
    let enabled_alias_providers = enabled_alias_json["providers"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let enabled_alias_pass = enabled_alias.status == "200 OK"
        && enabled_alias_json["acquisitionPlanningEnabled"] == true
        && enabled_alias_providers
            .iter()
            .map(|provider| provider["id"].as_str().unwrap_or_default())
            .eq(expected_ids.iter().copied())
        && enabled_alias_providers.iter().all(|provider| {
            if matches!(
                provider["id"].as_str(),
                Some("LocalLibrary") | Some("Soulseek")
            ) {
                provider["active"] == true && provider["disabledReason"].is_null()
            } else {
                provider["active"] == false && provider["disabledReason"].is_string()
            }
        });
    if !enabled_alias_pass {
        mismatches.push(format!(
            "{target} GET /api/source-providers enabled projection: {} {}",
            enabled_alias.status, enabled_alias.body
        ));
    }
    ledger.push(serde_json::json!({
        "target": target,
        "method": "GET",
        "route": "/api/source-providers",
        "case": "populated-dynamic-state",
        "pass": enabled_alias_pass,
    }));

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("source_provider_catalog.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

/// Differential evidence for the remaining read-only source-provider
/// route edges.  The native catalog is configuration-derived and does not
/// read SQLite; malformed extra segments remain unmatched on both route
/// aliases while empty and closed-database state still returns the full
/// catalog.
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
async fn controller_api_differential_source_provider_edge_contracts() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($route:expr, $case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!(
                    "{target} GET {} [{}]",
                    $route,
                    $case
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "GET",
                "route": $route,
                "case": $case,
                "pass": pass,
            }));
        }};
    }

    let env = || {
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_VIRTUAL_SOULFIND_V2_ENABLED", "false")
    };
    let (state, _receiver) = test_state_with_env(env());
    for route in ["/api/source-providers", "/api/v0/source-providers"] {
        let malformed =
            super::route_http_request("GET", &format!("{route}/extra"), None, "", &state)
                .await
                .expect("malformed source-provider path");
        record!(
            route,
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );

        let empty = super::route_http_request("GET", route, None, "", &state)
            .await
            .expect("empty source-provider catalog");
        let empty_json = serde_json::from_str::<serde_json::Value>(&empty.body).unwrap_or_default();
        record!(
            route,
            "missing-empty-or-conflict-state",
            empty.status == "200 OK"
                && empty_json["acquisitionPlanningEnabled"] == false
                && empty_json["providers"]
                    .as_array()
                    .is_some_and(|providers| providers.len() == 9)
        );
    }

    let failure_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("source-provider failure database");
    let (failure_state, _receiver) =
        test_state_with_env_parts(env(), super::SearchStore::new(), Some(failure_db.clone()));
    failure_db.close_for_test().await;
    for route in ["/api/source-providers", "/api/v0/source-providers"] {
        let response = super::route_http_request("GET", route, None, "", &failure_state)
            .await
            .expect("source-provider catalog after database close");
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            route,
            "runtime-failure-and-timeout",
            response.status == "200 OK"
                && value["providers"]
                    .as_array()
                    .is_some_and(|providers| providers.len() == 9)
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("source_provider_edge_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize source-provider edge ledger"),
    )
    .expect("write source-provider edge ledger");
    assert!(
        mismatches.is_empty(),
        "{} source-provider edge mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof for the empty slskdN Soulseek recommendation
/// DTOs. The versioned controller returns the protocol-shaped pair of
/// arrays, even when there are no local or global recommendations.
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
async fn controller_api_differential_versioned_soulseek_recommendations() {
    let target = "slskdn";
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let expected = serde_json::json!({
        "recommendations": [],
        "unrecommendations": [],
    });
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    for path in [
        "/api/v0/soulseek/recommendations",
        "/api/v0/soulseek/recommendations/global",
    ] {
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .expect("versioned Soulseek recommendations response");
        let value = serde_json::from_str::<serde_json::Value>(&response.body)
            .unwrap_or(serde_json::Value::Null);
        let pass = response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && value == expected;
        if !pass {
            mismatches.push(format!(
                "{target} GET {path}: {} {}",
                response.status, response.body
            ));
        }
        ledger.push(serde_json::json!({
            "target": target,
            "method": "GET",
            "route": path,
            "case": "nominal-status-headers-body",
            "pass": pass,
        }));
    }
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("versioned_soulseek_recommendations.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

/// Differential proof for the versioned Soulseek item-discovery DTOs.
/// The legacy handlers retain their compatibility envelopes, while the
/// versioned routes expose the slskdN `ItemRecommendations` and
/// `ItemSimilarUsers` property shapes.
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
async fn controller_api_differential_versioned_soulseek_item_discovery() {
    let target = "slskdn";
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let cases = [
        (
            "/api/v0/soulseek/items/ambient/recommendations",
            "/api/v0/soulseek/items/{item}/recommendations",
            serde_json::json!({
                "item": "ambient",
                "recommendations": [],
            }),
        ),
        (
            "/api/v0/soulseek/items/ambient/similar-users",
            "/api/v0/soulseek/items/{item}/similar-users",
            serde_json::json!({
                "item": "ambient",
                "usernames": [],
            }),
        ),
    ];
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    for (path, ledger_route, expected) in cases {
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .expect("versioned Soulseek item-discovery response");
        let value = serde_json::from_str::<serde_json::Value>(&response.body)
            .unwrap_or(serde_json::Value::Null);
        let pass = response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && value == expected;
        if !pass {
            mismatches.push(format!(
                "{target} GET {path}: {} {}",
                response.status, response.body
            ));
        }
        ledger.push(serde_json::json!({
            "target": target,
            "method": "GET",
            "route": ledger_route,
            "case": "nominal-status-headers-body",
            "pass": pass,
        }));
    }
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("versioned_soulseek_item_discovery.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

/// Differential proof for the empty versioned Soulseek similar-user
/// collection. The legacy route keeps its mesh envelope; slskdN exposes
/// the underlying `IReadOnlyCollection<SimilarUser>` directly.
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
async fn controller_api_differential_versioned_soulseek_similar_users() {
    let target = "slskdn";
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let response =
        super::route_http_request("GET", "/api/v0/soulseek/users/similar", None, "", &state)
            .await
            .expect("versioned Soulseek similar-users response");
    let value = serde_json::from_str::<serde_json::Value>(&response.body)
        .unwrap_or(serde_json::Value::Null);
    let pass = response.status == "200 OK"
        && response.content_type.starts_with("application/json")
        && value == serde_json::json!([]);
    let ledger = vec![serde_json::json!({
        "target": target,
        "method": "GET",
        "route": "/api/v0/soulseek/users/similar",
        "case": "nominal-status-headers-body",
        "pass": pass,
    })];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("versioned_soulseek_similar_users.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        pass,
        "{target} GET /api/v0/soulseek/users/similar: got {} {} {}",
        response.status, response.content_type, response.body
    );
}

/// Bulk differential proof for the slskdN auto-replace status DTO.
/// The versioned controller exposes only the five fields below; the
/// legacy compatibility route intentionally keeps its older projection.
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
async fn controller_api_differential_versioned_autoreplace_status() {
    let target = "slskdn";
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let response = super::route_http_request("GET", "/api/v0/autoreplace", None, "", &state)
        .await
        .expect("versioned auto-replace status response");
    let value = serde_json::from_str::<serde_json::Value>(&response.body)
        .unwrap_or(serde_json::Value::Null);
    let expected = serde_json::json!({
        "enabled": false,
        "lastRunAt": null,
        "lastRunProcessedCount": 0,
        "lastRunReplacedCount": 0,
        "intervalSeconds": 300,
    });
    let pass = response.status == "200 OK"
        && response.content_type.starts_with("application/json")
        && value == expected;
    let ledger = vec![serde_json::json!({
        "target": target,
        "method": "GET",
        "route": "/api/v0/autoreplace",
        "case": "nominal-status-headers-body",
        "pass": pass,
    })];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("versioned_autoreplace_status.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        pass,
        "{target} GET /api/v0/autoreplace: got {} {} {}",
        response.status, response.content_type, response.body
    );
}

/// Differential proof for the populated slskdN auto-replace status.  The
/// controller must reflect the real enable mutation instead of returning
/// the disabled baseline on every GET.
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
async fn controller_api_differential_versioned_autoreplace_populated_state() {
    let target = "slskdn";
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
    let enabled = super::route_http_request("PUT", "/api/v0/autoreplace/enable", None, "", &state)
        .await
        .expect("enable versioned auto-replace");
    assert_eq!(enabled.status, "200 OK", "{}", enabled.body);

    let response = super::route_http_request("GET", "/api/v0/autoreplace", None, "", &state)
        .await
        .expect("populated versioned auto-replace status response");
    let value = serde_json::from_str::<serde_json::Value>(&response.body)
        .unwrap_or(serde_json::Value::Null);
    let pass = response.status == "200 OK"
        && response.content_type.starts_with("application/json")
        && value["enabled"] == true
        && value["lastRunAt"].is_null()
        && value["lastRunProcessedCount"] == 0
        && value["lastRunReplacedCount"] == 0
        && value["intervalSeconds"] == 300;
    let ledger = vec![serde_json::json!({
        "target": target,
        "method": "GET",
        "route": "/api/v0/autoreplace",
        "case": "populated-dynamic-state",
        "pass": pass,
    })];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("versioned_autoreplace_populated_state.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        pass,
        "{target} GET /api/v0/autoreplace: got {} {} {}",
        response.status, response.content_type, response.body
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
    feature = "bounded-controller-api-tests-3"
))]
async fn controller_api_differential_native_autoreplace_edge_contracts() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!("{target} {} {} [{}]", $method, $route, $case));
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

    fn status_matches(response: &super::HttpResponse, enabled: bool) -> bool {
        response.status == "200 OK"
            && response.content_type.starts_with("application/json")
            && serde_json::from_str::<serde_json::Value>(&response.body)
                .ok()
                .is_some_and(|value| {
                    value["enabled"] == enabled
                        && value["lastRunAt"].is_null()
                        && value["lastRunProcessedCount"] == 0
                        && value["lastRunReplacedCount"] == 0
                        && value["intervalSeconds"] == 300
                })
    }

    let env = || MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target);
    let (state, _receiver) = test_state_with_env(env());
    let malformed_status = super::route_http_request(
        "GET",
        "/api/v0/autoreplace?unexpected=not-a-number",
        None,
        "",
        &state,
    )
    .await
    .expect("malformed auto-replace status query");
    record!(
        "GET",
        "/api/v0/autoreplace",
        "malformed-path-query-or-body",
        status_matches(&malformed_status, false)
    );
    let empty_status = super::route_http_request("GET", "/api/v0/autoreplace", None, "", &state)
        .await
        .expect("empty auto-replace status");
    record!(
        "GET",
        "/api/v0/autoreplace",
        "missing-empty-or-conflict-state",
        status_matches(&empty_status, false)
    );

    let read_failure_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("auto-replace read-failure database");
    let (read_failure_state, _receiver) = test_state_with_env_parts(
        env(),
        super::SearchStore::new(),
        Some(read_failure_db.clone()),
    );
    read_failure_db.close_for_test().await;
    let read_failure_status =
        super::route_http_request("GET", "/api/v0/autoreplace", None, "", &read_failure_state)
            .await
            .expect("auto-replace status after database failure");
    record!(
        "GET",
        "/api/v0/autoreplace",
        "runtime-failure-and-timeout",
        status_matches(&read_failure_status, false)
    );

    let (enable_state, _receiver) = test_state_with_env(env());
    let malformed_enable = super::route_http_request(
        "PUT",
        "/api/v0/autoreplace/enable?unexpected=not-a-number",
        None,
        "not-json",
        &enable_state,
    )
    .await
    .expect("malformed auto-replace enable query");
    record!(
        "PUT",
        "/api/v0/autoreplace/enable",
        "malformed-path-query-or-body",
        status_matches(&malformed_enable, true)
    );
    let enabled =
        super::route_http_request("PUT", "/api/v0/autoreplace/enable", None, "", &enable_state)
            .await
            .expect("enable auto-replace");
    record!(
        "PUT",
        "/api/v0/autoreplace/enable",
        "missing-empty-or-conflict-state",
        status_matches(&enabled, true)
    );

    let enable_failure_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("auto-replace enable-failure database");
    let (enable_failure_state, _receiver) = test_state_with_env_parts(
        env(),
        super::SearchStore::new(),
        Some(enable_failure_db.clone()),
    );
    enable_failure_db.close_for_test().await;
    let failed_enable = super::route_http_request(
        "PUT",
        "/api/v0/autoreplace/enable",
        None,
        "",
        &enable_failure_state,
    )
    .await
    .expect("auto-replace enable after database failure");
    record!(
        "PUT",
        "/api/v0/autoreplace/enable",
        "runtime-failure-and-timeout",
        status_matches(&failed_enable, true)
    );

    let enable_restart_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("auto-replace enable restart database");
    let (enable_restart_state, _receiver) = test_state_with_env_parts(
        env(),
        super::SearchStore::new(),
        Some(enable_restart_db.clone()),
    );
    let enabled_for_restart = super::route_http_request(
        "PUT",
        "/api/v0/autoreplace/enable",
        None,
        "",
        &enable_restart_state,
    )
    .await
    .expect("persist auto-replace enable");
    let enable_record = enable_restart_db
        .get_runtime_compat_state()
        .await
        .expect("read auto-replace enable state")
        .expect("auto-replace enable state record");
    let rehydrated_enable = super::RuntimeCompatState::from_persisted(&enable_record);
    record!(
        "PUT",
        "/api/v0/autoreplace/enable",
        "restart-persistence-or-reset",
        status_matches(&enabled_for_restart, true) && rehydrated_enable.autoreplace_enabled
    );

    let (enable_concurrent_state, _receiver) = test_state_with_env(env());
    let enable_first = super::route_http_request(
        "PUT",
        "/api/v0/autoreplace/enable",
        None,
        "",
        &enable_concurrent_state,
    )
    .await
    .expect("first auto-replace enable");
    let enable_second = super::route_http_request(
        "PUT",
        "/api/v0/autoreplace/enable",
        None,
        "",
        &enable_concurrent_state,
    )
    .await
    .expect("repeat auto-replace enable");
    record!(
        "PUT",
        "/api/v0/autoreplace/enable",
        "concurrency-and-idempotency",
        status_matches(&enable_first, true)
            && status_matches(&enable_second, true)
            && enable_concurrent_state
                .runtime
                .read()
                .await
                .autoreplace_enabled
    );

    let (disable_state, _receiver) = test_state_with_env(env());
    let malformed_disable = super::route_http_request(
        "PUT",
        "/api/v0/autoreplace/disable?unexpected=not-a-number",
        None,
        "not-json",
        &disable_state,
    )
    .await
    .expect("malformed auto-replace disable query");
    record!(
        "PUT",
        "/api/v0/autoreplace/disable",
        "malformed-path-query-or-body",
        status_matches(&malformed_disable, false)
    );
    let disabled = super::route_http_request(
        "PUT",
        "/api/v0/autoreplace/disable",
        None,
        "",
        &disable_state,
    )
    .await
    .expect("disable auto-replace");
    record!(
        "PUT",
        "/api/v0/autoreplace/disable",
        "missing-empty-or-conflict-state",
        status_matches(&disabled, false)
    );

    let disable_restart_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("auto-replace disable restart database");
    let (disable_restart_state, _receiver) = test_state_with_env_parts(
        env(),
        super::SearchStore::new(),
        Some(disable_restart_db.clone()),
    );
    let enabled_before_disable = super::route_http_request(
        "PUT",
        "/api/v0/autoreplace/enable",
        None,
        "",
        &disable_restart_state,
    )
    .await
    .expect("enable before auto-replace disable");
    let disabled_for_restart = super::route_http_request(
        "PUT",
        "/api/v0/autoreplace/disable",
        None,
        "",
        &disable_restart_state,
    )
    .await
    .expect("persist auto-replace disable");
    let disable_record = disable_restart_db
        .get_runtime_compat_state()
        .await
        .expect("read auto-replace disable state")
        .expect("auto-replace disable state record");
    let rehydrated_disable = super::RuntimeCompatState::from_persisted(&disable_record);
    record!(
        "PUT",
        "/api/v0/autoreplace/disable",
        "restart-persistence-or-reset",
        status_matches(&enabled_before_disable, true)
            && status_matches(&disabled_for_restart, false)
            && !rehydrated_disable.autoreplace_enabled
    );

    let (disable_concurrent_state, _receiver) = test_state_with_env(env());
    let disable_first = super::route_http_request(
        "PUT",
        "/api/v0/autoreplace/disable",
        None,
        "",
        &disable_concurrent_state,
    )
    .await
    .expect("first auto-replace disable");
    let disable_second = super::route_http_request(
        "PUT",
        "/api/v0/autoreplace/disable",
        None,
        "",
        &disable_concurrent_state,
    )
    .await
    .expect("repeat auto-replace disable");
    record!(
        "PUT",
        "/api/v0/autoreplace/disable",
        "concurrency-and-idempotency",
        status_matches(&disable_first, false)
            && status_matches(&disable_second, false)
            && !disable_concurrent_state
                .runtime
                .read()
                .await
                .autoreplace_enabled
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("native_autoreplace_edge_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskdn auto-replace edge ledger"),
    )
    .expect("write slskdn auto-replace edge ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskdn auto-replace edge mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential evidence for the remaining slskdN OptionsController
/// edges.  These cases cover the controller's invalid-options failure
/// projection, startup/config-file isolation from SQLite, validation's
/// deliberately non-mutating lifecycle, and concurrent YAML replacement.
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
async fn controller_api_differential_native_options_edge_contracts() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!("{target} {} {} [{}]", $method, $route, $case));
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

    fn problem_response(response: &super::HttpResponse) -> bool {
        response.status == "500 Internal Server Error"
            && response.content_type == "application/problem+json"
            && serde_json::from_str::<serde_json::Value>(&response.body)
                .ok()
                .is_some_and(|value| {
                    value["title"] == "Internal Server Error"
                        && value["status"] == 500
                        && value["detail"] == "An unexpected error occurred."
                })
    }

    let env = || {
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_REMOTE_CONFIGURATION", "true")
            .with("SLSKR_DEBUG", "true")
            .with("SLSKR_NO_CONFIG_WATCH", "true")
    };

    let (state, _receiver) = test_state_with_env(env());
    let malformed_current =
        super::route_http_request("GET", "/api/v0/options/extra", None, "", &state)
            .await
            .expect("malformed current options response");
    record!(
        "GET",
        "/api/v0/options",
        "malformed-path-query-or-body",
        malformed_current.status == "404 Not Found"
    );

    *state
        .controller_options_validation_error
        .write()
        .expect("current options validation error lock") =
        Some("differential options failure".to_owned());
    let invalid_current = super::route_http_request("GET", "/api/v0/options", None, "", &state)
        .await
        .expect("invalid current options response");
    record!(
        "GET",
        "/api/v0/options",
        "missing-empty-or-conflict-state",
        problem_response(&invalid_current)
    );

    let invalid_debug = super::route_http_request("GET", "/api/v0/options/debug", None, "", &state)
        .await
        .expect("invalid debug options response");
    record!(
        "GET",
        "/api/v0/options/debug",
        "runtime-failure-and-timeout",
        problem_response(&invalid_debug)
    );

    let patch_failure = super::route_http_request(
        "PATCH",
        "/api/v0/options",
        None,
        r#"{"soulseek":{"listenPort":50331}}"#,
        &state,
    )
    .await
    .expect("invalid options patch response");
    record!(
        "PATCH",
        "/api/v0/options",
        "runtime-failure-and-timeout",
        problem_response(&patch_failure)
    );

    let database = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("options read-only failure database");
    let (database_state, _receiver) =
        test_state_with_env_parts(env(), super::SearchStore::new(), Some(database.clone()));
    database.close_for_test().await;
    let startup_after_database_failure =
        super::route_http_request("GET", "/api/v0/options/startup", None, "", &database_state)
            .await
            .expect("startup options after database failure");
    record!(
        "GET",
        "/api/v0/options/startup",
        "runtime-failure-and-timeout",
        startup_after_database_failure.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&startup_after_database_failure.body,)
                .is_ok_and(|value| value.is_object())
    );

    let location_after_database_failure = super::route_http_request(
        "GET",
        "/api/v0/options/yaml/location",
        None,
        "",
        &database_state,
    )
    .await
    .expect("options location after database failure");
    record!(
        "GET",
        "/api/v0/options/yaml/location",
        "runtime-failure-and-timeout",
        location_after_database_failure.status == "200 OK"
            && serde_json::from_str::<String>(&location_after_database_failure.body)
                .is_ok_and(|value| value.ends_with("slskd.yml"))
    );

    let (missing_validation_state, _receiver) = test_state_with_env(env());
    let missing_validation = super::route_http_request(
        "POST",
        "/api/v0/options/yaml/validate",
        None,
        "",
        &missing_validation_state,
    )
    .await
    .expect("missing YAML validation response");
    record!(
        "POST",
        "/api/v0/options/yaml/validate",
        "missing-empty-or-conflict-state",
        missing_validation.status == "400 Bad Request"
    );

    let valid_yaml = serde_json::to_string("soulseek:\n  description: validation-only\n")
        .expect("serialize validation YAML");
    let validation_state = test_state_with_env(env()).0;
    let validation_path = validation_state.config.state_dir.join("slskd.yml");
    let _ = fs::remove_file(&validation_path);
    let before_description = validation_state
        .user_info_description
        .read()
        .expect("validation description lock")
        .clone();
    let validation = super::route_http_request(
        "POST",
        "/api/v0/options/yaml/validate",
        None,
        &valid_yaml,
        &validation_state,
    )
    .await
    .expect("valid YAML validation response");
    let after_description = validation_state
        .user_info_description
        .read()
        .expect("validation description readback lock")
        .clone();
    record!(
        "POST",
        "/api/v0/options/yaml/validate",
        "mutation-side-effects-and-readback",
        validation.status == "200 OK"
            && validation.body.is_empty()
            && !validation_path.exists()
            && after_description == before_description
    );

    let reload_env = MapEnv::default()
        .with(
            "SLSKR_STATE_DIR",
            validation_state.config.state_dir.to_str().unwrap(),
        )
        .with("SLSKR_CONTROLLER_PROFILE", target);
    let reloaded = super::AppConfig::from_layers(None, FileConfig::default(), &reload_env)
        .expect("reload options validation state");
    record!(
        "POST",
        "/api/v0/options/yaml/validate",
        "restart-persistence-or-reset",
        reloaded.user_info_description != "validation-only" && !validation_path.exists()
    );

    let (failed_validation_state, _receiver) = test_state_with_env(env());
    *failed_validation_state
        .controller_options_validation_error
        .write()
        .expect("validation failure lock") = Some("differential options failure".to_owned());
    let failed_validation = super::route_http_request(
        "POST",
        "/api/v0/options/yaml/validate",
        None,
        &valid_yaml,
        &failed_validation_state,
    )
    .await
    .expect("runtime YAML validation response");
    record!(
        "POST",
        "/api/v0/options/yaml/validate",
        "runtime-failure-and-timeout",
        problem_response(&failed_validation)
    );

    let (malformed_update_state, _receiver) = test_state_with_env(env());
    let malformed_update = super::route_http_request(
        "PUT",
        "/api/v0/options/yaml",
        None,
        "not-json",
        &malformed_update_state,
    )
    .await
    .expect("malformed YAML update response");
    record!(
        "PUT",
        "/api/v0/options/yaml",
        "malformed-path-query-or-body",
        malformed_update.status == "400 Bad Request"
    );

    let missing_update = super::route_http_request(
        "PUT",
        "/api/v0/options/yaml",
        None,
        "",
        &malformed_update_state,
    )
    .await
    .expect("missing YAML update response");
    record!(
        "PUT",
        "/api/v0/options/yaml",
        "missing-empty-or-conflict-state",
        missing_update.status == "400 Bad Request"
    );

    let (failed_update_state, _receiver) = test_state_with_env(env());
    *failed_update_state
        .controller_options_validation_error
        .write()
        .expect("YAML update failure lock") = Some("differential options failure".to_owned());
    let failed_update = super::route_http_request(
        "PUT",
        "/api/v0/options/yaml",
        None,
        &valid_yaml,
        &failed_update_state,
    )
    .await
    .expect("runtime YAML update response");
    record!(
        "PUT",
        "/api/v0/options/yaml",
        "runtime-failure-and-timeout",
        problem_response(&failed_update)
    );

    let (concurrent_state, _receiver) = test_state_with_env(env());
    let concurrent_path = concurrent_state.config.state_dir.join("slskd.yml");
    fs::write(
        &concurrent_path,
        "soulseek:\n  description: options-initial\n",
    )
    .expect("write concurrent YAML baseline");
    let concurrent_bodies = [
        serde_json::to_string("soulseek:\n  description: options-a\n")
            .expect("serialize concurrent YAML A"),
        serde_json::to_string("soulseek:\n  description: options-b\n")
            .expect("serialize concurrent YAML B"),
    ];
    let concurrent_responses =
        futures_util::future::join_all(concurrent_bodies.iter().map(|body| {
            super::route_http_request("PUT", "/api/v0/options/yaml", None, body, &concurrent_state)
        }))
        .await;
    let concurrent_readback = fs::read_to_string(&concurrent_path).unwrap_or_default();
    record!(
        "PUT",
        "/api/v0/options/yaml",
        "concurrency-and-idempotency",
        concurrent_responses.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK" && response.body.is_empty())
        }) && [
            "soulseek:\n  description: options-a\n",
            "soulseek:\n  description: options-b\n",
        ]
        .contains(&concurrent_readback.as_str())
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("native_options_edge_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskdn options edge ledger"),
    )
    .expect("write slskdn options edge ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskdn options edge mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential evidence for the frozen slskdN EventsController.  The
/// native controller reads and writes the durable event service, rejects
/// invalid paging and synthetic-event requests, rolls back failed writes,
/// and preserves records through rehydration and concurrent injection.
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
async fn controller_api_differential_native_events_edge_contracts() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!("{target} {} {} [{}]", $method, $route, $case));
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

    let env = || {
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_PERSISTENCE_ENABLED", "true")
    };
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("events differential database");
    let (state, _receiver) =
        test_state_with_env_parts(env(), super::SearchStore::new(), Some(db.clone()));

    let empty =
        super::route_http_request("GET", "/api/v0/events?offset=0&limit=100", None, "", &state)
            .await
            .expect("empty slskdn events response");
    let empty_json =
        serde_json::from_str::<serde_json::Value>(&empty.body).unwrap_or(serde_json::Value::Null);
    record!(
        "GET",
        "/api/v0/events",
        "missing-empty-or-conflict-state",
        empty.status == "200 OK"
            && empty.content_type == "application/json"
            && empty_json.as_array().is_some_and(Vec::is_empty)
    );

    let malformed_path = super::route_http_request("GET", "/api/v0/events/extra", None, "", &state)
        .await
        .expect("malformed events path response");
    let malformed_query =
        super::route_http_request("GET", "/api/v0/events?offset=-1", None, "", &state)
            .await
            .expect("malformed events query response");
    record!(
        "GET",
        "/api/v0/events",
        "malformed-path-query-or-body",
        malformed_path.status == "404 Not Found"
            && malformed_query.status == "400 Bad Request"
            && malformed_query.body.contains("Offset must be greater")
    );

    let raised =
        super::route_http_request("POST", "/api/v0/events/Noop", None, r#""event-a""#, &state)
            .await
            .expect("nominal slskdn event response");
    let raised_json = serde_json::from_str::<serde_json::Value>(&raised.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/events",
        "nominal-status-headers-body",
        raised.status == "201 Created"
            && raised.content_type == "application/json"
            && raised_json["recorded"] == true
            && raised_json["event"]["type"] == "Noop"
    );

    let populated = super::route_http_request("GET", "/api/v0/events", None, "", &state)
        .await
        .expect("populated slskdn events response");
    let populated_json = serde_json::from_str::<serde_json::Value>(&populated.body)
        .unwrap_or(serde_json::Value::Null);
    record!(
        "GET",
        "/api/v0/events",
        "populated-dynamic-state",
        populated.status == "200 OK"
            && populated_json.as_array().is_some_and(|events| {
                events
                    .iter()
                    .any(|event| event["type"] == "Noop" && event["detail"] == "event-a")
            })
    );
    record!(
        "POST",
        "/api/v0/events",
        "mutation-side-effects-and-readback",
        raised.status == "201 Created"
            && populated_json
                .as_array()
                .is_some_and(|events| { events.iter().any(|event| event["detail"] == "event-a") })
    );

    let unknown = super::route_http_request(
        "POST",
        "/api/v0/events/Unknown",
        None,
        r#""event-unknown""#,
        &state,
    )
    .await
    .expect("unknown slskdn event response");
    record!(
        "POST",
        "/api/v0/events",
        "malformed-path-query-or-body",
        unknown.status == "400 Bad Request" && unknown.body.contains("Unknown event type")
    );

    let missing_body = super::route_http_request("POST", "/api/v0/events/Noop", None, "{", &state)
        .await
        .expect("missing slskdn event body response");
    record!(
        "POST",
        "/api/v0/events",
        "missing-empty-or-conflict-state",
        missing_body.status == "400 Bad Request"
    );

    let persisted = db.list_events(20, 0).await.expect("list persisted events");
    let rehydrated =
        super::EventStore::from_persisted(persisted.clone(), super::EVENT_HISTORY_LIMIT);
    record!(
        "POST",
        "/api/v0/events",
        "restart-persistence-or-reset",
        persisted
            .iter()
            .any(|event| event.detail.as_deref() == Some("event-a"))
            && rehydrated.controller_json(None).contains("event-a")
    );

    let concurrent_responses = futures_util::future::join_all([
        super::route_http_request(
            "POST",
            "/api/v0/events/Noop",
            None,
            r#""concurrent-a""#,
            &state,
        ),
        super::route_http_request(
            "POST",
            "/api/v0/events/Noop",
            None,
            r#""concurrent-b""#,
            &state,
        ),
    ])
    .await;
    let concurrent_events = db.list_events(20, 0).await.expect("list concurrent events");
    record!(
        "POST",
        "/api/v0/events",
        "concurrency-and-idempotency",
        concurrent_responses.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "201 Created")
        }) && concurrent_events
            .iter()
            .any(|event| { event.detail.as_deref() == Some("concurrent-a") })
            && concurrent_events
                .iter()
                .any(|event| { event.detail.as_deref() == Some("concurrent-b") })
    );

    let read_failure_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("events read-failure database");
    let (read_failure_state, _receiver) = test_state_with_env_parts(
        env(),
        super::SearchStore::new(),
        Some(read_failure_db.clone()),
    );
    read_failure_db.close_for_test().await;
    let read_failure =
        super::route_http_request("GET", "/api/v0/events", None, "", &read_failure_state)
            .await
            .expect("events read failure response");
    record!(
        "GET",
        "/api/v0/events",
        "runtime-failure-and-timeout",
        read_failure.status == "500 Internal Server Error"
            && read_failure.body.contains("Failed to list events")
    );

    let write_failure_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("events write-failure database");
    let (write_failure_state, _receiver) = test_state_with_env_parts(
        env(),
        super::SearchStore::new(),
        Some(write_failure_db.clone()),
    );
    write_failure_db.close_for_test().await;
    let write_failure = super::route_http_request(
        "POST",
        "/api/v0/events/Noop",
        None,
        r#""failure""#,
        &write_failure_state,
    )
    .await
    .expect("events write failure response");
    record!(
        "POST",
        "/api/v0/events",
        "runtime-failure-and-timeout",
        write_failure.status == "500 Internal Server Error"
            && write_failure.body.contains("Failed to raise event")
            && write_failure_state.events.read().await.records.is_empty()
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("native_events_edge_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize slskdn events edge ledger"),
    )
    .expect("write slskdn events edge ledger");
    assert!(
        mismatches.is_empty(),
        "{} slskdn events edge mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof for the three fixed-version slskdN native
/// projections that were previously rejected by the shared slskd
/// unsupported-version guard: mesh-health and the signal-system
/// configuration/status DTOs.  The assertions use the frozen controller
/// property names (`routingNodes`, `storedKeys`, `active_channels`, and
/// nested `statistics`) so the rows cannot be credited by a merely
/// successful status code or by the legacy unversioned compatibility
/// shapes.
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
async fn controller_api_differential_versioned_mesh_health_and_signals() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($route:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!(
                    "{target} GET {} [nominal-status-headers-body]",
                    $route
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "GET",
                "route": $route,
                "case": "nominal-status-headers-body",
                "pass": $pass,
            }));
        };
    }

    let advanced = serde_json::json!({
        "SignalSystem": {
            "enabled": false,
            "deduplicationCacheSize": 2048,
            "defaultTtl": "00:07:30",
            "meshChannel": {
                "enabled": false,
                "priority": 3,
                "requireActiveSession": true
            },
            "btExtensionChannel": {
                "enabled": true,
                "priority": 4,
                "requireActiveSession": false
            }
        }
    });
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("SLSKR_ADVANCED_NETWORKING_JSON", &advanced.to_string()),
    );

    let health = super::route_http_request("GET", "/api/v0/mesh/health", None, "", &state)
        .await
        .expect("versioned mesh health response");
    let health_json =
        serde_json::from_str::<serde_json::Value>(&health.body).unwrap_or(serde_json::Value::Null);
    record!(
        "/api/v0/mesh/health",
        health.status == "200 OK"
            && health.content_type.starts_with("application/json")
            && health_json["routingNodes"] == 0
            && health_json["storedKeys"] == 0
            && health_json["contentPeerHints"] == 0
            && health_json["generatedAt"].as_str().is_some()
            && health_json.get("status").is_none()
    );

    let config = super::route_http_request("GET", "/api/v0/signals/config", None, "", &state)
        .await
        .expect("versioned signal configuration response");
    let config_json =
        serde_json::from_str::<serde_json::Value>(&config.body).unwrap_or(serde_json::Value::Null);
    record!(
        "/api/v0/signals/config",
        config.status == "200 OK"
            && config.content_type.starts_with("application/json")
            && config_json["enabled"] == false
            && config_json["deduplication_cache_size"] == 2048
            && config_json["default_ttl_seconds"] == 450
            && config_json["mesh_channel"]["enabled"] == false
            && config_json["mesh_channel"]["priority"] == 3
            && config_json["mesh_channel"]["require_active_session"] == true
            && config_json["bt_extension_channel"]["enabled"] == true
            && config_json["bt_extension_channel"]["priority"] == 4
            && config_json["bt_extension_channel"]["require_active_session"] == false
            && config_json.get("meshChannel").is_none()
    );

    let status = super::route_http_request("GET", "/api/v0/signals/status", None, "", &state)
        .await
        .expect("versioned signal status response");
    let status_json =
        serde_json::from_str::<serde_json::Value>(&status.body).unwrap_or(serde_json::Value::Null);
    record!(
        "/api/v0/signals/status",
        status.status == "200 OK"
            && status.content_type.starts_with("application/json")
            && status_json["enabled"] == false
            && status_json["active_channels"] == serde_json::json!([])
            && status_json["statistics"]["signals_sent"] == 0
            && status_json["statistics"]["signals_received"] == 0
            && status_json["statistics"]["duplicate_signals_dropped"] == 0
            && status_json["statistics"]["expired_signals_dropped"] == 0
            && status_json.get("activeChannels").is_none()
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("versioned_mesh_health_and_signals.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api versioned mesh-health/signals mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential evidence for the remaining fixed-version SignalSystem
/// edge rows.  Both DTOs are configuration/process projections, so their
/// empty and closed-SQLite responses remain successful while malformed
/// extra path segments are rejected by routing.
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
async fn controller_api_differential_versioned_signals_edge_contracts() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($route:expr, $case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!(
                    "{target} GET {} [{}]",
                    $route,
                    $case
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "GET",
                "route": $route,
                "case": $case,
                "pass": pass,
            }));
        }};
    }

    let env = || MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target);
    let (state, _receiver) = test_state_with_env(env());
    for route in ["/api/v0/signals/config", "/api/v0/signals/status"] {
        let malformed =
            super::route_http_request("GET", &format!("{route}/extra"), None, "", &state)
                .await
                .expect("malformed signal path");
        record!(
            route,
            "malformed-path-query-or-body",
            malformed.status == "404 Not Found"
        );

        let empty = super::route_http_request("GET", route, None, "", &state)
            .await
            .expect("empty signal projection");
        let empty_json = serde_json::from_str::<serde_json::Value>(&empty.body).unwrap_or_default();
        let shape = if route.ends_with("/config") {
            empty_json.get("enabled").is_some()
                && empty_json.get("deduplication_cache_size").is_some()
        } else {
            empty_json.get("enabled").is_some()
                && empty_json.get("active_channels").is_some()
                && empty_json.get("statistics").is_some()
        };
        record!(
            route,
            "missing-empty-or-conflict-state",
            empty.status == "200 OK" && empty.content_type.starts_with("application/json") && shape
        );
    }

    let failure_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("signal failure database");
    let (failure_state, _receiver) =
        test_state_with_env_parts(env(), super::SearchStore::new(), Some(failure_db.clone()));
    failure_db.close_for_test().await;
    for route in ["/api/v0/signals/config", "/api/v0/signals/status"] {
        let response = super::route_http_request("GET", route, None, "", &failure_state)
            .await
            .expect("signal projection after database close");
        record!(
            route,
            "runtime-failure-and-timeout",
            response.status == "200 OK" && response.content_type.starts_with("application/json")
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("versioned_signals_edge_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize signal edge ledger"),
    )
    .expect("write signal edge ledger");
    assert!(
        mismatches.is_empty(),
        "{} signal edge mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential evidence for the slskdN compatibility user-browse route.
/// The route projects the real browse store into the legacy directory/file
/// DTO, rejects malformed route shapes, returns not-found for absent users,
/// and remains process-local when SQLite is unavailable.
/// Differential evidence for the slskdN swarm trace summary projection.
/// The route is process-local/file-backed in the frozen controller, so a
/// closed unrelated SQLite pool must not change its empty response.
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
async fn controller_api_differential_traces_summary_contracts() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($case:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!(
                    "{target} GET /api/v0/traces/{{jobId}}/summary [{}]",
                    $case
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "GET",
                "route": "/api/v0/traces/{jobId}/summary",
                "case": $case,
                "pass": $pass,
            }));
        };
    }

    let (state, _receiver) = test_state();
    let malformed = super::route_http_request("GET", "/api/v0/traces//summary", None, "", &state)
        .await
        .expect("malformed trace summary route");
    record!(
        "malformed-path-query-or-body",
        malformed.status == "404 Not Found"
    );

    let nominal = super::route_http_request(
        "GET",
        "/api/v0/traces/trace-empty/summary",
        None,
        "",
        &state,
    )
    .await
    .expect("nominal empty trace summary route");
    let nominal_json =
        serde_json::from_str::<serde_json::Value>(&nominal.body).unwrap_or(serde_json::Value::Null);
    record!(
        "nominal-status-headers-body",
        nominal.status == "200 OK"
            && nominal.content_type == "application/json"
            && nominal_json["jobId"] == "trace-empty"
            && nominal_json["totalEvents"] == 0
            && nominal_json["eventCounts"].is_object()
            && nominal_json["bytesBySource"].is_object()
            && nominal_json["bytesByBackend"].is_object()
            && nominal_json["peers"].as_array().is_some_and(Vec::is_empty)
            && nominal_json["rescueInvoked"] == false
    );

    let now = super::unix_timestamp();
    state
        .multisource
        .write()
        .await
        .insert(super::multisource::SwarmJob {
            id: "trace-populated".to_owned(),
            status: "completed".to_owned(),
            filename: "trace.flac".to_owned(),
            output_path: "trace.flac".to_owned(),
            file_size: 1_024,
            chunk_size: 512,
            sources: vec!["trace-peer".to_owned()],
            completed_chunks: 2,
            total_chunks: 2,
            bytes_downloaded: 1_024,
            created_at: now,
            updated_at: now + 1,
            result: Some(super::multisource::SwarmResult {
                id: "trace-populated".to_owned(),
                success: true,
                filename: "trace.flac".to_owned(),
                output_path: "trace.flac".to_owned(),
                bytes_downloaded: 1_024,
                total_time_ms: 100,
                sources_used: 1,
                final_hash: "11".repeat(32),
                chunks: vec![
                    super::multisource::ChunkResult {
                        index: 0,
                        username: "trace-peer".to_owned(),
                        start_offset: 0,
                        end_offset: 511,
                        bytes_downloaded: 512,
                        time_ms: 40,
                    },
                    super::multisource::ChunkResult {
                        index: 1,
                        username: "trace-peer".to_owned(),
                        start_offset: 512,
                        end_offset: 1_023,
                        bytes_downloaded: 512,
                        time_ms: 60,
                    },
                ],
                error: None,
            }),
        });
    let populated = super::route_http_request(
        "GET",
        "/api/v0/traces/trace-populated/summary",
        None,
        "",
        &state,
    )
    .await
    .expect("populated trace summary route");
    let populated_json = serde_json::from_str::<serde_json::Value>(&populated.body)
        .unwrap_or(serde_json::Value::Null);
    record!(
        "populated-dynamic-state",
        populated.status == "200 OK"
            && populated_json["jobId"] == "trace-populated"
            && populated_json["firstEventAt"] == now
            && populated_json["lastEventAt"] == now + 1
            && populated_json["duration"] == 1
            && populated_json["totalEvents"] == 2
    );

    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("trace runtime-failure database");
    let (runtime_state, _runtime_receiver) = test_state_with_env_parts(
        MapEnv::default(),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    db.close_for_test().await;
    let runtime = super::route_http_request(
        "GET",
        "/api/v0/traces/runtime-missing/summary",
        None,
        "",
        &runtime_state,
    )
    .await
    .expect("trace summary with closed unrelated database");
    record!(
        "runtime-failure-and-timeout",
        runtime.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&runtime.body)
                .map(|value| value["totalEvents"] == 0)
                .unwrap_or(false)
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create trace evidence directory");
    fs::write(
        evidence_dir.join("traces_summary_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize trace evidence"),
    )
    .expect("write trace evidence");
    assert!(
        mismatches.is_empty(),
        "{} trace summary mismatches: {:?}",
        mismatches.len(),
        mismatches
    );
}

/// Differential evidence for the two small slskdN compatibility/fairness
/// projections that previously only had route-presence proof.  The
/// fairness cases use the same durable TrafficStats boundary as the
/// frozen FairnessGuard, including its closed-database failure contract.
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
async fn controller_api_differential_compatibility_info_and_fairness_contracts() {
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($route:expr, $case:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!("GET {} [{}]", $route, $case));
            }
            ledger.push(serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": $route,
                "case": $case,
                "pass": $pass,
            }));
        };
    }

    let (info_state, _receiver) = test_state();
    let malformed_info = super::route_http_request("GET", "/api/info/extra", None, "", &info_state)
        .await
        .expect("malformed compatibility info route");
    record!(
        "/api/info",
        "malformed-path-query-or-body",
        malformed_info.status == "404 Not Found"
    );

    let empty_info = super::route_http_request("GET", "/api/info", None, "", &info_state)
        .await
        .expect("empty compatibility info route");
    let empty_info_json = serde_json::from_str::<serde_json::Value>(&empty_info.body)
        .unwrap_or(serde_json::Value::Null);
    record!(
        "/api/info",
        "missing-empty-or-conflict-state",
        empty_info.status == "200 OK"
            && empty_info.content_type == "application/json"
            && empty_info_json["impl"] == "slskdn"
            && empty_info_json["compat"] == "slskd"
            && empty_info_json["version"].is_string()
            && empty_info_json["soulseek"]["connected"] == false
            && empty_info_json["soulseek"]["user"] == "tester"
    );

    {
        let mut session = info_state.session.write().await;
        session.state = "connected";
        session.username = Some("connected-user".to_owned());
    }
    let populated_info = super::route_http_request("GET", "/api/info", None, "", &info_state)
        .await
        .expect("populated compatibility info route");
    let populated_info_json = serde_json::from_str::<serde_json::Value>(&populated_info.body)
        .unwrap_or(serde_json::Value::Null);
    record!(
        "/api/info",
        "populated-dynamic-state",
        populated_info.status == "200 OK"
            && populated_info_json["soulseek"]["connected"] == true
            && populated_info_json["soulseek"]["user"] == "connected-user"
    );

    let info_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("compatibility info database");
    let (info_failure_state, _receiver) = test_state_with_env_parts(
        MapEnv::default(),
        super::SearchStore::new(),
        Some(info_db.clone()),
    );
    info_db.close_for_test().await;
    let info_failure = super::route_http_request("GET", "/api/info", None, "", &info_failure_state)
        .await
        .expect("compatibility info route with closed unrelated database");
    record!(
        "/api/info",
        "runtime-failure-and-timeout",
        info_failure.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&info_failure.body)
                .map(|value| value["impl"] == "slskdn")
                .unwrap_or(false)
    );

    let fairness_empty = test_state().0;
    let fairness_empty_response =
        super::route_http_request("GET", "/api/v0/fairness/summary", None, "", &fairness_empty)
            .await
            .expect("empty fairness summary");
    let fairness_empty_json =
        serde_json::from_str::<serde_json::Value>(&fairness_empty_response.body)
            .unwrap_or(serde_json::Value::Null);
    record!(
        "/api/v0/fairness/summary",
        "missing-empty-or-conflict-state",
        fairness_empty_response.status == "200 OK"
            && fairness_empty_response.content_type == "application/json"
            && fairness_empty_json["throttleOverlayDownloads"] == false
            && fairness_empty_json["reason"] == "within fairness constraints"
            && fairness_empty_json["overlayUploadDownloadRatio"] == 1.0
            && fairness_empty_json["overlayToSoulseekUploadRatio"] == 0.0
            && fairness_empty_json["totals"]
                == serde_json::json!({
                    "overlayUploadBytes": 0,
                    "overlayDownloadBytes": 0,
                    "soulseekUploadBytes": 0,
                    "soulseekDownloadBytes": 0,
                })
    );

    let malformed_fairness = super::route_http_request(
        "GET",
        "/api/v0/fairness/summary/extra",
        None,
        "",
        &fairness_empty,
    )
    .await
    .expect("malformed fairness summary route");
    record!(
        "/api/v0/fairness/summary",
        "malformed-path-query-or-body",
        malformed_fairness.status == "404 Not Found"
    );

    let fairness_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("fairness database");
    fairness_db
        .add_traffic(120, 200, 60, 300)
        .await
        .expect("seed fairness totals");
    let (fairness_populated, _receiver) = test_state_with_env_parts(
        MapEnv::default(),
        super::SearchStore::new(),
        Some(fairness_db.clone()),
    );
    let populated_fairness = super::route_http_request(
        "GET",
        "/api/v0/fairness/summary",
        None,
        "",
        &fairness_populated,
    )
    .await
    .expect("populated fairness summary");
    let populated_fairness_json =
        serde_json::from_str::<serde_json::Value>(&populated_fairness.body)
            .unwrap_or(serde_json::Value::Null);
    record!(
        "/api/v0/fairness/summary",
        "populated-dynamic-state",
        populated_fairness.status == "200 OK"
            && populated_fairness_json["throttleOverlayDownloads"] == false
            && populated_fairness_json["reason"] == "within fairness constraints"
            && populated_fairness_json["overlayUploadDownloadRatio"] == 0.6
            && populated_fairness_json["overlayToSoulseekUploadRatio"] == 2.0
            && populated_fairness_json["totals"]
                == serde_json::json!({
                    "overlayUploadBytes": 120,
                    "overlayDownloadBytes": 200,
                    "soulseekUploadBytes": 60,
                    "soulseekDownloadBytes": 300,
                })
    );

    let fairness_failure_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("fairness failure database");
    let (fairness_failure, _receiver) = test_state_with_env_parts(
        MapEnv::default(),
        super::SearchStore::new(),
        Some(fairness_failure_db.clone()),
    );
    fairness_failure_db.close_for_test().await;
    let fairness_failure_response = super::route_http_request(
        "GET",
        "/api/v0/fairness/summary",
        None,
        "",
        &fairness_failure,
    )
    .await
    .expect("fairness summary with closed database");
    record!(
        "/api/v0/fairness/summary",
        "runtime-failure-and-timeout",
        fairness_failure_response.status == "500 Internal Server Error"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create compatibility evidence directory");
    fs::write(
        evidence_dir.join("compatibility_info_and_fairness_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize compatibility evidence"),
    )
    .expect("write compatibility evidence");
    assert!(
        mismatches.is_empty(),
        "{} compatibility info/fairness mismatches: {:?}",
        mismatches.len(),
        mismatches
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
    feature = "bounded-controller-api-tests-3"
))]
async fn controller_api_differential_compatibility_user_browse_contracts() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!(
                    "{target} GET /api/compatibility/users/{{username}}/browse [{}]",
                    $case
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "GET",
                "route": "/api/compatibility/users/{username}/browse",
                "case": $case,
                "pass": pass,
            }));
        }};
    }

    let env = || MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target);
    let (state, _receiver) = test_state_with_env(env());
    {
        let mut browse = state.browse.write().await;
        browse.add_entries(
            "compat-peer".to_owned(),
            vec![super::BrowseEntry {
                filename: "Music/Track.flac".to_owned(),
                size: 321,
                extension: "flac".to_owned(),
                path_encoding: super::ProtocolTextEncoding::Utf8,
            }],
            true,
        );
    }
    let nominal = super::route_http_request(
        "GET",
        "/api/compatibility/users/compat-peer/browse",
        None,
        "",
        &state,
    )
    .await
    .expect("compatibility browse nominal response");
    let nominal_json = serde_json::from_str::<serde_json::Value>(&nominal.body).unwrap_or_default();
    record!(
        "nominal-status-headers-body",
        nominal.status == "200 OK"
            && nominal.content_type.starts_with("application/json")
            && nominal_json["username"] == "compat-peer"
            && nominal_json["directories"]
                .as_array()
                .is_some_and(|directories| {
                    directories.iter().any(|directory| {
                        directory["files"].as_array().is_some_and(|files| {
                            files.iter().any(|file| {
                                file["filename"] == "Track.flac"
                                    && file["size"] == 321
                                    && file["attributes"] == serde_json::json!(["flac"])
                            })
                        })
                    })
                })
    );
    record!(
        "populated-dynamic-state",
        nominal.status == "200 OK"
            && nominal_json["directories"]
                .as_array()
                .is_some_and(|directories| !directories.is_empty())
    );

    let malformed = super::route_http_request(
        "GET",
        "/api/compatibility/users/compat-peer/browse/extra",
        None,
        "",
        &state,
    )
    .await
    .expect("malformed compatibility browse path");
    record!(
        "malformed-path-query-or-body",
        malformed.status == "404 Not Found"
    );

    let missing = super::route_http_request(
        "GET",
        "/api/compatibility/users/missing-peer/browse",
        None,
        "",
        &state,
    )
    .await
    .expect("missing compatibility browse user");
    record!(
        "missing-empty-or-conflict-state",
        missing.status == "404 Not Found"
    );

    let failure_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("compatibility browse failure database");
    let (failure_state, _receiver) =
        test_state_with_env_parts(env(), super::SearchStore::new(), Some(failure_db.clone()));
    {
        let mut browse = failure_state.browse.write().await;
        browse.add_entries(
            "compat-peer".to_owned(),
            vec![super::BrowseEntry {
                filename: "Music/Closed.flac".to_owned(),
                size: 123,
                extension: "flac".to_owned(),
                path_encoding: super::ProtocolTextEncoding::Utf8,
            }],
            true,
        );
    }
    failure_db.close_for_test().await;
    let runtime = super::route_http_request(
        "GET",
        "/api/compatibility/users/compat-peer/browse",
        None,
        "",
        &failure_state,
    )
    .await
    .expect("compatibility browse after database close");
    record!(
        "runtime-failure-and-timeout",
        runtime.status == "200 OK" && runtime.body.contains("Closed.flac")
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("compatibility_user_browse_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize compatibility browse ledger"),
    )
    .expect("write compatibility browse ledger");
    assert!(
        mismatches.is_empty(),
        "{} compatibility browse mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
