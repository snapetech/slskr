//! Controller full media contracts ownership.

use super::*;

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn now_playing_routes_roll_back_when_persistence_fails() {
    for method in ["PUT", "POST"] {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        state.now_playing.write().await.upsert(
            "existing".to_owned(),
            "Original".to_owned(),
            "Track".to_owned(),
        );
        db.close_for_test().await;

        let response = crate::route_http_request(
            method,
            "/api/nowplaying",
            None,
            r#"{"username":"new","artist":"Changed","title":"Song"}"#,
            &state,
        )
        .await
        .expect("failed now-playing persistence response");
        assert_eq!(response.status, "503 Service Unavailable", "{method}");
        assert!(
            response.body.contains("now-playing persistence failed"),
            "{method}"
        );
        let now_playing = state.now_playing.read().await;
        assert_eq!(now_playing.records.len(), 1, "{method}");
        assert_eq!(now_playing.records[0].username, "existing", "{method}");
        assert_eq!(now_playing.records[0].artist, "Original", "{method}");
        assert_eq!(now_playing.records[0].title, "Track", "{method}");
    }

    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );
    state.now_playing.write().await.upsert(
        "existing".to_owned(),
        "Original".to_owned(),
        "Track".to_owned(),
    );
    db.close_for_test().await;

    let response = crate::route_http_request("DELETE", "/api/nowplaying", None, "", &state)
        .await
        .expect("failed now-playing clear response");
    assert_eq!(response.status, "503 Service Unavailable");
    assert!(response
        .body
        .contains("now-playing clear persistence failed"));
    let now_playing = state.now_playing.read().await;
    assert_eq!(now_playing.records.len(), 1);
    assert_eq!(now_playing.records[0].username, "existing");
    assert_eq!(now_playing.records[0].artist, "Original");
    assert_eq!(now_playing.records[0].title, "Track");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn songid_runs_are_bounded_snapshots_with_real_lookup() {
    let mut runtime = crate::RuntimeCompatState::new();
    let first = runtime
        .record_songid_run(vec![serde_json::json!({ "score": 1.0 })], 1, 1)
        .unwrap();
    let first_id = first["id"].as_str().unwrap().to_owned();
    for _ in 1..crate::MAX_SONGID_RUNS {
        runtime.record_songid_run(Vec::new(), 0, 0).unwrap();
    }
    assert_eq!(runtime.songid_run_records.len(), crate::MAX_SONGID_RUNS);
    assert!(runtime.songid_run(&first_id).is_some());
    runtime.record_songid_run(Vec::new(), 0, 0).unwrap();
    assert_eq!(runtime.songid_run_records.len(), crate::MAX_SONGID_RUNS);
    assert!(runtime.songid_run(&first_id).is_none());
    assert!(runtime.songid_run("songid-does-not-exist").is_none());

    runtime.songid_runs = u64::MAX;
    let wrapped = runtime.record_songid_run(Vec::new(), 0, 0).unwrap();
    assert_eq!(wrapped["id"], "songid-1");
    runtime.songid_runs = u64::MAX;
    let collision_avoiding = runtime.record_songid_run(Vec::new(), 0, 0).unwrap();
    assert_eq!(collision_avoiding["id"], "songid-2");
}
#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn now_playing_and_security_state_bound_remote_keys() {
    let mut now_playing = crate::NowPlayingStore::new();
    let bounded = now_playing.upsert(
        "é".repeat(crate::MAX_USER_USERNAME_BYTES),
        "a".repeat(crate::MAX_NOW_PLAYING_ARTIST_BYTES + 1),
        "t".repeat(crate::MAX_NOW_PLAYING_TITLE_BYTES + 1),
    );
    assert!(bounded.username.len() <= crate::MAX_USER_USERNAME_BYTES);
    assert_eq!(bounded.artist.len(), crate::MAX_NOW_PLAYING_ARTIST_BYTES);
    assert_eq!(bounded.title.len(), crate::MAX_NOW_PLAYING_TITLE_BYTES);
    now_playing.records.clear();
    for index in 0..crate::MAX_NOW_PLAYING_RECORDS {
        now_playing.upsert(
            format!("user-{index}"),
            "Artist".to_owned(),
            "Track".to_owned(),
        );
    }
    now_playing.upsert(
        "overflow".to_owned(),
        "Artist".to_owned(),
        "Track".to_owned(),
    );
    assert_eq!(now_playing.records.len(), crate::MAX_NOW_PLAYING_RECORDS);
    assert!(now_playing
        .records
        .iter()
        .any(|record| record.username == "overflow"));
    let count = now_playing.records.len();
    now_playing.upsert(
        "OVERFLOW".to_owned(),
        "Updated".to_owned(),
        "Track".to_owned(),
    );
    assert_eq!(now_playing.records.len(), count);

    let mut security = crate::SecurityState::new();
    let first = security.ban("username", "Spammer".to_owned()).unwrap();
    let duplicate = security.ban("username", "spammer".to_owned()).unwrap();
    assert_eq!(first.value, duplicate.value);
    assert_eq!(security.bans.len(), 1);
    assert!(security.unban("username", "SPAMMER"));
    let literal_pattern = security.ban("username", "user.*".to_owned()).unwrap();
    assert_eq!(literal_pattern.value, "user.*");
    assert!(!security.unban("username", "user123"));
    assert!(security.unban("username", "USER.*"));
    for index in 0..crate::MAX_SECURITY_BANS {
        security.ban("ip", format!("2001:db8::{index:x}")).unwrap();
    }
    assert!(security.ban("ip", "198.51.100.1".to_owned()).is_none());
    assert_eq!(security.bans.len(), crate::MAX_SECURITY_BANS);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn primary_stream_route_serves_authenticated_file_ranges() {
    run_controller_future_on_large_stack("primary-stream-file-ranges", || {
        primary_stream_route_serves_authenticated_file_ranges_impl()
    });
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn preview_ticket_get_is_anonymous_and_streams_local_audio_without_ranges() {
    run_controller_future_on_large_stack("local-preview-ticket-stream", || {
        preview_ticket_get_is_anonymous_and_streams_local_audio_without_ranges_impl()
    });
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn remote_preview_head_uses_ticket_metadata_without_a_source_connection() {
    use tokio::io::AsyncReadExt as _;

    let ticket = crate::PreviewStreamTicket {
        family: "mesh".to_owned(),
        source: "mesh-unresolved".to_owned(),
        content_id: "content".to_owned(),
        filename: "Remote.flac".to_owned(),
        peer_username: Some("remote".to_owned()),
        size: 1_234,
        content_type: "audio/flac".to_owned(),
        source_url: Some("https://127.0.0.1:9/content".to_owned()),
        source_authorization: None,
        overlay_peer_identity: None,
        expected_hash: Some("a".repeat(64)),
        created_at: 1,
        expires_at: u64::MAX,
    };
    let (mut client, mut server) = tokio::io::duplex(4 * 1024);
    let write = tokio::spawn(async move {
        crate::write_remote_preview_head_response(
            &mut server,
            &ticket,
            false,
            "X-Request-ID: test\r\n",
            std::time::Duration::from_secs(1),
        )
        .await
    });
    let mut response = Vec::new();
    client.read_to_end(&mut response).await.unwrap();
    let written = write.await.unwrap().unwrap();
    let response = String::from_utf8(response).unwrap();

    assert_eq!(written.content_length, 1_234);
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(response.contains("Content-Length: 1234\r\n"));
    assert!(response.contains("Content-Type: audio/flac\r\n"));
    assert!(response.ends_with("X-Request-ID: test\r\n\r\n"));

    let mut unknown_length = crate::PreviewStreamTicket {
        family: "peer".to_owned(),
        source: "peer-unresolved".to_owned(),
        content_id: "content".to_owned(),
        filename: "Remote.flac".to_owned(),
        peer_username: Some("remote".to_owned()),
        size: 0,
        content_type: "audio/flac".to_owned(),
        source_url: None,
        source_authorization: None,
        overlay_peer_identity: None,
        expected_hash: None,
        created_at: 1,
        expires_at: u64::MAX,
    };
    assert!(crate::remote_preview_head_ticket(Some(unknown_length.clone()), "peer").is_none());
    unknown_length.size = 1;
    assert!(crate::remote_preview_head_ticket(Some(unknown_length), "peer").is_some());
}
#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn listening_party_directory_ticket_streams_local_audio_ranges() {
    run_controller_future_on_large_stack("listening-party-directory-stream", || {
        listening_party_directory_ticket_streams_local_audio_ranges_impl()
    });
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn listening_party_stream_limits_match_frozen_caps() {
    let mut limits = crate::ListeningPartyStreamLimits::default();
    let mut party_permits = Vec::new();
    for index in 0..crate::LISTED_PARTY_MAX_CONCURRENT_STREAMS {
        party_permits.push(
            limits
                .try_acquire("party:limit-audit", &format!("ip-{index}"))
                .expect("party stream slot"),
        );
    }
    assert_eq!(
        limits
            .try_acquire("party:limit-audit", "ip-over-cap")
            .unwrap_err(),
        crate::ListeningPartyStreamLimitRejection::Party
    );

    let mut ip_permits = Vec::new();
    for _ in 0..crate::LISTED_PARTY_MAX_CONCURRENT_STREAMS_PER_IP {
        ip_permits.push(
            limits
                .try_acquire("party:ip-audit", "same-ip")
                .expect("per-IP stream slot"),
        );
    }
    assert_eq!(
        limits.try_acquire("party:ip-audit", "same-ip").unwrap_err(),
        crate::ListeningPartyStreamLimitRejection::Ip
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn playback_feedback_and_diagnostics_reflect_the_real_buffer_state() {
    // Matches the oracle's real PlaybackController: priority is
    // computed from PlaybackPriorityService.GetPriority (High when
    // buffer < 5s, Low when >= 30s, Mid otherwise), and diagnostics
    // reflects the most recently posted feedback for that job, not
    // multisource swarm-download progress.
    let (state, _receiver) = test_state();

    let missing = crate::route_http_request(
        "GET",
        "/api/v0/playback/track-audit/diagnostics",
        None,
        "",
        &state,
    )
    .await
    .expect("missing playback diagnostics");
    assert_eq!(missing.status, "404 Not Found");

    let low_buffer = crate::route_http_request(
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

    let diagnostics = crate::route_http_request(
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

    let comfortable = crate::route_http_request(
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

    let updated = crate::route_http_request(
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

    let mid = crate::route_http_request(
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

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn remote_path_registry_learns_search_response_encodings() {
    let response = crate::FileSearchResponse {
        username: "friend".to_owned(),
        token: 7,
        results: vec![FileEntry {
            filename_encoding: crate::ProtocolTextEncoding::Windows1251,
            extension_encoding: crate::ProtocolTextEncoding::Utf8,
            code: 1,
            filename: "Музыка/песня.flac".to_owned(),
            size: 123,
            extension: "flac".to_owned(),
            attributes: Vec::new(),
        }],
        slot_free: true,
        average_speed: 0,
        queue_length: 0,
        unknown: 0,
        private_results: Vec::new(),
    };
    let mut registry = crate::RemotePathEncodingRegistry::default();

    registry.remember_search_response(&response);

    assert_eq!(
        registry.encoding_for("friend", "Музыка/песня.flac"),
        crate::ProtocolTextEncoding::Windows1251
    );
    assert_eq!(
        registry.encoding_for("other", "Музыка/песня.flac"),
        crate::ProtocolTextEncoding::Utf8
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn remote_configuration_routes_are_forbidden_by_default() {
    let (state, _receiver) = test_state();
    for (method, path, body) in [
        ("PATCH", "/api/options", "{}"),
        ("PUT", "/api/options", "{}"),
        ("GET", "/api/options/debug", ""),
        ("GET", "/api/options/yaml/location", ""),
        ("GET", "/api/options/yaml", ""),
        ("PUT", "/api/options/yaml", r#""app: {}""#),
        ("POST", "/api/options/yaml/validate", r#""app: {}""#),
    ] {
        let response = crate::route_http_request(method, path, None, body, &state)
            .await
            .expect("remote configuration policy response");
        assert_eq!(response.status, "403 Forbidden", "{method} {path}");
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn remote_options_overlay_and_yaml_are_effective_and_durable() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_REMOTE_CONFIGURATION", "true"));

    let options = crate::route_http_request("GET", "/api/options", None, "", &state)
        .await
        .expect("options get");
    assert_eq!(options.status, "200 OK");
    let options_json = serde_json::from_str::<serde_json::Value>(&options.body).unwrap();
    assert_eq!(
        options_json["remoteConfiguration"], true,
        "{}",
        options.body
    );
    assert!(options_json.get("version").is_none());
    assert!(options_json.get("options").is_none());
    assert!(options_json.get("compatibility").is_none());
    assert!(options_json.get("nonPersistentMutationRoutes").is_none());

    let response = crate::route_http_request(
        "PATCH",
        "/api/options",
        None,
        r#"{"soulseek":{"listenIpAddress":"127.0.0.1","listenPort":50300,"privateMessageAutoResponse":{"enabled":true}}}"#,
        &state,
    )
    .await
    .expect("options response");

    assert_eq!(response.status, "200 OK");
    let overlay = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
    assert_eq!(overlay["soulseek"]["listenIpAddress"], "127.0.0.1");
    assert_eq!(overlay["soulseek"]["listenPort"], 50300);
    assert_eq!(
        overlay["soulseek"]["privateMessageAutoResponse"]["enabled"],
        true
    );
    assert!(
        state
            .private_message_auto_response_settings
            .read()
            .await
            .enabled
    );

    let invalid = crate::route_http_request("PATCH", "/api/options", None, "[]", &state)
        .await
        .expect("invalid options response");
    assert_eq!(invalid.status, "400 Bad Request");

    let location = crate::route_http_request("GET", "/api/options/yaml/location", None, "", &state)
        .await
        .expect("options location");
    assert_eq!(location.status, "200 OK");
    let location_path = serde_json::from_str::<String>(&location.body).unwrap();
    assert_eq!(
        PathBuf::from(location_path),
        state.config.state_dir.join("slskd.yml")
    );

    let config_text = crate::route_http_request("GET", "/api/options/yaml", None, "", &state)
        .await
        .expect("options config text");
    assert_eq!(config_text.status, "404 Not Found");

    let yaml = "soulseek:\n  description: test description\nweb:\n  authentication:\n    password: never-return-this\n";
    let uploaded = crate::route_http_request(
        "PUT",
        "/api/options/yaml",
        None,
        &serde_json::to_string(yaml).unwrap(),
        &state,
    )
    .await
    .expect("options upload");
    assert_eq!(uploaded.status, "200 OK");
    assert!(uploaded.body.is_empty());
    assert_eq!(
        fs::read_to_string(state.config.state_dir.join("slskd.yml")).unwrap(),
        yaml
    );

    let config_text = crate::route_http_request("GET", "/api/options/yaml", None, "", &state)
        .await
        .expect("options config text");
    assert_eq!(config_text.status, "200 OK");
    assert_eq!(
        serde_json::from_str::<String>(&config_text.body).unwrap(),
        yaml
    );

    let effective = crate::route_http_request("GET", "/api/options", None, "", &state)
        .await
        .expect("effective options");
    let effective = serde_json::from_str::<serde_json::Value>(&effective.body).unwrap();
    assert_eq!(effective["soulseek"]["description"], "test description");
    assert_eq!(effective["web"]["authentication"]["password"], "*****");
    assert_eq!(effective["soulseek"]["listenPort"], 50300);

    let validated = crate::route_http_request(
        "POST",
        "/api/options/yaml/validate",
        None,
        &serde_json::to_string("app: {}\n").unwrap(),
        &state,
    )
    .await
    .expect("valid options validate");
    assert_eq!(validated.status, "200 OK");
    assert!(validated.content_type.is_empty());
    assert!(validated.body.is_empty());

    let invalid_upload = crate::route_http_request("PUT", "/api/options/yaml", None, "{}", &state)
        .await
        .expect("invalid options upload");
    assert_eq!(invalid_upload.status, "400 Bad Request");

    let invalid_validate =
        crate::route_http_request("POST", "/api/options/yaml/validate", None, "{}", &state)
            .await
            .expect("invalid options validate");
    assert_eq!(invalid_validate.status, "400 Bad Request");

    let invalid_yaml = crate::route_http_request(
        "POST",
        "/api/options/yaml/validate",
        None,
        &serde_json::to_string("web: [unterminated").unwrap(),
        &state,
    )
    .await
    .expect("invalid YAML validation");
    assert_eq!(invalid_yaml.status, "200 OK");
    assert_eq!(invalid_yaml.body, r#""Invalid YAML configuration""#);
}
