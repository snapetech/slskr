use super::fixtures::*;

#[tokio::test]
async fn versioned_swarm_rejects_oversized_transfer_limits_before_discovery() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "slskdn"));

    let oversized = crate::route_http_request(
        "POST",
        "/api/v0/multisource/swarm/async",
        None,
        r#"{"filename":"Track.flac","size":17592186044417}"#,
        &state,
    )
    .await
    .expect("oversized versioned swarm response");
    assert_eq!(oversized.status, "400 Bad Request");
    assert!(oversized.body.contains("size exceeds"));

    let oversized_chunks = crate::route_http_request(
        "POST",
        "/api/v0/multisource/swarm/async",
        None,
        r#"{"filename":"Track.flac","size":42,"chunkSize":16777216}"#,
        &state,
    )
    .await
    .expect("oversized versioned swarm chunk response");
    assert_eq!(oversized_chunks.status, "400 Bad Request");
    assert!(oversized_chunks.body.contains("chunkSize must be between"));

    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn versioned_swarm_rejects_oversized_source_batches_before_deserialization() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "slskdn"));
    let oversized_sources = (0..crate::multisource::MAX_SOURCES + 1)
        .map(|index| {
            serde_json::json!({
                "username": format!("peer-{index}"),
                "url": "https://source.example/file"
            })
        })
        .collect::<Vec<_>>();
    let response = crate::route_http_request(
        "POST",
        "/api/v0/multisource/swarm/async",
        None,
        &serde_json::json!({
            "filename": "Track.flac",
            "size": 42,
            "expectedHash": "a".repeat(64),
            "sources": oversized_sources
        })
        .to_string(),
        &state,
    )
    .await
    .expect("oversized versioned swarm source response");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response
        .body
        .contains("source count exceeds the 16 source limit"));
    assert!(state.multisource.read().await.list().is_empty());

    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn merge_routes_reject_oversized_arrays_before_store_deserialization() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "slskdn"));

    let oversized_hash_body = serde_json::json!({
        "entries": vec![serde_json::json!({}); crate::content_discovery::MAX_MESH_MERGE_ENTRIES + 1]
    })
    .to_string();
    let hash_response = crate::route_http_request(
        "POST",
        "/api/v0/hashdb/sync/merge",
        None,
        &oversized_hash_body,
        &state,
    )
    .await
    .expect("oversized hash merge response");
    assert_eq!(hash_response.status, "400 Bad Request");
    assert!(hash_response.body.contains("at most 2000 entries"));

    let oversized_records_body = serde_json::json!({
        "records": vec![serde_json::json!({}); crate::content_discovery::MAX_SHADOW_MERGE_RECORDS + 1]
    })
    .to_string();
    let records_response = crate::route_http_request(
        "POST",
        "/api/v0/virtualsoulfind/shadow-index/sync/merge",
        None,
        &oversized_records_body,
        &state,
    )
    .await
    .expect("oversized shadow merge response");
    assert_eq!(records_response.status, "400 Bad Request");
    assert!(records_response.body.contains("at most 256 records"));

    let oversized_indexes_body = serde_json::json!({
        "records": [{"recordingId":"bounded-route-test","peerIds":[]}],
        "realmIndexes": vec![serde_json::json!({}); crate::realm_subject_index::MAX_INDEXES + 1]
    })
    .to_string();
    let indexes_response = crate::route_http_request(
        "POST",
        "/api/v0/virtualsoulfind/shadow-index/sync/merge",
        None,
        &oversized_indexes_body,
        &state,
    )
    .await
    .expect("oversized realm-index merge response");
    assert_eq!(indexes_response.status, "400 Bad Request");
    assert!(indexes_response.body.contains("at most 1024 indexes"));

    let oversized_nested_index = serde_json::json!({
        "id": "nested-limit",
        "realmId": crate::realm_subject_index::DEFAULT_REALM_ID,
        "subjectNamespace": "music",
        "revision": 1,
        "entries": vec![serde_json::json!({
            "subjectId": "nested-limit",
            "workRef": {
                "domain": "music",
                "title": "Nested Limit"
            }
        }); crate::realm_subject_index::MAX_ENTRIES_PER_INDEX + 1]
    });
    let nested_response = crate::route_http_request(
        "POST",
        "/api/v0/virtualsoulfind/shadow-index/sync/merge",
        None,
        &serde_json::json!({
            "records": [{"recordingId":"nested-limit","peerIds":["peer-a"]}],
            "realmIndexes": [oversized_nested_index]
        })
        .to_string(),
        &state,
    )
    .await
    .expect("oversized nested realm-index response");
    assert_eq!(nested_response.status, "400 Bad Request");
    assert!(
        nested_response.body.contains("at most 10000 entries"),
        "{}",
        nested_response.body
    );
    assert!(state
        .realm_subject_indexes
        .read()
        .await
        .indexes_for_realm(crate::realm_subject_index::DEFAULT_REALM_ID)
        .is_empty());
    assert!(state
        .content_discovery
        .read()
        .await
        .shadow_records()
        .iter()
        .all(|record| record.recording_id != "nested-limit"));

    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn browse_response_rejects_oversized_wire_batches_before_store_mutation() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let oversized_entries = (0..=crate::MAX_BROWSE_WIRE_FILES_PER_RESPONSE)
        .map(|index| serde_json::json!({"filename": format!("file-{index}.flac")}))
        .collect::<Vec<_>>();
    let entries_response = crate::route_http_request(
        "POST",
        "/api/v0/browse-responses",
        None,
        &serde_json::json!({
            "username": "oversized-entries",
            "entries": oversized_entries
        })
        .to_string(),
        &state,
    )
    .await
    .expect("oversized browse entries response");
    assert_eq!(entries_response.status, "400 Bad Request");
    assert!(entries_response
        .body
        .contains("browse response exceeds wire entry limits"));
    assert!(state.browse.read().await.get("oversized-entries").is_none());

    let oversized_directory_files = crate::MAX_BROWSE_WIRE_FILES_PER_RESPONSE / 2 + 1;
    let nested_response = crate::route_http_request(
        "POST",
        "/api/v0/browse-responses",
        None,
        &serde_json::json!({
            "username": "oversized-nested-files",
            "directories": [
                {
                    "name": "one",
                    "files": (0..oversized_directory_files)
                        .map(|index| serde_json::json!({"filename": format!("one-{index}.flac")}))
                        .collect::<Vec<_>>()
                },
                {
                    "name": "two",
                    "files": (0..oversized_directory_files)
                        .map(|index| serde_json::json!({"filename": format!("two-{index}.flac")}))
                        .collect::<Vec<_>>()
                }
            ]
        })
        .to_string(),
        &state,
    )
    .await
    .expect("oversized nested browse entries response");
    assert_eq!(nested_response.status, "400 Bad Request");
    assert!(state
        .browse
        .read()
        .await
        .get("oversized-nested-files")
        .is_none());

    let oversized_directories = (0..=crate::MAX_BROWSE_WIRE_FOLDERS_PER_RESPONSE)
        .map(|index| serde_json::json!({"name": format!("folder-{index}")}))
        .collect::<Vec<_>>();
    let directories_response = crate::route_http_request(
        "POST",
        "/api/v0/browse-responses",
        None,
        &serde_json::json!({
            "username": "oversized-directories",
            "directories": oversized_directories
        })
        .to_string(),
        &state,
    )
    .await
    .expect("oversized browse directory response");
    assert_eq!(directories_response.status, "400 Bad Request");
    assert!(state
        .browse
        .read()
        .await
        .get("oversized-directories")
        .is_none());

    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn search_response_rejects_oversized_wire_batches_before_store_mutation() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let search_response = crate::route_http_request(
        "POST",
        "/api/searches",
        None,
        r#"{"query":"oversized-response"}"#,
        &state,
    )
    .await
    .expect("create search for oversized response test");
    assert_eq!(search_response.status, "200 OK", "{}", search_response.body);
    let token = state
        .searches
        .read()
        .await
        .records
        .first()
        .map(|record| record.token)
        .expect("created search token");

    let oversized_files = (0..=crate::MAX_SEARCH_RESULTS_PER_SEARCH)
        .map(|index| serde_json::json!({"filename": format!("file-{index}.flac")}))
        .collect::<Vec<_>>();
    let response = crate::route_http_request(
        "POST",
        "/api/v0/search-responses",
        None,
        &serde_json::json!({"token": token, "files": oversized_files}).to_string(),
        &state,
    )
    .await
    .expect("oversized search response");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response
        .body
        .contains("search response exceeds result limits"));

    let searches = state.searches.read().await;
    let record = searches
        .records
        .iter()
        .find(|record| record.token == token)
        .expect("search remains present");
    assert!(record.results.is_empty());
    assert_eq!(record.raw_response_count, 0);
    drop(searches);
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn collection_reorder_rejects_oversized_wire_batches_before_store_mutation() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let (collection_id, item_id) = {
        let mut collections = state.collections.write().await;
        let collection = collections
            .create(
                "tester".to_owned(),
                "Reorder bounds".to_owned(),
                String::new(),
            )
            .expect("create collection");
        let item = collections
            .add_item(
                &collection.id,
                "content-1".to_owned(),
                "Artist".to_owned(),
                "Track".to_owned(),
                "Audio".to_owned(),
            )
            .expect("add collection item")
            .expect("collection item");
        (collection.id, item.id)
    };

    let oversized_ids = (0..=crate::MAX_COLLECTION_ITEMS)
        .map(|index| serde_json::json!(format!("item-{index}")))
        .collect::<Vec<_>>();
    let response = crate::route_http_request(
        "POST",
        &format!("/api/v0/collections/{collection_id}/items/reorder"),
        None,
        &serde_json::json!({"itemIds": oversized_ids}).to_string(),
        &state,
    )
    .await
    .expect("oversized collection reorder");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response
        .body
        .contains("collection reorder exceeds item limits"));

    let collections = state.collections.read().await;
    let record = collections
        .get(&collection_id)
        .expect("collection remains present");
    assert_eq!(record.items.len(), 1);
    assert_eq!(record.items[0].id, item_id);
    drop(collections);
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn musicbrainz_rejects_oversized_json_batches_before_state_mutation() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let oversized_recording_ids = (0..=crate::MAX_LIBRARY_ITEMS)
        .map(|index| serde_json::json!(format!("recording-{index}")))
        .collect::<Vec<_>>();
    let diff_response = crate::route_http_request(
        "POST",
        "/api/v0/musicbrainz/library-bloom/diffs",
        None,
        &serde_json::json!({"recordingIds": oversized_recording_ids}).to_string(),
        &state,
    )
    .await
    .expect("oversized MusicBrainz diff");
    assert_eq!(diff_response.status, "400 Bad Request");
    assert!(diff_response
        .body
        .contains("recordingIds must contain at most"));

    let oversized_suggestions = (0..=crate::MAX_WISHLIST_ITEMS)
        .map(|index| serde_json::json!({"artist": "Artist", "title": format!("Release {index}")}))
        .collect::<Vec<_>>();
    let wishlist_response = crate::route_http_request(
        "POST",
        "/api/v0/musicbrainz/library-bloom/wishlist",
        None,
        &serde_json::json!({"suggestions": oversized_suggestions}).to_string(),
        &state,
    )
    .await
    .expect("oversized MusicBrainz wishlist");
    assert_eq!(wishlist_response.status, "400 Bad Request");
    assert!(wishlist_response
        .body
        .contains("suggestions must contain at most"));
    assert!(state.wishlist.read().await.records.is_empty());

    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn transfer_rejects_oversized_file_batches_before_queue_mutation() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let files = (0..=crate::MAX_TRANSFER_REQUEST_FILES)
        .map(|index| serde_json::json!({"filename": format!("Music/{index}.flac"), "size": 1}))
        .collect::<Vec<_>>();
    let body = serde_json::json!({"username": "peer", "files": files}).to_string();
    for path in [
        "/api/v0/transfers/downloads/batches",
        "/api/v0/transfers/downloads/peer",
        "/api/transfers",
    ] {
        let response = crate::route_http_request("POST", path, None, &body, &state)
            .await
            .expect("oversized transfer request");
        assert_eq!(response.status, "400 Bad Request", "{path}");
        assert!(
            response.body.contains("file limits"),
            "{path}: {}",
            response.body
        );
    }
    assert!(state.transfers.read().await.entries.is_empty());
    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn string_array_routes_reject_oversized_wire_batches_before_mutation() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let recipients = (0..=crate::MAX_PRIVATE_MESSAGE_RECIPIENTS)
        .map(|index| serde_json::json!(format!("peer-{index}")))
        .collect::<Vec<_>>();
    let response = crate::route_http_request(
        "POST",
        "/api/v0/conversations/batch",
        None,
        &serde_json::json!({"usernames": recipients, "body": "hello"}).to_string(),
        &state,
    )
    .await
    .expect("oversized conversation batch");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response
        .body
        .contains("conversation batch exceeds recipient limits"));
    assert!(state.messages.read().await.records.is_empty());

    let item_ids = (0..=crate::MAX_WISHLIST_ITEMS)
        .map(|index| serde_json::json!(format!("00000000-0000-0000-0000-{index:012x}")))
        .collect::<Vec<_>>();
    let response = crate::route_http_request(
        "PUT",
        "/api/v0/wishlist/bulk-filter",
        None,
        &serde_json::json!({"itemIds": item_ids, "filter": "flac"}).to_string(),
        &state,
    )
    .await
    .expect("oversized wishlist filter batch");
    assert_eq!(response.status, "400 Bad Request");
    assert!(
        response
            .body
            .contains("wishlist bulk filter exceeds item limits"),
        "{}",
        response.body
    );
    assert!(state.wishlist.read().await.records.is_empty());

    let capabilities = (0..=crate::MAX_CAPABILITY_NEGOTIATION_ITEMS)
        .map(|index| serde_json::json!(format!("capability-{index}")))
        .collect::<Vec<_>>();
    let response = crate::route_http_request(
        "POST",
        "/api/v0/capabilities/negotiate",
        None,
        &serde_json::json!({"capabilities": capabilities}).to_string(),
        &state,
    )
    .await
    .expect("oversized capability negotiation");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response
        .body
        .contains("capabilities negotiation exceeds item limits"));

    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn mediacore_rejects_oversized_wire_batches_before_work() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let descriptors = (0..=crate::MAX_MEDIACORE_BATCH_ITEMS)
        .map(|index| serde_json::json!({"contentId": format!("content:test:{index}")}))
        .collect::<Vec<_>>();
    let content_ids = (0..=crate::MAX_MEDIACORE_BATCH_ITEMS)
        .map(|index| serde_json::json!(format!("content:test:{index}")))
        .collect::<Vec<_>>();
    let keys = (0..=crate::MAX_MEDIACORE_BATCH_ITEMS)
        .map(|index| serde_json::json!(format!("cache-key-{index}")))
        .collect::<Vec<_>>();
    for (path, body, expected) in [
        (
            "/api/v0/mediacore/publish/batch",
            serde_json::json!({"descriptors": descriptors}).to_string(),
            "descriptors must contain at most 100 items",
        ),
        (
            "/api/v0/mediacore/publish/republish",
            serde_json::json!({"contentIds": content_ids.clone()}).to_string(),
            "contentIds must contain at most 100 items",
        ),
        (
            "/api/v0/mediacore/retrieve/batch",
            serde_json::json!({"contentIds": content_ids}).to_string(),
            "contentIds must contain at most 100 items",
        ),
        (
            "/api/v0/mediacore/retrieve/cache/clear",
            serde_json::json!({"keys": keys}).to_string(),
            "keys must contain at most 100 items",
        ),
    ] {
        let response = crate::route_http_request("POST", path, None, &body, &state)
            .await
            .expect("oversized MediaCore request");
        assert_eq!(response.status, "400 Bad Request", "{path}");
        assert!(
            response.body.contains(expected),
            "{path}: {}",
            response.body
        );
    }

    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn remaining_controller_array_routes_reject_oversized_wire_batches_before_work() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());

    let download_items = (0..=crate::MAX_EXTENDED_DOWNLOAD_ITEMS)
        .map(|index| {
            serde_json::json!({
                "user": "peer",
                "remotePath": format!("Music/{index}.flac"),
            })
        })
        .collect::<Vec<_>>();
    let response = crate::route_http_request(
        "POST",
        "/api/v0/downloads",
        None,
        &serde_json::json!({"items": download_items}).to_string(),
        &state,
    )
    .await
    .expect("oversized extended download request");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response
        .body
        .contains("items must contain at most 1000 items"));
    assert!(state.transfers.read().await.entries.is_empty());

    let links = (0..=crate::MAX_MEDIACORE_BATCH_ITEMS)
        .map(|index| {
            serde_json::json!({
                "name": format!("link-{index}"),
                "target": format!("target-{index}"),
            })
        })
        .collect::<Vec<_>>();
    let response = crate::route_http_request(
        "POST",
        "/api/v0/mediacore/ipld/links/content:test:oversized",
        None,
        &serde_json::json!({"links": links}).to_string(),
        &state,
    )
    .await
    .expect("oversized MediaCore links request");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response
        .body
        .contains("links must contain at most 100 items"));

    let entries = (0..=crate::MAX_MEDIACORE_PORTABILITY_ENTRIES)
        .map(|index| serde_json::json!({"contentId": format!("content:test:{index}")}))
        .collect::<Vec<_>>();
    for (path, body, expected) in [
        (
            "/api/v0/mediacore/portability/analyze",
            serde_json::json!({"package": {"entries": entries.clone()}}).to_string(),
            "entries must contain at most 1000 items",
        ),
        (
            "/api/v0/mediacore/portability/import",
            serde_json::json!({"package": {"entries": entries}}).to_string(),
            "entries must contain at most 1000 items",
        ),
    ] {
        let response = crate::route_http_request("POST", path, None, &body, &state)
            .await
            .expect("oversized MediaCore portability request");
        assert_eq!(response.status, "400 Bad Request", "{path}");
        assert!(
            response.body.contains(expected),
            "{path}: {}",
            response.body
        );
    }

    let content_ids = (0..=crate::MAX_MEDIACORE_PORTABILITY_ENTRIES)
        .map(|index| serde_json::json!(format!("content:test:export:{index}")))
        .collect::<Vec<_>>();
    let response = crate::route_http_request(
        "POST",
        "/api/v0/mediacore/portability/export",
        None,
        &serde_json::json!({"contentIds": content_ids}).to_string(),
        &state,
    )
    .await
    .expect("oversized MediaCore export request");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response
        .body
        .contains("contentIds must contain at most 1000 items"));

    let sources = (0..=crate::multisource::MAX_SOURCES)
        .map(|index| serde_json::json!({"username": format!("peer-{index}")}))
        .collect::<Vec<_>>();
    let response = crate::route_http_request(
        "POST",
        "/api/multisource/test",
        None,
        &serde_json::json!({"sources": sources}).to_string(),
        &state,
    )
    .await
    .expect("oversized multisource test request");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response
        .body
        .contains("source count exceeds the 16 source limit"));

    let jurors = (0..=crate::MAX_QUARANTINE_JURY_ITEMS)
        .map(|index| serde_json::json!(format!("juror-{index}")))
        .collect::<Vec<_>>();
    let response = crate::route_http_request(
        "POST",
        "/api/v0/quarantine-jury/requests",
        None,
        &serde_json::json!({
            "localReason": "test",
            "jurors": jurors,
            "evidence": [{"reference": "safe-reference", "summary": "test"}],
        })
        .to_string(),
        &state,
    )
    .await
    .expect("oversized quarantine request");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response
        .body
        .contains("quarantine jury arrays must contain at most 100 items"));

    let usernames = (0..=crate::MAX_RANKING_BATCH_ITEMS)
        .map(|index| serde_json::json!(format!("peer-{index}")))
        .collect::<Vec<_>>();
    let response = crate::route_http_request(
        "POST",
        "/api/v0/ranking/history",
        None,
        &serde_json::to_string(&usernames).expect("ranking usernames JSON"),
        &state,
    )
    .await
    .expect("oversized ranking history request");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response.body.contains("at most 1000 usernames"));

    let candidates = (0..=crate::MAX_RANKING_BATCH_ITEMS)
        .map(|index| {
            serde_json::json!({
                "username": format!("peer-{index}"),
                "filename": "Music/test.flac",
            })
        })
        .collect::<Vec<_>>();
    let response = crate::route_http_request(
        "POST",
        "/api/v0/ranking/rank",
        None,
        &serde_json::to_string(&candidates).expect("ranking candidates JSON"),
        &state,
    )
    .await
    .expect("oversized ranking request");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response.body.contains("at most 1000 source candidates"));

    let issue_ids = (0..=25)
        .map(|index| serde_json::json!(format!("issue-{index}")))
        .collect::<Vec<_>>();
    let response = crate::route_http_request(
        "POST",
        "/api/slskdn/library/remediate",
        None,
        &serde_json::json!({"issue_ids": issue_ids}).to_string(),
        &state,
    )
    .await
    .expect("oversized library remediation request");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response
        .body
        .contains("issue_ids must contain 1 to 25 values"));

    let muted_release_group_ids = (0..=crate::MAX_RADAR_MUTED_RELEASE_GROUPS)
        .map(|index| serde_json::json!(format!("release-group-{index}")))
        .collect::<Vec<_>>();
    let response = crate::route_http_request(
        "POST",
        "/api/v0/musicbrainz/release-radar/subscriptions",
        None,
        &serde_json::json!({
            "artistId": "artist:test",
            "mutedReleaseGroupIds": muted_release_group_ids,
        })
        .to_string(),
        &state,
    )
    .await
    .expect("oversized release-radar subscription request");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response
        .body
        .contains("mutedReleaseGroupIds must contain at most 256 items"));

    let target_peer_ids = (0..=crate::MAX_ROUTING_TARGET_PEERS)
        .map(|index| serde_json::json!(format!("peer-{index}")))
        .collect::<Vec<_>>();
    let response = crate::route_http_request(
        "POST",
        "/api/v0/podcore/routing/route-to-peers",
        None,
        &serde_json::json!({
            "message": {"messageId": "message:test", "channelId": "channel:test"},
            "targetPeerIds": target_peer_ids,
        })
        .to_string(),
        &state,
    )
    .await
    .expect("oversized PodCore routing request");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response
        .body
        .contains("targetPeerIds must contain at most 256 items"));

    let tags = (0..=crate::MAX_POD_DISCOVERY_TAGS)
        .map(|index| serde_json::json!(format!("tag-{index}")))
        .collect::<Vec<_>>();
    let response = crate::route_http_request(
        "POST",
        "/api/v0/podcore/discovery/register",
        None,
        &serde_json::json!({
            "podId": "pod:test",
            "visibility": "listed",
            "name": "test",
            "tags": tags,
        })
        .to_string(),
        &state,
    )
    .await
    .expect("oversized PodCore discovery request");
    assert_eq!(response.status, "400 Bad Request");
    assert!(response
        .body
        .contains("tags must contain at most 100 items"));

    let _ = fs::remove_dir_all(&state.config.state_dir);
}

