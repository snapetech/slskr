use super::fixtures::*;

#[tokio::test]
async fn webhook_update_and_delete_share_one_persistence_order() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("create webhook ordering database");
    let (state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );
    let created = crate::route_http_request(
        "POST",
        "/api/webhooks",
        None,
        r#"{"url":"https://example.com/persistence-order","events":"search.created"}"#,
        &state,
    )
    .await
    .expect("create webhook");
    assert_eq!(created.status, "201 Created");
    let webhook_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
        .as_str()
        .expect("webhook id")
        .to_owned();

    let persistence_turn = state.webhook_persistence_lock.lock().await;
    let patch_state = Arc::clone(&state);
    let patch_path = format!("/api/webhooks/{webhook_id}");
    let mut patch = tokio::spawn(async move {
        crate::route_http_request(
            "PATCH",
            &patch_path,
            None,
            r#"{"active":false}"#,
            &patch_state,
        )
        .await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut patch)
        .await
        .is_err());

    let delete_state = Arc::clone(&state);
    let delete_path = format!("/api/webhooks/{webhook_id}");
    let mut delete = tokio::spawn(async move {
        crate::route_http_request("DELETE", &delete_path, None, "", &delete_state).await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut delete)
        .await
        .is_err());

    let readable = crate::route_http_request("GET", "/api/webhooks", None, "", &state)
        .await
        .expect("webhook read remains available while writes wait");
    assert_eq!(readable.status, "200 OK");
    let webhooks_json = serde_json::from_str::<serde_json::Value>(&readable.body).unwrap();
    assert_eq!(webhooks_json["webhooks"][0]["active"], true);

    drop(persistence_turn);
    assert_eq!(
        patch
            .await
            .expect("webhook update completes")
            .expect("patch webhook")
            .status,
        "200 OK"
    );
    assert_eq!(
        delete
            .await
            .expect("webhook deletion completes")
            .expect("delete webhook")
            .status,
        "200 OK"
    );
    assert!(state.webhooks.read().await.get(&webhook_id).is_none());
    assert!(db
        .list_webhooks()
        .await
        .expect("read final webhook rows")
        .is_empty());

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn security_ban_routes_share_one_persistence_order_across_dispatchers() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("create security-ban ordering database");
    let (state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );

    let persistence_turn = state.security_ban_persistence_lock.lock().await;
    let ban_state = Arc::clone(&state);
    let mut ban = tokio::spawn(async move {
        crate::route_http_request(
            "POST",
            "/api/security/bans/username",
            None,
            r#"{"username":"persistence-order-peer"}"#,
            &ban_state,
        )
        .await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut ban)
        .await
        .is_err());

    let unban_state = Arc::clone(&state);
    let mut unban = tokio::spawn(async move {
        crate::route_http_request(
            "DELETE",
            "/api/overlay/blocklist/username/persistence-order-peer",
            None,
            "",
            &unban_state,
        )
        .await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut unban)
        .await
        .is_err());

    let readable = crate::route_http_request("GET", "/api/security/bans", None, "", &state)
        .await
        .expect("security-ban read remains available while writes wait");
    assert_eq!(readable.status, "200 OK");
    assert!(
        serde_json::from_str::<serde_json::Value>(&readable.body).unwrap()["bans"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    drop(persistence_turn);
    assert_eq!(
        ban.await
            .expect("security ban completes")
            .expect("post security ban")
            .status,
        "200 OK"
    );
    assert_eq!(
        unban
            .await
            .expect("security unban completes")
            .expect("delete overlay blocklist entry")
            .status,
        "200 OK"
    );
    assert!(state.security.read().await.bans.is_empty());
    assert!(db
        .list_security_bans()
        .await
        .expect("read final security-ban rows")
        .is_empty());

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn overlay_blocklist_mutations_roll_back_when_persistence_fails() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("create overlay blocklist rollback database");
    let (state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );
    db.close_for_test().await;

    let created = crate::route_http_request(
        "POST",
        "/api/overlay/blocklist/username",
        None,
        r#"{"value":"must-not-persist"}"#,
        &state,
    )
    .await
    .expect("failed overlay blocklist ban response");
    assert_eq!(created.status, "503 Service Unavailable");
    assert!(state.security.read().await.bans.is_empty());

    state
        .security
        .write()
        .await
        .ban("username", "must-remain".to_owned())
        .expect("seed overlay blocklist ban");
    let deleted = crate::route_http_request(
        "DELETE",
        "/api/overlay/blocklist/username/must-remain",
        None,
        "",
        &state,
    )
    .await
    .expect("failed overlay blocklist unban response");
    assert_eq!(deleted.status, "503 Service Unavailable");
    let security = state.security.read().await;
    assert_eq!(security.bans.len(), 1);
    assert_eq!(security.bans[0].value, "must-remain");
}

#[tokio::test]
async fn oauth_state_consumption_is_ordered_and_keeps_reads_available() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("create OAuth state ordering database");
    let (state, _session_commands) = test_state_with_db(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        db.clone(),
    );
    let token = "queued-oauth-state".to_owned();
    let now = crate::unix_timestamp();
    let record = crate::OAuthStateRecord {
        provider: "spotify".to_owned(),
        redirect_uri: "http://localhost/callback".to_owned(),
        code_verifier: Some("test-verifier".to_owned()),
        created_at: now,
        expires_at: now.saturating_add(600),
    };
    state
        .oauth_states
        .write()
        .await
        .records
        .insert(token.clone(), record.clone());
    crate::oauth_state::persist_oauth_state_checked(&state, &token, &record)
        .await
        .expect("persist OAuth state");

    let persistence_turn = state.oauth_persistence_lock.lock().await;
    let first_state = Arc::clone(&state);
    let mut first = tokio::spawn(async move {
        crate::oauth_state::consume_oauth_state(&first_state, "spotify", &token).await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut first)
        .await
        .is_err());

    let second_state = Arc::clone(&state);
    let token = "queued-oauth-state".to_owned();
    let mut second = tokio::spawn(async move {
        crate::oauth_state::consume_oauth_state(&second_state, "spotify", &token).await
    });
    assert!(tokio::time::timeout(Duration::from_millis(10), &mut second)
        .await
        .is_err());

    let readable = state.oauth_states.read().await;
    assert!(readable.records.contains_key("queued-oauth-state"));
    drop(readable);
    drop(persistence_turn);

    assert_eq!(
        first
            .await
            .expect("first OAuth callback completes")
            .expect("consume persisted OAuth state"),
        Some(record)
    );
    assert!(second
        .await
        .expect("second OAuth callback completes")
        .expect("inspect already consumed OAuth state")
        .is_none());
    assert!(!state
        .oauth_states
        .read()
        .await
        .records
        .contains_key("queued-oauth-state"));
    assert!(db
        .list_oauth_states(i64::try_from(now).unwrap_or(i64::MAX), 10, 0)
        .await
        .expect("read final persisted OAuth state")
        .is_empty());

    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn spotify_disconnect_rejects_an_in_flight_connection_commit() {
    use std::sync::atomic::Ordering;

    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let stale_generation = state.spotify_connection_generation.load(Ordering::Acquire);
    let stale_connection = crate::SpotifyConnectionStore {
        access_token: "stale-access-token".to_owned(),
        refresh_token: "stale-refresh-token".to_owned(),
        scope: "user-library-read".to_owned(),
        expires_at: i64::try_from(crate::unix_timestamp())
            .unwrap_or(i64::MAX)
            .saturating_add(3_600),
        display_name: "Stale account".to_owned(),
        spotify_user_id: "stale-user".to_owned(),
    };

    crate::disconnect_spotify_connection(&state)
        .await
        .expect("disconnect Spotify connection");
    assert!(!crate::persist_spotify_connection_if_current(
        &state,
        stale_generation,
        &stale_connection,
    )
    .await
    .expect("reject stale Spotify connection commit"));

    assert_eq!(
        *state.spotify_connection.read().await,
        crate::SpotifyConnectionStore::default()
    );
    assert!(!crate::spotify_connection_path(&state.config.state_dir).exists());
    fs::remove_dir_all(&state.config.state_dir).expect("remove Spotify lifecycle test state");
}

#[tokio::test]
async fn server_state_response_releases_session_lock_while_credentials_are_queued() {
    use std::{future::Future, task::Poll};

    let (state, _receiver) = test_state_with_env(MapEnv::default());
    for (method, session_state, expected_status) in [
        ("GET", "connected", "200 OK"),
        ("POST", "connected", "202 Accepted"),
        ("POST", "disconnected", "202 Accepted"),
        ("DELETE", "disconnected", "202 Accepted"),
    ] {
        state.session.write().await.state = session_state;
        let held_credentials = state.runtime_credentials.write().await;
        let mut request = Box::pin(crate::route_http_request(
            method,
            "/api/server",
            None,
            "",
            &state,
        ));

        let waiting_on_credentials = std::future::poll_fn(|context| {
            Poll::Ready(request.as_mut().poll(context).is_pending())
        })
        .await;
        assert!(
            waiting_on_credentials,
            "{method} should wait for credential configuration"
        );
        assert!(
            state.session.try_write().is_ok(),
            "{method} waiting for credentials must not retain the session lock"
        );

        drop(held_credentials);
        let response = request.await.expect("server state response");
        assert_eq!(response.status, expected_status);
    }
    fs::remove_dir_all(&state.config.state_dir).expect("remove session lock test state");
}

#[tokio::test]
async fn musicbrainz_overlay_routes_accept_namespaces_and_reject_paths() {
    let (state, _session_commands) = test_state_with_env(MapEnv::default());
    let created = crate::route_http_request(
        "POST",
        "/api/v0/musicbrainz/overlays/edits",
        None,
        r#"{"type":"Other","targetType":"Recording","targetId":"recording-1","field":"title","value":"Title","evidence":[{"type":"WorkRef","reference":"reference-1"}]}"#,
        &state,
    ).await.expect("create overlay edit");
    assert_eq!(created.status, "200 OK");
    let edit_id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["edit"]
        ["editId"]
        .as_str()
        .expect("edit id")
        .to_owned();
    let route = format!("/api/v0/musicbrainz/overlays/edits/{edit_id}/routes");
    for (body, expected) in [
        (
            serde_json::json!({}),
            "At least one target peer is required.",
        ),
        (
            serde_json::json!({"targetPeerIds":["actor:peer-1"]}),
            "Routing backend is not available.",
        ),
        (
            serde_json::json!({"targetPeerIds":["https://example.com/peer"]}),
            "Route targets must be opaque and safe.",
        ),
        (
            serde_json::json!({"targetPeerIds":["../peer"]}),
            "Route targets must be opaque and safe.",
        ),
        (
            serde_json::json!({"targetPeerIds":["actor:peer-1"],"channelId":"/etc/passwd"}),
            "Route metadata must be opaque and safe.",
        ),
        (
            serde_json::json!({"targetPeerIds":["actor:peer-1"],"senderPeerId":"https://example.com/actor"}),
            "Route metadata must be opaque and safe.",
        ),
        (
            serde_json::json!({"targetPeerIds":["x".repeat(257)]}),
            "Route targets must be opaque and safe.",
        ),
    ] {
        let response = crate::route_http_request("POST", &route, None, &body.to_string(), &state)
            .await
            .expect("route overlay edit");
        assert_eq!(response.status, "400 Bad Request");
        let attempt = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
        assert_eq!(attempt["errorMessage"], expected, "{attempt}");
        assert_eq!(attempt["success"], false);
        if body.get("channelId").is_none() {
            assert_eq!(attempt["channelId"], format!("edit:{edit_id}"));
        }
        let readback = crate::route_http_request("GET", &route, None, "", &state)
            .await
            .expect("read overlay routing attempts");
        assert_eq!(readback.status, "200 OK");
        assert!(serde_json::from_str::<serde_json::Value>(&readback.body)
            .unwrap()
            .as_array()
            .unwrap()
            .contains(&attempt));
    }
    fs::remove_dir_all(&state.config.state_dir).expect("remove overlay route test state");
}
