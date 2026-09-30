//! Controller full bridge contracts ownership.

use super::*;

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn soulfind_bridge_wire_frames_match_target_parser_contract() {
    use tokio::io::AsyncWriteExt as _;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bridge test listener");
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let (message_type, payload) =
            crate::soulfind_bridge_runtime::bridge_read_frame(&mut stream)
                .await
                .expect("read bridge frame")
                .expect("bridge client frame");
        assert_eq!(message_type, crate::BRIDGE_LOGIN);
        let mut cursor = 0;
        assert_eq!(
            crate::bridge_read_string(&payload, &mut cursor).as_deref(),
            Some("legacy-client")
        );
        assert_eq!(
            crate::bridge_read_string(&payload, &mut cursor).as_deref(),
            Some("secret")
        );
        crate::soulfind_bridge_runtime::bridge_write_frame(
            &mut stream,
            crate::BRIDGE_LOGIN_RESPONSE,
            &crate::bridge_login_response(true, "Login successful"),
        )
        .await
        .expect("write bridge response");
    });

    let mut client = tokio::net::TcpStream::connect(address)
        .await
        .expect("bridge test client");
    let mut login = Vec::new();
    crate::bridge_write_string(&mut login, "legacy-client");
    crate::bridge_write_string(&mut login, "secret");
    let frame_length = u32::try_from(4 + login.len()).unwrap();
    client.write_all(&frame_length.to_le_bytes()).await.unwrap();
    client
        .write_all(&crate::BRIDGE_LOGIN.to_le_bytes())
        .await
        .unwrap();
    client.write_all(&login).await.unwrap();

    let (message_type, payload) = crate::soulfind_bridge_runtime::bridge_read_frame(&mut client)
        .await
        .expect("read login response")
        .expect("login response frame");
    assert_eq!(message_type, crate::BRIDGE_LOGIN_RESPONSE);
    assert_eq!(payload.first(), Some(&1));
    let mut cursor = 1;
    assert_eq!(
        crate::bridge_read_string(&payload, &mut cursor).as_deref(),
        Some("Login successful")
    );
    server.await.unwrap();
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn bridge_rejects_oversized_wire_frames() {
    use tokio::io::AsyncWriteExt as _;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bridge test listener");
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        assert_eq!(
            crate::soulfind_bridge_runtime::bridge_read_frame(&mut stream).await,
            Err("invalid bridge message length")
        );
    });
    let mut client = tokio::net::TcpStream::connect(address)
        .await
        .expect("bridge test client");
    let oversized = u32::try_from(crate::BRIDGE_MAX_FRAME_BYTES + 1).unwrap();
    client.write_all(&oversized.to_le_bytes()).await.unwrap();
    server.await.unwrap();
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn bridge_frame_reads_have_a_deadline() {
    use tokio::io::AsyncWriteExt as _;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bridge timeout listener");
    let address = listener.local_addr().expect("bridge timeout address");
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener
            .accept()
            .await
            .expect("accept bridge timeout client");
        crate::soulfind_bridge_runtime::bridge_read_frame_with_timeout(
            &mut stream,
            Duration::from_millis(20),
        )
        .await
    });
    let mut client = tokio::net::TcpStream::connect(address)
        .await
        .expect("connect bridge timeout client");
    client
        .write_all(&5_u32.to_le_bytes())
        .await
        .expect("write partial bridge frame");
    assert_eq!(
        server.await.expect("bridge timeout server task"),
        Err("bridge read timed out")
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn virtual_soulfind_v2_routes_execute_bounded_local_intent_workflow() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_VIRTUAL_SOULFIND_V2_ENABLED", "true"));
    state.library.write().await.create(
        "Known Artist".to_owned(),
        "Known Track".to_owned(),
        "Album".to_owned(),
    );

    let artists = crate::route_http_request(
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

    let releases = crate::route_http_request(
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

    let tracks = crate::route_http_request(
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

    let plan = crate::route_http_request(
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

    let created = crate::route_http_request(
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

    let processing = crate::route_http_request(
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

    let intent = crate::route_http_request(
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
        crate::route_http_request("GET", "/api/v1/virtualsoulfind/v2/stats", None, "", &state)
            .await
            .expect("get v2 stats");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap();
    assert_eq!(stats_json["totalProcessed"], 1);
    assert_eq!(stats_json["successCount"], 1);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn virtual_soulfind_v2_routes_honor_explicit_disabled_gate() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_VIRTUAL_SOULFIND_V2_ENABLED", "false"));
    let response =
        crate::route_http_request("GET", "/api/v1/virtualsoulfind/v2/stats", None, "", &state)
            .await
            .expect("disabled v2 status");
    assert_eq!(response.status, "503 Service Unavailable");
    assert_eq!(response.body, r#""VirtualSoulfind v2 is disabled""#);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn bridge_projections_redact_internal_endpoint() {
    let internal_host = "bridge.internal.private";
    let internal_port = "43127";
    let env = MapEnv::default()
        .with("SLSKR_BRIDGE_ENABLED", "true")
        .with("SLSKR_BRIDGE_HOST", internal_host)
        .with("SLSKR_BRIDGE_PORT", internal_port);
    let (state, _receiver) = test_state_with_env_parts(env, crate::SearchStore::new(), None);

    let sanitized = state.config.integrations.bridge.sanitized_json();
    assert!(sanitized.contains("\"host\":null"));
    assert!(sanitized.contains("\"port\":null"));
    assert!(!sanitized.contains(internal_host));
    assert!(!sanitized.contains(internal_port));

    for path in [
        "/api/bridge/admin/config",
        "/api/bridge/admin/dashboard",
        "/api/bridge/status",
        "/api/application",
    ] {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .expect("bridge projection");
        assert_eq!(response.status, "200 OK", "{path}");
        assert!(!response.body.contains(internal_host), "{path}");
        assert!(!response.body.contains(internal_port), "{path}");
    }
}