#[tokio::test]
async fn wishlist_csv_import_rejects_oversized_row_batches() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let oversized_csv = (0..=crate::MAX_CSV_IMPORT_ROWS)
        .map(|index| format!("artist-{index},title-{index}"))
        .collect::<Vec<_>>()
        .join("\n");

    let unversioned_response = crate::route_http_request(
        "POST",
        "/api/wishlist/import/csv",
        None,
        &oversized_csv,
        &state,
    )
    .await
    .expect("oversized unversioned CSV import");
    assert_eq!(unversioned_response.status, "400 Bad Request");
    assert!(unversioned_response
        .body
        .contains("CSV import exceeds 10000 rows"));

    let versioned_response = crate::route_http_request(
        "POST",
        "/api/v0/wishlist/import/csv",
        None,
        &serde_json::json!({"csvText": oversized_csv}).to_string(),
        &state,
    )
    .await
    .expect("oversized versioned CSV import");
    assert_eq!(versioned_response.status, "400 Bad Request");
    assert!(versioned_response
        .body
        .contains("CSV import exceeds 10000 rows"));
    let preview_response = crate::route_http_request(
        "POST",
        "/api/v0/source-feed-imports/preview",
        None,
        &serde_json::json!({
            "sourceText": oversized_csv,
            "sourceKind": "csv",
            "fetchProviderUrls": false,
            "limit": 500,
        })
        .to_string(),
        &state,
    )
    .await
    .expect("oversized source preview");
    assert_eq!(preview_response.status, "400 Bad Request");
    assert!(preview_response
        .body
        .contains("CSV import exceeds 10000 rows"));
    assert!(state.wishlist.read().await.records.is_empty());

    let _ = fs::remove_dir_all(&state.config.state_dir);
}
