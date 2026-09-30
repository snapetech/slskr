use super::*;

#[test]
fn rust_player_surface_summarizes_live_payloads() {
    let (title, detail) = player_now_playing_text(
        r#"{"now_playing":[{"artist":"Archive Artist","album":"Open Sessions","title":"Public Domain Theme"}]}"#,
    );
    assert_eq!(title, "Public Domain Theme");
    assert_eq!(detail, "Archive Artist / Open Sessions");
    assert_eq!(
        player_transfer_text(r#"{"downloads":2,"uploads":1}"#),
        "2 down / 1 up"
    );
    assert_eq!(
        player_party_text(r#"{"count":3,"active_parties":[]}"#),
        "3 listening parties"
    );
    assert_eq!(
        player_visualizer_text(r#"{"status":"configured","configured":true}"#),
        "configured"
    );
    let track = serde_json::json!({
        "artist": "Archive Artist",
        "album": "Open Sessions",
        "title": "Public Domain Theme"
    });
    assert_eq!(
        player_rating_key(&track),
        "meta:archive artist|open sessions|public domain theme"
    );
    assert_eq!(
        player_stream_url(&serde_json::json!({"contentId":"sha256:track"})),
        "/api/v0/streams/sha256%3Atrack"
    );
    assert_eq!(
        player_stream_url(&serde_json::json!({"streamUrl":"/custom/stream"})),
        "/custom/stream"
    );
    assert_eq!(player_rating_summary(1), "Discovery caution");
    assert_eq!(player_rating_summary(3), "Neutral rating");
    assert_eq!(player_rating_summary(5), "Discovery boost");
    assert_eq!(player_rating_summary(0), "Not rated");
    let radio_plan = build_player_radio_plan(Some(&track));
    assert!(radio_plan.ready);
    assert_eq!(
        radio_plan.seed_label,
        "Archive Artist - Public Domain Theme"
    );
    assert_eq!(
        radio_plan.queries,
        vec![
            PlayerRadioQuery {
                id: "radio-query-1".to_string(),
                query: "Archive Artist Public Domain Theme".to_string(),
                reason: "Similar track seed",
            },
            PlayerRadioQuery {
                id: "radio-query-2".to_string(),
                query: "Archive Artist Open Sessions".to_string(),
                reason: "Album neighborhood",
            },
            PlayerRadioQuery {
                id: "radio-query-3".to_string(),
                query: "Archive Artist".to_string(),
                reason: "Artist and genre seed",
            },
        ]
    );
    assert_eq!(
        build_player_radio_search_path(&radio_plan.primary_query),
        "/searches?q=Archive%20Artist%20Public%20Domain%20Theme"
    );
    assert_eq!(
        player_radio_queries(&radio_plan, 2),
        vec![
            "Archive Artist Public Domain Theme".to_string(),
            "Archive Artist Open Sessions".to_string(),
        ]
    );
    assert!(player_radio_copy_text(&radio_plan)
        .contains("Similar track seed: \"Archive Artist Public Domain Theme\""));
    assert_eq!(
        player_radio_query_from_now_playing_body(
            r#"{"track":{"artist":"Archive Artist","title":"Public Domain Theme"}}"#
        ),
        "Archive Artist Public Domain Theme"
    );
    assert_eq!(
        build_player_radio_plan(None),
        PlayerRadioPlan {
            basis: Vec::new(),
            primary_query: String::new(),
            queries: Vec::new(),
            ready: false,
            seed_label: "No track selected".to_string(),
        }
    );
}

#[test]
fn rust_player_surface_builds_similar_queue_candidates() {
    let current = serde_json::json!({
        "album": "Fixture Album",
        "artist": "Fixture Artist",
        "contentId": "sha256:current",
        "genre": "Fixture Genre",
        "title": "Current Song"
    });
    let history = vec![
        serde_json::json!({
            "album": "Fixture Album",
            "artist": "Fixture Artist",
            "contentId": "sha256:album-match",
            "title": "Album Match"
        }),
        serde_json::json!({
            "artist": "Other Artist",
            "contentId": "sha256:tag-match",
            "tags": ["Fixture Genre"],
            "title": "Tag Match"
        }),
        serde_json::json!({
            "artist": "Other Artist",
            "contentId": "sha256:miss",
            "title": "Miss"
        }),
    ];
    let queue = vec![serde_json::json!({"contentId": "sha256:current"})];
    let candidates = build_similar_queue_candidates(Some(&current), &history, &queue, 5);
    assert_eq!(
        candidates
            .iter()
            .map(|candidate| candidate.item["contentId"].as_str().unwrap_or_default())
            .collect::<Vec<_>>(),
        vec!["sha256:album-match", "sha256:tag-match"]
    );
    assert_eq!(
        similar_queue_search_queries(&candidates, 3),
        vec![
            "Fixture Artist Album Match".to_string(),
            "Other Artist Tag Match".to_string(),
        ]
    );

    let duplicate_history = vec![
        serde_json::json!({
            "artist": "Fixture Artist",
            "contentId": "sha256:queued",
            "title": "Already Queued"
        }),
        serde_json::json!({
            "artist": "Fixture Artist",
            "contentId": "sha256:new",
            "title": "New Candidate"
        }),
        serde_json::json!({
            "artist": "Fixture Artist",
            "contentId": "sha256:new",
            "title": "Duplicate Candidate"
        }),
    ];
    let duplicate_queue = vec![
        serde_json::json!({"contentId": "sha256:current"}),
        serde_json::json!({"contentId": "sha256:queued"}),
    ];
    let candidates =
        build_similar_queue_candidates(Some(&current), &duplicate_history, &duplicate_queue, 5);
    assert_eq!(
        candidates
            .iter()
            .map(|candidate| candidate.item["contentId"].as_str().unwrap_or_default())
            .collect::<Vec<_>>(),
        vec!["sha256:new"]
    );
    assert!(build_similar_queue_candidates(None, &history, &queue, 5).is_empty());
}

#[test]
fn rust_browser_local_system_panels_match_react_surfaces() {
    assert_eq!(experience_preferences().len(), 18);
    let defaults = default_experience_preferences();
    assert_eq!(
        defaults
            .get("searchRankingProfile")
            .and_then(|value| value.as_str()),
        Some("balanced")
    );
    assert_eq!(
        defaults
            .get("playerKeyboardShortcuts")
            .and_then(|value| value.as_bool()),
        Some(true)
    );
    let report = experience_preferences_report(&defaults);
    assert!(report.contains("slskr experience preferences"));
    assert!(report.contains("Player: queue_auto_fill=false"));
    assert!(experience_settings_panel_html().contains("data-slskr-pref=\"playerRadioSeedMode\""));

    assert_eq!(automation_recipes().len(), 7);
    let mut state = serde_json::Map::new();
    state.insert(
        "wishlist-retry".to_string(),
        serde_json::json!({
            "enabled": true,
            "lastDryRunAt": "browser-local"
        }),
    );
    let (total, enabled, disabled) = automation_summary_from_state(&state);
    assert_eq!((total, enabled, disabled), (7, 4, 3));
    let dry_run = automation_dry_run_report(automation_recipes()[3], "browser-local");
    assert_eq!(dry_run["recipeId"], "wishlist-retry");
    assert_eq!(dry_run["executed"], false);
    let history = automation_history_report(&state);
    assert!(history.contains("slskr automation review history"));
    assert!(history.contains("Wishlist Retry"));
    assert!(automation_center_panel_html().contains("data-slskr-recipe=\"library-health-scan\""));
}

#[test]
fn rust_search_planner_matches_react_search_helpers() {
    let strong = serde_json::json!({
        "files": [{
            "bitDepth": 16,
            "filename": "Boards of Canada/Music Has The Right/01 Wildlife Analysis.flac",
            "sampleRate": 44100,
            "size": 24000000
        }],
        "hasFreeUploadSlot": true,
        "queueLength": 0,
        "uploadSpeed": 4000000,
        "username": "good-peer"
    });
    let rank = rank_search_candidate(
        &strong,
        "boards canada wildlife analysis",
        "lossless-exact",
        Some(&serde_json::json!({"successfulDownloads": 4, "failedDownloads": 0})),
        None,
        None,
    );
    assert!(rank.score >= 80);
    assert!(rank.reasons.contains(&"strong filename match".to_string()));
    assert!(rank.reasons.contains(&"mostly lossless files".to_string()));
    assert!(rank.reasons.contains(&"free upload slot".to_string()));

    let weak = serde_json::json!({
        "files": [{"bitRate": 128, "filename": "misc/upload/track.mp3", "size": 2000000}],
        "hasFreeUploadSlot": false,
        "queueLength": 8,
        "uploadSpeed": 64000,
        "username": "rough-peer"
    });
    let weak_rank = rank_search_candidate(
        &weak,
        "boards canada wildlife analysis",
        "lossless-exact",
        Some(&serde_json::json!({"successfulDownloads": 0, "failedDownloads": 4})),
        None,
        None,
    );
    assert!(weak_rank.score < 35);
    assert!(weak_rank.reasons.contains(&"long queue".to_string()));
    assert!(weak_rank
        .reasons
        .contains(&"poor download history".to_string()));

    let fast = serde_json::json!({
        "files": [{"bitRate": 320, "filename": "Stereolab/Peng!/Super Falling Star.mp3", "size": 8000000}],
        "hasFreeUploadSlot": true,
        "queueLength": 1,
        "uploadSpeed": 1000000
    });
    let fast_rank = rank_search_candidate(
        &fast,
        "stereolab super falling star",
        "fast-good-enough",
        None,
        None,
        None,
    );
    assert!(fast_rank.score >= 60);
    assert!(fast_rank
        .reasons
        .contains(&"high bitrate fast-good-enough candidate".to_string()));

    let responses = vec![
        serde_json::json!({
            "files": [{"filename": "Artist/Album/01 Track.flac", "size": 24000000}],
            "primarySource": "mesh",
            "sourceProviders": ["mesh"],
            "username": "best-peer"
        }),
        serde_json::json!({
            "files": [{"filename": "Different Root/01 Track.flac", "size": 24000000}],
            "primarySource": "soulseek",
            "sourceProviders": ["soulseek"],
            "username": "backup-peer"
        }),
        serde_json::json!({
            "files": [{"filename": "Artist/Album/02 Other.flac", "size": 22000000}],
            "username": "other-peer"
        }),
    ];
    let (folded, groups) = deduplicate_search_response_groups(&responses, true);
    assert_eq!(folded, 1);
    assert_eq!(groups[0].candidate_count, 2);
    assert_eq!(groups[0].folded_count, 1);
    assert_eq!(
        groups[0].usernames,
        vec!["backup-peer".to_string(), "best-peer".to_string()]
    );

    let preview = build_search_action_preview(
        &serde_json::json!({
            "hasFreeUploadSlot": false,
            "queueLength": 7,
            "sourceProviders": ["pod", "scene"],
            "username": "peer"
        }),
        &[
            serde_json::json!({"filename": "Artist/Album/01 Track.flac", "size": 20}),
            serde_json::json!({"filename": "Artist/Album/02 Track.flac", "locked": true, "size": 30}),
        ],
        Some(&SearchCandidateRank {
            reasons: Vec::new(),
            score: 38,
        }),
        Some(&serde_json::json!({
            "override": {"mode": "ignore", "note": "Known private peer."},
            "score": -6
        })),
        "download",
    );
    assert_eq!(preview.file_count, 2);
    assert_eq!(preview.locked_count, 1);
    assert!(preview
        .warnings
        .contains(&"No free upload slot is currently advertised".to_string()));
    let text = format_search_action_preview(&preview);
    assert!(text.contains("Action: download"));
    assert!(text.contains("Candidate score: 38/100"));
    assert!(
        search_planner_report("public domain theme", "lossless-exact", true)
            .contains("Search planner")
    );
}
