#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn mesh_gateway_disabled_post_short_circuits_compatibility_fallback() {
    let (state, _receiver) = test_state();
    let response = super::route_http_request(
        "POST",
        "/mesh/http/route-audit-id/route-audit-id",
        None,
        "{}",
        &state,
    )
    .await
    .expect("mesh gateway response");
    assert_eq!(response.status, "404 Not Found");
    assert_eq!(response.body, r#"{"error":"gateway_disabled"}"#);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn mesh_gateway_enabled_enforces_allowlist_and_provider_discovery() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKD_MESH_GATEWAY_ENABLED", "true")
            .with("SLSKD_MESH_GATEWAY_ALLOWED_SERVICES", "pods"),
    );

    let rejected =
        super::route_http_request("POST", "/mesh/http/route-audit-id/List", None, "{}", &state)
            .await
            .expect("allowlist response");
    assert_eq!(rejected.status, "403 Forbidden");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&rejected.body).unwrap(),
        serde_json::json!({
            "error": "service_not_allowed",
            "message": "Requested service is not allowed",
        })
    );

    let unavailable = super::route_http_request("POST", "/mesh/http/pods/List", None, "{}", &state)
        .await
        .expect("provider response");
    assert_eq!(unavailable.status, "503 Service Unavailable");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&unavailable.body).unwrap(),
        serde_json::json!({
            "error": "service_unavailable",
            "message": "No providers found for the requested service",
        })
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn mesh_gateway_enabled_enforces_target_auth_and_origin_contract() {
    run_controller_future_on_large_stack("mesh-gateway-auth-origin", || {
        mesh_gateway_enabled_enforces_target_auth_and_origin_contract_impl()
    });
}

#[cfg(feature = "full-controller-tests")]
async fn mesh_gateway_enabled_enforces_target_auth_and_origin_contract_impl() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "false")
            .with("SLSKR_API_TOKEN", "route-token")
            .with("SLSKD_MESH_GATEWAY_ENABLED", "true")
            .with("SLSKD_MESH_GATEWAY_ALLOWED_SERVICES", "pods")
            .with("SLSKD_MESH_GATEWAY_API_KEY", "gateway-key")
            .with("SLSKD_MESH_GATEWAY_CSRF_TOKEN", "csrf-token"),
    );
    assert_eq!(
        state.config.mesh_gateway.api_key.as_deref(),
        Some("gateway-key")
    );

    let remote = super::RequestSecurityHeaders {
        remote_addr: Some("192.0.2.30:1234".parse().unwrap()),
        ..super::RequestSecurityHeaders::default()
    };
    let unauthorized = Box::pin(super::route_http_request_with_headers(
        "POST",
        "/mesh/http/pods/List",
        None,
        "{}",
        &state,
        remote.clone(),
    ))
    .await
    .expect("remote auth response");
    assert_eq!(unauthorized.status, "401 Unauthorized");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&unauthorized.body).unwrap(),
        serde_json::json!({
            "error": "unauthorized",
            "message": "Valid X-Slskdn-ApiKey header is required",
        })
    );

    let mut authorized = remote;
    authorized.x_gateway_api_key = Some("gateway-key".to_owned());
    assert!(super::mesh_gateway_auth_failure(&state, &authorized).is_none());
    let unavailable = Box::pin(super::route_http_request_with_headers(
        "POST",
        "/mesh/http/pods/List",
        Some("Bearer route-token"),
        "{}",
        &state,
        authorized,
    ))
    .await
    .expect("authorized remote response");
    assert_eq!(unavailable.status, "503 Service Unavailable");

    let local = super::RequestSecurityHeaders {
        remote_addr: Some("127.0.0.1:1234".parse().unwrap()),
        origin: Some("https://evil.example".to_owned()),
        x_gateway_csrf: Some("csrf-token".to_owned()),
        ..super::RequestSecurityHeaders::default()
    };
    let origin_denied = Box::pin(super::route_http_request_with_headers(
        "POST",
        "/mesh/http/pods/List",
        Some("Bearer route-token"),
        "{}",
        &state,
        local,
    ))
    .await
    .expect("origin response");
    assert_eq!(origin_denied.status, "403 Forbidden");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&origin_denied.body).unwrap(),
        serde_json::json!({
            "error": "origin_not_allowed",
            "message": "Cross-origin requests to localhost are not allowed by default",
        })
    );

    let valid_local = super::RequestSecurityHeaders {
        remote_addr: Some("127.0.0.1:1234".parse().unwrap()),
        origin: Some("https://localhost:3000".to_owned()),
        x_gateway_csrf: Some("csrf-token".to_owned()),
        ..super::RequestSecurityHeaders::default()
    };
    let local_unavailable = Box::pin(super::route_http_request_with_headers(
        "POST",
        "/mesh/http/pods/List",
        Some("Bearer route-token"),
        "{}",
        &state,
        valid_local,
    ))
    .await
    .expect("valid local response");
    assert_eq!(local_unavailable.status, "503 Service Unavailable");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn mesh_gateway_enabled_dispatches_to_real_local_service_handlers() {
    let root = std::env::temp_dir().join(format!(
        "slskr-mesh-http-gateway-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).expect("mesh gateway state directory");
    let (mut state, _receiver) = test_state_with_env(
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
        .expect("mesh gateway"),
    );
    Arc::get_mut(&mut state)
        .expect("unshared test state")
        .private_gateway = Some(gateway);

    let response = super::route_http_request("POST", "/mesh/http/pods/List", None, "{}", &state)
        .await
        .expect("local service response");
    assert_eq!(response.status, "200 OK", "{}", response.body);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&response.body).unwrap(),
        serde_json::json!([])
    );
    std::fs::remove_dir_all(root).expect("remove mesh gateway state directory");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn activitypub_music_actor_and_webfinger_match_target_discovery_contract() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("FEDERATION_ENABLED", "true")
            .with("FEDERATION_MODE", "Public")
            .with("FEDERATION_DOMAIN", "social.example")
            .with("FEDERATION_BASE_URL", "https://social.example/")
            .with("FEDERATION_PAGE_SIZE", "10"),
    );

    let actor = super::route_http_request("GET", "/actors/music", None, "", &state)
        .await
        .expect("music actor response");
    assert_eq!(actor.status, "200 OK");
    assert_eq!(actor.content_type, "application/activity+json");
    let actor_json = serde_json::from_str::<serde_json::Value>(&actor.body).unwrap();
    assert_eq!(actor_json["id"], "https://social.example/actors/music");
    assert_eq!(actor_json["type"], "Service");
    assert_eq!(actor_json["preferredUsername"], "music");
    assert_eq!(actor_json["name"], "Music Library");
    assert!(actor_json["publicKey"]["publicKeyPem"]
        .as_str()
        .is_some_and(|key| key.starts_with("-----BEGIN PUBLIC KEY-----")));

    let acct = super::route_http_request(
        "GET",
        "/.well-known/webfinger?resource=acct%3Amusic%40social.example",
        None,
        "",
        &state,
    )
    .await
    .expect("acct WebFinger response");
    assert_eq!(acct.status, "200 OK");
    assert_eq!(acct.content_type, "application/jrd+json");
    let acct_json = serde_json::from_str::<serde_json::Value>(&acct.body).unwrap();
    assert_eq!(acct_json["subject"], "acct:music@social.example");
    assert_eq!(acct_json["links"].as_array().unwrap().len(), 2);

    let https_resource = super::route_http_request(
        "GET",
        "/.well-known/webfinger?resource=https%3A%2F%2Fsocial.example%2F%40music",
        None,
        "",
        &state,
    )
    .await
    .expect("https WebFinger response");
    assert_eq!(https_resource.status, "200 OK");
    let https_json = serde_json::from_str::<serde_json::Value>(&https_resource.body).unwrap();
    assert_eq!(https_json["subject"], "https://social.example/@music");
    assert_eq!(https_json["links"].as_array().unwrap().len(), 2);

    let filtered = super::route_http_request(
        "GET",
        "/.well-known/webfinger?resource=acct%3Amusic%40social.example&rel=self",
        None,
        "",
        &state,
    )
    .await
    .expect("filtered WebFinger response");
    assert_eq!(filtered.status, "200 OK");
    let filtered_json = serde_json::from_str::<serde_json::Value>(&filtered.body).unwrap();
    assert_eq!(filtered_json["links"].as_array().unwrap().len(), 1);

    let generic = super::route_http_request("GET", "/actors/books", None, "", &state)
        .await
        .expect("generic actor response");
    assert_eq!(generic.status, "404 Not Found");
}

// Chaining this many calls through the giant `route_http_request_with_
// headers` dispatcher inside one debug-build async fn overflows the
// default test-thread stack (same fix as `configured_api_token_
// protects_api_routes`): run the real test body on a thread with a
// larger, explicit stack instead of splitting a genuine lifecycle
// sequence into disconnected fragments.
#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn activitypub_relationship_collections_track_target_lifecycle() {
    std::thread::Builder::new()
        .name("activitypub-relationship-lifecycle-test".to_owned())
        .stack_size(64 * 1024 * 1024)
        .spawn(|| {
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(activitypub_relationship_collections_track_target_lifecycle_impl())
        })
        .unwrap()
        .join()
        .unwrap();
}

