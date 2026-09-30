//! Controller full integrations contracts 02 ownership.

use super::*;

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn webhook_registration_rejects_blocked_urls() {
    let (state, _receiver) = test_state();

    for body in [
        r#"{"url":"ftp://example.test/hook","events":"search.created"}"#,
        r#"{"url":"http://localhost/hook","events":"search.created"}"#,
        r#"{"url":"http://127.0.0.1/hook","events":"search.created"}"#,
        r#"{"url":"http://169.254.169.254/hook","events":"search.created"}"#,
    ] {
        let response = crate::route_http_request("POST", "/api/webhooks", None, body, &state)
            .await
            .expect("blocked webhook URL");
        assert_eq!(response.status, "400 Bad Request");
    }

    let admin_response = crate::route_http_request(
        "POST",
        "/api/admin/webhooks",
        None,
        r#"{"url":"http://127.0.0.1/hook"}"#,
        &state,
    )
    .await
    .expect("blocked admin webhook URL");
    assert_eq!(admin_response.status, "400 Bad Request");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn webhook_patch_requires_active_boolean() {
    let (state, _receiver) = test_state();
    let created = crate::route_http_request(
        "POST",
        "/api/webhooks",
        None,
        r#"{"url":"https://example.test/hook","events":"search.created"}"#,
        &state,
    )
    .await
    .expect("create webhook");
    let id = serde_json::from_str::<serde_json::Value>(&created.body).expect("created json")["id"]
        .as_str()
        .expect("webhook id")
        .to_owned();

    let missing =
        crate::route_http_request("PATCH", &format!("/api/webhooks/{id}"), None, "{}", &state)
            .await
            .expect("missing active");
    assert_eq!(missing.status, "400 Bad Request");
    assert_eq!(missing.body, "{\"error\":\"active boolean is required\"}");

    let patched = crate::route_http_request(
        "PATCH",
        &format!("/api/webhooks/{id}"),
        None,
        r#"{"active":false}"#,
        &state,
    )
    .await
    .expect("patch webhook");
    assert_eq!(patched.status, "200 OK");
    assert!(patched.body.contains("\"active\":false"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn webhook_mutations_and_tests_require_exact_paths() {
    let (state, _receiver) = test_state();
    let created = crate::route_http_request(
        "POST",
        "/api/webhooks",
        None,
        r#"{"url":"https://example.test/hook","events":"search.created"}"#,
        &state,
    )
    .await
    .unwrap();
    let id = serde_json::from_str::<serde_json::Value>(&created.body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    crate::route_http_request(
        "PATCH",
        &format!("/api/webhooks/extra/{id}"),
        None,
        r#"{"active":false}"#,
        &state,
    )
    .await
    .unwrap();
    assert!(state.webhooks.read().await.get(&id).unwrap().active);

    let _all_delivery_permits = Arc::clone(&state.webhook_deliveries)
        .acquire_many_owned(crate::MAX_WEBHOOK_DELIVERY_TASKS as u32)
        .await
        .unwrap();
    let malformed_test = crate::route_http_request(
        "POST",
        &format!("/api/webhooks/extra/{id}/test"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_ne!(malformed_test.status, "429 Too Many Requests");
    let malformed_admin_test = crate::route_http_request(
        "POST",
        &format!("/api/admin/webhooks/extra/{id}/test"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_ne!(malformed_admin_test.body, "{\"status\":\"test_sent\"}");

    for path in [
        format!("/api/webhooks/extra/{id}"),
        format!("/api/admin/webhooks/extra/{id}"),
    ] {
        crate::route_http_request("DELETE", &path, None, "", &state)
            .await
            .unwrap();
        assert!(state.webhooks.read().await.get(&id).is_some());
    }

    assert_eq!(
        crate::webhook_resource_id(&format!("/api/webhooks/{id}"), "/api/webhooks/"),
        Some(id.as_str())
    );
    assert_eq!(
        crate::webhook_resource_id(&format!("/api/webhooks/extra/{id}"), "/api/webhooks/"),
        None
    );
    let deleted =
        crate::route_http_request("DELETE", &format!("/api/webhooks/{id}"), None, "", &state)
            .await
            .unwrap();
    assert_eq!(deleted.status, "200 OK");
    assert!(state.webhooks.read().await.get(&id).is_none());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn external_visualizer_launch_requires_explicit_enable_flag() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_EXTERNAL_VISUALIZER_COMMAND", "true"));

    let status =
        crate::route_http_request("GET", "/api/player/external-visualizer", None, "", &state)
            .await
            .expect("external visualizer status");
    assert_eq!(status.status, "200 OK");
    assert!(status.body.contains("\"configured\":true"));
    assert!(status.body.contains("\"enabled\":false"));

    let launch = crate::route_http_request(
        "POST",
        "/api/player/external-visualizer/launch",
        None,
        "",
        &state,
    )
    .await
    .expect("external visualizer launch");
    assert_eq!(launch.status, "400 Bad Request");
    assert!(launch.body.contains("launching is disabled"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn external_visualizer_launch_records_audit_event_when_enabled() {
    let command = if cfg!(windows) { "where.exe" } else { "true" };
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_EXTERNAL_VISUALIZER_COMMAND", command)
            .with("SLSKR_EXTERNAL_VISUALIZER_LAUNCH_ENABLED", "true"),
    );

    let launch = crate::route_http_request(
        "POST",
        "/api/player/external-visualizer/launch",
        None,
        "",
        &state,
    )
    .await
    .expect("external visualizer launch");
    assert_eq!(launch.status, "200 OK");
    assert!(launch.body.contains("\"started\":true"));
    assert!(!launch.body.contains("\"command\""));
    let events = state.events.read().await;
    let event = events
        .records
        .iter()
        .find(|event| event.kind == "external_visualizer.launch")
        .expect("launch event");
    assert_eq!(event.resource, "external_visualizer");
    assert!(!event.resource.contains("true"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn external_visualizer_launch_errors_redact_command_details() {
    let command = "/private/missing-visualizer-secret";
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_EXTERNAL_VISUALIZER_COMMAND", command)
            .with("SLSKR_EXTERNAL_VISUALIZER_LAUNCH_ENABLED", "true"),
    );

    let launch = crate::route_http_request(
        "POST",
        "/api/player/external-visualizer/launch",
        None,
        "",
        &state,
    )
    .await
    .expect("external visualizer launch");
    assert_eq!(launch.status, "400 Bad Request");
    assert!(!launch.body.contains(command));
    let events = state.events.read().await;
    let event = events
        .records
        .iter()
        .find(|event| event.kind == "external_visualizer.launch.failed")
        .expect("failed launch event");
    assert_eq!(event.resource, "external_visualizer");
    assert_eq!(event.detail.as_deref(), Some("launch failed"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn external_visualizer_launch_rejects_when_process_pool_is_full() {
    let command = if cfg!(windows) { "where.exe" } else { "true" };
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_EXTERNAL_VISUALIZER_COMMAND", command)
            .with("SLSKR_EXTERNAL_VISUALIZER_LAUNCH_ENABLED", "true"),
    );
    let _permits = Arc::clone(&state.external_visualizer_processes)
        .acquire_many_owned(crate::MAX_EXTERNAL_VISUALIZER_PROCESSES as u32)
        .await
        .expect("configured visualizer permits");

    let launch = crate::route_http_request(
        "POST",
        "/api/player/external-visualizer/launch",
        None,
        "",
        &state,
    )
    .await
    .expect("external visualizer launch");
    assert_eq!(launch.status, "503 Service Unavailable");
    assert!(launch.body.contains("process limit reached"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn musicbrainz_overlay_export_review_and_approval_match_real_oracle_gating() {
    // Matches the oracle's real MusicBrainzOverlayService: export
    // review/approval reflects the edit's real exportable-type gate
    // (IsExportableEdit) and validates a real opaque approvedBy and
    // note length, rather than always approving with an invented
    // {editId, isApproved:true} shape.
    let (state, _receiver) = test_state();

    let non_exportable = crate::route_http_request(
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

    let review = crate::route_http_request(
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

    let rejected_approval = crate::route_http_request(
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
    let unsafe_edit = crate::route_http_request(
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
    let unsafe_approval = crate::route_http_request(
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
    let exportable = crate::route_http_request(
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

    let approval = crate::route_http_request(
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

    let post_approval_review = crate::route_http_request(
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

    let repeat_approval = crate::route_http_request(
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
pub(super) async fn source_discovery_routes_dispatch_and_project_bounded_search_sources() {
    let (state, mut receiver) = test_state();
    let started = crate::route_http_request(
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
        crate::SessionCommand::Search {
            token,
            query,
            target: crate::SearchDispatchTarget::Global,
        } => {
            assert_eq!(query, "rare recording");
            token
        }
        command => panic!("unexpected command: {command:?}"),
    };

    let conflict = crate::route_http_request(
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
        record.results.push(crate::SearchResultEntry {
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

    let status = crate::route_http_request("GET", "/api/v0/discovery", None, "", &state)
        .await
        .expect("source discovery status");
    let status_json = serde_json::from_str::<serde_json::Value>(&status.body).unwrap();
    assert_eq!(status_json["isRunning"], true);
    assert_eq!(status_json["stats"]["totalFiles"], 1);
    assert_eq!(status_json["stats"]["totalUsers"], 1);
    assert_eq!(status_json["stats"]["searchCycles"], 1);

    let by_size = crate::route_http_request(
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

    let by_name = crate::route_http_request(
        "GET",
        "/api/v0/discovery/sources/by-filename?pattern=recording&limit=10",
        None,
        "",
        &state,
    )
    .await
    .expect("sources by filename");
    assert!(by_name.body.contains("Rare/Recording.flac"));

    let summaries = crate::route_http_request(
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

    let stopped = crate::route_http_request("POST", "/api/v0/discovery/stop", None, "{}", &state)
        .await
        .expect("stop source discovery");
    let stopped_json = serde_json::from_str::<serde_json::Value>(&stopped.body).unwrap();
    assert_eq!(stopped_json["stats"]["lastCycleNewFiles"], 1);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn source_discovery_generation_ignores_stale_completion() {
    let mut discovery = crate::SourceDiscoveryState::default();
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

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn lidarr_projections_redact_endpoint_and_errors() {
    let env = MapEnv::default()
        .with("SLSKR_LIDARR_ENABLED", "true")
        .with("SLSKR_LIDARR_URL", "http://lidarr.internal:8686/private")
        .with("SLSKR_LIDARR_API_KEY", "lidarr-secret")
        .with("SLSKR_LIDARR_DELETE_REJECTED_DOWNLOADS", "true")
        .with("SLSKR_LIDARR_BLACKLIST_REJECTED_DOWNLOADS", "true");
    let config =
        crate::AppConfig::from_layers(None, FileConfig::default(), &env).expect("Lidarr config");
    let json = config.integrations.lidarr.sanitized_json();

    assert!(json.contains("\"url\":null"));
    assert!(json.contains("\"url_configured\":true"));
    assert!(!json.contains("lidarr.internal"));
    assert!(!json.contains("/private"));
    assert!(!json.contains("lidarr-secret"));
    assert!(json.contains("\"delete_rejected_downloads\":true"));
    assert!(json.contains("\"blacklist_rejected_downloads\":true"));
    assert_eq!(
        crate::public_lidarr_error(Some(
            "request to http://lidarr.internal:8686/private failed"
        )),
        Some("Lidarr connection failed")
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn lidarr_rejected_candidates_emit_distinct_portable_filenames() {
    let candidates = vec![
        serde_json::json!({
            "id": 1,
            "path": "/lidarr/Album/accepted.flac",
            "artist": {"id": 2},
            "album": {"id": 3},
            "albumReleaseId": 4,
            "tracks": [{"id": 5}],
            "quality": {"quality": {"id": 6}},
            "additionalFile": false,
            "rejections": []
        }),
        serde_json::json!({
            "path": r"C:\lidarr\Album\Rejected.FLAC",
            "rejections": [{"reason": "quality mismatch"}]
        }),
        serde_json::json!({
            "path": "/lidarr/Album/rejected.flac",
            "rejections": [{"reason": "quality mismatch"}]
        }),
    ];

    assert_eq!(
        crate::lidarr_rejected_filenames(&candidates),
        vec!["Rejected.FLAC".to_owned()]
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn lidarr_import_mapping_preserves_native_windows_destination_separators() {
    assert_eq!(
        crate::lidarr_map_import_path(
            "/music/downloaded/2 Chainz - T.R.U. REALigion (Anniversary Edition)",
            "/music/downloaded",
            r"D:\downloaded",
        ),
        r"D:\downloaded\2 Chainz - T.R.U. REALigion (Anniversary Edition)"
    );
    assert_eq!(
        crate::lidarr_map_import_path("/downloads/Album", "/downloads", "/library",),
        "/library/Album"
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn lidarr_rejected_file_deletion_stays_in_the_completed_directory() {
    let root = std::env::temp_dir().join(format!(
        "slskr-lidarr-rejected-files-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&root).unwrap();
    let inside = root.join("rejected.flac");
    let outside = root.parent().unwrap().join(format!(
        "slskr-lidarr-outside-{}-{}.flac",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::write(&inside, b"rejected").unwrap();
    std::fs::write(&outside, b"protected").unwrap();

    let deleted = crate::delete_lidarr_rejected_files(
        root.to_str().unwrap(),
        &[
            "rejected.flac".to_owned(),
            "../".to_owned() + outside.file_name().unwrap().to_str().unwrap(),
        ],
    )
    .unwrap();
    assert_eq!(deleted, 1);
    assert!(!inside.exists());
    assert!(outside.exists());

    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_file(&outside);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn lidarr_rejection_policy_blacklists_wishlist_origin_and_deletes_files() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_LIDARR_DELETE_REJECTED_DOWNLOADS", "true")
            .with("SLSKR_LIDARR_BLACKLIST_REJECTED_DOWNLOADS", "true"),
    );
    let item = state
        .wishlist
        .write()
        .await
        .add_item("Artist".to_owned(), "Album".to_owned(), "Audio".to_owned())
        .unwrap();
    let root = std::env::temp_dir().join(format!(
        "slskr-lidarr-policy-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&root).unwrap();
    let file = root.join("track.flac");
    std::fs::write(&file, b"rejected").unwrap();
    let transfer = {
        let mut transfers = state.transfers.write().await;
        transfers.create_with_details(
            0,
            Some("peer".to_owned()),
            "Remote/Album/track.flac".to_owned(),
            Some(file.display().to_string()),
            Some(8),
            None,
            crate::TransferRequestDetails {
                wishlist_item_id: Some(item.id.clone()),
                ..Default::default()
            },
        )
    };
    let result = serde_json::json!({
        "rejectedCandidateCount": 1,
        "rejectedFilenames": ["track.flac"]
    });

    crate::apply_lidarr_rejection_policy(&state, &transfer, root.to_str().unwrap(), &result)
        .await
        .unwrap();

    assert!(!file.exists());
    let ignored = state
        .wishlist
        .read()
        .await
        .list_ignored_results(&item.id)
        .unwrap();
    assert_eq!(ignored.len(), 1);
    assert_eq!(ignored[0].username, "peer");
    assert_eq!(ignored[0].directory, "Remote/Album");

    let _ = std::fs::remove_dir_all(&root);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn spotify_diagnostic_projections_redact_redirect_uri() {
    let private_redirect = "https://spotify.internal/private/callback";
    let env = MapEnv::default()
        .with("SLSKR_SPOTIFY_ENABLED", "true")
        .with("SLSKR_SPOTIFY_CLIENT_ID", "client-id")
        .with("SLSKR_SPOTIFY_REDIRECT_URI", private_redirect);
    let (state, _receiver) = test_state_with_env_parts(env, crate::SearchStore::new(), None);

    let sanitized = state.config.integrations.spotify.sanitized_json();
    assert!(sanitized.contains("\"redirect_uri\":null"));
    assert!(sanitized.contains("\"redirect_uri_configured\":true"));
    assert!(!sanitized.contains("spotify.internal"));
    assert!(!sanitized.contains("/private/callback"));

    let status =
        crate::route_http_request("GET", "/api/integrations/spotify/status", None, "", &state)
            .await
            .expect("Spotify status");
    assert!(!status.body.contains(private_redirect));
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&status.body).unwrap(),
        serde_json::json!({
            "configured": true,
            "connected": false,
            "displayName": "",
            "spotifyUserId": "",
            "scope": "",
        })
    );

    let authorize = crate::route_http_request(
        "POST",
        "/api/v0/integrations/spotify/authorize",
        None,
        "",
        &state,
    )
    .await
    .expect("Spotify authorize");
    assert!(authorize.body.contains("spotify.internal"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn lidarr_scheduler_tracks_the_next_cycle_without_syncing_when_disabled_by_policy()
{
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_LIDARR_ENABLED", "true")
            .with("SLSKR_LIDARR_URL", "http://127.0.0.1:9")
            .with("SLSKR_LIDARR_API_KEY", "fixture-key")
            .with("SLSKR_LIDARR_SYNC_WANTED", "false")
            .with("SLSKR_LIDARR_SYNC_INTERVAL", "600"),
    );
    state.lidarr_sync_state.write().await.next_sync_at = None;

    let delay = crate::run_lidarr_sync_scheduler_cycle(&state).await;

    assert_eq!(delay, std::time::Duration::from_secs(600));
    let sync = state.lidarr_sync_state.read().await;
    assert!(!sync.is_syncing);
    assert!(sync.last_sync_at.is_none());
    assert!(sync.last_result.is_none());
    assert!(sync.last_error.is_none());
    assert!(sync.next_sync_at.is_some());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn lidarr_scheduler_records_external_failure_and_clears_running_state() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let unavailable = listener.local_addr().unwrap();
    drop(listener);
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_LIDARR_ENABLED", "true")
            .with("SLSKR_LIDARR_URL", &format!("http://{unavailable}"))
            .with("SLSKR_LIDARR_API_KEY", "fixture-key")
            .with("SLSKR_LIDARR_TIMEOUT", "1")
            .with("SLSKR_LIDARR_SYNC_WANTED", "true")
            .with("SLSKR_LIDARR_SYNC_INTERVAL", "600"),
    );

    let delay = crate::run_lidarr_sync_scheduler_cycle(&state).await;

    assert_eq!(delay, std::time::Duration::from_secs(600));
    let sync = state.lidarr_sync_state.read().await;
    assert!(!sync.is_syncing);
    assert!(sync.last_sync_at.is_none());
    assert!(sync.last_result.is_none());
    assert!(sync
        .last_error
        .as_deref()
        .is_some_and(|error| error.contains("Lidarr wanted request failed")));
    assert!(sync.next_sync_at.is_some());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn lidarr_wanted_sync_rolls_back_wishlist_when_persistence_fails() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind Lidarr wanted fixture");
    let address = listener.local_addr().expect("Lidarr fixture address");
    let fixture = tokio::spawn(async move {
        serve_json_fixture(
            &listener,
            serde_json::json!({
                "totalRecords": 1,
                "records": [{
                    "title": "Persistence Failure Album",
                    "artist": {"artistName": "Persistence Failure Artist"}
                }]
            }),
        )
        .await
    });
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKR_PERSISTENCE_ENABLED", "true")
            .with("SLSKR_LIDARR_ENABLED", "true")
            .with("SLSKR_LIDARR_URL", &format!("http://{address}"))
            .with("SLSKR_LIDARR_API_KEY", "fixture-key")
            .with("SLSKR_LIDARR_TIMEOUT", "1"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );
    let previous_wishlist = state.wishlist.read().await.clone();
    let lidarr = state.integration_settings.read().await.lidarr.clone();
    db.close_for_test().await;

    let error = crate::sync_lidarr_wanted_to_wishlist(&state, &lidarr)
        .await
        .expect_err("closed database should fail the enabled wanted sync");
    let request = fixture.await.expect("Lidarr wanted fixture task");

    assert!(error.starts_with("wishlist persistence failed:"), "{error}");
    assert!(request.starts_with("GET /api/v1/wanted/missing?page=1&pageSize=250"));
    assert_eq!(*state.wishlist.read().await, previous_wishlist);
}
