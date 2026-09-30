//! Controller full federation contracts ownership.

use super::*;

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn activitypub_music_actor_and_webfinger_match_target_discovery_contract() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("FEDERATION_ENABLED", "true")
            .with("FEDERATION_MODE", "Public")
            .with("FEDERATION_DOMAIN", "social.example")
            .with("FEDERATION_BASE_URL", "https://social.example/")
            .with("FEDERATION_PAGE_SIZE", "10"),
    );

    let actor = crate::route_http_request("GET", "/actors/music", None, "", &state)
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

    let acct = crate::route_http_request(
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

    let https_resource = crate::route_http_request(
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

    let filtered = crate::route_http_request(
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

    let generic = crate::route_http_request("GET", "/actors/books", None, "", &state)
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
pub(super) fn activitypub_relationship_collections_track_target_lifecycle() {
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

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn actors_routes_are_gated_by_the_social_federation_feature_flag() {
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
        let response = crate::route_http_request("GET", path, None, "", &state)
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
        let response = crate::route_http_request("POST", path, None, "{}", &state)
            .await
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        assert_eq!(response.status, "404 Not Found", "{path}");
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn solid_webid_route_extracts_oidc_issuers_from_profile() {
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

    let response = crate::route_http_request(
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
pub(super) async fn solid_client_id_document_is_anonymous_and_uses_configured_origin() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));
    {
        let mut media_services = state.media_services.write().await;
        media_services.solid.client_id_url =
            Some("https://solid.example/clientid.jsonld".to_owned());
        media_services.solid.redirect_path = "/oidc/callback".to_owned();
    }

    let response = crate::route_http_request("GET", "/solid/clientid.jsonld", None, "", &state)
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
    let missing = crate::route_http_request("GET", "/solid/clientid.jsonld", None, "", &state)
        .await
        .expect("missing client ID document");
    assert_eq!(missing.status, "404 Not Found");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn solid_webid_policy_blocks_localhost_without_explicit_test_opt_in() {
    let (state, _receiver) = test_state();
    {
        let mut media_services = state.media_services.write().await;
        media_services.solid.allow_insecure_http = true;
        media_services.solid.allow_localhost_for_web_id = false;
        media_services.solid.allowed_hosts = vec!["127.0.0.1".to_owned()];
    }
    let response = crate::route_http_request(
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

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn activitypub_signature_header_parses_declared_fields_and_rejects_incomplete_headers() {
    let parsed = crate::parse_activitypub_signature_header(
        r#"keyId="https://peer.example/actors/alice#main-key",algorithm="ed25519",headers="(request-target) host date digest",signature="c2ln",created="1700000000""#,
    )
    .expect("well-formed signature header");
    assert_eq!(parsed.key_id, "https://peer.example/actors/alice#main-key");
    assert_eq!(parsed.algorithm, "ed25519");
    assert_eq!(
        parsed.headers,
        vec!["(request-target)", "host", "date", "digest"]
    );
    assert_eq!(parsed.signature_b64, "c2ln");
    assert_eq!(parsed.created, Some(1_700_000_000));

    for missing in [
        r#"algorithm="ed25519",headers="host",signature="c2ln""#,
        r#"keyId="k",headers="host",signature="c2ln""#,
        r#"keyId="k",algorithm="ed25519",signature="c2ln""#,
        r#"keyId="k",algorithm="ed25519",headers="host""#,
        "",
    ] {
        assert!(
            crate::parse_activitypub_signature_header(missing).is_none(),
            "{missing}"
        );
    }
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn activitypub_signature_required_headers_and_freshness_match_oracle_contract() {
    let owned = |values: &[&str]| {
        values
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
    };

    assert!(crate::activitypub_signature_has_required_headers(
        &owned(&["(request-target)", "host", "date"]),
        false,
    ));
    assert!(!crate::activitypub_signature_has_required_headers(
        &owned(&["(request-target)", "host", "date"]),
        true, // body-bearing without a signed digest must fail
    ));
    assert!(crate::activitypub_signature_has_required_headers(
        &owned(&["(request-target)", "host", "date", "digest"]),
        true,
    ));
    assert!(crate::activitypub_signature_has_required_headers(
        &owned(&["(request-target)", "host", "(created)"]),
        false,
    ));
    assert!(!crate::activitypub_signature_has_required_headers(
        &owned(&["(request-target)", "host"]), // no date or (created) at all
        false,
    ));
    assert!(!crate::activitypub_signature_has_required_headers(
        &owned(&["(request-target)", "Host", "date", "host"]), // duplicate, case-insensitive
        false,
    ));

    let now = i64::try_from(crate::unix_timestamp()).unwrap();
    assert!(crate::activitypub_signature_created_is_fresh(now));
    assert!(crate::activitypub_signature_created_is_fresh(now - 299));
    assert!(!crate::activitypub_signature_created_is_fresh(now - 301));
    assert!(!crate::activitypub_signature_created_is_fresh(now + 301));
    assert!(!crate::activitypub_signature_date_is_fresh("not a date"));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn activitypub_pkix_ed25519_key_round_trips_through_slskrs_own_encoder() {
    let (state, _receiver) = test_state();
    let pem = crate::activitypub_public_key_pem(&state);
    let decoded = crate::decode_ed25519_pkix_public_key(&pem).expect("decode slskR's own PKIX key");
    assert_eq!(
        decoded.as_bytes(),
        state.capability_signing_key.verifying_key().as_bytes()
    );

    assert!(crate::decode_ed25519_pkix_public_key("not a pem at all").is_err());
    assert!(crate::decode_ed25519_pkix_public_key(
        "-----BEGIN PUBLIC KEY-----\nAAAA\n-----END PUBLIC KEY-----"
    )
    .is_err());
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn activitypub_actor_binding_matches_oracle_contract() {
    let activity = |actor: &str| serde_json::json!({"actor": actor});
    assert!(crate::activitypub_actor_bound_to_signature(
        &activity("https://peer.example/actors/alice"),
        "https://peer.example/actors/alice",
    ));
    assert!(crate::activitypub_actor_bound_to_signature(
        &activity("https://peer.example/actors/alice"),
        "https://peer.example/actors/alice#main-key",
    ));
    assert!(crate::activitypub_actor_bound_to_signature(
        &activity("https://peer.example/actors/alice"),
        "https://peer.example/actors/alice/main-key",
    ));
    assert!(!crate::activitypub_actor_bound_to_signature(
        &activity("https://peer.example/actors/alice"),
        "https://peer.example/actors/mallory#main-key",
    ));
    assert!(!crate::activitypub_actor_bound_to_signature(
        &serde_json::json!({}),
        "https://peer.example/actors/alice#main-key",
    ));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn activitypub_inbox_rejects_a_request_with_no_signature_header() {
    let fixture = ActivityPubSignatureFixture::spawn().await;
    let (state, _receiver) = fixture.state();
    let unsigned = Box::pin(crate::route_http_request_with_headers(
        "POST",
        "/actors/music/inbox",
        None,
        &fixture.body,
        &state,
        crate::RequestSecurityHeaders::default(),
    ))
    .await
    .expect("unsigned inbox response");
    assert_eq!(unsigned.status, "401 Unauthorized", "{}", unsigned.body);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn activitypub_inbox_accepts_a_genuinely_valid_signature() {
    let fixture = ActivityPubSignatureFixture::spawn().await;
    let (state, _receiver) = fixture.state();
    let created = crate::unix_timestamp();
    let signature_b64 = fixture.sign(created);
    let accepted = Box::pin(crate::route_http_request_with_headers(
        "POST",
        "/actors/music/inbox",
        None,
        &fixture.body,
        &state,
        fixture.headers(&signature_b64, created),
    ))
    .await
    .expect("signed inbox response");
    assert_eq!(accepted.status, "202 Accepted", "{}", accepted.body);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn activitypub_inbox_rejects_a_tampered_signature() {
    let fixture = ActivityPubSignatureFixture::spawn().await;
    let (state, _receiver) = fixture.state();
    let created = crate::unix_timestamp();
    let mut tampered = fixture.sign(created);
    let last = tampered.pop().unwrap();
    tampered.push(if last == 'A' { 'B' } else { 'A' });
    let tampered_response = Box::pin(crate::route_http_request_with_headers(
        "POST",
        "/actors/music/inbox",
        None,
        &fixture.body,
        &state,
        fixture.headers(&tampered, created),
    ))
    .await
    .expect("tampered inbox response");
    assert_eq!(
        tampered_response.status, "401 Unauthorized",
        "{}",
        tampered_response.body
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn activitypub_inbox_rejects_a_stale_created_timestamp() {
    let fixture = ActivityPubSignatureFixture::spawn().await;
    let (state, _receiver) = fixture.state();
    let stale_created = crate::unix_timestamp() - 600;
    let stale_signature_b64 = fixture.sign(stale_created);
    let stale_response = Box::pin(crate::route_http_request_with_headers(
        "POST",
        "/actors/music/inbox",
        None,
        &fixture.body,
        &state,
        fixture.headers(&stale_signature_b64, stale_created),
    ))
    .await
    .expect("stale inbox response");
    assert_eq!(
        stale_response.status, "401 Unauthorized",
        "{}",
        stale_response.body
    );
}