async fn activitypub_relationship_collections_track_target_lifecycle_impl() {
    use sha2::Digest;
    let fixture = ActivityPubSignatureFixture::spawn().await;
    let (state, _receiver) = fixture.state();
    let follower_one = fixture.actor_url("follower-one");
    let follower_two = fixture.actor_url("follower-two");
    let following_one = fixture.actor_url("following-one");
    let signed_inbox_post = |actor_name: &str, activity: serde_json::Value| {
        let body = activity.to_string();
        let digest = format!(
            "SHA-256={}",
            base64::engine::general_purpose::STANDARD.encode(sha2::Sha256::digest(body.as_bytes())),
        );
        let created = super::unix_timestamp();
        let signature_b64 = fixture.sign_request("post", "/actors/music/inbox", &digest, created);
        let headers = fixture.headers_for(
            &fixture.key_id(actor_name),
            &digest,
            &signature_b64,
            created,
        );
        (body, headers)
    };

    let (body, headers) = signed_inbox_post(
        "follower-one",
        serde_json::json!({
            "id": "follow-one",
            "type": "Follow",
            "actor": follower_one,
            "object": "https://social.example/actors/music"
        }),
    );
    let response = super::route_http_request_with_headers(
        "POST",
        "/actors/music/inbox",
        None,
        &body,
        &state,
        headers,
    )
    .await
    .expect("inbound follow");
    assert_eq!(response.status, "202 Accepted", "{}", response.body);

    let response = super::route_http_request(
        "POST",
        "/actors/music/outbox",
        None,
        &serde_json::json!({
            "id": "follow-two",
            "type": "Follow",
            "object": following_one
        })
        .to_string(),
        &state,
    )
    .await
    .expect("outbound follow");
    assert_eq!(response.status, "200 OK");

    let (body, headers) = signed_inbox_post(
        "follower-two",
        serde_json::json!({
            "id": "accept-two",
            "type": "Accept",
            "actor": follower_two,
            "object": {"type": "Follow", "object": "https://social.example/actors/music"}
        }),
    );
    let response = super::route_http_request_with_headers(
        "POST",
        "/actors/music/inbox",
        None,
        &body,
        &state,
        headers,
    )
    .await
    .expect("inbound accept");
    assert_eq!(response.status, "202 Accepted", "{}", response.body);

    let response = super::route_http_request("GET", "/actors/music/followers", None, "", &state)
        .await
        .expect("followers collection");
    let followers = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
    assert_eq!(followers["totalItems"], 1);
    assert_eq!(followers["orderedItems"][0], follower_one);

    let response = super::route_http_request("GET", "/actors/music/following", None, "", &state)
        .await
        .expect("following collection");
    let following = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
    assert_eq!(following["totalItems"], 2);
    assert!(following["orderedItems"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item == following_one.as_str()));
    assert!(following["orderedItems"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item == follower_two.as_str()));

    let (body, headers) = signed_inbox_post(
        "follower-two",
        serde_json::json!({
            "id": "reject-two",
            "type": "Reject",
            "actor": follower_two,
            "object": {"type": "Follow", "object": "https://social.example/actors/music"}
        }),
    );
    let response = super::route_http_request_with_headers(
        "POST",
        "/actors/music/inbox",
        None,
        &body,
        &state,
        headers,
    )
    .await
    .expect("inbound reject");
    assert_eq!(response.status, "202 Accepted", "{}", response.body);

    let (body, headers) = signed_inbox_post(
        "follower-two",
        serde_json::json!({
            "id": "follow-three",
            "type": "Follow",
            "actor": follower_two,
            "object": "https://social.example/actors/music"
        }),
    );
    let response = super::route_http_request_with_headers(
        "POST",
        "/actors/music/inbox",
        None,
        &body,
        &state,
        headers,
    )
    .await
    .expect("second inbound follow");
    assert_eq!(response.status, "202 Accepted", "{}", response.body);

    let response = super::route_http_request(
        "POST",
        "/actors/music/outbox",
        None,
        &serde_json::json!({
            "id": "follow-three-outbound",
            "type": "Follow",
            "object": follower_two
        })
        .to_string(),
        &state,
    )
    .await
    .expect("second outbound follow");
    assert_eq!(response.status, "200 OK");

    let (body, headers) = signed_inbox_post(
        "follower-two",
        serde_json::json!({
            "id": "remove-three",
            "type": "Remove",
            "actor": follower_two,
            "object": {"type": "Follow", "actor": follower_two}
        }),
    );
    let response = super::route_http_request_with_headers(
        "POST",
        "/actors/music/inbox",
        None,
        &body,
        &state,
        headers,
    )
    .await
    .expect("inbound remove");
    assert_eq!(response.status, "202 Accepted", "{}", response.body);

    let (body, headers) = signed_inbox_post(
        "follower-one",
        serde_json::json!({
            "id": "undo-one",
            "type": "Undo",
            "actor": follower_one,
            "object": {"type": "Follow", "actor": follower_one}
        }),
    );
    let response = super::route_http_request_with_headers(
        "POST",
        "/actors/music/inbox",
        None,
        &body,
        &state,
        headers,
    )
    .await
    .expect("inbound undo");
    assert_eq!(response.status, "202 Accepted", "{}", response.body);

    let response = super::route_http_request(
        "POST",
        "/actors/music/outbox",
        None,
        &serde_json::json!({
            "id": "undo-one-outbound",
            "type": "Undo",
            "object": {"type": "Follow", "object": following_one}
        })
        .to_string(),
        &state,
    )
    .await
    .expect("outbound undo");
    assert_eq!(response.status, "200 OK");

    for (path, expected) in [
        ("/actors/music/followers", 0),
        ("/actors/music/following", 0),
    ] {
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .expect("empty relationship collection");
        let collection = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
        assert_eq!(collection["totalItems"], expected, "{path}");
        assert!(collection["orderedItems"].as_array().unwrap().is_empty());
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn actors_routes_are_gated_by_the_social_federation_feature_flag() {
    // Matches the oracle's real [FeatureGate(FeatureId.SocialFederation)]
    // on ActivityPubController, which 404s every action (actor,
    // inbox, outbox, followers, following) when the feature is
    // disabled. The feature-disabled check previously only covered
    // "/api/federation"/"/api/activitypub"/webfinger -- "/actors/"
    // itself was never included, so disabling the feature left the
    // actor/inbox/outbox/followers/following routes fully reachable.
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));
    state
        .media_services
        .write()
        .await
        .features
        .social_federation = false;

    for path in [
        "/actors/library",
        "/actors/library/inbox",
        "/actors/library/outbox",
        "/actors/library/followers",
        "/actors/library/following",
        "/api/federation/diagnostics",
    ] {
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        assert_eq!(response.status, "404 Not Found", "{path}");
    }

    // Matches the oracle's real [FeatureGate(FeatureId.SocialFederation)]
    // on TasteRecommendationsController, a sibling federation surface
    // that was previously missing from this same gate.
    for path in [
        "/api/v0/taste-recommendations/wishlist",
        "/api/v0/taste-recommendations/release-radar",
        "/api/v0/taste-recommendations/graph-preview",
    ] {
        let response = super::route_http_request("POST", path, None, "{}", &state)
            .await
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        assert_eq!(response.status, "404 Not Found", "{path}");
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn playback_feedback_and_diagnostics_reflect_the_real_buffer_state() {
    // Matches the oracle's real PlaybackController: priority is
    // computed from PlaybackPriorityService.GetPriority (High when
    // buffer < 5s, Low when >= 30s, Mid otherwise), and diagnostics
    // reflects the most recently posted feedback for that job, not
    // multisource swarm-download progress.
    let (state, _receiver) = test_state();

    let missing = super::route_http_request(
        "GET",
        "/api/v0/playback/track-audit/diagnostics",
        None,
        "",
        &state,
    )
    .await
    .expect("missing playback diagnostics");
    assert_eq!(missing.status, "404 Not Found");

    let low_buffer = super::route_http_request(
        "POST",
        "/api/v0/playback/feedback",
        None,
        r#"{"jobId":"track-audit","trackId":"track-1","positionMs":1000,"bufferAheadMs":500}"#,
        &state,
    )
    .await
    .expect("low-buffer feedback");
    assert_eq!(low_buffer.status, "200 OK");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&low_buffer.body).unwrap()["priority"],
        "High"
    );

    let diagnostics = super::route_http_request(
        "GET",
        "/api/v0/playback/track-audit/diagnostics",
        None,
        "",
        &state,
    )
    .await
    .expect("playback diagnostics");
    assert_eq!(diagnostics.status, "200 OK");
    let diagnostics_json = serde_json::from_str::<serde_json::Value>(&diagnostics.body).unwrap();
    assert_eq!(diagnostics_json["jobId"], "track-audit");
    assert_eq!(diagnostics_json["trackId"], "track-1");
    assert_eq!(diagnostics_json["positionMs"], 1_000);
    assert_eq!(diagnostics_json["bufferAheadMs"], 500);
    assert_eq!(diagnostics_json["priority"], "High");

    let comfortable = super::route_http_request(
        "POST",
        "/api/v0/playback/feedback",
        None,
        r#"{"jobId":"track-audit","trackId":"track-1","positionMs":5000,"bufferAheadMs":45000}"#,
        &state,
    )
    .await
    .expect("comfortable-buffer feedback");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&comfortable.body).unwrap()["priority"],
        "Low"
    );

    let updated = super::route_http_request(
        "GET",
        "/api/v0/playback/track-audit/diagnostics",
        None,
        "",
        &state,
    )
    .await
    .expect("updated playback diagnostics");
    let updated_json = serde_json::from_str::<serde_json::Value>(&updated.body).unwrap();
    assert_eq!(updated_json["positionMs"], 5_000);
    assert_eq!(updated_json["bufferAheadMs"], 45_000);
    assert_eq!(updated_json["priority"], "Low");

    let mid = super::route_http_request(
        "POST",
        "/api/v0/playback/feedback",
        None,
        r#"{"jobId":"track-audit","positionMs":6000,"bufferAheadMs":15000}"#,
        &state,
    )
    .await
    .expect("mid-buffer feedback");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&mid.body).unwrap()["priority"],
        "Mid"
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn musicbrainz_overlay_export_review_and_approval_match_real_oracle_gating() {
    // Matches the oracle's real MusicBrainzOverlayService: export
    // review/approval reflects the edit's real exportable-type gate
    // (IsExportableEdit) and validates a real opaque approvedBy and
    // note length, rather than always approving with an invented
    // {editId, isApproved:true} shape.
    let (state, _receiver) = test_state();

    let non_exportable = super::route_http_request(
        "POST",
        "/api/v0/musicbrainz/overlays/edits",
        None,
        r#"{"type":"Other","targetType":"Recording","targetId":"rec-1","field":"title","value":"New Title","evidence":[{"type":"WorkRef","reference":"opaque-ref"}]}"#,
        &state,
    )
    .await
    .expect("create non-exportable edit");
    assert_eq!(non_exportable.status, "200 OK", "{}", non_exportable.body);
    let non_exportable_id = serde_json::from_str::<serde_json::Value>(&non_exportable.body)
        .unwrap()["edit"]["editId"]
        .as_str()
        .unwrap()
        .to_owned();

    let review = super::route_http_request(
        "GET",
        &format!("/api/v0/musicbrainz/overlays/edits/{non_exportable_id}/export-review"),
        None,
        "",
        &state,
    )
    .await
    .expect("export review");
    assert_eq!(review.status, "200 OK", "{}", review.body);
    let review_json = serde_json::from_str::<serde_json::Value>(&review.body).unwrap();
    assert_eq!(review_json["upstreamTarget"], "Recording:rec-1");
    assert_eq!(review_json["proposedChange"], "title => New Title");
    assert_eq!(review_json["canApproveExport"], false);
    assert_eq!(
        review_json["reviewReason"],
        "Overlay edit type is not exportable."
    );
    assert!(review_json["decision"].is_null());

    let rejected_approval = super::route_http_request(
        "POST",
        &format!("/api/v0/musicbrainz/overlays/edits/{non_exportable_id}/approve-export"),
        None,
        "{}",
        &state,
    )
    .await
    .expect("reject non-exportable approval");
    assert_eq!(rejected_approval.status, "400 Bad Request");
    let rejected_json = serde_json::from_str::<serde_json::Value>(&rejected_approval.body).unwrap();
    assert!(
        rejected_json["errors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|error| error == "Overlay edit type is not exportable."),
        "{rejected_json}"
    );
    assert!(rejected_json["decision"].is_null());

    // An unsafe approvedBy identifier must be rejected even for an
    // otherwise-exportable edit type.
    let unsafe_edit = super::route_http_request(
        "POST",
        "/api/v0/musicbrainz/overlays/edits",
        None,
        r#"{"type":"TitleCorrection","targetType":"Recording","targetId":"rec-2","field":"title","value":"Corrected","evidence":[{"type":"WorkRef","reference":"opaque-ref"}]}"#,
        &state,
    )
    .await
    .expect("create exportable edit for unsafe-approver check");
    let unsafe_edit_id = serde_json::from_str::<serde_json::Value>(&unsafe_edit.body).unwrap()
        ["edit"]["editId"]
        .as_str()
        .unwrap()
        .to_owned();
    let unsafe_approval = super::route_http_request(
        "POST",
        &format!("/api/v0/musicbrainz/overlays/edits/{unsafe_edit_id}/approve-export"),
        None,
        r#"{"approvedBy":"/etc/passwd"}"#,
        &state,
    )
    .await
    .expect("reject unsafe approvedBy");
    assert_eq!(unsafe_approval.status, "400 Bad Request");
    assert!(
        serde_json::from_str::<serde_json::Value>(&unsafe_approval.body).unwrap()["errors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|error| error == "Approved-by identifier must be opaque and safe."),
        "{}",
        unsafe_approval.body
    );

    // A real, exportable edit is approved for real, and approving it
    // again is idempotent -- it returns the original decision
    // unchanged rather than re-validating or overwriting it.
    let exportable = super::route_http_request(
        "POST",
        "/api/v0/musicbrainz/overlays/edits",
        None,
        r#"{"type":"TitleCorrection","targetType":"Recording","targetId":"rec-3","field":"title","value":"Corrected Title","evidence":[{"type":"WorkRef","reference":"opaque-ref"}]}"#,
        &state,
    )
    .await
    .expect("create exportable edit");
    let exportable_id = serde_json::from_str::<serde_json::Value>(&exportable.body).unwrap()
        ["edit"]["editId"]
        .as_str()
        .unwrap()
        .to_owned();

    let approval = super::route_http_request(
        "POST",
        &format!("/api/v0/musicbrainz/overlays/edits/{exportable_id}/approve-export"),
        None,
        r#"{"approvedBy":"reviewer-1","note":"looks good"}"#,
        &state,
    )
    .await
    .expect("approve exportable edit");
    assert_eq!(approval.status, "200 OK", "{}", approval.body);
    let approval_json = serde_json::from_str::<serde_json::Value>(&approval.body).unwrap();
    assert_eq!(approval_json["errors"], serde_json::json!([]));
    assert_eq!(approval_json["decision"]["approvedBy"], "reviewer-1");
    assert_eq!(approval_json["decision"]["note"], "looks good");
    assert_eq!(approval_json["decision"]["editId"], exportable_id);
    assert_eq!(
        approval_json["decision"]["upstreamTarget"],
        "Recording:rec-3"
    );
    assert_eq!(
        approval_json["decision"]["proposedChange"],
        "title => Corrected Title"
    );
    assert!(approval_json["decision"]["id"]
        .as_str()
        .unwrap()
        .starts_with("musicbrainz-overlay-export:"));

    let post_approval_review = super::route_http_request(
        "GET",
        &format!("/api/v0/musicbrainz/overlays/edits/{exportable_id}/export-review"),
        None,
        "",
        &state,
    )
    .await
    .expect("export review after approval");
    let post_approval_review_json =
        serde_json::from_str::<serde_json::Value>(&post_approval_review.body).unwrap();
    assert_eq!(post_approval_review_json["canApproveExport"], false);
    assert_eq!(
        post_approval_review_json["reviewReason"],
        "Upstream export has already been approved locally."
    );
    assert_eq!(
        post_approval_review_json["decision"]["approvedBy"],
        "reviewer-1"
    );

    let repeat_approval = super::route_http_request(
        "POST",
        &format!("/api/v0/musicbrainz/overlays/edits/{exportable_id}/approve-export"),
        None,
        r#"{"approvedBy":"different-reviewer"}"#,
        &state,
    )
    .await
    .expect("idempotent re-approval");
    assert_eq!(repeat_approval.status, "200 OK");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&repeat_approval.body).unwrap()["decision"]
            ["approvedBy"],
        "reviewer-1",
        "re-approval must not overwrite the original decision"
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn extended_controller_mutations_are_stateful_and_domain_backed() {
    let (state, _receiver) = test_state();

    let opinion = super::route_http_request(
        "POST",
        "/api/v0/opinions",
        None,
        r#"{"issuer":"stateful-test","subjectType":"Track","subjectId":"recording-1","kind":"Like","strength":0.75,"confidence":1,"comment":"good"}"#,
        &state,
    )
    .await
    .expect("create opinion");
    assert_eq!(opinion.status, "200 OK");
    let opinion_json = serde_json::from_str::<serde_json::Value>(&opinion.body).unwrap();
    let opinion_id = opinion_json["id"].as_str().unwrap();
    let opinions = super::route_http_request("GET", "/api/v0/opinions", None, "", &state)
        .await
        .expect("list opinions");
    assert!(opinions.body.contains("recording-1"));

    let circuit = super::route_http_request(
        "POST",
        "/api/v0/security/circuits",
        None,
        r#"{"id":"circuit-1","peerId":"peer-1","active":true}"#,
        &state,
    )
    .await
    .expect("create circuit");
    assert_eq!(circuit.status, "400 Bad Request");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&circuit.body).unwrap(),
        serde_json::json!({"error": "Circuit building failed"})
    );
    let circuits = super::route_http_request("GET", "/api/v0/security/circuits", None, "", &state)
        .await
        .expect("list circuits");
    assert_eq!(circuits.body, "[]");

    let descriptor = super::route_http_request(
        "POST",
        "/api/v0/mediacore/publish/descriptor",
        None,
        &serde_json::json!({
            "descriptor": {
                "contentId": "cid-1",
                "hashes": [{"algorithm": "sha256", "hex": "0123456789abcdef"}],
                "signature": {
                    "publicKey": "key",
                    "signature": "0123456789abcdef",
                    "timestampUnixMs": super::unix_timestamp_millis(),
                },
            },
        })
        .to_string(),
        &state,
    )
    .await
    .expect("publish descriptor");
    assert_eq!(descriptor.status, "200 OK");
    let descriptor_stats = super::route_http_request(
        "GET",
        "/api/v0/mediacore/stats/descriptors",
        None,
        "",
        &state,
    )
    .await
    .expect("descriptor stats");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&descriptor_stats.body).unwrap()
            ["activeCacheEntries"],
        0
    );

    let created_pod = super::route_http_request(
        "POST",
        "/api/v0/pods",
        None,
        r#"{"pod":{"podId":"pod-controller","name":"Controller Pod","isPublic":true}}"#,
        &state,
    )
    .await
    .expect("create pod");
    assert_eq!(created_pod.status, "201 Created");
    let channel = super::route_http_request(
        "POST",
        "/api/v0/podcore/pod-controller/channels",
        None,
        r#"{"channelId":"general","name":"General"}"#,
        &state,
    )
    .await
    .expect("create pod channel");
    assert_eq!(channel.status, "201 Created");
    let channels = super::route_http_request(
        "GET",
        "/api/v0/podcore/pod-controller/channels",
        None,
        "",
        &state,
    )
    .await
    .expect("list pod channels");
    assert!(channels.body.contains("general"));

    let keypair = super::route_http_request(
        "POST",
        "/api/v0/podcore/signing/generate-keypair",
        None,
        "{}",
        &state,
    )
    .await
    .expect("generate pod signing keypair");
    let keys = serde_json::from_str::<serde_json::Value>(&keypair.body).unwrap();
    // Verification resolves the sender's public key from real pod
    // membership, matching the oracle -- not from a client-supplied
    // field, which would let anyone "verify" a self-made signature
    // against a self-made key. Register "tester" as a real member
    // with the generated public key so verification has a real key
    // to check against.
    state
        .pods
        .write()
        .await
        .upsert_member(
            "pod-controller",
            super::pods::PodMember {
                peer_id: "tester".to_owned(),
                role: "member".to_owned(),
                is_banned: false,
                public_key: keys["publicKey"].as_str().map(str::to_owned),
                joined_at: None,
                last_seen: None,
            },
        )
        .expect("add tester as a real pod member with a signing public key");
    let signed = super::route_http_request(
        "POST",
        "/api/v0/podcore/signing/sign",
        None,
        &serde_json::json!({
            "privateKey": keys["privateKey"],
            "message": {
                "messageId":"message-1",
                "podId":"pod-controller",
                "senderPeerId":"tester",
                "body":"hello",
                "timestampUnixMs": super::unix_timestamp() * 1000,
            }
        })
        .to_string(),
        &state,
    )
    .await
    .expect("sign pod message");
    assert_eq!(signed.status, "200 OK");
    let signed_json = serde_json::from_str::<serde_json::Value>(&signed.body).unwrap();
    assert!(
        signed_json["signature"]
            .as_str()
            .unwrap()
            .starts_with("ed25519:"),
        "{signed_json}"
    );
    let verified = super::route_http_request(
        "POST",
        "/api/v0/podcore/signing/verify",
        None,
        &signed.body,
        &state,
    )
    .await
    .expect("verify pod message");
    assert_eq!(verified.body, r#"{"isValid":true}"#, "{}", verified.body);

    // A signature that doesn't match the sender's real registered
    // public key must fail -- not a fake "isValid: true" for whatever
    // key the caller happens to supply.
    let mut forged = signed_json.clone();
    forged["message"]["senderPeerId"] = serde_json::json!("someone-else");
    let forged_verified = super::route_http_request(
        "POST",
        "/api/v0/podcore/signing/verify",
        None,
        &forged.to_string(),
        &state,
    )
    .await
    .expect("verify forged sender");
    assert_eq!(forged_verified.body, r#"{"isValid":false}"#);

    let ranked = super::route_http_request(
        "POST",
        "/api/v0/ranking/rank",
        None,
        r#"[{"username":"slow","filename":"x.flac","uploadSpeed":10},{"username":"fast","filename":"x.flac","uploadSpeed":10000,"hasFreeUploadSlot":true}]"#,
        &state,
    )
    .await
    .expect("rank sources");
    let ranked_json = serde_json::from_str::<serde_json::Value>(&ranked.body).unwrap();
    assert_eq!(ranked_json[0]["username"], "fast");

    let removed_opinion = super::route_http_request(
        "DELETE",
        &format!("/api/v0/opinions/{opinion_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("remove opinion");
    assert_eq!(removed_opinion.status, "204 No Content");
    let removed_descriptor = super::route_http_request(
        "DELETE",
        "/api/v0/mediacore/publish/descriptor/cid-1",
        None,
        "",
        &state,
    )
    .await
    .expect("remove descriptor");
    assert_eq!(removed_descriptor.status, "200 OK");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&removed_descriptor.body).unwrap()
            ["wasPublished"],
        true
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn pod_signature_verification_fails_for_a_sender_with_no_registered_key() {
    let (state, _receiver) = test_state();
    let pod_id = "pod:00000000000000000000000000000ba08";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<super::pods::PodRecord>(serde_json::json!({
                "podId": pod_id,
                "name": "Signing Audit",
            }))
            .expect("deserialize pod record fixture"),
            "owner-peer".to_owned(),
        )
        .expect("create pod");
    // "tester" (the fixture's default configured Soulseek identity,
    // required by the sign endpoint's own auth check) is a real pod
    // member, but has never registered a public key -- a well-formed
    // signature from them must still be rejected, since there is
    // nothing real to verify it against.
    state
        .pods
        .write()
        .await
        .upsert_member(
            pod_id,
            super::pods::PodMember {
                peer_id: "tester".to_owned(),
                role: "member".to_owned(),
                is_banned: false,
                public_key: None,
                joined_at: None,
                last_seen: None,
            },
        )
        .expect("add member with no public key");

    let keypair = super::route_http_request(
        "POST",
        "/api/v0/podcore/signing/generate-keypair",
        None,
        "{}",
        &state,
    )
    .await
    .expect("generate keypair");
    let keys = serde_json::from_str::<serde_json::Value>(&keypair.body).unwrap();
    let signed = super::route_http_request(
        "POST",
        "/api/v0/podcore/signing/sign",
        None,
        &serde_json::json!({
            "privateKey": keys["privateKey"],
            "message": {
                "messageId": "message-1",
                "podId": pod_id,
                "senderPeerId": "tester",
                "body": "hello",
                "timestampUnixMs": super::unix_timestamp() * 1000,
            }
        })
        .to_string(),
        &state,
    )
    .await
    .expect("sign message");
    assert_eq!(signed.status, "200 OK");

    let verified = super::route_http_request(
        "POST",
        "/api/v0/podcore/signing/verify",
        None,
        &signed.body,
        &state,
    )
    .await
    .expect("verify message");
    assert_eq!(verified.body, r#"{"isValid":false}"#);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn pod_signing_stats_reflect_real_activity_not_hardcoded_zeros() {
    let (state, _receiver) = test_state();
    let baseline =
        super::route_http_request("GET", "/api/v0/podcore/signing/stats", None, "", &state)
            .await
            .expect("baseline signing stats");
    let baseline_json = serde_json::from_str::<serde_json::Value>(&baseline.body).unwrap();
    assert_eq!(baseline_json["totalSignaturesCreated"], 0);
    assert_eq!(baseline_json["totalSignaturesVerified"], 0);
    assert_eq!(
        baseline_json["lastSignatureOperation"],
        super::PODCORE_MIN_DATETIME
    );

    let pod_id = "pod:00000000000000000000000000000ba09";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<super::pods::PodRecord>(serde_json::json!({
                "podId": pod_id,
                "name": "Signing Stats Audit",
            }))
            .expect("deserialize pod record fixture"),
            "owner-peer".to_owned(),
        )
        .expect("create pod");

    let keypair = super::route_http_request(
        "POST",
        "/api/v0/podcore/signing/generate-keypair",
        None,
        "{}",
        &state,
    )
    .await
    .expect("generate keypair");
    let keys = serde_json::from_str::<serde_json::Value>(&keypair.body).unwrap();
    state
        .pods
        .write()
        .await
        .upsert_member(
            pod_id,
            super::pods::PodMember {
                peer_id: "tester".to_owned(),
                role: "member".to_owned(),
                is_banned: false,
                public_key: keys["publicKey"].as_str().map(str::to_owned),
                joined_at: None,
                last_seen: None,
            },
        )
        .expect("add tester as a real pod member with a signing public key");

    let signed = super::route_http_request(
        "POST",
        "/api/v0/podcore/signing/sign",
        None,
        &serde_json::json!({
            "privateKey": keys["privateKey"],
            "message": {
                "messageId": "message-1",
                "podId": pod_id,
                "senderPeerId": "tester",
                "body": "hello",
                "timestampUnixMs": super::unix_timestamp() * 1000,
            }
        })
        .to_string(),
        &state,
    )
    .await
    .expect("sign message");
    assert_eq!(signed.status, "200 OK");

    let verified = super::route_http_request(
        "POST",
        "/api/v0/podcore/signing/verify",
        None,
        &signed.body,
        &state,
    )
    .await
    .expect("verify message");
    assert_eq!(verified.body, r#"{"isValid":true}"#);

    // A forged sender fails verification -- this must also count as a
    // real (failed) verification, not be silently dropped from stats.
    let signed_json = serde_json::from_str::<serde_json::Value>(&signed.body).unwrap();
    let mut forged = signed_json.clone();
    forged["message"]["senderPeerId"] = serde_json::json!("someone-else");
    let forged_verified = super::route_http_request(
        "POST",
        "/api/v0/podcore/signing/verify",
        None,
        &forged.to_string(),
        &state,
    )
    .await
    .expect("verify forged sender");
    assert_eq!(forged_verified.body, r#"{"isValid":false}"#);

    let stats = super::route_http_request("GET", "/api/v0/podcore/signing/stats", None, "", &state)
        .await
        .expect("signing stats");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap();
    assert_eq!(stats_json["totalSignaturesCreated"], 1, "{stats_json}");
    assert_eq!(stats_json["totalSignaturesVerified"], 2, "{stats_json}");
    assert_eq!(stats_json["successfulVerifications"], 1, "{stats_json}");
    assert_eq!(stats_json["failedVerifications"], 1, "{stats_json}");
    assert!(
        stats_json["lastSignatureOperation"].is_string(),
        "{stats_json}"
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn pod_verification_message_checks_real_membership_and_signature() {
    let (state, _receiver) = test_state();
    // Unlike other pod fixtures in this suite, this podId deliberately
    // contains no colon: verification/message derives the membership
    // podId by splitting channelId on the first ':' as "podId:channel",
    // a convention that wouldn't round-trip through this test if the
    // podId itself contained a colon.
    let pod_id = "verification-audit-pod";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<super::pods::PodRecord>(serde_json::json!({
                "podId": pod_id,
                "name": "Verification Audit",
            }))
            .expect("deserialize pod record fixture"),
            "owner-peer".to_owned(),
        )
        .expect("create pod");

    let keypair = super::route_http_request(
        "POST",
        "/api/v0/podcore/signing/generate-keypair",
        None,
        "{}",
        &state,
    )
    .await
    .expect("generate keypair");
    let keys = serde_json::from_str::<serde_json::Value>(&keypair.body).unwrap();
    state
        .pods
        .write()
        .await
        .upsert_member(
            pod_id,
            super::pods::PodMember {
                peer_id: "tester".to_owned(),
                role: "member".to_owned(),
                is_banned: false,
                public_key: keys["publicKey"].as_str().map(str::to_owned),
                joined_at: None,
                last_seen: None,
            },
        )
        .expect("add tester as a real pod member with a signing public key");

    let message = serde_json::json!({
        "messageId": "message-1",
        "podId": pod_id,
        "channelId": format!("{pod_id}:general"),
        "senderPeerId": "tester",
        "body": "hello",
        "timestampUnixMs": super::unix_timestamp() * 1000,
    });
    let signed = super::route_http_request(
        "POST",
        "/api/v0/podcore/signing/sign",
        None,
        &serde_json::json!({"privateKey": keys["privateKey"], "message": message}).to_string(),
        &state,
    )
    .await
    .expect("sign message");
    assert_eq!(signed.status, "200 OK");
    let signed_json = serde_json::from_str::<serde_json::Value>(&signed.body).unwrap();
    let mut verified_message = message.clone();
    verified_message["signature"] = signed_json["signature"].clone();

    let verified = super::route_http_request(
        "POST",
        "/api/v0/podcore/verification/message",
        None,
        &verified_message.to_string(),
        &state,
    )
    .await
    .expect("verify message");
    assert_eq!(verified.status, "200 OK");
    let verified_json = serde_json::from_str::<serde_json::Value>(&verified.body).unwrap();
    assert_eq!(
        verified_json,
        serde_json::json!({
            "isValid": true,
            "isFromValidMember": true,
            "hasValidSignature": true,
            "isNotBanned": true,
        }),
        "{verified_json}"
    );

    // A sender who was never registered as a real pod member fails
    // the membership check, even with a structurally valid message.
    let mut unknown_sender = verified_message.clone();
    unknown_sender["senderPeerId"] = serde_json::json!("stranger");
    let unknown_verified = super::route_http_request(
        "POST",
        "/api/v0/podcore/verification/message",
        None,
        &unknown_sender.to_string(),
        &state,
    )
    .await
    .expect("verify unknown sender");
    let unknown_json = serde_json::from_str::<serde_json::Value>(&unknown_verified.body).unwrap();
    assert_eq!(unknown_json["isValid"], false, "{unknown_json}");
    assert_eq!(unknown_json["isFromValidMember"], false, "{unknown_json}");
    assert_eq!(unknown_json["isNotBanned"], true, "{unknown_json}");

    // A channelId with no "podId:channel" separator is a structurally
    // invalid request, reported honestly rather than treated as some
    // other kind of failure.
    let mut bad_channel = verified_message.clone();
    bad_channel["channelId"] = serde_json::json!("no-colon-here");
    let bad_channel_verified = super::route_http_request(
        "POST",
        "/api/v0/podcore/verification/message",
        None,
        &bad_channel.to_string(),
        &state,
    )
    .await
    .expect("verify bad channel id");
    assert_eq!(
        bad_channel_verified.body,
        serde_json::json!({
            "isValid": false,
            "isFromValidMember": false,
            "hasValidSignature": false,
            "isNotBanned": false,
            "errorMessage": "Invalid channel ID format",
        })
        .to_string()
    );

    // Missing required fields is a real 400, not a silently-accepted
    // always-false result.
    let missing_pod_id = super::route_http_request(
        "POST",
        "/api/v0/podcore/verification/message",
        None,
        r#"{"messageId":"message-1"}"#,
        &state,
    )
    .await
    .expect("verify missing podId");
    assert_eq!(missing_pod_id.status, "400 Bad Request");

    let stats = super::route_http_request(
        "GET",
        "/api/v0/podcore/verification/stats",
        None,
        "",
        &state,
    )
    .await
    .expect("verification stats");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap();
    // Only 2 real verification attempts are recorded: the successful
    // one and the unknown-sender failure. Matching the oracle exactly,
    // the invalid-channel-format and missing-field cases never reach
    // the verifier (they fail before any membership/signature check
    // runs), so neither counts as a verification attempt.
    assert_eq!(stats_json["totalVerifications"], 2, "{stats_json}");
    assert_eq!(stats_json["successfulVerifications"], 1, "{stats_json}");
    assert_eq!(stats_json["failedMembershipChecks"], 1, "{stats_json}");
    assert!(stats_json["lastVerification"].is_string(), "{stats_json}");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn controller_feature_state_persists_bounded_records() {
    let root = std::env::temp_dir().join(format!(
        "slskr-controller-feature-state-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let mut state = super::ControllerFeatureState::load(&root).unwrap();
    state
        .upsert(
            "opinion/one".to_owned(),
            serde_json::json!({"id":"one","score":1.0}),
        )
        .unwrap();
    drop(state);
    let reloaded = super::ControllerFeatureState::load(&root).unwrap();
    assert_eq!(reloaded.get("opinion/one").unwrap()["score"], 1.0);
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn source_discovery_routes_dispatch_and_project_bounded_search_sources() {
    let (state, mut receiver) = test_state();
    let started = super::route_http_request(
        "POST",
        "/api/v0/discovery/start",
        None,
        r#"{"searchTerm":"rare recording","enableHashVerification":true}"#,
        &state,
    )
    .await
    .expect("start source discovery");
    assert_eq!(started.status, "200 OK");
    let token = match receiver.recv().await.expect("discovery search command") {
        super::SessionCommand::Search {
            token,
            query,
            target: super::SearchDispatchTarget::Global,
        } => {
            assert_eq!(query, "rare recording");
            token
        }
        command => panic!("unexpected command: {command:?}"),
    };

    let conflict = super::route_http_request(
        "POST",
        "/api/v0/discovery/start",
        None,
        r#"{"searchTerm":"second"}"#,
        &state,
    )
    .await
    .expect("reject overlapping discovery");
    assert_eq!(conflict.status, "409 Conflict");

    {
        let mut searches = state.searches.write().await;
        let record = searches
            .records
            .iter_mut()
            .find(|record| record.token == token)
            .expect("discovery search record");
        record.results.push(super::SearchResultEntry {
            peer_username: Some("source-peer".to_owned()),
            filename: "Rare/Recording.flac".to_owned(),
            size: 42,
            extension: "flac".to_owned(),
            bit_rate: None,
            sample_rate: None,
            bit_depth: None,
            length_seconds: None,
            locked: false,
            slot_free: Some(true),
            average_speed: Some(1234),
            queue_length: Some(0),
        });
    }

    let status = super::route_http_request("GET", "/api/v0/discovery", None, "", &state)
        .await
        .expect("source discovery status");
    let status_json = serde_json::from_str::<serde_json::Value>(&status.body).unwrap();
    assert_eq!(status_json["isRunning"], true);
    assert_eq!(status_json["stats"]["totalFiles"], 1);
    assert_eq!(status_json["stats"]["totalUsers"], 1);
    assert_eq!(status_json["stats"]["searchCycles"], 1);

    let by_size = super::route_http_request(
        "GET",
        "/api/v0/discovery/sources/by-size/42?limit=10",
        None,
        "",
        &state,
    )
    .await
    .expect("sources by size");
    let by_size_json = serde_json::from_str::<serde_json::Value>(&by_size.body).unwrap();
    assert_eq!(by_size_json["sourceCount"], 1);
    assert_eq!(by_size_json["sources"][0]["username"], "source-peer");

    let by_name = super::route_http_request(
        "GET",
        "/api/v0/discovery/sources/by-filename?pattern=recording&limit=10",
        None,
        "",
        &state,
    )
    .await
    .expect("sources by filename");
    assert!(by_name.body.contains("Rare/Recording.flac"));

    let summaries = super::route_http_request(
        "GET",
        "/api/v0/discovery/summaries?minSources=1",
        None,
        "",
        &state,
    )
    .await
    .expect("source summaries");
    let summaries_json = serde_json::from_str::<serde_json::Value>(&summaries.body).unwrap();
    assert_eq!(summaries_json["summaries"][0]["size"], 42);

    let stopped = super::route_http_request("POST", "/api/v0/discovery/stop", None, "{}", &state)
        .await
        .expect("stop source discovery");
    let stopped_json = serde_json::from_str::<serde_json::Value>(&stopped.body).unwrap();
    assert_eq!(stopped_json["stats"]["lastCycleNewFiles"], 1);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn source_discovery_generation_ignores_stale_completion() {
    let mut discovery = super::SourceDiscoveryState::default();
    let first = discovery
        .begin_start("first".to_owned(), true)
        .expect("first start reservation");
    assert!(discovery.is_running());
    assert!(discovery.begin_start("second".to_owned(), false).is_none());

    assert!(discovery.stop());
    let second = discovery
        .begin_start("second".to_owned(), false)
        .expect("second start reservation");
    assert_ne!(first, second);
    assert!(!discovery.finish_start(first, 1));
    assert!(!discovery.record_dispatch_if_current(first, 2));
    assert!(discovery.finish_start(second, 3));
    assert!(discovery.running);
    assert_eq!(discovery.search_term, "second");
    assert_eq!(discovery.search_tokens.as_slices().0, &[3]);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn virtual_soulfind_v2_routes_execute_bounded_local_intent_workflow() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_VIRTUAL_SOULFIND_V2_ENABLED", "true"));
    state.library.write().await.create(
        "Known Artist".to_owned(),
        "Known Track".to_owned(),
        "Album".to_owned(),
    );

    let artists = super::route_http_request(
        "GET",
        "/api/v1/virtualsoulfind/v2/catalogue/artists/search?query=known&limit=10",
        None,
        "",
        &state,
    )
    .await
    .expect("search v2 artists");
    assert_eq!(artists.status, "200 OK");
    let artists_json = serde_json::from_str::<serde_json::Value>(&artists.body).unwrap();
    let artist_id = artists_json[0]["artistId"].as_str().unwrap();

    let releases = super::route_http_request(
        "GET",
        &format!("/api/v1/virtualsoulfind/v2/catalogue/artists/{artist_id}/releases"),
        None,
        "",
        &state,
    )
    .await
    .expect("list v2 releases");
    let releases_json = serde_json::from_str::<serde_json::Value>(&releases.body).unwrap();
    let release_id = releases_json[0]["releaseGroupId"].as_str().unwrap();

    let tracks = super::route_http_request(
        "GET",
        &format!("/api/v1/virtualsoulfind/v2/catalogue/releases/{release_id}/tracks"),
        None,
        "",
        &state,
    )
    .await
    .expect("list v2 tracks");
    let tracks_json = serde_json::from_str::<serde_json::Value>(&tracks.body).unwrap();
    let track_id = tracks_json[0]["trackId"].as_str().unwrap();

    let plan = super::route_http_request(
        "POST",
        "/api/v1/virtualsoulfind/v2/plans",
        None,
        &serde_json::json!({ "domain": "Music", "trackId": track_id }).to_string(),
        &state,
    )
    .await
    .expect("create v2 plan");
    let plan_json = serde_json::from_str::<serde_json::Value>(&plan.body).unwrap();
    assert_eq!(plan_json["status"], "Ready");
    assert_eq!(plan_json["steps"][0]["backend"], "LocalLibrary");

    let created = super::route_http_request(
        "POST",
        "/api/v1/virtualsoulfind/v2/intents/tracks",
        None,
        &serde_json::json!({
            "domain": "Music",
            "trackId": track_id,
            "priority": "High",
        })
        .to_string(),
        &state,
    )
    .await
    .expect("create v2 intent");
    assert_eq!(created.status, "201 Created");
    let created_json = serde_json::from_str::<serde_json::Value>(&created.body).unwrap();
    let intent_id = created_json["desiredTrackId"].as_str().unwrap();

    let processing = super::route_http_request(
        "POST",
        &format!("/api/v1/virtualsoulfind/v2/intents/tracks/{intent_id}/process"),
        None,
        "",
        &state,
    )
    .await
    .expect("process v2 intent");
    assert_eq!(processing.status, "202 Accepted");
    tokio::task::yield_now().await;

    let intent = super::route_http_request(
        "GET",
        &format!("/api/v1/virtualsoulfind/v2/intents/tracks/{intent_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("get processed v2 intent");
    let intent_json = serde_json::from_str::<serde_json::Value>(&intent.body).unwrap();
    assert_eq!(intent_json["status"], "Completed");

    let stats =
        super::route_http_request("GET", "/api/v1/virtualsoulfind/v2/stats", None, "", &state)
            .await
            .expect("get v2 stats");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap();
    assert_eq!(stats_json["totalProcessed"], 1);
    assert_eq!(stats_json["successCount"], 1);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn virtual_soulfind_v2_routes_honor_explicit_disabled_gate() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_VIRTUAL_SOULFIND_V2_ENABLED", "false"));
    let response =
        super::route_http_request("GET", "/api/v1/virtualsoulfind/v2/stats", None, "", &state)
            .await
            .expect("disabled v2 status");
    assert_eq!(response.status, "503 Service Unavailable");
    assert_eq!(response.body, r#""VirtualSoulfind v2 is disabled""#);
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
    feature = "bounded-controller-api-tests-1"
))]
async fn controller_api_differential_library_jobs_and_discovery_projections() {
    let (state, _receiver) = test_state();
    let mut ledger = Vec::new();
    macro_rules! record_evidence {
        ($method:expr, $route:expr, $case:expr) => {
            ledger.push(serde_json::json!({
                "target": "slskdn",
                "method": $method,
                "route": $route,
                "case": $case,
                "pass": true,
            }));
        };
    }

    super::route_http_request(
        "POST",
        "/api/library/items",
        None,
        r#"{"artist":"","title":"Untitled","kind":"Audio"}"#,
        &state,
    )
    .await
    .expect("create incomplete library item");
    super::route_http_request(
        "POST",
        "/api/library/items",
        None,
        r#"{"artist":"Known","title":"Release","kind":"Audio"}"#,
        &state,
    )
    .await
    .expect("create complete library item");

    let health = super::route_http_request("GET", "/api/library/health/issues", None, "", &state)
        .await
        .expect("library issues");
    let health_json = serde_json::from_str::<serde_json::Value>(&health.body).unwrap();
    assert_eq!(health_json["totalCount"], 1);
    assert_eq!(health_json["issues"][0]["type"], "MissingMetadata");
    assert_eq!(
        health_json["issues"][0]["metadata"]["missingField"],
        "missing_artist"
    );
    assert_eq!(health_json["filter"]["limit"], 100);
    assert_eq!(health_json["filter"]["offset"], 0);

    let by_artist = super::route_http_request(
        "GET",
        "/api/library/health/issues/by-artist",
        None,
        "",
        &state,
    )
    .await
    .expect("issues by artist");
    let by_artist_json = serde_json::from_str::<serde_json::Value>(&by_artist.body).unwrap();
    assert!(by_artist_json["groups"].as_array().unwrap().is_empty());
    assert_eq!(by_artist_json["totalArtists"], 0);

    let summary = super::route_http_request(
        "GET",
        "/api/library/health/summary?LibraryPath=%2Fmusic",
        None,
        "",
        &state,
    )
    .await
    .expect("library health summary");
    assert_eq!(summary.status, "200 OK");
    let summary_json = serde_json::from_str::<serde_json::Value>(&summary.body).unwrap();
    assert_eq!(summary_json["libraryPath"], "/music");
    assert_eq!(summary_json["totalIssues"], 1);
    assert_eq!(summary_json["issuesOpen"], 1);

    let dashboard = super::route_http_request(
        "GET",
        "/api/library/health/dashboard?libraryPath=%2Fmusic&artistLimit=1&issueLimit=1",
        None,
        "",
        &state,
    )
    .await
    .expect("library health dashboard");
    assert_eq!(dashboard.status, "200 OK");
    let dashboard_json = serde_json::from_str::<serde_json::Value>(&dashboard.body).unwrap();
    assert_eq!(dashboard_json["summary"]["libraryPath"], "/music");
    assert_eq!(dashboard_json["issuesByType"][0]["type"], "MissingMetadata");
    assert!(dashboard_json["issuesByArtist"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(dashboard_json["issues"].as_array().unwrap().len(), 1);
    assert_eq!(dashboard_json["totalIssues"], 1);

    let by_codec = super::route_http_request(
        "GET",
        "/api/library/health/issues/by-codec",
        None,
        "",
        &state,
    )
    .await
    .expect("issues by codec");
    let by_codec_json = serde_json::from_str::<serde_json::Value>(&by_codec.body).unwrap();
    assert_eq!(by_codec_json["groups"].as_array().unwrap().len(), 1);
    assert_eq!(by_codec_json["groups"][0]["codec"], "UNKNOWN");
    assert_eq!(by_codec_json["groups"][0]["count"], 1);
    assert_eq!(by_codec_json["groups"][0]["transcodeSuspect"], 0);
    assert_eq!(by_codec_json["totalIssues"], 1);

    let filtered = super::route_http_request(
        "GET",
        "/api/library/health/issues?LibraryPath=%2Fmusic&types=CorruptedFile&severities=Medium&statuses=Detected&Limit=2&Offset=999999",
        None,
        "",
        &state,
    )
    .await
    .expect("filtered library issues");
    assert_eq!(filtered.status, "200 OK");
    let filtered_json = serde_json::from_str::<serde_json::Value>(&filtered.body).unwrap();
    assert_eq!(filtered_json["totalCount"], 0);
    assert_eq!(filtered_json["filter"]["libraryPath"], "/music");
    assert_eq!(filtered_json["filter"]["types"][0], "CorruptedFile");
    assert_eq!(filtered_json["filter"]["severities"][0], "Medium");
    assert_eq!(filtered_json["filter"]["statuses"][0], "Detected");
    assert_eq!(filtered_json["filter"]["limit"], 2);
    assert_eq!(filtered_json["filter"]["offset"], 999999);

    let by_type = super::route_http_request(
        "GET",
        "/api/library/health/issues/by-type",
        None,
        "",
        &state,
    )
    .await
    .expect("issues by type");
    assert_eq!(by_type.status, "200 OK");
    let by_type_json = serde_json::from_str::<serde_json::Value>(&by_type.body).unwrap();
    assert_eq!(by_type_json["groups"][0]["type"], "MissingMetadata");
    assert_eq!(by_type_json["totalIssues"], 1);

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
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        assert_eq!(response.status, "400 Bad Request", "{path}");
    }
    for path in [
        "/api/library/health/summary-untrusted",
        "/api/library/health/issues/by-type-untrusted",
        "/api/library/health/issues/by-type/missing_artist/untrusted",
    ] {
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        assert_eq!(response.status, "404 Not Found", "{path}");
    }

    let lidarr_missing = super::route_http_request(
        "GET",
        "/api/integrations/lidarr/wanted/missing",
        None,
        "",
        &state,
    )
    .await
    .expect("lidarr missing fallback");
    let lidarr_missing_json =
        serde_json::from_str::<serde_json::Value>(&lidarr_missing.body).unwrap();
    assert_eq!(lidarr_missing.status, "200 OK");
    assert_eq!(lidarr_missing_json["status"], "local");
    assert_eq!(lidarr_missing_json["source"], "library-health");
    assert_eq!(lidarr_missing_json["count"], 1);
    assert_eq!(
        lidarr_missing_json["missing_albums"][0]["issueType"],
        "missing_artist"
    );
    let versioned_lidarr_missing = super::route_http_request(
        "GET",
        "/api/v0/integrations/lidarr/wanted/missing",
        None,
        "",
        &state,
    )
    .await
    .expect("versioned lidarr missing fallback");
    let versioned_lidarr_missing_json =
        serde_json::from_str::<serde_json::Value>(&versioned_lidarr_missing.body).unwrap();
    assert_eq!(versioned_lidarr_missing.status, "200 OK");
    assert_eq!(versioned_lidarr_missing_json["status"], "local");
    assert_eq!(versioned_lidarr_missing_json["source"], "library-health");
    assert_eq!(versioned_lidarr_missing_json["count"], 1);
    let lidarr_sync = super::route_http_request(
        "POST",
        "/api/integrations/lidarr/wanted/sync",
        None,
        "{}",
        &state,
    )
    .await
    .expect("lidarr sync fallback");
    let lidarr_sync_json = serde_json::from_str::<serde_json::Value>(&lidarr_sync.body).unwrap();
    assert_eq!(lidarr_sync_json["status"], "local");
    assert_eq!(lidarr_sync_json["missingCount"], 1);
    assert_eq!(lidarr_sync_json["runs"], 1);

    let patched_issue = super::route_http_request(
        "PATCH",
        "/api/v0/library/health/issues/lib-1-missing-artist",
        None,
        r#"{"artist":"Recovered Artist"}"#,
        &state,
    )
    .await
    .expect("patch library health issue");
    assert_eq!(patched_issue.status, "204 No Content");
    assert_eq!(
        state.library.read().await.get("lib-1").unwrap().artist,
        "Recovered Artist"
    );

    super::route_http_request(
        "POST",
        "/api/library/items",
        None,
        r#"{"artist":"Fixable","title":"Kindless","kind":""}"#,
        &state,
    )
    .await
    .expect("create fixable library item");
    let scan = super::route_http_request(
        "POST",
        "/api/v0/library/health/scans",
        None,
        r#"{"libraryPath":"/music"}"#,
        &state,
    )
    .await
    .expect("library scan");
    let scan_json = serde_json::from_str::<serde_json::Value>(&scan.body).unwrap();
    assert_eq!(scan.status, "200 OK");
    assert!(scan_json["scanId"].as_str().is_some());
    assert_eq!(scan_json["message"], "Scan started successfully");
    let active_id = scan_json["scanId"].as_str().unwrap();
    let second_scan = super::route_http_request(
        "POST",
        "/api/v0/library/health/scans",
        None,
        r#"{"libraryPath":"/music"}"#,
        &state,
    )
    .await
    .expect("second library scan");
    assert_eq!(second_scan.status, "409 Conflict");
    assert!(second_scan.body.contains(active_id));
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    let completed = super::route_http_request(
        "GET",
        &format!("/api/library/health/scans/{active_id}"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&completed.body).unwrap()["status"],
        "completed"
    );
    let missing_scan = super::route_http_request(
        "GET",
        "/api/library/health/scans/scan-does-not-exist",
        None,
        "",
        &state,
    )
    .await
    .expect("missing library scan");
    assert_eq!(missing_scan.status, "404 Not Found");
    let scan_detail = super::route_http_request(
        "GET",
        &format!(
            "/api/library/health/scans/{}",
            scan_json["scanId"].as_str().unwrap()
        ),
        None,
        "",
        &state,
    )
    .await
    .expect("library scan detail");
    let scan_detail_json = serde_json::from_str::<serde_json::Value>(&scan_detail.body).unwrap();
    assert_eq!(scan_detail_json["issues_found"], 1);
    let aliased_scan = super::route_http_request(
        "GET",
        &format!(
            "/api/library/health/scans/{}/untrusted",
            scan_json["scanId"].as_str().unwrap()
        ),
        None,
        "",
        &state,
    )
    .await
    .expect("reject aliased library scan");
    assert_eq!(aliased_scan.status, "404 Not Found");
    let fixed = super::route_http_request(
        "POST",
        "/api/v0/slskdn/library/remediate",
        None,
        r#"{"issue_ids":["lib-3-missing-kind"]}"#,
        &state,
    )
    .await
    .expect("fix library issues");
    let fixed_json = serde_json::from_str::<serde_json::Value>(&fixed.body).unwrap();
    assert!(fixed_json["id"].as_str().is_some());
    assert_eq!(fixed_json["kind"], "library_remediation");
    assert_eq!(fixed_json["status"], "completed");
    assert_eq!(fixed_json["fixedCount"], 1);
    assert_eq!(
        fixed_json["issueIds"],
        serde_json::json!(["lib-3-missing-kind"])
    );
    let job = super::route_http_request(
        "GET",
        &format!("/api/jobs/{}", fixed_json["id"].as_str().unwrap()),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(job.status, "200 OK");
    let job_json = serde_json::from_str::<serde_json::Value>(&job.body).unwrap();
    assert_eq!(job_json["id"], fixed_json["id"]);
    assert_eq!(fixed_json["remaining"], serde_json::Value::Null);
    let stored_scan = super::route_http_request(
        "GET",
        &format!(
            "/api/library/health/scans/{}",
            scan_json["scanId"].as_str().unwrap()
        ),
        None,
        "",
        &state,
    )
    .await
    .expect("stored library scan snapshot");
    let stored_scan_json = serde_json::from_str::<serde_json::Value>(&stored_scan.body).unwrap();
    assert_eq!(stored_scan_json["issues_found"], 1);

    let lidarr_import = super::route_http_request(
        "POST",
        "/api/integrations/lidarr/manualimport",
        None,
        r#"{"directory":"/imports/Manual Album","artist":"Imported Artist"}"#,
        &state,
    )
    .await
    .expect("lidarr manual import fallback");
    let lidarr_import_json =
        serde_json::from_str::<serde_json::Value>(&lidarr_import.body).unwrap();
    assert_eq!(lidarr_import_json["status"], "local");
    assert_eq!(lidarr_import_json["imported"], 1);
    assert_eq!(lidarr_import_json["items"][0]["artist"], "Imported Artist");
    assert_eq!(lidarr_import_json["items"][0]["title"], "Manual Album");

    let completion = super::route_http_request(
        "GET",
        "/api/musicbrainz/albums/completion",
        None,
        "",
        &state,
    )
    .await
    .expect("completion");
    let completion_json = serde_json::from_str::<serde_json::Value>(&completion.body).unwrap();
    assert_eq!(completion.status, "200 OK");
    assert_eq!(completion_json["count"], 4);
    let versioned_completion = super::route_http_request(
        "GET",
        "/api/v0/musicbrainz/albums/completion",
        None,
        "",
        &state,
    )
    .await
    .expect("versioned completion");
    let versioned_completion_json =
        serde_json::from_str::<serde_json::Value>(&versioned_completion.body).unwrap();
    assert_eq!(versioned_completion.status, "200 OK");
    assert_eq!(versioned_completion_json["count"], 4);

    let coverage = super::route_http_request(
        "GET",
        "/api/musicbrainz/artist/Known/discography-coverage",
        None,
        "",
        &state,
    )
    .await
    .expect("coverage");
    let coverage_json = serde_json::from_str::<serde_json::Value>(&coverage.body).unwrap();
    assert_eq!(coverage.status, "200 OK");
    assert_eq!(coverage_json["releases"], 1);
    let versioned_coverage = super::route_http_request(
        "GET",
        "/api/v0/musicbrainz/artist/Known/discography-coverage",
        None,
        "",
        &state,
    )
    .await
    .expect("versioned coverage");
    let versioned_coverage_json =
        serde_json::from_str::<serde_json::Value>(&versioned_coverage.body).unwrap();
    assert_eq!(versioned_coverage.status, "200 OK");
    assert_eq!(versioned_coverage_json["releases"], 1);
    let aliased_coverage = super::route_http_request(
        "GET",
        "/api/musicbrainz/artist/Known/extra/discography-coverage",
        None,
        "",
        &state,
    )
    .await
    .expect("reject aliased artist coverage");
    assert_eq!(aliased_coverage.status, "404 Not Found");

    let target = super::route_http_request(
        "POST",
        "/api/musicbrainz/targets",
        None,
        r#"{"artist":"Known","title":"New Target"}"#,
        &state,
    )
    .await
    .expect("musicbrainz target");
    let target_json = serde_json::from_str::<serde_json::Value>(&target.body).unwrap();
    assert_eq!(target_json["created"], true);
    assert_eq!(target_json["item"]["artist"], "Known");
    assert!(target_json["projection"]["count"].as_u64().unwrap() >= 2);

    super::route_http_request(
        "POST",
        "/api/wishlist",
        None,
        r#"{"artist":"Radar","title":"Need"}"#,
        &state,
    )
    .await
    .expect("wishlist seed");
    let radar = super::route_http_request(
        "GET",
        "/api/musicbrainz/release-radar/notifications",
        None,
        "",
        &state,
    )
    .await
    .expect("release radar notifications");
    let radar_json = serde_json::from_str::<serde_json::Value>(&radar.body).unwrap();
    assert_eq!(radar.status, "200 OK");
    assert_eq!(radar_json[0]["artist"], "Radar");
    let versioned_radar = super::route_http_request(
        "GET",
        "/api/v0/musicbrainz/release-radar/notifications",
        None,
        "",
        &state,
    )
    .await
    .expect("versioned release radar notifications");
    let versioned_radar_json =
        serde_json::from_str::<serde_json::Value>(&versioned_radar.body).unwrap();
    assert_eq!(versioned_radar.status, "200 OK");
    assert!(versioned_radar_json.is_array());

    super::route_http_request(
        "POST",
        "/api/soulseek/interests",
        None,
        r#"{"name":"jazz"}"#,
        &state,
    )
    .await
    .expect("interest seed");
    let taste = super::route_http_request("POST", "/api/taste-recommendations", None, "{}", &state)
        .await
        .expect("taste recommendations");
    let taste_json = serde_json::from_str::<serde_json::Value>(&taste.body).unwrap();
    assert_eq!(taste_json["recommendations"][0]["query"], "jazz");

    let graph = super::route_http_request("POST", "/api/discovery-graph", None, "{}", &state)
        .await
        .expect("discovery graph");
    let graph_json = serde_json::from_str::<serde_json::Value>(&graph.body).unwrap();
    assert_eq!(graph_json["status"], "ready");
    assert!(graph_json["count"].as_u64().unwrap() >= 2);

    {
        let mut session = state.session.write().await;
        session.state = "connected";
    }
    super::route_http_request("POST", "/api/v0/rooms/listening/join", None, "", &state)
        .await
        .expect("join listening room");
    let parties = super::route_http_request("GET", "/api/listening-party", None, "", &state)
        .await
        .expect("listening parties");
    // Matches the oracle's real directory contract: it reflects
    // real, currently-listed pod listen-along events, not unrelated
    // joined chat rooms -- joining a room named "listening" above
    // must not fabricate an entry here.
    let parties_json = serde_json::from_str::<serde_json::Value>(&parties.body).unwrap();
    assert_eq!(parties.status, "200 OK");
    assert_eq!(parties_json, serde_json::json!([]));
    let versioned_parties =
        super::route_http_request("GET", "/api/v0/listening-party", None, "", &state)
            .await
            .expect("versioned listening parties");
    let versioned_parties_json =
        serde_json::from_str::<serde_json::Value>(&versioned_parties.body).unwrap();
    assert_eq!(versioned_parties.status, "200 OK");
    assert_eq!(versioned_parties_json, serde_json::json!([]));
    let party_content = super::route_http_request(
        "POST",
        "/api/listening-party/radio/party/content",
        None,
        r#"{"room":"listening","artist":"Party Artist","title":"Party Track"}"#,
        &state,
    )
    .await
    .expect("party content");
    let party_content_json =
        serde_json::from_str::<serde_json::Value>(&party_content.body).unwrap();
    assert_eq!(party_content_json["activePartyCount"], 1);
    assert_eq!(party_content_json["party"]["message_count"], 1);
    assert_eq!(party_content_json["nowPlaying"]["title"], "Party Track");

    let destination = super::route_http_request(
        "POST",
        "/api/destinations/validate",
        None,
        r#"{"path":"/home/user/Downloads"}"#,
        &state,
    )
    .await
    .expect("destination validate");
    let destination_json = serde_json::from_str::<serde_json::Value>(&destination.body).unwrap();
    assert_eq!(destination_json["valid"], true);
    assert_eq!(destination_json["known"], true);
    assert_eq!(destination_json["matched"]["id"], "default");

    super::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        r#"{"query":"job search"}"#,
        &state,
    )
    .await
    .expect("search job seed");
    super::route_http_request(
        "POST",
        "/api/v0/transfers",
        None,
        r#"{"direction":0,"peer_username":"peer","filename":"Remote/Song.flac","size":100}"#,
        &state,
    )
    .await
    .expect("multisource transfer seed");
    let jobs = super::route_http_request("GET", "/api/jobs", None, "", &state)
        .await
        .expect("jobs");
    let jobs_json = serde_json::from_str::<serde_json::Value>(&jobs.body).unwrap();
    assert_eq!(jobs.status, "200 OK");
    assert!(jobs_json["jobs"].is_array());
    assert!(jobs_json["limit"].is_number());
    assert!(jobs_json["offset"].is_number());
    assert!(jobs_json["has_more"].is_boolean());
    assert!(jobs_json["total"].as_u64().unwrap() >= 2);

    let discography = super::route_http_request(
        "POST",
        "/api/v0/jobs/discography",
        None,
        r#"{"artist":"Known"}"#,
        &state,
    )
    .await
    .expect("discography job");
    let discography_json = serde_json::from_str::<serde_json::Value>(&discography.body).unwrap();
    assert_eq!(discography_json["kind"], "discography");
    assert_eq!(discography_json["status"], "queued");
    let discography_id = discography_json["search_id"].as_str().unwrap();
    let discography_detail = super::route_http_request(
        "GET",
        &format!("/api/jobs/{discography_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("discography detail");
    let discography_detail_json =
        serde_json::from_str::<serde_json::Value>(&discography_detail.body).unwrap();
    assert_eq!(discography_detail_json["kind"], "search");
    assert_eq!(discography_detail_json["query"], "Known discography");
    let aliased_job = super::route_http_request(
        "GET",
        &format!("/api/jobs/{discography_id}/untrusted"),
        None,
        "",
        &state,
    )
    .await
    .expect("reject aliased job detail");
    assert_eq!(aliased_job.status, "404 Not Found");

    let mb_release = super::route_http_request(
        "POST",
        "/api/v0/jobs/mb-release",
        None,
        r#"{"artist":"Known","title":"Release"}"#,
        &state,
    )
    .await
    .expect("mb release job");
    let mb_release_json = serde_json::from_str::<serde_json::Value>(&mb_release.body).unwrap();
    assert_eq!(mb_release_json["kind"], "mb-release");
    assert_eq!(mb_release_json["query"], "Known Release");

    {
        let mut shares = state.shares.write().await;
        shares.entries.push(FileEntry {
            filename_encoding: Default::default(),
            extension_encoding: Default::default(),
            code: 1,
            filename: "Library/Known/Release.flac".to_owned(),
            size: 321,
            extension: "flac".to_owned(),
            attributes: Vec::new(),
        });
    }
    let hash_entries = super::route_http_request("GET", "/api/hashdb/entries", None, "", &state)
        .await
        .expect("hashdb entries");
    let hash_entries_json = serde_json::from_str::<serde_json::Value>(&hash_entries.body).unwrap();
    assert!(hash_entries_json["count"].as_u64().unwrap() >= 1);
    assert!(hash_entries_json["entries"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["filename"] == "Library/Known/Release.flac"
            && entry["extension"] == "flac"));
    assert!(!hash_entries_json["entries"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["filename"] == "Library/Known/Release.jpg"));

    let backfill_candidates =
        super::route_http_request("GET", "/api/backfill/candidates", None, "", &state)
            .await
            .expect("backfill candidates");
    let backfill_candidates_json =
        serde_json::from_str::<serde_json::Value>(&backfill_candidates.body).unwrap();
    assert!(backfill_candidates_json["count"].as_u64().unwrap() <= 10);
    let backfill_stats = super::route_http_request("GET", "/api/backfill/stats", None, "", &state)
        .await
        .expect("backfill stats");
    let backfill_stats_json =
        serde_json::from_str::<serde_json::Value>(&backfill_stats.body).unwrap();
    assert_eq!(backfill_stats_json["totalAttempts"], 0);
    assert_eq!(backfill_stats_json["active"], 0);
    assert_eq!(backfill_stats_json["isIdle"], false);
    let backfill_config =
        super::route_http_request("GET", "/api/backfill/config", None, "", &state)
            .await
            .expect("backfill config");
    let backfill_config_json =
        serde_json::from_str::<serde_json::Value>(&backfill_config.body).unwrap();
    assert_eq!(backfill_config_json["maxGlobalConnections"], 2);
    assert_eq!(backfill_config_json["maxHeaderBytes"], 65_536);
    let disabled = super::route_http_request(
        "POST",
        "/api/backfill/enable?enabled=false",
        None,
        "",
        &state,
    )
    .await
    .expect("disable backfill");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&disabled.body).unwrap()["enabled"],
        false
    );
    let idle = super::route_http_request("POST", "/api/backfill/idle", None, "", &state)
        .await
        .expect("mark backfill idle");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&idle.body).unwrap()["isIdle"],
        true
    );
    let busy = super::route_http_request("POST", "/api/backfill/busy", None, "", &state)
        .await
        .expect("mark backfill busy");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&busy.body).unwrap()["isIdle"],
        false
    );
    let invalid_file = super::route_http_request(
        "POST",
        "/api/backfill/file",
        None,
        r#"{"peerId":"","path":"","size":0}"#,
        &state,
    )
    .await
    .expect("reject invalid backfill file");
    assert_eq!(invalid_file.status, "400 Bad Request");
    let hash_backfill = super::route_http_request(
        "POST",
        "/api/v0/hashdb/backfill/from-history",
        None,
        "{}",
        &state,
    )
    .await
    .expect("hashdb backfill");
    let hash_backfill_json =
        serde_json::from_str::<serde_json::Value>(&hash_backfill.body).unwrap();
    assert_eq!(hash_backfill.status, "200 OK");
    assert!(hash_backfill_json.get("searchesProcessed").is_some());
    assert!(hash_backfill_json.get("flacsDiscovered").is_some());
    let unversioned_hash_backfill = super::route_http_request(
        "POST",
        "/api/hashdb/backfill/from-history",
        None,
        "{}",
        &state,
    )
    .await
    .expect("reject unversioned hashdb backfill");
    assert_eq!(unversioned_hash_backfill.status, "400 Bad Request");
    let aliased_hash_backfill = super::route_http_request(
        "POST",
        "/api/hashdb/backfill/from-history-untrusted",
        None,
        "{}",
        &state,
    )
    .await
    .expect("reject aliased hashdb backfill");
    assert_eq!(aliased_hash_backfill.status, "404 Not Found");
    let runtime_backfill = super::route_http_request("POST", "/api/backfill", None, "{}", &state)
        .await
        .expect("runtime backfill");
    let runtime_backfill_json =
        serde_json::from_str::<serde_json::Value>(&runtime_backfill.body).unwrap();
    assert_eq!(runtime_backfill_json["runs"], 1);
    assert!(runtime_backfill_json["queued"].as_u64().unwrap() >= 4);

    let song_runs = super::route_http_request("GET", "/api/songid/runs", None, "", &state)
        .await
        .expect("song id runs");
    let song_runs_json = serde_json::from_str::<serde_json::Value>(&song_runs.body).unwrap();
    assert!(song_runs_json.as_array().unwrap().is_empty());
    let song_run = super::route_http_request(
        "POST",
        "/api/songid/runs",
        None,
        r#"{"source":"route-audit"}"#,
        &state,
    )
    .await
    .expect("song id run");
    let song_run_json = serde_json::from_str::<serde_json::Value>(&song_run.body).unwrap();
    assert!(song_run_json["matchCount"].as_u64().unwrap() >= 1);
    assert_eq!(song_run_json["runs"], 1);
    assert_eq!(song_run_json["persisted"], true);
    let song_id = song_run_json["id"].as_str().unwrap();
    let song_detail = super::route_http_request(
        "GET",
        &format!("/api/songid/runs/{song_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("song id detail");
    let song_detail_json = serde_json::from_str::<serde_json::Value>(&song_detail.body).unwrap();
    assert_eq!(song_detail_json["id"], song_id);
    assert!(song_detail_json["matchCount"].as_u64().unwrap() >= 1);
    let missing_song_run = super::route_http_request(
        "GET",
        "/api/songid/runs/songid-does-not-exist",
        None,
        "",
        &state,
    )
    .await
    .expect("missing song id run");
    assert_eq!(missing_song_run.status, "404 Not Found");
    let song_matrix = super::route_http_request(
        "GET",
        &format!("/api/songid/runs/{song_id}/forensic-matrix"),
        None,
        "",
        &state,
    )
    .await
    .expect("song id matrix");
    let song_matrix_json = serde_json::from_str::<serde_json::Value>(&song_matrix.body).unwrap();
    assert!(song_matrix_json["count"].as_u64().unwrap() >= 1);

    let multisource = super::route_http_request("GET", "/api/multisource/jobs", None, "", &state)
        .await
        .expect("multisource jobs");
    let multisource_json = serde_json::from_str::<serde_json::Value>(&multisource.body).unwrap();
    assert_eq!(multisource_json["jobs"][0]["sources"][0], "peer");
    let multisource_id = multisource_json["jobs"][0]["id"].as_str().unwrap();
    let multisource_detail = super::route_http_request(
        "GET",
        &format!("/api/multisource/jobs/{multisource_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("multisource job detail");
    assert_eq!(multisource_detail.status, "200 OK");
    let aliased_multisource = super::route_http_request(
        "GET",
        &format!("/api/multisource/jobs/{multisource_id}/untrusted"),
        None,
        "",
        &state,
    )
    .await
    .expect("reject aliased multisource job detail");
    assert_eq!(aliased_multisource.status, "404 Not Found");
    // Matches the oracle's real GetJobStatus: an unknown job id must be
    // a real 404, not a fabricated 200 with an invented "not_found"
    // status string.
    let missing_multisource = super::route_http_request(
        "GET",
        "/api/multisource/jobs/multisource-does-not-exist",
        None,
        "",
        &state,
    )
    .await
    .expect("missing multisource job");
    assert_eq!(missing_multisource.status, "404 Not Found");

    let slskdn = super::route_http_request("GET", "/api/slskdn", None, "", &state)
        .await
        .expect("slskdn summary");
    let native_json = serde_json::from_str::<serde_json::Value>(&slskdn.body).unwrap();
    assert_eq!(native_json["status"], "local");
    assert!(native_json["libraryItems"].as_u64().unwrap() >= 4);
    let native_health =
        super::route_http_request("GET", "/api/slskdn/library/health", None, "", &state)
            .await
            .expect("slskdn library health");
    let native_health_json =
        serde_json::from_str::<serde_json::Value>(&native_health.body).unwrap();
    assert_eq!(native_health_json["summary"]["total_issues"], 0);
    let podcore_search = super::route_http_request(
        "GET",
        "/api/podcore/content/search?query=Release",
        None,
        "",
        &state,
    )
    .await
    .expect("podcore search");
    let podcore_search_json =
        serde_json::from_str::<serde_json::Value>(&podcore_search.body).unwrap();
    let podcore_search_results = podcore_search_json.as_array().unwrap();
    assert!(!podcore_search_results.is_empty(), "{podcore_search_json}");
    assert!(
        podcore_search_results[0]["contentId"]
            .as_str()
            .unwrap()
            .starts_with("content:audio:track:"),
        "{podcore_search_json}"
    );
    let stream = super::route_http_request(
        "GET",
        "/api/streams/Library/Known/Release.flac",
        None,
        "",
        &state,
    )
    .await
    .expect("stream status");
    let stream_json = serde_json::from_str::<serde_json::Value>(&stream.body).unwrap();
    assert_eq!(stream_json["status"], "available");

    // The test above exercises a larger compatibility projection surface,
    // but only these rows correspond to frozen slskdn controller subjects
    // and have assertions strong enough to prove the named case.  Keep
    // compatibility aliases that are not in the frozen controller
    // inventory out of the ledger rather than treating route reachability
    // as behavioral proof.
    for (method, route, case) in [
        (
            "GET",
            "/api/library/health/summary",
            "malformed-path-query-or-body",
        ),
        (
            "GET",
            "/api/library/health/dashboard",
            "malformed-path-query-or-body",
        ),
        (
            "GET",
            "/api/library/health/issues/by-codec",
            "malformed-path-query-or-body",
        ),
        (
            "GET",
            "/api/library/health/issues/by-release",
            "malformed-path-query-or-body",
        ),
        (
            "GET",
            "/api/library/health/issues/by-type",
            "malformed-path-query-or-body",
        ),
        (
            "GET",
            "/api/library/health/issues",
            "missing-empty-or-conflict-state",
        ),
        (
            "POST",
            "/api/v0/library/health/scans",
            "concurrency-and-idempotency",
        ),
        (
            "GET",
            "/api/library/health/scans/{scanId}",
            "populated-dynamic-state",
        ),
        ("GET", "/api/jobs", "populated-dynamic-state"),
        ("GET", "/api/jobs", "nominal-status-headers-body"),
        (
            "GET",
            "/api/v0/integrations/lidarr/wanted/missing",
            "nominal-status-headers-body",
        ),
        (
            "GET",
            "/api/v0/listening-party",
            "nominal-status-headers-body",
        ),
        (
            "GET",
            "/api/v0/musicbrainz/albums/completion",
            "nominal-status-headers-body",
        ),
        (
            "GET",
            "/api/v0/musicbrainz/artist/{artistId}/discography-coverage",
            "nominal-status-headers-body",
        ),
        (
            "GET",
            "/api/v0/musicbrainz/release-radar/notifications",
            "nominal-status-headers-body",
        ),
        (
            "POST",
            "/api/v0/jobs/discography",
            "nominal-status-headers-body",
        ),
        (
            "POST",
            "/api/v0/jobs/discography",
            "mutation-side-effects-and-readback",
        ),
        ("GET", "/api/jobs/{id}", "populated-dynamic-state"),
        (
            "POST",
            "/api/v0/jobs/mb-release",
            "nominal-status-headers-body",
        ),
        (
            "GET",
            "/api/slskdn/library/health",
            "populated-dynamic-state",
        ),
    ] {
        record_evidence!(method, route, case);
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("library_jobs_and_discovery_projections.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn hashdb_history_backfill_batches_persists_inventory_and_progress() {
    let (state, _receiver) = test_state();
    {
        let mut searches = state.searches.write().await;
        for index in 1..=11_u64 {
            searches.records.push(super::SearchRecord {
                id: format!("history-{index}"),
                token: u32::try_from(index).unwrap(),
                query: format!("history {index}"),
                target: "global",
                target_name: None,
                status: "completed",
                results: vec![super::SearchResultEntry {
                    peer_username: Some(format!("peer-{index}")),
                    filename: format!("Library/Track-{index}.flac"),
                    size: 32_768 + index,
                    extension: "flac".to_owned(),
                    bit_rate: None,
                    sample_rate: None,
                    bit_depth: None,
                    length_seconds: None,
                    locked: false,
                    slot_free: Some(true),
                    average_speed: Some(1_000),
                    queue_length: Some(0),
                }],
                raw_response_count: 1,
                filtered_out_count: 0,
                ignored_result_count: 0,
                hidden_locked_count: 0,
                fallback_attempts: 0,
                ttl_seconds: super::DEFAULT_SEARCH_TTL_SECONDS,
                expires_at: 0,
                created_at: index,
                updated_at: index,
            });
        }
    }

    let first = super::route_http_request(
        "POST",
        "/api/v0/hashdb/backfill/from-history?batchSize=10",
        None,
        "",
        &state,
    )
    .await
    .expect("first history backfill batch");
    assert_eq!(first.status, "200 OK");
    let first_json = serde_json::from_str::<serde_json::Value>(&first.body).unwrap();
    assert_eq!(first_json["searchesProcessed"], 10);
    assert_eq!(first_json["flacsDiscovered"], 10);
    assert_eq!(first_json["totalSearches"], 11);
    assert_eq!(first_json["remainingSearches"], 1);
    assert_eq!(first_json["complete"], false);

    let candidates = super::route_http_request(
        "GET",
        "/api/v0/hashdb/backfill/candidates?limit=20",
        None,
        "",
        &state,
    )
    .await
    .expect("hashdb backfill candidates");
    let candidates_json = serde_json::from_str::<serde_json::Value>(&candidates.body).unwrap();
    assert_eq!(candidates_json["count"], 10, "{candidates_json}");
    assert_eq!(
        candidates_json["entries"].as_array().unwrap().len(),
        10,
        "{candidates_json}"
    );

    let second = super::route_http_request(
        "POST",
        "/api/v0/hashdb/backfill/from-history?batchSize=10",
        None,
        "",
        &state,
    )
    .await
    .expect("second history backfill batch");
    let second_json = serde_json::from_str::<serde_json::Value>(&second.body).unwrap();
    assert_eq!(second_json["searchesProcessed"], 1);
    assert_eq!(second_json["flacsDiscovered"], 1);
    assert_eq!(second_json["remainingSearches"], 0);
    assert_eq!(second_json["complete"], true);

    let complete = super::route_http_request(
        "POST",
        "/api/v0/hashdb/backfill/from-history",
        None,
        "",
        &state,
    )
    .await
    .expect("completed history backfill");
    let complete_json = serde_json::from_str::<serde_json::Value>(&complete.body).unwrap();
    assert_eq!(complete_json["searchesProcessed"], 0);
    assert_eq!(complete_json["flacsDiscovered"], 0);
    assert_eq!(complete_json["complete"], true);

    let reset = super::route_http_request(
        "POST",
        "/api/v0/hashdb/backfill/from-history?reset=true",
        None,
        "",
        &state,
    )
    .await
    .expect("reset history backfill");
    let reset_json = serde_json::from_str::<serde_json::Value>(&reset.body).unwrap();
    assert_eq!(reset_json["searchesProcessed"], 11);
    assert_eq!(reset_json["flacsDiscovered"], 11);
    assert_eq!(reset_json["complete"], true);

    let stats = super::route_http_request("GET", "/api/v0/hashdb/stats", None, "", &state)
        .await
        .expect("hashdb stats after backfill");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap();
    assert_eq!(stats_json["totalFlacEntries"], 11);
    assert_eq!(stats_json["hashedFlacEntries"], 0);

    let analysis =
        super::route_http_request("GET", "/api/v0/hashdb/optimize/analyze", None, "", &state)
            .await
            .expect("hashdb optimize analysis after backfill");
    let analysis_json = serde_json::from_str::<serde_json::Value>(&analysis.body).unwrap();
    assert_eq!(analysis_json["flacInventoryEntryCount"], 11);
    assert_eq!(analysis_json["peerCount"], 11);

    let peers = super::route_http_request("GET", "/api/v0/hashdb/peers", None, "", &state)
        .await
        .expect("hashdb peers after backfill");
    let peers_json = serde_json::from_str::<serde_json::Value>(&peers.body).unwrap();
    assert_eq!(peers_json["count"], 0);

    let compatibility_peers =
        super::route_http_request("GET", "/api/hashdb/peers", None, "", &state)
            .await
            .expect("compatibility hashdb peers after backfill");
    let compatibility_peers_json =
        serde_json::from_str::<serde_json::Value>(&compatibility_peers.body).unwrap();
    assert_eq!(compatibility_peers_json["count"], 11);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn versioned_hashdb_paging_matches_sequence_controller_contract() {
    let (state, _receiver) = test_state();
    let hash_a = "a".repeat(64);
    let hash_b = "b".repeat(64);
    state
        .content_discovery
        .write()
        .await
        .merge_hash_entries(vec![
            super::content_discovery::HashDbEntry {
                flac_key: hash_a.clone(),
                byte_hash: hash_a,
                size: 100,
                ..Default::default()
            },
            super::content_discovery::HashDbEntry {
                flac_key: hash_b.clone(),
                byte_hash: hash_b,
                size: 200,
                ..Default::default()
            },
        ])
        .expect("seed hashdb sequence");

    let first =
        super::route_http_request("GET", "/api/v0/hashdb/entries?limit=1", None, "", &state)
            .await
            .expect("first hashdb page");
    assert_eq!(first.status, "200 OK", "{}", first.body);
    let first_json = serde_json::from_str::<serde_json::Value>(&first.body).unwrap();
    assert_eq!(first_json["latestSeq"], 2);
    assert_eq!(first_json["count"], 1);
    assert_eq!(first_json["entries"][0]["seqId"], 1);
    assert!(first_json.get("offset").is_none());
    assert!(first_json.get("limit").is_none());

    let second = super::route_http_request(
        "GET",
        "/api/v0/hashdb/entries?offset=1&limit=1",
        None,
        "",
        &state,
    )
    .await
    .expect("sequence-offset hashdb page");
    let second_json = serde_json::from_str::<serde_json::Value>(&second.body).unwrap();
    assert_eq!(second_json["entries"][0]["seqId"], 2);

    let sync = super::route_http_request(
        "GET",
        "/api/v0/hashdb/sync/since/1?limit=1",
        None,
        "",
        &state,
    )
    .await
    .expect("hashdb sync page");
    let sync_json = serde_json::from_str::<serde_json::Value>(&sync.body).unwrap();
    assert_eq!(sync_json["latestSeq"], 2);
    assert_eq!(sync_json["count"], 1);
    assert_eq!(sync_json["entries"][0]["seqId"], 2);
    assert!(sync_json.get("fromSeqId").is_none());
    assert!(sync_json.get("hasMore").is_none());
}

#[cfg_attr(test, tokio::test(flavor = "multi_thread", worker_threads = 2))]
#[cfg(feature = "full-controller-tests")]
async fn compatibility_projections_use_local_state_for_system_mutation_shells() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));
    {
        let mut advanced = state.advanced_networking.write().await;
        advanced.mesh.enabled = true;
        advanced.mesh.enable_overlay = true;
    }

    super::route_http_request(
        "POST",
        "/api/v0/transfers",
        None,
        r#"{"direction":0,"peer_username":"peer","filename":"Remote/System.flac","size":100}"#,
        &state,
    )
    .await
    .expect("transfer seed");
    super::route_http_request(
        "POST",
        "/api/v0/transfers/1/progress",
        None,
        r#"{"bytes_transferred":55}"#,
        &state,
    )
    .await
    .expect("transfer progress");
    super::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        r#"{"query":"system stats"}"#,
        &state,
    )
    .await
    .expect("search seed");

    let admin = super::route_http_request("GET", "/api/admin/stats", None, "", &state)
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
        let response = super::route_http_request(method, path, None, "", &state)
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
    let updated_profile = super::route_http_request(
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
    let profile = super::route_http_request("GET", "/api/profile/me", None, "", &state)
        .await
        .expect("profile me");
    let profile_json = serde_json::from_str::<serde_json::Value>(&profile.body).unwrap();
    assert_eq!(profile_json["username"], "updated-user");
    assert_eq!(profile_json["user_type"], "privileged");

    let batch = super::route_http_request(
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

    let plugins = super::route_http_request("GET", "/api/config/plugins", None, "", &state)
        .await
        .expect("plugins");
    let plugins_json = serde_json::from_str::<serde_json::Value>(&plugins.body).unwrap();
    assert_eq!(plugins_json["count"], 4);
    assert_eq!(plugins_json["plugins"][0]["id"], "spotify");

    let invite = super::route_http_request("POST", "/api/profile/invite", None, "{}", &state)
        .await
        .expect("profile invite");
    let invite_json = serde_json::from_str::<serde_json::Value>(&invite.body).unwrap();
    assert_eq!(invite_json["count"], 1);
    assert_eq!(invite_json["persisted"], true);

    let warm_cache =
        super::route_http_request("POST", "/api/slskdn/warm-cache", None, "{}", &state)
            .await
            .expect("warm cache");
    let warm_cache_json = serde_json::from_str::<serde_json::Value>(&warm_cache.body).unwrap();
    assert_eq!(warm_cache_json["runs"], 1);
    assert_eq!(warm_cache_json["persisted"], true);

    let bridge_config = super::route_http_request(
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
        super::route_http_request("POST", "/api/v0/bridge/start", None, "{}", &state)
            .await
            .expect("bridge start");
    let bridge_start_json = serde_json::from_str::<serde_json::Value>(&bridge_start.body).unwrap();
    assert_eq!(bridge_start_json["persisted"], true);
    let bridge_status = super::route_http_request("GET", "/api/bridge/status", None, "", &state)
        .await
        .expect("bridge status");
    let bridge_status_json =
        serde_json::from_str::<serde_json::Value>(&bridge_status.body).unwrap();
    assert_eq!(bridge_status_json["configUpdates"], 1);
    let bridge_stop = super::route_http_request("POST", "/api/v0/bridge/stop", None, "{}", &state)
        .await
        .expect("bridge stop");
    let bridge_stop_json = serde_json::from_str::<serde_json::Value>(&bridge_stop.body).unwrap();
    assert_eq!(bridge_stop_json["stopped"], true);

    let application_restart =
        super::route_http_request("PUT", "/api/application", None, "{}", &state)
            .await
            .expect("application restart request");
    assert_eq!(application_restart.status, "204 No Content");
    assert!(application_restart.body.is_empty());
    let application = super::route_http_request("GET", "/api/application", None, "", &state)
        .await
        .expect("application state");
    let application_json = serde_json::from_str::<serde_json::Value>(&application.body).unwrap();
    assert_eq!(application_json["pendingRestart"], true);
    assert_eq!(application_json["bridge"]["configUpdates"], 1);
    assert_eq!(application_json["operations"]["profileInvitesCreated"], 1);
    assert_eq!(application_json["operations"]["cacheWarmRuns"], 1);
    let gc = super::route_http_request("POST", "/api/application/gc", None, "", &state)
        .await
        .expect("application gc");
    let gc_json = serde_json::from_str::<serde_json::Value>(&gc.body).unwrap();
    assert_eq!(gc_json["collected"], true);
    assert_eq!(gc_json["gcRuns"], 1);
    let application_restart_clear =
        super::route_http_request("DELETE", "/api/application", None, "", &state)
            .await
            .expect("application restart clear");
    assert_eq!(application_restart_clear.status, "204 No Content");
    assert!(application_restart_clear.body.is_empty());
    let application = super::route_http_request("GET", "/api/application", None, "", &state)
        .await
        .expect("application state after shutdown request");
    let application_json = serde_json::from_str::<serde_json::Value>(&application.body).unwrap();
    assert_eq!(application_json["pendingRestart"], false);

    let autoreplace = super::route_http_request("PUT", "/api/autoreplace/enable", None, "", &state)
        .await
        .expect("autoreplace enable");
    let autoreplace_json = serde_json::from_str::<serde_json::Value>(&autoreplace.body).unwrap();
    assert_eq!(autoreplace_json["enabled"], true);
    assert_eq!(autoreplace_json["persisted"], true);
    let preferences = super::route_http_request(
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

    let relay = super::route_http_request("PUT", "/api/relay", None, r#"{"enabled":true}"#, &state)
        .await
        .expect("relay enable");
    let relay_json = serde_json::from_str::<serde_json::Value>(&relay.body).unwrap();
    assert_eq!(relay_json["relay_enabled"], true);
    let relay_agent = super::route_http_request(
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
    let relay_files = super::route_http_request(
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
    let relay_shares = super::route_http_request(
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
        let response = super::route_http_request(method, path, None, "{}", &state)
            .await
            .expect("reject nested relay token route");
        assert_eq!(response.status, "404 Not Found", "{method} {path}");
    }
    let application = super::route_http_request("GET", "/api/application", None, "", &state)
        .await
        .expect("application with relay");
    let application_json = serde_json::from_str::<serde_json::Value>(&application.body).unwrap();
    assert_eq!(application_json["relay"]["enabled"], true);
    assert_eq!(application_json["relay"]["agentEnabled"], true);
    let relay_agent_deleted =
        super::route_http_request("DELETE", "/api/relay/agent", None, "", &state)
            .await
            .expect("relay agent disable");
    let relay_agent_deleted_json =
        serde_json::from_str::<serde_json::Value>(&relay_agent_deleted.body).unwrap();
    assert_eq!(relay_agent_deleted_json["relayAgentEnabled"], false);
    let relay_deleted = super::route_http_request("DELETE", "/api/relay", None, "", &state)
        .await
        .expect("relay disable");
    let relay_deleted_json =
        serde_json::from_str::<serde_json::Value>(&relay_deleted.body).unwrap();
    assert_eq!(relay_deleted_json["relay_enabled"], false);

    let recorded_event = super::route_http_request(
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
    super::record_daemon_log(
        &state,
        super::logging::LogLevel::Info,
        "compat.event",
        "compat event",
    )
    .await;
    let logs = super::route_http_request("GET", "/api/logs", None, "", &state)
        .await
        .expect("logs");
    let logs_json = serde_json::from_str::<serde_json::Value>(&logs.body).unwrap();
    assert_eq!(logs_json["entries"][0]["category"], "compat.event");
    assert_eq!(logs_json["entries"][0]["message"], "compat event");
    let compat_logs = super::route_http_request("GET", "/api/v0/logs", None, "", &state)
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
        super::route_http_request("POST", "/api/mesh/sync/mesh-peer", None, "{}", &state)
            .await
            .expect("mesh sync");
    let mesh_sync_json = serde_json::from_str::<serde_json::Value>(&mesh_sync.body).unwrap();
    assert_eq!(mesh_sync_json["queued"], true);
    assert_eq!(mesh_sync_json["status"], "watched");

    let kpis = super::route_http_request("GET", "/api/telemetry/metrics/kpis", None, "", &state)
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

    let pod_created = super::route_http_request(
        "POST",
        "/api/pods",
        None,
        r#"{"pod":{"podId":"pod:mesh-peer","name":"mesh-peer","isPublic":true,"channels":[]},"requestingPeerId":"mesh-peer"}"#,
        &state,
    )
    .await
    .expect("create compatibility pod");
    assert_eq!(pod_created.status, "201 Created");
    let pods = super::route_http_request("GET", "/api/pods", None, "", &state)
        .await
        .expect("pods");
    let pods_json = serde_json::from_str::<serde_json::Value>(&pods.body).unwrap();
    assert!(pods_json
        .as_array()
        .unwrap()
        .iter()
        .any(|pod| pod["name"] == "mesh-peer"));

    let federation =
        super::route_http_request("GET", "/api/v0/federation/diagnostics", None, "", &state)
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

    let security = super::route_http_request("GET", "/api/security/dashboard", None, "", &state)
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
    super::route_http_request(
        "POST",
        "/api/security/bans/username",
        None,
        r#"{"username":"mesh-peer"}"#,
        &state,
    )
    .await
    .expect("security ban");
    let security_status =
        super::route_http_request("GET", "/api/security/status", None, "", &state)
            .await
            .expect("security status");
    let security_status_json =
        serde_json::from_str::<serde_json::Value>(&security_status.body).unwrap();
    assert_eq!(security_status_json["activeBans"], 1);
    let security = super::route_http_request("GET", "/api/security/dashboard", None, "", &state)
        .await
        .expect("security dashboard after ban");
    let security_json = serde_json::from_str::<serde_json::Value>(&security.body).unwrap();
    assert_eq!(security_json["stats"]["banStats"]["activeBans"], 1);

    let fairness = super::route_http_request("GET", "/api/fairness", None, "", &state)
        .await
        .expect("fairness");
    let fairness_json = serde_json::from_str::<serde_json::Value>(&fairness.body).unwrap();
    assert_eq!(fairness_json["status"], "ready");
    assert_eq!(fairness_json["items"][0]["username"], "mesh-peer");
    let ranking = super::route_http_request("GET", "/api/ranking", None, "", &state)
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
        super::route_http_request("GET", "/api/portforwarding/status", None, "", &state)
            .await
            .expect("portforwarding");
    let portforwarding_json =
        serde_json::from_str::<serde_json::Value>(&portforwarding.body).unwrap();
    assert!(portforwarding_json.as_array().unwrap().is_empty());

    let imported = super::route_http_request(
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
    let wishlist_search = super::route_http_request(
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

    let collection = super::route_http_request(
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
    let solid = super::route_http_request("GET", "/api/solid/status", None, "", &state)
        .await
        .expect("solid status");
    let solid_json = serde_json::from_str::<serde_json::Value>(&solid.body).unwrap();
    assert_eq!(solid_json["enabled"], true);
    super::route_http_request(
        "POST",
        &format!("/api/collections/{collection_id}/items"),
        None,
        r#"{"content_id":"lib-1","artist":"A","title":"One"}"#,
        &state,
    )
    .await
    .expect("collection item");
    let grant = super::route_http_request(
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
    let token = super::route_http_request(
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
    let backfill = super::route_http_request(
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

async fn configured_relay_test_state() -> (Arc<super::AppState>, &'static str, u64) {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));
    let secret = "test-token-0123456789";
    {
        let mut advanced = state.advanced_networking.write().await;
        advanced.relay.enabled = true;
        advanced.relay.mode = "controller".to_owned();
        advanced.relay.agents.insert(
            "edge".to_owned(),
            super::config::RelayAgentSettings {
                instance_name: "edge-one".to_owned(),
                secret: secret.to_owned(),
                cidr: "127.0.0.1/32".to_owned(),
            },
        );
    }
    let relay_settings = state.advanced_networking.read().await.relay.clone();
    let now = super::unix_timestamp();
    let challenge = state
        .relay
        .write()
        .await
        .protocol
        .issue_challenge("connection-1", now);
    let challenge_credential = super::relay::credential_for_test(secret, "edge-one", &challenge);
    {
        let mut relay = state.relay.write().await;
        assert!(relay.protocol.authenticate_agent(
            &relay_settings,
            super::relay::credential_scheme(state.config.controller_profile),
            "connection-1",
            "edge-one",
            &challenge_credential,
            "127.0.0.1".parse().unwrap(),
            now,
        ));
        assert_eq!(
            relay.protocol.registered_agent_remote_ip("edge-one"),
            Some("127.0.0.1".parse().unwrap())
        );
    }
    (state, secret, now)
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn native_relay_credential_profile_authenticates_agent() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let secret = "test-token-0123456789";
    {
        let mut advanced = state.advanced_networking.write().await;
        advanced.relay.enabled = true;
        advanced.relay.mode = "controller".to_owned();
        advanced.relay.agents.insert(
            "edge".to_owned(),
            super::config::RelayAgentSettings {
                instance_name: "edge-one".to_owned(),
                secret: secret.to_owned(),
                cidr: "127.0.0.1/32".to_owned(),
            },
        );
    }
    let now = super::unix_timestamp();
    let challenge = state
        .relay
        .write()
        .await
        .protocol
        .issue_challenge("slskdn-connection", now);
    let credential = super::relay::credential_for_target(
        super::config::ControllerProfile::Native,
        secret,
        "edge-one",
        &challenge,
    );
    let settings = state.advanced_networking.read().await.relay.clone();
    assert!(state.relay.write().await.protocol.authenticate_agent(
        &settings,
        super::relay::credential_scheme(super::config::ControllerProfile::Native,),
        "slskdn-connection",
        "edge-one",
        &credential,
        "127.0.0.1".parse().unwrap(),
        now,
    ));
}

#[cfg_attr(test, tokio::test(flavor = "multi_thread", worker_threads = 2))]
#[cfg(feature = "full-controller-tests")]
async fn versioned_relay_controller_download_binds_token_to_agent() {
    let (state, secret, now) = configured_relay_test_state().await;
    let download_token = state
        .relay
        .write()
        .await
        .protocol
        .issue_download_tokens("Relay/Agent.txt", now)
        .into_iter()
        .next()
        .expect("registered agent download token")
        .1;
    let download_credential =
        super::relay::credential_for_test(secret, "edge-one", &download_token);
    let root = super::effective_downloads_dir(&state);
    fs::create_dir_all(root.join("Relay")).expect("relay download root");
    fs::write(root.join("Relay/Agent.txt"), b"relay payload").expect("relay fixture");
    let headers = super::RequestSecurityHeaders {
        x_relay_agent: Some("edge-one".to_owned()),
        x_relay_credential: Some(download_credential),
        remote_addr: Some("127.0.0.1:1".parse().unwrap()),
        ..super::RequestSecurityHeaders::default()
    };
    let download = Box::pin(super::route_http_request_with_headers(
        "GET",
        &format!("/api/v0/relay/controller/downloads/{download_token}"),
        None,
        "",
        &state,
        headers.clone(),
    ))
    .await
    .expect("relay download route");
    assert_eq!(download.status, "200 OK");
    assert_eq!(download.content_type, "application/octet-stream");
    assert!(download.body.is_empty());
    let mut stream = super::open_relay_controller_download(&state, &download_token, &headers)
        .await
        .expect("relay download stream");
    let mut payload = Vec::new();
    std::io::Read::read_to_end(&mut stream.file, &mut payload).expect("read relay payload");
    assert_eq!(payload, b"relay payload");

    let invalid = Box::pin(super::route_http_request_with_headers(
        "GET",
        "/api/v0/relay/controller/downloads/not-a-guid",
        None,
        "",
        &state,
        headers.clone(),
    ))
    .await
    .expect("invalid relay token route");
    assert_eq!(invalid.status, "400 Bad Request");
    let missing_credential = super::route_http_request(
        "GET",
        &format!("/api/v0/relay/controller/downloads/{download_token}"),
        None,
        "",
        &state,
    )
    .await
    .expect("missing relay credential route");
    assert_eq!(missing_credential.status, "401 Unauthorized");
    let _ = fs::remove_dir_all(root);
}

#[cfg_attr(test, tokio::test(flavor = "multi_thread", worker_threads = 2))]
#[cfg(feature = "full-controller-tests")]
async fn versioned_relay_controller_upload_tokens_are_one_use() {
    let (state, secret, now) = configured_relay_test_state().await;
    let (upload_token, upload_receiver) = state
        .relay
        .write()
        .await
        .protocol
        .begin_file_stream("edge-one", "Upload.flac", 0, now)
        .expect("relay upload stream");
    let upload_credential =
        super::relay::credential_for_test(secret, "edge-one", &upload_token.to_string());
    let upload_body = "--relay\r\nContent-Disposition: form-data; name=\"file\"; filename=\"Upload.flac\"\r\n\r\npayload\r\n--relay--\r\n";
    let upload_headers = super::RequestSecurityHeaders {
        content_type: Some("multipart/form-data; boundary=relay".to_owned()),
        x_relay_agent: Some("edge-one".to_owned()),
        x_relay_credential: Some(upload_credential),
        remote_addr: Some("127.0.0.1:1".parse().unwrap()),
        ..super::RequestSecurityHeaders::default()
    };
    let upload = Box::pin(super::route_http_request_with_headers(
        "POST",
        &format!("/api/v0/relay/controller/files/{upload_token}"),
        None,
        upload_body,
        &state,
        upload_headers.clone(),
    ))
    .await
    .expect("relay file upload route");
    assert_eq!(upload.status, "200 OK");
    let uploaded = upload_receiver
        .await
        .expect("relay upload receiver")
        .expect("relay upload result");
    assert_eq!(uploaded.filename, "Upload.flac");
    let stored_upload = state
        .config
        .state_dir
        .join("relay")
        .join("incoming")
        .join(format!("file-{}.part", upload_token.simple()));
    assert_eq!(
        fs::read(&stored_upload).expect("stored relay upload"),
        b"payload"
    );
    let (abandoned_token, abandoned_receiver) = state
        .relay
        .write()
        .await
        .protocol
        .begin_file_stream("edge-one", "Abandoned.flac", 0, now)
        .expect("abandoned relay upload stream");
    drop(abandoned_receiver);
    let mut abandoned_headers = upload_headers.clone();
    abandoned_headers.x_relay_credential = Some(super::relay::credential_for_test(
        secret,
        "edge-one",
        &abandoned_token.to_string(),
    ));
    let abandoned = Box::pin(super::route_http_request_with_headers(
        "POST",
        &format!("/api/v0/relay/controller/files/{abandoned_token}"),
        None,
        "--relay\r\nContent-Disposition: form-data; name=\"file\"; filename=\"Abandoned.flac\"\r\n\r\nstranded\r\n--relay--\r\n",
        &state,
        abandoned_headers,
    ))
    .await
    .expect("abandoned relay upload route");
    assert_eq!(abandoned.status, "503 Service Unavailable");
    assert!(!state
        .config
        .state_dir
        .join("relay")
        .join("incoming")
        .join(format!("file-{}.part", abandoned_token.simple()))
        .exists());
    let replay = Box::pin(super::route_http_request_with_headers(
        "POST",
        &format!("/api/v0/relay/controller/files/{upload_token}"),
        None,
        upload_body,
        &state,
        upload_headers.clone(),
    ))
    .await
    .expect("relay upload replay route");
    assert_eq!(replay.status, "401 Unauthorized");

    let share_token = state
        .relay
        .write()
        .await
        .protocol
        .issue_share_upload_token("edge-one", now)
        .expect("share token");
    let share_credential = super::relay::credential_for_test(secret, "edge-one", &share_token);
    let share_headers = super::RequestSecurityHeaders {
        content_type: Some("multipart/form-data; boundary=relay".to_owned()),
        x_relay_agent: Some("edge-one".to_owned()),
        x_relay_credential: Some(share_credential),
        remote_addr: Some("127.0.0.1:1".parse().unwrap()),
        ..super::RequestSecurityHeaders::default()
    };
    let database_source = state.config.state_dir.join("relay-test-source.db");
    super::relay::write_share_database(
        &database_source,
        super::ControllerProfile::Legacy,
        &[super::relay::RemoteShare {
            filename: "Remote/Agent.flac".to_owned(),
            size: 6,
        }],
    )
    .await
    .expect("relay share database");
    let database_bytes = fs::read(&database_source).expect("read relay share database");
    let mut share_body = Vec::new();
    share_body.extend_from_slice(
        b"--relay\r\nContent-Disposition: form-data; name=\"shares\"\r\n\r\n[]\r\n--relay\r\nContent-Disposition: form-data; name=\"database\"; filename=\"shares.db\"\r\n\r\n",
    );
    share_body.extend_from_slice(&database_bytes);
    share_body.extend_from_slice(b"\r\n--relay--\r\n");
    let shares = super::versioned_relay_request_bytes(
        "POST",
        &format!("/api/v0/relay/controller/shares/{share_token}"),
        &share_body,
        &share_headers,
        &state,
    )
    .await
    .expect("relay share upload route");
    assert_eq!(shares.status, "200 OK");
    assert_eq!(
        state
            .relay
            .read()
            .await
            .protocol
            .remote_file_for_agent("edge-one", "Remote/Agent.flac"),
        Some(("Remote/Agent.flac".to_owned(), 6))
    );
    let stored_share = state
        .config
        .state_dir
        .join("relay")
        .join("incoming")
        .join(format!(
            "share-{}.db",
            uuid::Uuid::parse_str(&share_token)
                .expect("share token UUID")
                .simple()
        ));
    assert_eq!(
        fs::read(&stored_share)
            .expect("stored relay database")
            .get(..16),
        Some(b"SQLite format 3\0".as_slice())
    );
    let _ = fs::remove_file(database_source);
}

#[cfg_attr(test, tokio::test(flavor = "multi_thread", worker_threads = 2))]
#[cfg(feature = "full-controller-tests")]
async fn relay_stream_requests_agent_and_serves_matching_upload() {
    let (state, secret, _now) = configured_relay_test_state().await;
    state
        .media_services
        .write()
        .await
        .features
        .streaming_relay_fallback = true;
    let source = state.config.state_dir.join("agent-source.flac");
    let payload = b"relay-stream-payload";
    fs::write(&source, payload).expect("relay source");
    let remote_filename = "Music/Agent.flac";
    state
        .relay
        .write()
        .await
        .protocol
        .record_share_upload(
            uuid::Uuid::new_v4(),
            "edge-one".to_owned(),
            1,
            vec![super::relay::RemoteShare {
                filename: remote_filename.to_owned(),
                size: payload.len() as u64,
            }],
            state.config.state_dir.join("remote-shares.db"),
            _now,
        )
        .expect("record relay stream share upload");
    let content_id = super::stable_content_hash(remote_filename, payload.len() as u64).to_string();

    let (hub_sender, mut hub_receiver) =
        tokio::sync::mpsc::channel(super::relay::HUB_OUTBOUND_QUEUE_CAPACITY);
    super::relay::register_hub_connection("connection-1".to_owned(), hub_sender);
    let open_state = Arc::clone(&state);
    let open = tokio::spawn(async move {
        super::open_relay_controller_stream(&open_state, &content_id, Some("agentName=edge-one"))
            .await
    });
    let info_invocation = hub_receiver
        .recv()
        .await
        .expect("request file info invocation");
    let info_invocation: serde_json::Value =
        serde_json::from_str(&info_invocation).expect("info invocation");
    assert_eq!(info_invocation["target"], "RequestFileInfo");
    let info_token = info_invocation["arguments"][1]
        .as_str()
        .expect("file info token")
        .parse::<uuid::Uuid>()
        .expect("file info UUID");
    assert!(state.relay.write().await.protocol.complete_file_info(
        "connection-1",
        info_token,
        true,
        payload.len() as u64
    ));

    let invocation = hub_receiver
        .recv()
        .await
        .expect("request file upload invocation");
    let invocation: serde_json::Value = serde_json::from_str(&invocation).expect("invocation");
    assert_eq!(invocation["target"], "RequestFileUpload");
    assert_eq!(invocation["arguments"][0], remote_filename);
    let token = invocation["arguments"][2]
        .as_str()
        .expect("file stream token")
        .to_owned();
    let credential = super::relay::credential_for_test(secret, "edge-one", &token);
    let headers = super::RequestSecurityHeaders {
        content_type: Some("multipart/form-data; boundary=relay".to_owned()),
        x_relay_agent: Some("edge-one".to_owned()),
        x_relay_credential: Some(credential),
        remote_addr: Some("127.0.0.1:1".parse().unwrap()),
        ..super::RequestSecurityHeaders::default()
    };
    let body = b"--relay\r\nContent-Disposition: form-data; name=\"file\"; filename=\"Music/Agent.flac\"\r\n\r\nrelay-stream-payload\r\n--relay--\r\n";
    let response = super::versioned_relay_request_bytes(
        "POST",
        &format!("/api/v0/relay/controller/files/{token}"),
        body,
        &headers,
        &state,
    )
    .await
    .expect("relay upload response");
    assert_eq!(response.status, "200 OK");
    let mut stream = open
        .await
        .expect("relay stream task")
        .expect("relay stream");
    let mut received = Vec::new();
    std::io::Read::read_to_end(&mut stream.file, &mut received).expect("read relay stream");
    assert_eq!(received, payload);
    if let Some(path) = stream.cleanup_path {
        let _ = fs::remove_file(path);
    }
    super::relay::unregister_hub_connection("connection-1");
    let _ = fs::remove_file(source);
}

#[cfg_attr(test, tokio::test(flavor = "multi_thread", worker_threads = 2))]
#[cfg(feature = "full-controller-tests")]
async fn relay_signalr_hub_authenticates_agent_and_issues_share_token() {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::{connect_async, tungstenite::Message};

    let (state, secret, _) = configured_relay_test_state().await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("relay hub listener");
    let address = listener.local_addr().expect("relay hub address");
    let server_state = Arc::clone(&state);
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("relay hub connection");
        super::handle_http_connection(stream, server_state).await
    });

    let (mut socket, _) = connect_async(format!("ws://{address}/hub/relay"))
        .await
        .expect("relay SignalR connection");
    socket
        .send(Message::Text("{}\x1e".to_owned().into()))
        .await
        .expect("relay SignalR handshake");
    let handshake = socket
        .next()
        .await
        .expect("handshake response")
        .expect("handshake websocket message");
    assert_eq!(handshake.to_text().expect("handshake text"), "{}\x1e");

    let challenge = socket
        .next()
        .await
        .expect("challenge response")
        .expect("challenge websocket message");
    let challenge = challenge
        .to_text()
        .expect("challenge text")
        .trim_end_matches('\x1e');
    let challenge: serde_json::Value = serde_json::from_str(challenge).expect("challenge JSON");
    assert_eq!(challenge["target"], "Challenge");
    let challenge = challenge["arguments"][0].as_str().expect("challenge token");
    let credential = super::relay::credential_for_test(secret, "edge-one", challenge);
    let login = serde_json::json!({
        "type": 1,
        "invocationId": "1",
        "target": "Login",
        "arguments": ["edge-one", credential],
    });
    socket
        .send(Message::Text(format!("{}\x1e", login).into()))
        .await
        .expect("relay login");
    let login_result = socket
        .next()
        .await
        .expect("login completion")
        .expect("login websocket message");
    let login_result: serde_json::Value = serde_json::from_str(
        login_result
            .to_text()
            .expect("login completion text")
            .trim_end_matches('\x1e'),
    )
    .expect("login completion JSON");
    assert_eq!(login_result["type"], 3);
    assert!(login_result.get("error").is_none());

    let begin = serde_json::json!({
        "type": 1,
        "invocationId": "2",
        "target": "BeginShareUpload",
        "arguments": [],
    });
    socket
        .send(Message::Text(format!("{}\x1e", begin).into()))
        .await
        .expect("relay share token request");
    let token_result = socket
        .next()
        .await
        .expect("share token completion")
        .expect("share token websocket message");
    let token_result: serde_json::Value = serde_json::from_str(
        token_result
            .to_text()
            .expect("share token completion text")
            .trim_end_matches('\x1e'),
    )
    .expect("share token completion JSON");
    let token = token_result["result"].as_str().expect("share upload token");
    let token_credential = super::relay::credential_for_test(secret, "edge-one", token);
    let settings = state.advanced_networking.read().await.relay.clone();
    assert!(state
        .relay
        .write()
        .await
        .protocol
        .validate_share_upload(
            &settings,
            super::relay::credential_scheme(state.config.controller_profile),
            uuid::Uuid::parse_str(token).expect("share token UUID"),
            &token_credential,
            super::unix_timestamp(),
        )
        .is_some());

    let download_token = state
        .relay
        .write()
        .await
        .protocol
        .issue_download_tokens("Relay/Completed.flac", super::unix_timestamp())
        .into_iter()
        .next()
        .expect("download notification token")
        .1;
    assert!(super::relay::send_hub_invocation(
        &state.relay.read().await.protocol,
        "edge-one",
        "NotifyFileDownloadCompleted",
        vec![
            serde_json::Value::String("Relay/Completed.flac".to_owned()),
            serde_json::Value::String(download_token),
        ],
    ));
    let notification = socket
        .next()
        .await
        .expect("download notification")
        .expect("download notification websocket message");
    let notification: serde_json::Value = serde_json::from_str(
        notification
            .to_text()
            .expect("download notification text")
            .trim_end_matches('\x1e'),
    )
    .expect("download notification JSON");
    assert_eq!(notification["target"], "NotifyFileDownloadCompleted");

    socket.close(None).await.expect("relay websocket close");
    server
        .await
        .expect("relay hub server task")
        .expect("relay hub server");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn solid_webid_route_extracts_oidc_issuers_from_profile() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let (state, _receiver) = test_state();
    {
        let mut media_services = state.media_services.write().await;
        media_services.solid.allow_insecure_http = true;
        media_services.solid.allow_localhost_for_web_id = true;
        media_services.solid.allowed_hosts = vec!["127.0.0.1".to_owned()];
    }

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind profile fixture");
    let port = listener
        .local_addr()
        .expect("profile fixture address")
        .port();
    let web_id = format!("http://127.0.0.1:{port}/profile/card#me");
    let profile = format!(
        "@prefix solid: <http://www.w3.org/ns/solid/terms#>.\n<{web_id}> solid:oidcIssuer <https://issuer.example/oidc>.\n"
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
            profile.len(), profile
        );
        stream
            .write_all(response.as_bytes())
            .await
            .expect("write profile response");
    });

    let response = super::route_http_request(
        "POST",
        "/api/solid/resolve-webid",
        None,
        &serde_json::json!({"webId": web_id}).to_string(),
        &state,
    )
    .await
    .expect("resolve WebID");
    server.await.expect("profile fixture task");

    assert_eq!(response.status, "200 OK");
    let response_json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
    assert_eq!(response_json["webId"], web_id);
    assert_eq!(
        response_json["oidcIssuers"],
        serde_json::json!(["https://issuer.example/oidc"])
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn solid_client_id_document_is_anonymous_and_uses_configured_origin() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));
    {
        let mut media_services = state.media_services.write().await;
        media_services.solid.client_id_url =
            Some("https://solid.example/clientid.jsonld".to_owned());
        media_services.solid.redirect_path = "/oidc/callback".to_owned();
    }

    let response = super::route_http_request("GET", "/solid/clientid.jsonld", None, "", &state)
        .await
        .expect("client ID document");
    assert_eq!(response.status, "200 OK");
    assert_eq!(response.content_type, "application/ld+json");
    let document = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
    assert_eq!(
        document["@context"],
        "https://www.w3.org/ns/solid/oidc-context.jsonld"
    );
    assert_eq!(
        document["client_id"],
        "https://solid.example/clientid.jsonld"
    );
    assert_eq!(document["client_name"], "slskdn");
    assert_eq!(document["application_type"], "web");
    assert_eq!(document["scope"], "openid webid");
    assert_eq!(
        document["redirect_uris"],
        serde_json::json!(["https://solid.example/oidc/callback"])
    );

    state.media_services.write().await.solid.client_id_url = None;
    let missing = super::route_http_request("GET", "/solid/clientid.jsonld", None, "", &state)
        .await
        .expect("missing client ID document");
    assert_eq!(missing.status, "404 Not Found");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn solid_webid_policy_blocks_localhost_without_explicit_test_opt_in() {
    let (state, _receiver) = test_state();
    {
        let mut media_services = state.media_services.write().await;
        media_services.solid.allow_insecure_http = true;
        media_services.solid.allow_localhost_for_web_id = false;
        media_services.solid.allowed_hosts = vec!["127.0.0.1".to_owned()];
    }
    let response = super::route_http_request(
        "POST",
        "/api/solid/resolve-webid",
        None,
        r#"{"webId":"http://127.0.0.1:1/profile/card#me"}"#,
        &state,
    )
    .await
    .expect("localhost policy response");
    assert_eq!(response.status, "400 Bad Request");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&response.body).unwrap(),
        serde_json::json!({"error": "WebID resolution was blocked by policy."})
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn versioned_solid_resolution_uses_native_problem_details() {
    let (state, _receiver) = test_state();
    let invalid =
        super::route_http_request("POST", "/api/v0/solid/resolve-webid", None, "{}", &state)
            .await
            .expect("invalid WebID response");
    assert_eq!(invalid.status, "400 Bad Request");
    assert_eq!(invalid.content_type, "application/problem+json");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&invalid.body).unwrap(),
        serde_json::json!({
            "type": "about:blank",
            "title": "Invalid WebID",
            "status": 400,
            "detail": "WebId must be an absolute URI."
        })
    );

    let blocked = super::route_http_request(
        "POST",
        "/api/v0/solid/resolve-webid",
        None,
        r#"{"webId":"https://example.com/profile#me"}"#,
        &state,
    )
    .await
    .expect("blocked WebID response");
    assert_eq!(blocked.status, "400 Bad Request");
    let blocked_json = serde_json::from_str::<serde_json::Value>(&blocked.body).unwrap();
    assert_eq!(blocked_json["title"], "Solid fetch blocked");
    assert_eq!(blocked_json["status"], 400);
    assert_eq!(
        blocked_json["detail"],
        "WebID resolution was blocked by policy."
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn browse_store_bounds_records_and_entries_but_updates_existing_users() {
    let mut browse = super::BrowseStore::with_limits(1, 2);
    browse.request("alice".to_owned()).unwrap();
    assert!(browse.request("bob".to_owned()).is_none());

    let entries = (0..3)
        .map(|index| super::BrowseEntry {
            path_encoding: Default::default(),
            filename: format!("file-{index}.flac"),
            size: index,
            extension: "flac".to_owned(),
        })
        .collect();
    let record = browse
        .add_entries("alice".to_owned(), entries, false)
        .unwrap();
    assert_eq!(record.entries.len(), 2);
    assert_eq!(record.entries[0].filename, "file-0.flac");
    assert_eq!(record.entries[1].filename, "file-1.flac");
    assert!(browse
        .add_entries("bob".to_owned(), Vec::new(), true)
        .is_none());
    assert_eq!(browse.records.len(), 1);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn browse_store_bounds_text_and_aggregate_entries() {
    let oversized_username = "é".repeat(super::browse_store::MAX_BROWSE_USERNAME_BYTES);
    let mut browse =
        super::BrowseStore::with_limits(2, super::browse_store::MAX_TOTAL_BROWSE_ENTRIES + 1);
    let requested = browse.request(oversized_username.clone()).unwrap();
    assert!(requested.username.len() <= super::browse_store::MAX_BROWSE_USERNAME_BYTES);
    browse.request("other".to_owned()).unwrap();

    let oversized_entry = super::BrowseEntry {
        path_encoding: Default::default(),
        filename: "f".repeat(super::browse_store::MAX_BROWSE_FILENAME_BYTES + 1),
        size: 1,
        extension: "e".repeat(super::browse_store::MAX_BROWSE_EXTENSION_BYTES + 1),
    };
    let record = browse
        .add_entries(oversized_username.clone(), vec![oversized_entry], false)
        .unwrap();
    assert_eq!(
        record.entries[0].filename.len(),
        super::browse_store::MAX_BROWSE_FILENAME_BYTES
    );
    assert_eq!(
        record.entries[0].extension.len(),
        super::browse_store::MAX_BROWSE_EXTENSION_BYTES
    );

    browse.records[0].entries = (0..super::browse_store::MAX_TOTAL_BROWSE_ENTRIES)
        .map(|index| super::BrowseEntry {
            path_encoding: Default::default(),
            filename: format!("file-{index}"),
            size: 1,
            extension: String::new(),
        })
        .collect();
    let record = browse
        .add_entries(
            "other".to_owned(),
            vec![super::BrowseEntry {
                path_encoding: Default::default(),
                filename: "rejected".to_owned(),
                size: 1,
                extension: String::new(),
            }],
            true,
        )
        .unwrap();
    assert!(record.entries.is_empty());
    assert_eq!(
        browse.total_entries(),
        super::browse_store::MAX_TOTAL_BROWSE_ENTRIES
    );
    assert!(browse.get(&oversized_username).is_some());
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
    feature = "bounded-controller-api-tests-1"
))]
async fn controller_api_differential_user_browse_api_requests_and_ingests_entries() {
    let (state, mut receiver) = test_state();

    let unavailable = super::route_http_request(
        "POST",
        "/api/v0/users/friend/browse/request",
        None,
        "",
        &state,
    )
    .await
    .expect("disconnected browse request");
    assert_eq!(unavailable.status, "503 Service Unavailable");
    assert!(unavailable
        .body
        .contains("Soulseek server connection is not ready"));
    state.session.write().await.state = "connected";

    let requested = super::route_http_request(
        "POST",
        "/api/v0/users/friend/browse/request",
        None,
        "",
        &state,
    )
    .await
    .expect("browse request");
    assert_eq!(requested.status, "202 Accepted");
    assert!(requested.body.contains("\"status\":\"requested\""));
    assert_eq!(
        receiver.try_recv().expect("browse command"),
        super::SessionCommand::BrowseUser("friend".to_owned())
    );

    let folder_requested = super::route_http_request(
        "POST",
        "/api/v0/users/friend/browse/folder",
        None,
        "{\"folder\":\"Remote/Album\"}",
        &state,
    )
    .await
    .expect("folder browse request");
    assert_eq!(folder_requested.status, "202 Accepted");
    assert!(folder_requested.body.contains("\"status\":\"requested\""));
    assert!(folder_requested
        .body
        .contains("\"folder\":\"Remote/Album\""));
    assert_eq!(
        receiver.try_recv().expect("folder browse command"),
        super::SessionCommand::BrowseFolder {
            username: "friend".to_owned(),
            folder: "Remote/Album".to_owned()
        }
    );

    let ingested = super::route_http_request(
        "POST",
        "/api/v0/browse-responses",
        None,
        "{\"username\":\"friend\",\"complete\":false,\"entries\": [{\"filename\":\"Remote/Album/Song.flac\",\"size\":123}]}",
        &state,
    )
    .await
    .expect("browse ingest");
    assert_eq!(ingested.status, "200 OK");
    assert!(ingested.body.contains("\"status\":\"partial\""));
    assert!(ingested.body.contains("\"count\":1"));
    assert!(ingested.body.contains("\"total_bytes\":123"));
    assert!(ingested.body.contains("\"extension\":\"flac\""));

    let listed = super::route_http_request(
        "GET",
        "/api/v0/browse?status=partial&q=friend",
        None,
        "",
        &state,
    )
    .await
    .expect("partial browse list");
    assert_eq!(listed.status, "200 OK");
    assert!(listed.body.contains("\"filtered_count\":1"));

    let ingested = super::route_http_request(
        "POST",
        "/api/v0/browse-responses",
        None,
        "{\"username\":\"friend\",\"entries\":[{\"filename\":\"Remote/Album/Cover.jpg\",\"size\":10,\"extension\":\"jpg\"}]}",
        &state,
    )
    .await
    .expect("browse complete ingest");
    assert_eq!(ingested.status, "200 OK");
    assert!(ingested.body.contains("\"status\":\"ready\""));
    assert!(ingested.body.contains("\"count\":2"));
    assert!(ingested.body.contains("\"total_bytes\":133"));
    assert!(ingested.body.contains("\"extension\":\"jpg\""));

    let fetched = super::route_http_request("GET", "/api/v0/users/friend/browse", None, "", &state)
        .await
        .expect("browse fetch");
    assert_eq!(fetched.status, "200 OK");
    assert!(fetched.content_type.contains("application/json"));
    let fetched_json = serde_json::from_str::<serde_json::Value>(&fetched.body).unwrap();
    assert_eq!(fetched_json["directoryCount"], 1);
    assert_eq!(fetched_json["directories"][0]["name"], "Remote/Album");
    assert_eq!(
        fetched_json["directories"][0]["files"][0]["filename"],
        "Song.flac"
    );

    let controller_browse =
        super::route_http_request("GET", "/api/users/friend/browse", None, "", &state)
            .await
            .expect("slskd browse fetch");
    assert_eq!(controller_browse.status, "200 OK");
    let controller_browse_json =
        serde_json::from_str::<serde_json::Value>(&controller_browse.body).unwrap();
    assert_eq!(controller_browse_json["directoryCount"], 1);
    assert_eq!(
        controller_browse_json["directories"][0]["name"],
        "Remote/Album"
    );
    assert_eq!(controller_browse_json["directories"][0]["fileCount"], 2);
    assert_eq!(
        controller_browse_json["directories"][0]["files"][0]["filename"],
        "Song.flac"
    );

    let controller_directory = super::route_http_request(
        "POST",
        "/api/users/friend/directory",
        None,
        "{\"directory\":\"Remote/Album\"}",
        &state,
    )
    .await
    .expect("slskd directory fetch");
    assert_eq!(controller_directory.status, "200 OK");
    let controller_directory_json =
        serde_json::from_str::<serde_json::Value>(&controller_directory.body).unwrap();
    assert_eq!(controller_directory_json[0]["name"], "Remote/Album");
    assert_eq!(controller_directory_json[0]["fileCount"], 2);

    let listed = super::route_http_request(
        "GET",
        "/api/v0/browse?status=ready&q=friend&limit=1",
        None,
        "",
        &state,
    )
    .await
    .expect("browse list");
    assert_eq!(listed.status, "200 OK");
    assert!(listed.body.contains("\"count\":1"));
    assert!(listed.body.contains("\"filtered_count\":1"));
    assert!(listed.body.contains("\"limit\":1"));

    let failed = super::route_http_request(
        "POST",
        "/api/v0/users/friend/browse/fail",
        None,
        "{\"reason\":\"peer timed out\"}",
        &state,
    )
    .await
    .expect("browse fail");
    assert_eq!(failed.status, "200 OK");
    assert!(failed.body.contains("\"status\":\"failed\""));
    assert!(failed.body.contains("\"reason\":\"peer timed out\""));

    let listed = super::route_http_request(
        "GET",
        "/api/v0/browse?status=failed&q=friend",
        None,
        "",
        &state,
    )
    .await
    .expect("failed browse list");
    assert_eq!(listed.status, "200 OK");
    assert!(listed.body.contains("\"filtered_count\":1"));

    let ledger = vec![
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/users/{username}/browse",
            "case": "nominal-status-headers-body",
            "pass": true,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/users/{username}/browse",
            "case": "populated-dynamic-state",
            "pass": true,
        }),
    ];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("user_browse_projection.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn one_shot_user_requests_reject_before_mutation_when_dispatch_is_unavailable() {
    for (path, body) in [
        ("/api/v1/users/friend/stats/request", ""),
        ("/api/v0/users/friend/browse/request", ""),
        (
            "/api/v0/users/friend/browse/folder",
            r#"{"folder":"Remote/Album"}"#,
        ),
    ] {
        let (state, receiver) = test_state();
        state.session.write().await.state = "connected";
        drop(receiver);

        let response = super::route_http_request("POST", path, None, body, &state)
            .await
            .expect("unavailable dispatch response");
        assert_eq!(response.status, "503 Service Unavailable", "{path}");
        assert!(
            response.body.contains("session manager is not running"),
            "{path}"
        );
        assert!(state.browse.read().await.records.is_empty(), "{path}");
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn browse_routes_roll_back_when_persistence_fails() {
    for (path, body) in [
        ("/api/v0/users/friend/browse/request", ""),
        (
            "/api/v0/users/friend/browse/folder",
            r#"{"folder":"Remote/Album"}"#,
        ),
        ("/api/v0/users/friend/browse/fail", r#"{"reason":"failed"}"#),
        (
            "/api/v0/users/friend/browse/cancel",
            r#"{"reason":"cancelled"}"#,
        ),
        (
            "/api/v0/browse-responses",
            r#"{"username":"friend","entries":[{"filename":"Song.flac"}]}"#,
        ),
    ] {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, mut receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        state.session.write().await.state = "connected";
        let previous = state.browse.read().await.clone();
        db.close_for_test().await;

        let response = super::route_http_request("POST", path, None, body, &state)
            .await
            .expect("failed browse persistence response");
        assert_eq!(response.status, "503 Service Unavailable", "{path}");
        assert!(
            response.body.contains("browse persistence failed"),
            "{path}"
        );
        assert_eq!(*state.browse.read().await, previous, "{path}");
        assert!(receiver.try_recv().is_err(), "{path}");
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn async_browse_projection_reports_persistence_failure() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    let record = state
        .browse
        .write()
        .await
        .request("friend".to_owned())
        .unwrap();
    db.close_for_test().await;

    super::browse_runtime::persist_browse_projection(&state, &record).await;
    assert!(state
        .session
        .read()
        .await
        .last_error
        .as_deref()
        .is_some_and(|error| error.contains("browse persistence")));
    assert!(state.browse.read().await.get("friend").is_some());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn browse_response_api_accepts_single_flattened_entry() {
    let (state, _receiver) = test_state();

    let response = super::route_http_request(
        "POST",
        "/api/v0/browse-responses",
        None,
        "{\"username\":\"friend\",\"filename\":\"Remote/One.mp3\",\"size\":7}",
        &state,
    )
    .await
    .expect("flat browse response");

    assert_eq!(response.status, "200 OK");
    assert!(response.body.contains("\"count\":1"));
    assert!(response.body.contains("\"extension\":\"mp3\""));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn browse_response_api_accepts_controller_directory_payload() {
    let (state, _receiver) = test_state();

    let response = super::route_http_request(
        "POST",
        "/api/v0/browse-responses",
        None,
        "{\"username\":\"friend\",\"directories\":[{\"name\":\"Remote/Album\",\"files\":[{\"filename\":\"One.flac\",\"size\":1},{\"filename\":\"Remote/Album/Two.mp3\",\"size\":2}]}]}",
        &state,
    )
    .await
    .expect("slskd browse response");

    assert_eq!(response.status, "200 OK");
    let record_json = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
    assert_eq!(record_json["count"], 2);
    assert_eq!(
        record_json["entries"][0]["filename"],
        "Remote/Album/One.flac"
    );
    assert_eq!(
        record_json["entries"][1]["filename"],
        "Remote/Album/Two.mp3"
    );

    let root = super::route_http_request("GET", "/api/users/friend/browse", None, "", &state)
        .await
        .expect("slskd browse root");
    let root_json = serde_json::from_str::<serde_json::Value>(&root.body).unwrap();
    assert_eq!(root_json["directoryCount"], 1);
    assert_eq!(root_json["directories"][0]["name"], "Remote/Album");
    assert_eq!(root_json["directories"][0]["fileCount"], 2);

    let directory = super::route_http_request(
        "POST",
        "/api/users/friend/directory",
        None,
        "{\"directory\":\"Remote/Album\"}",
        &state,
    )
    .await
    .expect("slskd directory");
    let directory_json = serde_json::from_str::<serde_json::Value>(&directory.body).unwrap();
    assert_eq!(
        directory_json[0]["files"][0]["filename"],
        "Remote/Album/One.flac"
    );
    assert_eq!(
        directory_json[0]["files"][1]["filename"],
        "Remote/Album/Two.mp3"
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
    feature = "bounded-controller-api-tests-1"
))]
async fn controller_api_differential_controller_browse_status_tracks_request_failure_and_completion(
) {
    let (state, _receiver) = test_state();
    state.session.write().await.state = "connected";

    let requested = super::route_http_request(
        "POST",
        "/api/v0/users/friend/browse/request",
        None,
        "",
        &state,
    )
    .await
    .expect("request browse");
    assert_eq!(requested.status, "202 Accepted");

    let status = super::route_http_request(
        "GET",
        "/api/v0/users/friend/browse/status",
        None,
        "",
        &state,
    )
    .await
    .expect("requested browse status");
    assert_eq!(status.status, "200 OK");
    assert!(status.content_type.contains("application/json"));
    let status_json = serde_json::from_str::<serde_json::Value>(&status.body).unwrap();
    assert_eq!(status_json["status"], "requested");
    assert_eq!(status_json["state"], "InProgress");
    assert_eq!(status_json["isComplete"], false);
    assert_eq!(status_json["percentComplete"], 0.0);

    let failed = super::route_http_request(
        "POST",
        "/api/v0/users/friend/browse/fail",
        None,
        "{\"reason\":\"timed out\"}",
        &state,
    )
    .await
    .expect("fail browse");
    assert_eq!(failed.status, "200 OK");

    let status = super::route_http_request(
        "GET",
        "/api/v0/users/friend/browse/status",
        None,
        "",
        &state,
    )
    .await
    .expect("failed browse status");
    assert_eq!(status.status, "200 OK");
    assert!(status.content_type.contains("application/json"));
    let status_json = serde_json::from_str::<serde_json::Value>(&status.body).unwrap();
    assert_eq!(status_json["status"], "failed");
    assert_eq!(status_json["state"], "Failed");
    assert_eq!(status_json["reason"], "browse failed");

    super::route_http_request(
        "POST",
        "/api/v0/browse-responses",
        None,
        "{\"username\":\"friend\",\"entries\":[{\"filename\":\"Remote/Album/One.flac\",\"size\":7}]}",
        &state,
    )
    .await
    .expect("complete browse");

    let status = super::route_http_request(
        "GET",
        "/api/v0/users/friend/browse/status",
        None,
        "",
        &state,
    )
    .await
    .expect("completed browse status");
    assert_eq!(status.status, "200 OK");
    assert!(status.content_type.contains("application/json"));
    let status_json = serde_json::from_str::<serde_json::Value>(&status.body).unwrap();
    assert_eq!(status_json["status"], "ready");
    assert_eq!(status_json["state"], "Completed");
    assert_eq!(status_json["isComplete"], true);
    assert_eq!(status_json["size"], 7);
    assert_eq!(status_json["bytesTransferred"], 7);
    assert_eq!(status_json["fileCount"], 1);
    assert_eq!(status_json["directoryCount"], 1);

    let ledger = vec![
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/users/{username}/browse/status",
            "case": "nominal-status-headers-body",
            "pass": true,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/users/{username}/browse/status",
            "case": "populated-dynamic-state",
            "pass": true,
        }),
    ];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("user_browse_status_projection.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn browse_cancel_route_preserves_terminal_status_projection() {
    let (state, _receiver) = test_state();
    state.session.write().await.state = "connected";

    super::route_http_request(
        "POST",
        "/api/v0/users/friend/browse/request",
        None,
        "",
        &state,
    )
    .await
    .expect("request browse");

    let cancelled = super::route_http_request(
        "POST",
        "/api/v0/users/friend/browse/cancel",
        None,
        r#"{"reason":"user navigated away"}"#,
        &state,
    )
    .await
    .expect("cancel browse");
    assert_eq!(cancelled.status, "200 OK");
    let cancelled_json = serde_json::from_str::<serde_json::Value>(&cancelled.body).unwrap();
    assert_eq!(cancelled_json["status"], "cancelled");
    assert_eq!(cancelled_json["reason"], "browse cancelled");

    let status =
        super::route_http_request("GET", "/api/users/friend/browse/status", None, "", &state)
            .await
            .expect("cancelled browse status");
    let status_json = serde_json::from_str::<serde_json::Value>(&status.body).unwrap();
    assert_eq!(status_json["status"], "cancelled");
    assert_eq!(status_json["state"], "Cancelled");
    assert_eq!(status_json["reason"], "browse cancelled");

    let listed =
        super::route_http_request("GET", "/api/v0/browse?status=cancelled", None, "", &state)
            .await
            .expect("cancelled browse list");
    let listed_json = serde_json::from_str::<serde_json::Value>(&listed.body).unwrap();
    assert_eq!(listed_json["filtered_count"], 1);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn controller_user_browse_routes_page_directories_and_directory_files() {
    let (state, _receiver) = test_state();
    super::route_http_request(
        "POST",
        "/api/v0/browse-responses",
        None,
        "{\"username\":\"friend\",\"entries\":[{\"filename\":\"Remote/Album/One.flac\",\"size\":1},{\"filename\":\"Remote/Album/Two.flac\",\"size\":2},{\"filename\":\"Remote/Album/Three.flac\",\"size\":3},{\"filename\":\"Remote/Other/Four.flac\",\"size\":4}]}",
        &state,
    )
    .await
    .expect("browse ingest");

    let root = super::route_http_request(
        "GET",
        "/api/users/friend/browse?offset=1&limit=1",
        None,
        "",
        &state,
    )
    .await
    .expect("paged root browse");
    assert_eq!(root.status, "200 OK");
    let root_json = serde_json::from_str::<serde_json::Value>(&root.body).unwrap();
    assert_eq!(root_json["directoryCount"], 2);
    assert_eq!(root_json["filteredDirectoryCount"], 2);
    assert_eq!(root_json["fileCount"], 4);
    assert_eq!(root_json["filteredFileCount"], 4);
    assert_eq!(root_json["totalBytes"], 10);
    assert_eq!(root_json["offset"], 1);
    assert_eq!(root_json["limit"], 1);
    assert_eq!(root_json["directories"].as_array().unwrap().len(), 1);
    assert_eq!(root_json["directories"][0]["name"], "Remote/Other");
    assert_eq!(root_json["directories"][0]["filteredFileCount"], 1);
    assert_eq!(root_json["directories"][0]["totalBytes"], 4);

    let directory = super::route_http_request(
        "POST",
        "/api/users/friend/directory?offset=1&limit=1",
        None,
        "{\"directory\":\"Remote/Album\"}",
        &state,
    )
    .await
    .expect("paged directory browse");
    assert_eq!(directory.status, "200 OK");
    let directory_json = serde_json::from_str::<serde_json::Value>(&directory.body).unwrap();
    assert_eq!(directory_json[0]["name"], "Remote/Album");
    assert_eq!(directory_json[0]["fileCount"], 3);
    assert_eq!(directory_json[0]["filteredFileCount"], 3);
    assert_eq!(directory_json[0]["totalBytes"], 6);
    assert_eq!(directory_json[0]["offset"], 1);
    assert_eq!(directory_json[0]["limit"], 1);
    assert_eq!(directory_json[0]["files"].as_array().unwrap().len(), 1);
    assert_eq!(
        directory_json[0]["files"][0]["filename"],
        "Remote/Album/Two.flac"
    );
}
