use std::fs;

use super::{serve_json_fixture, test_state, FileConfig, MapEnv};

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn songid_capabilities_report_real_tool_presence_not_a_fake_string_array() {
    let capabilities = crate::songid_capabilities_json(None);
    let capabilities = capabilities.as_array().expect("capabilities array");
    // Matches the oracle's full SongIdCapabilityReporter id set, not
    // the old three-string placeholder.
    assert_eq!(capabilities.len(), 18);

    let by_id = |id: &str| {
        capabilities
            .iter()
            .find(|capability| capability["id"] == id)
            .unwrap_or_else(|| panic!("missing capability {id}"))
    };

    // An absent integration configuration is reported as unavailable;
    // this is a configuration state, not a fabricated implementation
    // status.
    for id in ["chromaprint_fingerprint", "acoustid_lookup"] {
        let capability = by_id(id);
        assert_eq!(capability["available"], false, "{id}");
        assert!(
            capability["reason"].as_str().unwrap().contains("disabled")
                || capability["reason"].as_str().unwrap().contains("Requires"),
            "{id}: {}",
            capability["reason"]
        );
    }
    assert_eq!(by_id("musicbrainz_lookup")["available"], false);
    assert!(by_id("musicbrainz_lookup")["reason"]
        .as_str()
        .unwrap()
        .contains("base URL"));
    assert_eq!(by_id("text_query")["status"], "stable");
    assert_eq!(by_id("text_query")["available"], true);
    assert_eq!(by_id("url_parsing")["available"], true);
    assert_eq!(by_id("local_file_intake")["available"], true);
    assert_eq!(by_id("spotify_page_metadata")["available"], true);

    // Tool-gated capabilities must reflect real PATH state, not a
    // hardcoded value.
    let youtube_metadata = by_id("youtube_metadata");
    assert_eq!(
        youtube_metadata["available"],
        crate::command_exists_on_path("yt-dlp")
    );
    assert_eq!(
        youtube_metadata["requirements"],
        serde_json::json!(["yt-dlp"])
    );

    let youtube_audio = by_id("youtube_audio");
    assert_eq!(
        youtube_audio["available"],
        crate::command_exists_on_path("yt-dlp") && crate::command_exists_on_path("ffmpeg")
    );

    let hash_flag = by_id("hash_from_audio_file_flag");
    assert_eq!(hash_flag["status"], "broken");
    assert_eq!(hash_flag["available"], false);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn songid_source_classification_and_local_root_guard_match_target() {
    assert_eq!(crate::songid_source_type("Artist - Track"), "text_query");
    assert_eq!(
        crate::songid_source_type("https://www.youtube.com/watch?v=track"),
        "youtube_url"
    );
    assert_eq!(
        crate::songid_source_type("https://open.spotify.com/track/abc"),
        "spotify_url"
    );
    assert_eq!(
        crate::songid_source_type("https://example.test/audio"),
        "url"
    );
    let spotify_metadata = crate::songid_spotify_metadata_from_html(
        "https://open.spotify.com/track/abc123?si=fixture",
        r#"<html><head>
            <meta property="og:title" content="Track &amp; More">
            <meta property="og:description" content="Artist · Album">
            <meta property="og:audio" content="https://cdn.example/preview.mp3">
        </head></html>"#,
    );
    assert_eq!(spotify_metadata["query"], "Artist - Track & More");
    assert_eq!(spotify_metadata["metadata"]["artist"], "Artist");
    assert_eq!(spotify_metadata["metadata"]["album"], "Album");
    assert_eq!(spotify_metadata["metadata"]["spotifyTrackId"], "abc123");
    assert_eq!(
        spotify_metadata["metadata"]["previewUrl"],
        "https://cdn.example/preview.mp3"
    );

    let state_dir = std::env::temp_dir().join(format!("slskr-songid-{}", uuid::Uuid::new_v4()));
    let config = crate::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default().with("SLSKR_STATE_DIR", state_dir.to_str().unwrap()),
    )
    .expect("SongID test config");
    fs::create_dir_all(&config.downloads_dir).unwrap();
    let allowed = config.downloads_dir.join("allowed.flac");
    fs::write(&allowed, b"fixture").unwrap();
    assert!(crate::songid_local_file_is_allowed(
        &config,
        allowed.to_str().unwrap()
    ));

    let outside_dir = state_dir.join("outside");
    fs::create_dir_all(&outside_dir).unwrap();
    let outside = outside_dir.join("outside.flac");
    fs::write(&outside, b"fixture").unwrap();
    assert!(!crate::songid_local_file_is_allowed(
        &config,
        outside.to_str().unwrap()
    ));
    assert_eq!(
        crate::songid_fallback_query(allowed.to_str().unwrap(), "local_file"),
        "allowed"
    );
    fs::remove_dir_all(state_dir).unwrap();
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn songid_chromaprint_path_uses_configured_ffmpeg_and_fpcalc() {
    if !crate::configured_command_exists("ffmpeg") || !crate::command_exists_on_path("fpcalc") {
        return;
    }
    let source = std::env::temp_dir().join(format!(
        "slskr-songid-chromaprint-{}.wav",
        uuid::Uuid::new_v4().simple()
    ));
    let generated = std::process::Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=10",
            "-ar",
            "44100",
            "-ac",
            "2",
        ])
        .arg(&source)
        .status()
        .expect("ffmpeg test fixture");
    assert!(generated.success());
    let settings = crate::ChromaprintIntegrationSettings {
        enabled: true,
        algorithm: 1,
        ffmpeg_path: "ffmpeg".to_owned(),
        sample_rate: 44_100,
        channels: 2,
        duration_seconds: 10,
    };
    let fingerprint = crate::songid_extract_chromaprint(source.to_str().unwrap(), &settings)
        .await
        .expect("Chromaprint fingerprint");
    assert!(fingerprint.len() > 20);
    let _ = fs::remove_file(source);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn songid_acoustid_lookup_matches_target_form_contract() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind AcoustID fixture");
    let address = listener.local_addr().expect("AcoustID fixture address");
    let server = tokio::spawn(async move {
        serve_json_fixture(
            &listener,
            serde_json::json!({
                "status": "ok",
                "results": [{
                    "id": "acoustid-audit",
                    "score": 0.97,
                    "recordings": [{
                        "id": "recording-audit",
                        "title": "Audit Track",
                        "artists": [{"name": "Audit Artist"}]
                    }]
                }]
            }),
        )
        .await
    });
    let settings = crate::AcoustIdIntegrationSettings {
        enabled: true,
        client_id: Some("fixture-client".to_owned()),
        base_url: format!("http://{address}/v2"),
    };
    let result = crate::songid_acoustid_lookup("fixture-fingerprint", &settings, 44_100, 120)
        .await
        .expect("AcoustID fixture lookup")
        .expect("AcoustID result");
    assert_eq!(result["id"], "acoustid-audit");
    let request = server.await.expect("AcoustID fixture task");
    assert!(request.starts_with("POST /v2/lookup HTTP/1.1"), "{request}");
    assert!(request.contains("client=fixture-client"), "{request}");
    assert!(
        request.contains("fingerprint=fixture-fingerprint"),
        "{request}"
    );
    assert!(request.contains("duration=120"), "{request}");
    assert!(request.contains("sample_rate=44100"), "{request}");
    assert!(request.contains("meta=recordings"), "{request}");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn songid_run_creation_requires_a_real_non_empty_source() {
    let (state, _receiver) = test_state();

    // Matches the oracle's SongIdController.CreateRun: an empty (or
    // missing) source is rejected before a run is ever queued, not
    // silently accepted as an empty-source run.
    for body in ["{}", r#"{"source":""}"#, r#"{"source":"   "}"#] {
        let response = crate::route_http_request("POST", "/api/v0/songid/runs", None, body, &state)
            .await
            .unwrap_or_else(|error| panic!("{body}: {error}"));
        assert_eq!(response.status, "400 Bad Request", "{body}");
        assert!(
            response.body.contains("SongID source is required."),
            "{body}: {}",
            response.body
        );
    }

    let before = crate::route_http_request("GET", "/api/v0/songid/runs", None, "", &state)
        .await
        .expect("runs before");
    let before_count = serde_json::from_str::<serde_json::Value>(&before.body)
        .unwrap()
        .as_array()
        .unwrap()
        .len();

    let valid = crate::route_http_request(
        "POST",
        "/api/v0/songid/runs",
        None,
        r#"{"source":"route-audit"}"#,
        &state,
    )
    .await
    .expect("valid run");
    assert_eq!(valid.status, "202 Accepted", "{}", valid.body);

    let after = crate::route_http_request("GET", "/api/v0/songid/runs", None, "", &state)
        .await
        .expect("runs after");
    let after_count = serde_json::from_str::<serde_json::Value>(&after.body)
        .unwrap()
        .as_array()
        .unwrap()
        .len();
    assert_eq!(after_count, before_count + 1);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn songid_run_reports_its_real_completed_status_not_stuck_at_queued() {
    let (state, _receiver) = test_state();

    // slskR analyzes synchronously (unlike the oracle's real async
    // queue+worker pipeline), so by the time the create response is
    // built the run has already really finished -- it must report
    // that real status immediately, not a fake "queued" placeholder
    // that nothing would ever advance past.
    let created = crate::route_http_request(
        "POST",
        "/api/v0/songid/runs",
        None,
        r#"{"source":"route-audit"}"#,
        &state,
    )
    .await
    .expect("create run");
    assert_eq!(created.status, "202 Accepted", "{}", created.body);
    let created_json = serde_json::from_str::<serde_json::Value>(&created.body).unwrap();
    assert_eq!(created_json["status"], "completed", "{created_json}");
    assert_eq!(created_json["currentStage"], "completed", "{created_json}");
    assert_eq!(created_json["percentComplete"], 1.0, "{created_json}");
    let run_id = created_json["id"].as_str().unwrap().to_owned();

    // Polling the run afterward must reflect the same real, final
    // status -- not revert to a stale "queued" snapshot.
    let polled = crate::route_http_request(
        "GET",
        &format!("/api/v0/songid/runs/{run_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("poll run");
    let polled_json = serde_json::from_str::<serde_json::Value>(&polled.body).unwrap();
    assert_eq!(polled_json["status"], "completed", "{polled_json}");

    // The queue summary must count this as a real completion, never
    // as perpetually queued/running.
    let queue = crate::route_http_request("GET", "/api/v0/songid/runs/queue", None, "", &state)
        .await
        .expect("queue summary");
    let queue_json = serde_json::from_str::<serde_json::Value>(&queue.body).unwrap();
    assert!(
        queue_json["completedCount"].as_u64().unwrap() >= 1,
        "{queue_json}"
    );
    assert_eq!(queue_json["queuedCount"], 0, "{queue_json}");
    assert_eq!(queue_json["runningCount"], 0, "{queue_json}");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn songid_run_evidence_package_reshapes_real_stored_run_fields() {
    let (state, _receiver) = test_state();
    let created = crate::route_http_request(
        "POST",
        "/api/v0/songid/runs",
        None,
        r#"{"source":"route-audit","query":"Evidence Package Audit"}"#,
        &state,
    )
    .await
    .expect("create run");
    assert_eq!(created.status, "202 Accepted", "{}", created.body);
    let created_json = serde_json::from_str::<serde_json::Value>(&created.body).unwrap();
    let run_id = created_json["id"].as_str().unwrap().to_owned();

    // Matches the oracle's real SongIdRunEvidencePackage contract,
    // reshaped from the same stored run fields -- not a missing
    // endpoint or an invented shape.
    let package = crate::route_http_request(
        "GET",
        &format!("/api/v0/songid/runs/{run_id}/evidence-package"),
        None,
        "",
        &state,
    )
    .await
    .expect("evidence package");
    assert_eq!(package.status, "200 OK", "{}", package.body);
    let package_json = serde_json::from_str::<serde_json::Value>(&package.body).unwrap();
    assert_eq!(package_json["runId"], run_id, "{package_json}");
    assert_eq!(package_json["status"], "completed", "{package_json}");
    assert_eq!(
        package_json["query"], "Evidence Package Audit",
        "{package_json}"
    );
    // slskR's synchronous analysis completes instantly, so
    // completedAt is real (equal to createdAt), not fabricated.
    assert_eq!(
        package_json["completedAt"], package_json["createdAt"],
        "{package_json}"
    );
    assert_eq!(package_json["trackCandidates"], serde_json::json!([]));
    assert_eq!(package_json["artifacts"], serde_json::json!([]));
    // Honest warnings reflecting the real (empty) analysis state,
    // not fabricated ones.
    let warnings = package_json["warnings"].as_array().unwrap();
    assert!(
        warnings
            .iter()
            .any(|warning| warning == "No track candidates were produced."),
        "{package_json}"
    );
    assert!(
        warnings
            .iter()
            .any(|warning| warning == "No recognizer hits were recorded."),
        "{package_json}"
    );
    assert!(
        warnings
            .iter()
            .any(|warning| warning == "No forensic matrix was generated."),
        "{package_json}"
    );

    let missing = crate::route_http_request(
        "GET",
        "/api/v0/songid/runs/songid-does-not-exist/evidence-package",
        None,
        "",
        &state,
    )
    .await
    .expect("missing run evidence package");
    assert_eq!(missing.status, "404 Not Found");

    // The sibling run-detail and forensic-matrix routes must still
    // work unaffected by the new routing guard.
    let detail = crate::route_http_request(
        "GET",
        &format!("/api/v0/songid/runs/{run_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("run detail");
    assert_eq!(detail.status, "200 OK");
    let matrix = crate::route_http_request(
        "GET",
        &format!("/api/v0/songid/runs/{run_id}/forensic-matrix"),
        None,
        "",
        &state,
    )
    .await
    .expect("forensic matrix");
    assert_eq!(matrix.status, "200 OK");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn songid_runs_respect_a_real_limit_and_newest_first_order() {
    let (state, _receiver) = test_state();
    for index in 0..3 {
        let response = crate::route_http_request(
            "POST",
            "/api/v0/songid/runs",
            None,
            &format!(r#"{{"source":"route-audit-{index}"}}"#),
            &state,
        )
        .await
        .unwrap_or_else(|error| panic!("create run {index}: {error}"));
        assert_eq!(response.status, "202 Accepted");
    }

    // Matches the oracle's real ListRuns: newest-first, bounded by
    // the real `limit` query param -- not the full, unbounded,
    // creation-order history.
    let limited = crate::route_http_request("GET", "/api/v0/songid/runs?limit=2", None, "", &state)
        .await
        .expect("limited runs");
    let limited_json = serde_json::from_str::<serde_json::Value>(&limited.body).unwrap();
    let limited_runs = limited_json.as_array().unwrap();
    assert_eq!(limited_runs.len(), 2, "{limited_json}");
    assert_eq!(
        limited_runs[0]["source"], "route-audit-2",
        "newest run must come first: {limited_json}"
    );
    assert_eq!(limited_runs[1]["source"], "route-audit-1", "{limited_json}");

    // A non-positive limit falls back to the oracle's default of 10,
    // not an empty or unbounded result.
    let zero_limit =
        crate::route_http_request("GET", "/api/v0/songid/runs?limit=0", None, "", &state)
            .await
            .expect("zero limit runs");
    let zero_limit_json = serde_json::from_str::<serde_json::Value>(&zero_limit.body).unwrap();
    assert_eq!(
        zero_limit_json.as_array().unwrap().len(),
        3,
        "{zero_limit_json}"
    );
}
