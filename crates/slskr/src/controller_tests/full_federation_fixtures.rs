//! Controller full federation fixtures ownership.

use super::*;

pub(super) async fn activitypub_relationship_collections_track_target_lifecycle_impl() {
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
        let created = crate::unix_timestamp();
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
    let response = crate::route_http_request_with_headers(
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

    let response = crate::route_http_request(
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
    let response = crate::route_http_request_with_headers(
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

    let response = crate::route_http_request("GET", "/actors/music/followers", None, "", &state)
        .await
        .expect("followers collection");
    let followers = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
    assert_eq!(followers["totalItems"], 1);
    assert_eq!(followers["orderedItems"][0], follower_one);

    let response = crate::route_http_request("GET", "/actors/music/following", None, "", &state)
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
    let response = crate::route_http_request_with_headers(
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
    let response = crate::route_http_request_with_headers(
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

    let response = crate::route_http_request(
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
    let response = crate::route_http_request_with_headers(
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
    let response = crate::route_http_request_with_headers(
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

    let response = crate::route_http_request(
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
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .expect("empty relationship collection");
        let collection = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
        assert_eq!(collection["totalItems"], expected, "{path}");
        assert!(collection["orderedItems"].as_array().unwrap().is_empty());
    }
}

pub(super) async fn configured_relay_test_state() -> (Arc<crate::AppState>, &'static str, u64) {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));
    let secret = "test-token-0123456789";
    {
        let mut advanced = state.advanced_networking.write().await;
        advanced.relay.enabled = true;
        advanced.relay.mode = "controller".to_owned();
        advanced.relay.agents.insert(
            "edge".to_owned(),
            crate::config::RelayAgentSettings {
                instance_name: "edge-one".to_owned(),
                secret: secret.to_owned(),
                cidr: "127.0.0.1/32".to_owned(),
            },
        );
    }
    let relay_settings = state.advanced_networking.read().await.relay.clone();
    let now = crate::unix_timestamp();
    let challenge = state
        .relay
        .write()
        .await
        .protocol
        .issue_challenge("connection-1", now);
    let challenge_credential = crate::relay::credential_for_test(secret, "edge-one", &challenge);
    {
        let mut relay = state.relay.write().await;
        assert!(relay.protocol.authenticate_agent(
            &relay_settings,
            crate::relay::credential_scheme(state.config.controller_profile),
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

/// Test-only fixture: a real local HTTP server serving a real
/// ActivityPub actor document with a real Ed25519 PKIX public key, plus
/// everything needed to sign a request the same way a genuine remote
/// federated peer would. Split across several small tests (rather than
/// one large one) because chaining several calls through the giant
/// `route_http_request_with_headers` dispatcher inside a single debug
/// build async fn overflows the default test-thread stack.
/// `SLSKR_ALLOW_PRIVATE_INTEGRATION_URLS` is process-global state, and
/// Rust's default test harness runs tests in parallel threads within
/// one process -- without serializing every test that flips it, one
/// test's `Drop` could clear the override while a concurrently running
/// test is still relying on it, intermittently failing key fetches for
/// a reason that has nothing to do with the signature under test.
pub(super) static ACTIVITYPUB_ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

pub(super) struct ActivityPubSignatureFixture {
    pub(super) signing_key: crate::SigningKey,
    pub(super) address: std::net::SocketAddr,
    pub(super) body: String,
    pub(super) digest: String,
    pub(super) _server: tokio::task::JoinHandle<()>,
    pub(super) _env_guard: tokio::sync::MutexGuard<'static, ()>,
}

impl ActivityPubSignatureFixture {
    /// One real Ed25519 keypair backs every actor path this fixture
    /// serves -- the server reflects whatever `/actors/{name}` path was
    /// actually requested back into the document's `id`/`publicKey.id`,
    /// so a single fixture can stand in for any number of distinct
    /// remote peers a test needs (each still gets its own real,
    /// per-path `keyId`, matching a real deployment's one-key-per-actor
    /// shape closely enough for signature-verification purposes).
    pub(super) async fn spawn() -> Self {
        use sha2::Digest;
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

        let env_guard = ACTIVITYPUB_ENV_LOCK.lock().await;
        let signing_key = crate::controller_capabilities::new_capability_signing_key()
            .expect("generate fixture Ed25519 signing key");
        const ED25519_SPKI_PREFIX: [u8; 12] = [
            0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00,
        ];
        let mut der = ED25519_SPKI_PREFIX.to_vec();
        der.extend_from_slice(signing_key.verifying_key().as_bytes());
        let public_key_pem = format!(
            "-----BEGIN PUBLIC KEY-----\n{}\n-----END PUBLIC KEY-----",
            base64::engine::general_purpose::STANDARD.encode(der)
        );

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind fixture actor server");
        let address = listener.local_addr().expect("fixture address");
        let server = tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    return;
                };
                let mut reader = BufReader::new(stream);
                let mut request_line = String::new();
                if reader.read_line(&mut request_line).await.unwrap_or(0) == 0 {
                    continue;
                }
                let path = request_line
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or("/actors/unknown")
                    .to_owned();
                let actor_url = format!("http://{address}{path}");
                let key_id = format!("{actor_url}#main-key");
                let document = serde_json::json!({
                    "id": actor_url,
                    "publicKey": {"id": key_id, "owner": actor_url, "publicKeyPem": public_key_pem},
                })
                .to_string();
                let mut stream = reader.into_inner();
                let _ = stream
                    .write_all(
                        format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/activity+json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                            document.len(),
                            document
                        )
                        .as_bytes(),
                    )
                    .await;
            }
        });

        std::env::set_var("SLSKR_ALLOW_PRIVATE_INTEGRATION_URLS", "1");
        let actor_url = format!("http://{address}/actors/remote-peer");
        let body = serde_json::json!({
            "id": "https://peer.example/activities/1",
            "type": "Follow",
            "actor": actor_url,
        })
        .to_string();
        let digest = format!(
            "SHA-256={}",
            base64::engine::general_purpose::STANDARD.encode(sha2::Sha256::digest(body.as_bytes()))
        );
        Self {
            signing_key,
            address,
            body,
            digest,
            _server: server,
            _env_guard: env_guard,
        }
    }

    pub(super) fn actor_url(&self, name: &str) -> String {
        format!("http://{}/actors/{name}", self.address)
    }

    pub(super) fn key_id(&self, name: &str) -> String {
        format!("{}#main-key", self.actor_url(name))
    }

    pub(super) fn sign(&self, created: u64) -> String {
        self.sign_request("post", "/actors/music/inbox", &self.digest, created)
    }

    pub(super) fn sign_request(
        &self,
        method: &str,
        path: &str,
        digest: &str,
        created: u64,
    ) -> String {
        let signing_string =
            format!("(request-target): {method} {path}\nhost: 127.0.0.1\ndigest: {digest}\n(created): {created}");
        let signature: crate::Signature =
            crate::Signer::sign(&self.signing_key, signing_string.as_bytes());
        base64::engine::general_purpose::STANDARD.encode(signature.to_bytes())
    }

    pub(super) fn headers(
        &self,
        signature_b64: &str,
        created: u64,
    ) -> crate::RequestSecurityHeaders {
        self.headers_for(
            &self.key_id("remote-peer"),
            &self.digest,
            signature_b64,
            created,
        )
    }

    pub(super) fn headers_for(
        &self,
        key_id: &str,
        digest: &str,
        signature_b64: &str,
        created: u64,
    ) -> crate::RequestSecurityHeaders {
        let signature_header = format!(
            r#"keyId="{key_id}",algorithm="ed25519",headers="(request-target) host digest (created)",signature="{signature_b64}",created="{created}""#
        );
        crate::RequestSecurityHeaders {
            host: Some("127.0.0.1".to_owned()),
            digest: Some(digest.to_owned()),
            signature: Some(signature_header),
            ..Default::default()
        }
    }

    pub(super) fn state(&self) -> (Arc<crate::AppState>, mpsc::Receiver<crate::SessionCommand>) {
        test_state_with_env(
            MapEnv::default()
                .with("FEDERATION_ENABLED", "true")
                .with("FEDERATION_MODE", "Public")
                .with("FEDERATION_DOMAIN", "social.example")
                .with("FEDERATION_BASE_URL", "https://social.example/"),
        )
    }
}

impl Drop for ActivityPubSignatureFixture {
    fn drop(&mut self) {
        std::env::remove_var("SLSKR_ALLOW_PRIVATE_INTEGRATION_URLS");
    }
}
