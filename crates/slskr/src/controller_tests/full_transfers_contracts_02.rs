//! Controller full transfers contracts 02 ownership.

use super::*;

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn download_layout_destination_slots_and_pacing_are_live_runtime_consumers() {
    let (slskdn, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKD_DOWNLOAD_SLOTS", "1")
            .with("SLSKD_DOWNLOAD_SPEED_LIMIT", "100"),
    );
    let batch_id = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
    for (layout, expected) in [
        ("remote_folder", "Albums/Record/Song.flac"),
        ("uploader_folder", "friend/Record/Song.flac"),
        ("batch_id", "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa/Song.flac"),
        ("flat", "Song.flac"),
    ] {
        slskdn
            .transfer_download_settings
            .write()
            .await
            .completed_layout = layout.to_owned();
        assert_eq!(
            crate::render_configured_completed_download_path(
                &slskdn,
                "friend",
                "Albums/Record/Song.flac",
                Some(batch_id),
                None,
                0,
            )
            .await
            .unwrap(),
            expected
        );
    }
    let active_id = {
        let mut transfers = slskdn.transfers.write().await;
        let active = transfers.create(
            0,
            Some("friend".to_owned()),
            "Remote/Active.flac".to_owned(),
            None,
            Some(1),
        );
        transfers.update_status(active.id, "in_progress", None, None);
        active.id
    };
    assert!(!crate::file_transfer_runtime::download_capacity_available(&slskdn, None).await);
    assert!(
        crate::file_transfer_runtime::download_capacity_available(&slskdn, Some(active_id)).await
    );
    assert_eq!(crate::effective_download_pacing_limit(&slskdn).await, 100);
    {
        let mut settings = slskdn.transfer_download_settings.write().await;
        settings.slots = 2;
    }
    {
        let mut transfers = slskdn.transfers.write().await;
        let second = transfers.create(
            0,
            Some("ally".to_owned()),
            "Remote/Second.flac".to_owned(),
            None,
            Some(1),
        );
        transfers.update_status(second.id, "in_progress", None, None);
    }
    assert_eq!(crate::effective_download_pacing_limit(&slskdn).await, 50);

    let (slskd, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));
    assert_eq!(
        crate::render_configured_completed_download_path(
            &slskd,
            "friend",
            "Albums/Record/Song.flac",
            None,
            None,
            0,
        )
        .await
        .unwrap(),
        "Record/Song.flac"
    );
    slskd
        .transfer_download_settings
        .write()
        .await
        .destination
        .subdirectory = Some("${SOURCE_USERNAME}".to_owned());
    assert_eq!(
        crate::render_configured_completed_download_path(
            &slskd,
            "friend",
            "Albums/Record/Song.flac",
            None,
            None,
            0,
        )
        .await
        .unwrap(),
        "friend/Song.flac"
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn download_retry_destination_permissions_and_auto_replace_settings_drive_runtime()
{
    use std::io::Write;

    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with(
                "SLSKR_FROZEN_TRANSFER_DOWNLOAD_JSON",
                r#"{"retry":{"partial":"resume","attempts":4,"delay":10000,"max_delay":30000},"destination":{"exists":"rename","permissions":{"mode":"0640"}}}"#,
            ),
    );
    let retry = state.transfer_download_settings.read().await.retry.clone();
    assert_eq!(
        crate::download_retry_delay(&retry, 1),
        Duration::from_secs(10)
    );
    assert_eq!(
        crate::download_retry_delay(&retry, 2),
        Duration::from_secs(20)
    );
    assert_eq!(
        crate::download_retry_delay(&retry, 3),
        Duration::from_secs(30)
    );

    let incomplete_root = crate::effective_incomplete_dir(&state);
    let incomplete = crate::safe_download_path(&incomplete_root, "1-1.part").unwrap();
    let incomplete =
        crate::ensure_scoped_download_path(&incomplete_root, incomplete.to_string_lossy().as_ref())
            .unwrap();
    fs::write(&incomplete, [1_u8, 2, 3]).unwrap();
    let (mut resumed, offset) =
        crate::prepare_incomplete_download_file(&incomplete_root, &incomplete, "resume", 4)
            .unwrap();
    assert_eq!(offset, 3);
    resumed.write_all(&[4]).unwrap();
    drop(resumed);
    let (overwritten, offset) =
        crate::prepare_incomplete_download_file(&incomplete_root, &incomplete, "overwrite", 4)
            .unwrap();
    assert_eq!(offset, 0);
    assert_eq!(overwritten.metadata().unwrap().len(), 0);
    drop(overwritten);

    let downloads = crate::effective_downloads_dir(&state);
    if let Some(destination) = state
        .destinations
        .write()
        .await
        .records
        .iter_mut()
        .find(|destination| destination.is_default)
    {
        destination.path = downloads.display().to_string();
    }
    let existing = crate::safe_download_path(&downloads, "Album/Song.flac").unwrap();
    fs::create_dir_all(existing.parent().unwrap()).unwrap();
    fs::write(&existing, b"existing").unwrap();
    let renamed = crate::configured_download_destination_path(&state, "Album/Song.flac")
        .await
        .unwrap();
    assert_ne!(renamed, existing);
    assert!(renamed
        .file_name()
        .unwrap()
        .to_string_lossy()
        .starts_with("Song_"));
    assert_eq!(fs::read(&existing).unwrap(), b"existing");

    state
        .transfer_download_settings
        .write()
        .await
        .destination
        .exists = "overwrite".to_owned();
    assert_eq!(
        crate::configured_download_destination_path(&state, "Album/Song.flac")
            .await
            .unwrap(),
        existing
    );

    let completed = {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create(
            0,
            Some("friend".to_owned()),
            "Remote/Mode.flac".to_owned(),
            Some(existing.display().to_string()),
            Some(8),
        );
        transfers
            .update_local_execution(entry.id, "succeeded", 8, Some(8), None)
            .unwrap()
    };
    let completed = crate::apply_completed_download_permissions(&state, completed).await;
    assert_eq!(completed.status, "succeeded");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&existing).unwrap().permissions().mode() & 0o777,
            0o640
        );
    }

    let (slskdn, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with(
                "SLSKR_FROZEN_TRANSFER_DOWNLOAD_JSON",
                r#"{"auto_replace_stuck":true,"auto_replace_threshold":7.5,"auto_replace_interval":91}"#,
            ),
    );
    let base = slskdn.transfer_auto_retry_settings.read().await.clone();
    let download = slskdn.transfer_download_settings.read().await.clone();
    let auto_replace = crate::auto_replace_retry_settings(&base, &download);
    assert!(auto_replace.enabled);
    assert_eq!(auto_replace.retry_delay, Duration::ZERO);
    assert_eq!(auto_replace.check_interval, Duration::from_secs(91));
    assert!(auto_replace.alternate_sources_enabled);
    assert_eq!(auto_replace.alternate_source_size_tolerance_percent, 7.5);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn transfer_active_statuses_include_peer_lifecycle() {
    assert!(crate::is_active_transfer_status("in_progress"));
    assert!(crate::is_active_transfer_status("peer_lookup"));
    assert!(crate::is_active_transfer_status("peer_negotiating"));
    assert!(crate::is_active_transfer_status("accepted"));
    assert!(crate::is_active_transfer_status("indirect_pending"));
    assert!(!crate::is_active_transfer_status("queued"));
    assert!(!crate::is_active_transfer_status("failed"));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn transfer_queue_records_rejections_with_limit() {
    let mut queue = crate::TransferQueue::new_in_memory(1);

    queue.record_rejected_request(1, 10, "a.flac".to_owned(), Some(5), "disabled".to_owned());
    queue.record_rejected_request(1, 11, "b.flac".to_owned(), Some(6), "disabled".to_owned());

    assert_eq!(queue.entries.len(), 1);
    assert_eq!(queue.entries[0].id, 2);
    assert_eq!(queue.entries[0].filename, "b.flac");
    assert_eq!(queue.entries[0].status, "rejected");

    let _ = std::fs::remove_file(queue.events_path);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn outbound_peer_dial_order_honors_compatibility_prefer_and_disabled_modes() {
    let compatibility =
        crate::AppConfig::from_layers(None, FileConfig::default(), &MapEnv::default())
            .expect("compatibility config");
    assert_eq!(
        crate::outbound_peer_dial_order(&compatibility, true, true),
        vec![crate::OutboundPeerTransport::Regular]
    );

    let compatibility_with_obfuscation = crate::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSK_OBFUSCATION", "true"),
    )
    .expect("compatibility obfuscation config");
    assert_eq!(
        crate::outbound_peer_dial_order(&compatibility_with_obfuscation, false, true),
        vec![crate::OutboundPeerTransport::Obfuscated]
    );

    let prefer = crate::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default().with("SLSK_OBFUSCATION_MODE", "prefer"),
    )
    .expect("prefer config");
    assert_eq!(
        crate::outbound_peer_dial_order(&prefer, true, true),
        vec![
            crate::OutboundPeerTransport::Obfuscated,
            crate::OutboundPeerTransport::Regular,
        ]
    );

    let disabled = crate::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default().with("SLSK_OBFUSCATION", "false"),
    )
    .expect("disabled config");
    assert_eq!(
        crate::outbound_peer_dial_order(&disabled, true, true),
        vec![crate::OutboundPeerTransport::Regular]
    );

    let prefer_disabled = crate::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSK_OBFUSCATION_MODE", "prefer")
            .with("SLSK_OBFUSCATION_PREFER_OUTBOUND", "false"),
    )
    .expect("prefer disabled config");
    assert_eq!(
        crate::outbound_peer_dial_order(&prefer_disabled, true, true),
        vec![crate::OutboundPeerTransport::Regular]
    );

    let invalid_frozen = crate::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSK_OBFUSCATION_ADVERTISE_REGULAR_PORT", "false"),
    )
    .expect("frozen compatibility profile accepts the legacy option combination");
    let frozen_error = crate::frozen_obfuscation_startup_error(&invalid_frozen)
        .expect("frozen compatibility profile must report its startup limitation");
    assert!(frozen_error.contains("regular peer port must be advertised"));

    let invalid_current = crate::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default().with("SLSK_OBFUSCATION_ADVERTISE_REGULAR_PORT", "false"),
    )
    .expect_err("current behavior must reject hiding the regular port");
    assert!(invalid_current.contains("regular peer port must be advertised"));

    let valid_frozen = crate::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"),
    )
    .expect("valid frozen obfuscation options");
    assert!(crate::frozen_obfuscation_startup_error(&valid_frozen).is_none());
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn auto_retry_tracker_prunes_evicted_and_expired_state() {
    let now = crate::unix_timestamp();
    let mut queue = crate::TransferQueue::new_in_memory(4);
    let live = queue.create(
        0,
        Some("live-peer".to_owned()),
        "Remote/Live.flac".to_owned(),
        None,
        Some(100),
    );
    let mut tracker = crate::AutoRetryTracker::default();
    tracker.retried_ids.extend([live.id, live.id + 100]);
    tracker
        .alternate_search_requested_at
        .extend([(live.id, now), (live.id + 100, now)]);
    tracker.retry_counts.extend([
        (crate::auto_retry_key("live-peer", &live.filename), 1),
        (crate::auto_retry_key("evicted-peer", "Gone.mp3"), 2),
    ]);
    tracker.peer_retry_after.extend([
        ("live-peer".to_owned(), now + 30),
        ("expired".to_owned(), now),
    ]);

    crate::prune_auto_retry_tracker(&mut tracker, &queue.entries, now);

    assert_eq!(tracker.retried_ids, HashSet::from([live.id]));
    assert_eq!(tracker.alternate_search_requested_at.len(), 1);
    assert!(tracker.alternate_search_requested_at.contains_key(&live.id));
    assert_eq!(tracker.retry_counts.len(), 1);
    assert!(tracker
        .retry_counts
        .contains_key(&crate::auto_retry_key("live-peer", &live.filename)));
    assert_eq!(tracker.peer_retry_after.len(), 1);
    assert_eq!(tracker.peer_retry_after["live-peer"], now + 30);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn auto_retry_plan_is_bounded_latest_only_and_network_friendly() {
    assert!(crate::alternate_size_is_eligible(Some(1_000), 1_050, 5.0));
    assert!(!crate::alternate_size_is_eligible(Some(1_000), 1_051, 5.0));
    let config = crate::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_TRANSFER_AUTO_RETRY_DELAY_SECONDS", "10")
            .with("SLSKR_TRANSFER_AUTO_RETRY_MAX_ATTEMPTS", "1")
            .with(
                "SLSKR_TRANSFER_AUTO_RETRY_MAX_FILES_PER_PEER_PER_CYCLE",
                "1",
            ),
    )
    .expect("auto retry config");
    let now = crate::unix_timestamp();
    let mut queue = crate::TransferQueue::new_in_memory(16);
    for (peer, filename) in [
        ("peer-a", "Remote/First.flac"),
        ("peer-a", "Remote/Second.mp3"),
        ("peer-b", "Remote/Notes.txt"),
    ] {
        let entry = queue.create(
            0,
            Some(peer.to_owned()),
            filename.to_owned(),
            None,
            Some(100),
        );
        queue.update_status(entry.id, "failed", None, Some("connection lost".to_owned()));
        queue
            .entries
            .iter_mut()
            .find(|candidate| candidate.id == entry.id)
            .unwrap()
            .updated_at = now - 11;
    }
    let searches = crate::SearchStore::new();
    let mut tracker = crate::AutoRetryTracker::default();
    let plan = crate::create_auto_retry_plan(
        &queue.entries,
        &searches,
        &tracker,
        &config.transfer_auto_retry,
        now,
        10,
    );
    assert_eq!(plan.len(), 1);
    assert_eq!(plan[0].source.filename, "Remote/First.flac");

    tracker
        .retry_counts
        .insert(crate::auto_retry_key("peer-a", "Remote/First.flac"), 1);
    let next = crate::create_auto_retry_plan(
        &queue.entries,
        &searches,
        &tracker,
        &config.transfer_auto_retry,
        now,
        10,
    );
    assert_eq!(next.len(), 1);
    assert_eq!(next[0].source.filename, "Remote/Second.mp3");
    tracker
        .retry_counts
        .insert(crate::auto_retry_key("peer-a", "Remote/Second.mp3"), 1);
    assert!(crate::create_auto_retry_plan(
        &queue.entries,
        &searches,
        &tracker,
        &config.transfer_auto_retry,
        now,
        10,
    )
    .is_empty());

    tracker.retry_counts.clear();
    tracker
        .peer_retry_after
        .insert("peer-a".to_owned(), now + 60);
    assert!(crate::create_auto_retry_plan(
        &queue.entries,
        &searches,
        &tracker,
        &config.transfer_auto_retry,
        now,
        10,
    )
    .is_empty());

    let _ = std::fs::remove_file(queue.events_path);
    let _ = std::fs::remove_file(queue.state_path);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn auto_retry_cycle_creates_a_metadata_preserving_attempt_once() {
    let (state, mut receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_TRANSFER_AUTO_RETRY_DELAY_SECONDS", "10")
            .with(
                "SLSKR_TRANSFER_AUTO_RETRY_ALTERNATE_SOURCES_ENABLED",
                "false",
            ),
        crate::SearchStore::new(),
        None,
    );
    let now = crate::unix_timestamp();
    let source = {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create_with_details(
            0,
            Some("peer-a".to_owned()),
            "Remote/Album/Song.flac".to_owned(),
            Some("/tmp/Song.flac".to_owned()),
            Some(1_000),
            Some("batch-1".to_owned()),
            crate::TransferRequestDetails {
                request_id: Some("request-1".to_owned()),
                request_name: Some("Song".to_owned()),
                artist: Some("Artist".to_owned()),
                ..Default::default()
            },
        );
        transfers.update_status(
            entry.id,
            "failed",
            Some(123),
            Some("connection lost".to_owned()),
        );
        let source = transfers
            .entries
            .iter_mut()
            .find(|candidate| candidate.id == entry.id)
            .unwrap();
        source.updated_at = now - 11;
        source.clone()
    };
    let mut tracker = crate::AutoRetryTracker::default();

    assert_eq!(
        crate::transfer_recovery_runtime::run_download_auto_retry_cycle(&state, &mut tracker)
            .await
            .expect("auto retry cycle"),
        1
    );
    let command = receiver.recv().await.expect("transfer dispatch");
    let crate::SessionCommand::TransferPeer { id, username } = command else {
        panic!("unexpected command: {command:?}");
    };
    assert_eq!(username, "peer-a");
    let transfers = state.transfers.read().await;
    let original = transfers
        .entries
        .iter()
        .find(|entry| entry.id == source.id)
        .unwrap();
    let replacement = transfers
        .entries
        .iter()
        .find(|entry| entry.id == id)
        .unwrap();
    assert_eq!(original.status, "failed");
    assert_eq!(replacement.status, "peer_lookup");
    assert_eq!(replacement.request_id.as_deref(), Some("request-1"));
    assert_eq!(replacement.batch_id.as_deref(), Some("batch-1"));
    assert_eq!(replacement.artist.as_deref(), Some("Artist"));
    assert_eq!(replacement.bytes_transferred, 0);
    drop(transfers);

    assert_eq!(
        crate::transfer_recovery_runtime::run_download_auto_retry_cycle(&state, &mut tracker)
            .await
            .expect("deduplicated cycle"),
        0
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn auto_retry_searches_then_uses_a_verified_cached_alternate() {
    let (state, mut receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_TRANSFER_AUTO_RETRY_DELAY_SECONDS", "10"),
        crate::SearchStore::new(),
        None,
    );
    let now = crate::unix_timestamp();
    let source_id = {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create(
            0,
            Some("peer-a".to_owned()),
            "Remote/Album/Song.flac".to_owned(),
            None,
            Some(1_000),
        );
        transfers.update_status(entry.id, "failed", None, Some("timed out".to_owned()));
        transfers
            .entries
            .iter_mut()
            .find(|candidate| candidate.id == entry.id)
            .unwrap()
            .updated_at = now - 11;
        entry.id
    };
    let mut tracker = crate::AutoRetryTracker::default();

    assert_eq!(
        crate::transfer_recovery_runtime::run_download_auto_retry_cycle(&state, &mut tracker)
            .await
            .expect("discovery cycle"),
        0
    );
    let crate::SessionCommand::Search { token, query, .. } =
        receiver.recv().await.expect("search dispatch")
    else {
        panic!("expected alternate-source search");
    };
    assert_eq!(query, "Song.flac");
    state
        .searches
        .write()
        .await
        .add_peer_response(&FileSearchResponse {
            username: "peer-b".to_owned(),
            token,
            results: vec![FileEntry {
                filename_encoding: Default::default(),
                extension_encoding: Default::default(),
                code: 1,
                filename: "Other/Album/Song.flac".to_owned(),
                size: 1_040,
                extension: "flac".to_owned(),
                attributes: Vec::new(),
            }],
            slot_free: true,
            average_speed: 10_000,
            queue_length: 0,
            unknown: 0,
            private_results: Vec::new(),
        })
        .expect("search response");

    assert_eq!(
        crate::transfer_recovery_runtime::run_download_auto_retry_cycle(&state, &mut tracker)
            .await
            .expect("alternate retry cycle"),
        1
    );
    let crate::SessionCommand::TransferPeer { id, username } =
        receiver.recv().await.expect("alternate dispatch")
    else {
        panic!("expected transfer dispatch");
    };
    assert_eq!(username, "peer-b");
    let transfers = state.transfers.read().await;
    assert_eq!(
        transfers
            .entries
            .iter()
            .find(|entry| entry.id == id)
            .unwrap()
            .filename,
        "Other/Album/Song.flac"
    );
    assert!(tracker.retried_ids.contains(&source_id));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn rescue_plan_detects_queue_throughput_and_stall_but_skips_sidecars() {
    let config = crate::AppConfig::from_layers(
        None,
        FileConfig::default(),
        &MapEnv::default()
            .with("SLSKR_TRANSFER_RESCUE_MAX_QUEUE_TIME_SECONDS", "60")
            .with("SLSKR_TRANSFER_RESCUE_MIN_THROUGHPUT_KBPS", "1")
            .with("SLSKR_TRANSFER_RESCUE_MIN_DURATION_SECONDS", "60")
            .with("SLSKR_TRANSFER_RESCUE_STALLED_TIMEOUT_SECONDS", "30")
            .with("SLSKR_TRANSFER_RESCUE_MAX_FILES_PER_CYCLE", "20"),
    )
    .expect("rescue config");
    let now = crate::unix_timestamp();
    let mut queue = crate::TransferQueue::new_in_memory(16);
    let queued_audio = queue.create(
        0,
        Some("slow-peer".to_owned()),
        "Remote/Queued.flac".to_owned(),
        None,
        Some(100),
    );
    let sidecar = queue.create(
        0,
        Some("slow-peer".to_owned()),
        "Remote/Cover.jpg".to_owned(),
        None,
        Some(100),
    );
    let stalled = queue.create(
        0,
        Some("stalled-peer".to_owned()),
        "Remote/Stalled.mp3".to_owned(),
        None,
        Some(2_000_000),
    );
    for id in [queued_audio.id, sidecar.id] {
        queue
            .entries
            .iter_mut()
            .find(|entry| entry.id == id)
            .unwrap()
            .requested_at = now - 61;
    }
    let stalled_entry = queue
        .entries
        .iter_mut()
        .find(|entry| entry.id == stalled.id)
        .unwrap();
    stalled_entry.status = "in_progress".to_owned();
    stalled_entry.started_at = Some(now - 61);
    stalled_entry.bytes_transferred = 1_000_000;

    let searches = crate::SearchStore::new();
    let mut tracker = crate::RescueTracker::default();
    let initial = crate::create_rescue_plan(
        &queue.entries,
        &searches,
        &mut tracker,
        &config.transfer_rescue,
        now,
    );
    assert_eq!(initial.len(), 1);
    assert_eq!(initial[0].source.id, queued_audio.id);
    assert_eq!(
        initial[0].reason,
        crate::UnderperformanceReason::QueuedTooLong
    );
    assert!(initial
        .iter()
        .all(|candidate| candidate.source.id != sidecar.id));

    tracker
        .retry_after
        .insert(queued_audio.id, now.saturating_add(300));
    let stalled_plan = crate::create_rescue_plan(
        &queue.entries,
        &searches,
        &mut tracker,
        &config.transfer_rescue,
        now + 31,
    );
    assert_eq!(stalled_plan.len(), 1);
    assert_eq!(stalled_plan[0].source.id, stalled.id);
    assert_eq!(
        stalled_plan[0].reason,
        crate::UnderperformanceReason::Stalled
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn transfer_changes_applies_completed_filter_only_to_initial_snapshot() {
    let (state, _receiver) = test_state();
    {
        let mut transfers = state.transfers.write().await;
        let active = transfers.create(
            0,
            Some("active-peer".to_owned()),
            "Remote/Active.flac".to_owned(),
            None,
            Some(10),
        );
        let completed = transfers.create(
            0,
            Some("completed-peer".to_owned()),
            "Remote/Completed.flac".to_owned(),
            None,
            Some(10),
        );
        transfers
            .update_local_execution(completed.id, "succeeded", 10, Some(10), None)
            .expect("complete transfer");
        transfers
            .entries
            .iter_mut()
            .find(|entry| entry.id == active.id)
            .expect("active transfer")
            .updated_at_ms = 100;
        transfers
            .entries
            .iter_mut()
            .find(|entry| entry.id == completed.id)
            .expect("completed transfer")
            .updated_at_ms = 200;
    }

    let response = crate::route_http_request(
        "GET",
        "/api/v0/transfers/changes?since=0&includeCompleted=false",
        None,
        "",
        &state,
    )
    .await
    .expect("transfer changes");
    let body = serde_json::from_str::<serde_json::Value>(&response.body).expect("changes JSON");
    let rows = body["transfers"].as_array().expect("transfer rows");

    assert_eq!(rows.len(), 2);
    assert!(rows.iter().any(|row| row["username"] == "active-peer"));
    assert!(rows.iter().any(|row| row["username"] == "completed-peer"));

    let future_since = 9_999_999_999_999_u64;
    let response = crate::route_http_request(
        "GET",
        &format!("/api/v0/transfers/changes?since={future_since}"),
        None,
        "",
        &state,
    )
    .await
    .expect("future transfer cursor");
    let body = serde_json::from_str::<serde_json::Value>(&response.body).expect("changes JSON");
    assert!(body["cursor"].as_u64().unwrap() < future_since);
    assert_eq!(body["transfers"].as_array().unwrap().len(), 0);

    let response = crate::route_http_request(
        "GET",
        &format!("/api/v0/transfers/history?direction=download&asOf={future_since}"),
        None,
        "",
        &state,
    )
    .await
    .expect("future history snapshot");
    let body = serde_json::from_str::<serde_json::Value>(&response.body).expect("history JSON");
    assert_eq!(body["asOf"], future_since);
    assert_eq!(body["transfers"].as_array().unwrap().len(), 1);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn rescue_cycle_atomically_swaps_an_underperforming_audio_source() {
    let mut searches = crate::SearchStore::new();
    let search = searches
        .create(
            None,
            "Song.flac".to_owned(),
            "global",
            None,
            Vec::new(),
            crate::DEFAULT_SEARCH_TTL_SECONDS,
        )
        .expect("search")
        .record;
    searches
        .add_peer_response(&FileSearchResponse {
            username: "rescue-peer".to_owned(),
            token: search.token,
            results: vec![FileEntry {
                filename_encoding: Default::default(),
                extension_encoding: Default::default(),
                code: 1,
                filename: "Alternate/Album/Song.flac".to_owned(),
                size: 1_020,
                extension: "flac".to_owned(),
                attributes: Vec::new(),
            }],
            slot_free: true,
            average_speed: 100_000,
            queue_length: 0,
            unknown: 0,
            private_results: Vec::new(),
        })
        .expect("search response");
    let (state, mut receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_TRANSFER_RESCUE_MAX_QUEUE_TIME_SECONDS", "60")
            .with(
                "SLSKR_TRANSFER_RESCUE_ALTERNATE_SOURCE_SIZE_TOLERANCE_PERCENT",
                "5",
            ),
        searches,
        None,
    );
    let now = crate::unix_timestamp();
    let source = {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create_with_details(
            0,
            Some("original-peer".to_owned()),
            "Remote/Album/Song.flac".to_owned(),
            Some("downloads/Song.flac".to_owned()),
            Some(1_000),
            Some("batch-rescue".to_owned()),
            crate::TransferRequestDetails {
                request_id: Some("request-rescue".to_owned()),
                artist: Some("Artist".to_owned()),
                ..Default::default()
            },
        );
        let source = transfers
            .entries
            .iter_mut()
            .find(|candidate| candidate.id == entry.id)
            .unwrap();
        source.requested_at = now - 61;
        source.clone()
    };
    let mut tracker = crate::RescueTracker::default();

    assert_eq!(
        crate::run_transfer_rescue_cycle(&state, &mut tracker)
            .await
            .expect("rescue cycle"),
        1
    );
    let crate::SessionCommand::TransferPeer { id, username } =
        receiver.recv().await.expect("rescue dispatch")
    else {
        panic!("expected rescue transfer dispatch");
    };
    assert_eq!(username, "rescue-peer");
    let transfers = state.transfers.read().await;
    let original = transfers
        .entries
        .iter()
        .find(|entry| entry.id == source.id)
        .unwrap();
    let replacement = transfers
        .entries
        .iter()
        .find(|entry| entry.id == id)
        .unwrap();
    assert_eq!(original.status, "cancelled");
    assert_eq!(replacement.status, "peer_lookup");
    assert_eq!(replacement.peer_username.as_deref(), Some("rescue-peer"));
    assert_eq!(replacement.request_id.as_deref(), Some("request-rescue"));
    assert_eq!(replacement.batch_id.as_deref(), Some("batch-rescue"));
    assert_eq!(replacement.artist.as_deref(), Some("Artist"));
    assert_eq!(replacement.local_path, source.local_path);
    drop(transfers);
    assert!(crate::transfer_is_cancelled(&state, source.id).await);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn rescue_cycle_promotes_verified_mesh_swarm_without_overwriting_partial_file() {
    use sha2::{Digest, Sha256};

    let content = Arc::new(b"verified automatic rescue swarm".to_vec());
    let expected_hash = hex::encode(Sha256::digest(content.as_slice()));
    let (source_a, task_a) = spawn_mesh_range_source(Arc::clone(&content)).await;
    let (source_b, task_b) = spawn_mesh_range_source(Arc::clone(&content)).await;
    let (state, mut receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_TRANSFER_RESCUE_MAX_QUEUE_TIME_SECONDS", "60"),
        crate::SearchStore::new(),
        None,
    );
    {
        let mut discovery = state.content_discovery.write().await;
        discovery
            .merge_hash_entries(vec![crate::content_discovery::HashDbEntry {
                flac_key: crate::content_discovery::generate_flac_key(
                    "Remote/Album/Song.flac",
                    content.len() as u64,
                ),
                size: content.len() as u64,
                full_file_hash: expected_hash.clone(),
                music_brainz_id: "recording-rescue".to_owned(),
                ..Default::default()
            }])
            .expect("merge rescue hash");
        discovery
            .merge_shadow_records(vec![crate::content_discovery::ShadowIndexRecord {
                recording_id: "recording-rescue".to_owned(),
                peer_ids: vec!["rescue-peer-a".to_owned(), "rescue-peer-b".to_owned()],
                updated_at: 0,
            }])
            .expect("merge rescue shadow peers");
    }
    {
        let mut mesh = state.mesh.write().await;
        for (username, peer_id, address) in [
            ("mesh-a", "rescue-peer-a", source_a),
            ("mesh-b", "rescue-peer-b", source_b),
        ] {
            let mut descriptor = test_capability_descriptor(
                username,
                vec![slskr_client::capabilities::FEATURE_MESH_V1.to_owned()],
            );
            descriptor.peer_id = peer_id.to_owned();
            descriptor.endpoints = vec![format!("http://{address}/content")];
            mesh.capability_records.push(descriptor);
        }
    }

    let partial_path = state.config.downloads_dir.join("Song.partial");
    std::fs::create_dir_all(partial_path.parent().unwrap()).expect("create download root");
    std::fs::write(&partial_path, b"keep partial until verified").expect("write partial");
    let now = crate::unix_timestamp();
    let source = {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create_with_details(
            0,
            Some("slow-peer".to_owned()),
            "Remote/Album/Song.flac".to_owned(),
            Some(partial_path.display().to_string()),
            Some(content.len() as u64),
            Some("batch-mesh-rescue".to_owned()),
            crate::TransferRequestDetails {
                request_id: Some("request-mesh-rescue".to_owned()),
                artist: Some("Artist".to_owned()),
                ..Default::default()
            },
        );
        let source = transfers
            .entries
            .iter_mut()
            .find(|candidate| candidate.id == entry.id)
            .unwrap();
        source.requested_at = now - 61;
        source.clone()
    };
    let mut tracker = crate::RescueTracker::default();

    assert_eq!(
        crate::run_transfer_rescue_cycle(&state, &mut tracker)
            .await
            .expect("mesh rescue cycle"),
        1
    );
    assert!(receiver.try_recv().is_err());
    let transfers = state.transfers.read().await;
    let original = transfers
        .entries
        .iter()
        .find(|entry| entry.id == source.id)
        .expect("original transfer");
    let replacement = transfers
        .entries
        .iter()
        .find(|entry| entry.id != source.id)
        .expect("mesh replacement");
    assert_eq!(original.status, "cancelled");
    assert_eq!(replacement.status, "succeeded");
    assert_eq!(replacement.peer_username.as_deref(), Some("mesh-swarm"));
    assert_eq!(
        replacement.request_id.as_deref(),
        Some("request-mesh-rescue")
    );
    assert_eq!(replacement.batch_id.as_deref(), Some("batch-mesh-rescue"));
    assert_eq!(replacement.artist.as_deref(), Some("Artist"));
    assert_eq!(replacement.bytes_transferred, content.len() as u64);
    let replacement_path = replacement.local_path.clone().expect("rescue output path");
    assert_ne!(replacement_path, partial_path.display().to_string());
    drop(transfers);
    assert_eq!(
        std::fs::read(replacement_path).expect("read verified rescue output"),
        content.as_slice()
    );
    assert_eq!(
        std::fs::read(&partial_path).expect("read untouched partial"),
        b"keep partial until verified"
    );
    let jobs = state.multisource.read().await;
    assert_eq!(jobs.list().len(), 1);
    assert_eq!(jobs.list()[0].status, "completed");
    drop(jobs);

    task_a.abort();
    task_b.abort();
    std::fs::remove_dir_all(&state.config.state_dir).expect("remove test state directory");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn cancelled_transfer_progress_cannot_revive_or_advance_the_attempt() {
    let (state, _receiver) =
        test_state_with_env_parts(MapEnv::default(), crate::SearchStore::new(), None);
    let transfer = {
        let mut transfers = state.transfers.write().await;
        let transfer = transfers.create(
            0,
            Some("slow-peer".to_owned()),
            "Remote/Song.flac".to_owned(),
            Some("downloads/Song.flac".to_owned()),
            Some(1_000),
        );
        transfers.update_status(transfer.id, "in_progress", Some(400), None);
        transfers.update_status(transfer.id, "cancelled", Some(400), None)
    }
    .expect("cancelled transfer");

    crate::update_transfer_progress(&state, transfer.id, 900).await;

    let transfers = state.transfers.read().await;
    let current = transfers
        .entries
        .iter()
        .find(|entry| entry.id == transfer.id)
        .expect("transfer remains projected");
    assert_eq!(current.status, "cancelled");
    assert_eq!(current.bytes_transferred, 400);
    drop(transfers);

    let path = std::env::temp_dir().join(format!(
        "slskr-cancelled-chunk-{}-{}",
        std::process::id(),
        transfer.id
    ));
    let _ = std::fs::remove_file(&path);
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)
        .expect("create cancelled transfer test file");
    let error = crate::write_download_chunk_if_active(
        &state,
        transfer.id,
        &mut file,
        b"must-not-be-written",
        900,
    )
    .await
    .expect_err("cancelled download must reject a received chunk");
    assert_eq!(error, "transfer cancelled");
    assert_eq!(file.metadata().expect("test file metadata").len(), 0);
    drop(file);
    std::fs::remove_file(path).expect("remove cancelled transfer test file");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn auto_retry_rollback_preserves_concurrent_transfer_allocations() {
    let mut queue = crate::TransferQueue::new_in_memory(16);
    let previous_next_id = queue.next_id;
    let previous_next_token = queue.next_token;
    let replacement = queue.create(
        0,
        Some("retry-peer".to_owned()),
        "Remote/Retry.flac".to_owned(),
        None,
        Some(100),
    );
    let replacement_next_id = queue.next_id;
    let replacement_next_token = queue.next_token;
    let concurrent = queue.create(
        0,
        Some("other-peer".to_owned()),
        "Remote/Other.flac".to_owned(),
        None,
        Some(200),
    );
    let concurrent_next_id = queue.next_id;
    let concurrent_next_token = queue.next_token;

    crate::transfer_recovery_runtime::rollback_auto_retry_replacement(
        &mut queue,
        &replacement,
        previous_next_id,
        previous_next_token,
        replacement_next_id,
        replacement_next_token,
    );

    assert!(queue.entries.iter().all(|entry| entry.id != replacement.id));
    assert!(queue.entries.iter().any(|entry| entry.id == concurrent.id));
    assert_eq!(queue.next_id, concurrent_next_id);
    assert_eq!(queue.next_token, concurrent_next_token);
    let _ = std::fs::remove_file(queue.events_path);
    let _ = std::fs::remove_file(queue.state_path);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn transfer_queue_bounds_retained_text_and_normalizes_lookups() {
    let oversized_username = "é".repeat(crate::MAX_TRANSFER_USERNAME_BYTES);
    let oversized_filename = "f".repeat(crate::MAX_TRANSFER_FILENAME_BYTES + 1);
    let oversized_path = "p".repeat(crate::MAX_TRANSFER_LOCAL_PATH_BYTES + 1);
    let oversized_batch = "b".repeat(crate::MAX_TRANSFER_BATCH_ID_BYTES + 1);
    let oversized_reason = "r".repeat(crate::MAX_TRANSFER_REASON_BYTES + 1);
    let mut queue = crate::TransferQueue::new_in_memory(8);

    let entry = queue.create_with_batch(
        0,
        Some(oversized_username.clone()),
        oversized_filename,
        Some(oversized_path),
        Some(100),
        Some(oversized_batch),
    );
    assert!(entry.peer_username.unwrap().len() <= crate::MAX_TRANSFER_USERNAME_BYTES);
    assert_eq!(entry.filename.len(), crate::MAX_TRANSFER_FILENAME_BYTES);
    assert!(entry.local_path.is_none());
    assert_eq!(
        entry.batch_id.unwrap().len(),
        crate::MAX_TRANSFER_BATCH_ID_BYTES
    );

    let updated = queue
        .update_status(entry.id, "peer_lookup", None, Some(oversized_reason))
        .unwrap();
    assert_eq!(
        updated.reason.unwrap().len(),
        crate::MAX_TRANSFER_REASON_BYTES
    );
    assert!(queue.pending_peer_transfer(&oversized_username).is_some());

    let _ = std::fs::remove_file(queue.events_path);
    let _ = std::fs::remove_file(queue.state_path);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn transfer_queue_records_progress_events_with_bytes() {
    let mut queue = crate::TransferQueue::new_in_memory(8);
    let entry = queue.create(1, None, "Remote/Song.flac".to_owned(), None, Some(100));

    queue.update_status(entry.id, "in_progress", None, None);
    queue.update_progress(entry.id, 64);
    queue.update_local_execution(entry.id, "succeeded", 100, Some(100), None);

    let events = std::fs::read_to_string(&queue.events_path).expect("events");
    assert!(events.starts_with("slskr-transfer-events-v2\n"));
    assert!(events.contains("bytes_transferred"));
    assert!(events.contains("\t64\tin_progress\t\tRemote/Song.flac"));
    assert!(events.contains("\t100\tsucceeded\t\tRemote/Song.flac"));

    let _ = std::fs::remove_file(queue.events_path);
    let _ = std::fs::remove_file(queue.state_path);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn transfer_projection_reports_elapsed_speed_and_remaining_time() {
    let mut queue = crate::TransferQueue::new_in_memory(8);
    let entry = queue.create(
        0,
        Some("friend".to_owned()),
        "Remote/Song.flac".to_owned(),
        None,
        Some(325),
    );
    let entry = queue
        .entries
        .iter_mut()
        .find(|candidate| candidate.id == entry.id)
        .unwrap();
    entry.status = "succeeded".to_owned();
    entry.requested_at = 90;
    entry.started_at = Some(100);
    entry.start_offset = 25;
    entry.bytes_transferred = 225;
    entry.updated_at = 104;

    let projection = entry.controller_file_json();
    assert_eq!(projection["startOffset"], 25);
    assert_eq!(projection["startedAt"], "100");
    assert_eq!(projection["endedAt"], "104");
    assert_eq!(projection["averageSpeed"], 50.0);
    assert_eq!(projection["elapsedTime"], "00:00:04");
    assert_eq!(projection["remainingTime"], "");

    entry.status = "in_progress".to_owned();
    entry.bytes_transferred = 500;
    let overrun = entry.controller_file_json();
    assert_eq!(overrun["percentComplete"], 100.0);
    assert_eq!(overrun["bytesRemaining"], 0);
    assert_eq!(overrun["remainingTime"], "");

    let encoded = serde_json::to_value(&*entry).unwrap();
    let mut legacy = encoded.as_object().unwrap().clone();
    legacy.remove("started_at");
    legacy.remove("start_offset");
    let decoded = serde_json::from_value::<crate::TransferEntry>(legacy.into()).unwrap();
    assert_eq!(decoded.started_at, None);
    assert_eq!(decoded.start_offset, 0);

    let _ = std::fs::remove_file(queue.events_path);
    let _ = std::fs::remove_file(queue.state_path);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn transfer_reports_aggregate_measured_speeds() {
    let mut queue = crate::TransferQueue::new_in_memory(8);
    for (direction, username, bytes, started_at, updated_at) in [
        (0, "friend", 200, 100, 104),
        (0, "friend", 300, 100, 103),
        (1, "uploader", 120, 100, 104),
    ] {
        let created = queue.create(
            direction,
            Some(username.to_owned()),
            format!("Remote/{bytes}.flac"),
            None,
            Some(bytes),
        );
        let entry = queue
            .entries
            .iter_mut()
            .find(|entry| entry.id == created.id)
            .unwrap();
        entry.status = "succeeded".to_owned();
        entry.started_at = Some(started_at);
        entry.bytes_transferred = bytes;
        entry.updated_at = updated_at;
    }

    let summary = serde_json::from_str::<serde_json::Value>(
        &crate::controller_transfer_summary_report(Some("direction=Download"), &queue),
    )
    .unwrap();
    assert_eq!(summary["averageSpeed"], 75.0);

    let leaderboard = serde_json::from_str::<serde_json::Value>(
        &crate::controller_transfer_leaderboard_report(Some("direction=Download"), &queue)
            .expect("direction is present"),
    )
    .unwrap();
    assert_eq!(leaderboard[0]["username"], "friend");
    assert_eq!(leaderboard[0]["averageSpeed"], 75.0);

    let now = crate::unix_timestamp();
    for entry in &mut queue.entries {
        entry.status = "in_progress".to_owned();
        entry.started_at = Some(now.saturating_sub(4));
        entry.start_offset = 0;
    }
    let speeds =
        serde_json::from_str::<serde_json::Value>(&crate::controller_transfer_speeds_json(&queue))
            .unwrap();
    assert!(speeds["download"].as_f64().unwrap() > 0.0);
    assert!(speeds["upload"].as_f64().unwrap() > 0.0);
    assert_eq!(speeds["download"], speeds["downloadSpeed"]);
    assert_eq!(speeds["upload"], speeds["uploadSpeed"]);
    assert_eq!(speeds["total"], speeds["soulseek"]);
    assert_eq!(speeds["mesh"], 0.0);
    assert_eq!(speeds["sessionBytesDownloaded"], 500);
    assert_eq!(speeds["sessionBytesUploaded"], 120);
    assert_eq!(speeds["sessionBytesTotal"], 620);

    let _ = std::fs::remove_file(queue.events_path);
    let _ = std::fs::remove_file(queue.state_path);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn transfer_leaderboard_excludes_incomplete_transfers_not_hardcoded_status() {
    let mut queue = crate::TransferQueue::new_in_memory(8);
    // A failed download must never count toward the leaderboard --
    // matching the oracle's GetTransferLeaderboard, which filters
    // State = 48 (Completed | Succeeded) at the SQL layer.
    let failed = queue.create(
        0,
        Some("flaky".to_owned()),
        "Remote/Flaky.flac".to_owned(),
        None,
        Some(9999),
    );
    queue
        .entries
        .iter_mut()
        .find(|entry| entry.id == failed.id)
        .unwrap()
        .status = "failed".to_owned();
    let succeeded = queue.create(
        0,
        Some("reliable".to_owned()),
        "Remote/Reliable.flac".to_owned(),
        None,
        Some(50),
    );
    queue
        .entries
        .iter_mut()
        .find(|entry| entry.id == succeeded.id)
        .unwrap()
        .status = "succeeded".to_owned();

    let leaderboard = serde_json::from_str::<serde_json::Value>(
        &crate::controller_transfer_leaderboard_report(Some("direction=Download"), &queue)
            .expect("direction is present"),
    )
    .unwrap();
    let rows = leaderboard.as_array().unwrap();
    assert!(
        rows.iter().all(|row| row["username"] != "flaky"),
        "{leaderboard}"
    );
    assert!(
        rows.iter().any(|row| row["username"] == "reliable"),
        "{leaderboard}"
    );

    let _ = std::fs::remove_file(queue.events_path);
    let _ = std::fs::remove_file(queue.state_path);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn transfer_directories_report_is_completed_uploads_only() {
    let mut queue = crate::TransferQueue::new_in_memory(8);
    // Matches the oracle's GetTransferDirectoryFrequency exactly: it
    // always reports on Uploads (what others downloaded from local
    // shares) and only completed ones -- never downloads, and never
    // queued/failed transfers.
    let download = queue.create(
        0,
        Some("downloader".to_owned()),
        "Shared/DownloadOnly/File.flac".to_owned(),
        None,
        Some(10),
    );
    queue
        .entries
        .iter_mut()
        .find(|entry| entry.id == download.id)
        .unwrap()
        .status = "succeeded".to_owned();
    let incomplete_upload = queue.create(
        1,
        Some("stalled-peer".to_owned()),
        "Shared/StillQueued/File.flac".to_owned(),
        None,
        Some(10),
    );
    queue
        .entries
        .iter_mut()
        .find(|entry| entry.id == incomplete_upload.id)
        .unwrap()
        .status = "queued".to_owned();
    let completed_upload = queue.create(
        1,
        Some("real-uploader".to_owned()),
        "Shared/Popular/File.flac".to_owned(),
        None,
        Some(10),
    );
    queue
        .entries
        .iter_mut()
        .find(|entry| entry.id == completed_upload.id)
        .unwrap()
        .status = "succeeded".to_owned();

    let report = serde_json::from_str::<serde_json::Value>(
        &crate::controller_transfer_directories_report(None, &queue),
    )
    .unwrap();
    let paths = report
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["path"].as_str().unwrap_or_default())
        .collect::<Vec<_>>();
    assert_eq!(paths, vec!["Shared/Popular"], "{report}");

    let _ = std::fs::remove_file(queue.events_path);
    let _ = std::fs::remove_file(queue.state_path);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn transfer_queue_persists_and_reloads_resume_state() {
    let state_dir = std::env::temp_dir().join(format!(
        "slskr-transfer-state-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&state_dir).expect("state dir");
    let env = MapEnv::default()
        .with("SLSKR_STATE_DIR", &state_dir.display().to_string())
        .with("SLSKR_AUTO_CONNECT", "false");
    let config = crate::AppConfig::from_layers(None, FileConfig::default(), &env).expect("config");

    {
        let mut queue = crate::TransferQueue::new(&config);
        let entry = queue.create(
            0,
            Some("friend".to_owned()),
            "Remote/Song.flac".to_owned(),
            Some(state_dir.join("Song.flac").display().to_string()),
            Some(100),
        );
        queue.update_status(entry.id, "in_progress", Some(40), None);
    }

    let events_path = crate::transfer_events_path(&state_dir);
    let events_before_restart = std::fs::read_to_string(&events_path).expect("transfer events");
    assert!(events_before_restart.contains("\tqueued\t"));
    assert!(events_before_restart.contains("\tin_progress\t"));
    assert_eq!(events_before_restart.lines().count(), 4);

    let reloaded = crate::TransferQueue::new(&config);
    let events_after_restart = std::fs::read_to_string(&events_path).expect("reloaded events");
    assert_eq!(events_after_restart, events_before_restart);
    let entry = reloaded.entries.first().expect("reloaded transfer");
    assert_eq!(entry.status, "queued");
    assert_eq!(entry.bytes_transferred, 40);
    assert_eq!(entry.reason.as_deref(), Some("resumed after restart"));
    assert_eq!(entry.started_at, None);
    assert_eq!(entry.start_offset, 40);
    assert_eq!(reloaded.next_id, 2);
    assert_eq!(reloaded.next_token, 2);

    let _ = std::fs::remove_dir_all(state_dir);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn transfer_queue_rehydrates_from_sqlite_on_startup() {
    let state_dir = std::env::temp_dir().join(format!(
        "slskr-transfer-sqlite-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&state_dir).expect("state dir");
    let env = MapEnv::default()
        .with("SLSKR_STATE_DIR", &state_dir.display().to_string())
        .with("SLSKR_AUTO_CONNECT", "false")
        .with("SLSKR_PERSISTENCE_ENABLED", "true");
    let config = crate::AppConfig::from_layers(None, FileConfig::default(), &env).expect("config");

    let db_path = state_dir.join("slskr.db");
    let db = crate::persistence::DatabaseManager::new(db_path.to_str().unwrap_or("slskr.db"))
        .await
        .expect("database");

    let record = crate::persistence::TransferRecord {
        id: "42".to_owned(),
        direction: "download".to_owned(),
        filename: "Remote/SQLite.flac".to_owned(),
        peer_username: "sqlite-peer".to_owned(),
        filesize: 2048,
        progress: 512,
        status: "queued".to_owned(),
        started_at: 1000,
        completed_at: None,
        request_id: None,
        wishlist_item_id: None,
        request_name: None,
        destination_directory: None,
        local_path: None,
        batch_id: None,
        reason: None,
        bit_rate: None,
        sample_rate: None,
        bit_depth: None,
        length_seconds: None,
        artist: None,
        album: None,
        title: None,
        track_number: None,
        year: None,
        attempts: 1,
        auto_replace_attempts: 0,
        next_attempt_at: None,
        updated_at_ms: 0,
    };
    db.insert_transfer(&record).await.expect("insert transfer");

    let mut queue = crate::TransferQueue::new(&config);
    assert!(queue.entries.is_empty(), "JSON queue should be empty");

    queue.rehydrate_from_database(&db).await;

    assert_eq!(
        queue.entries.len(),
        1,
        "should have rehydrated one transfer"
    );
    let entry = &queue.entries[0];
    assert_eq!(entry.id, 42);
    assert_eq!(entry.direction, 0);
    assert_eq!(entry.peer_username.as_deref(), Some("sqlite-peer"));
    assert_eq!(entry.filename, "Remote/SQLite.flac");
    assert_eq!(entry.size, Some(2048));
    assert_eq!(entry.bytes_transferred, 512);
    assert_eq!(entry.status, "queued");
    assert_eq!(queue.next_id, 43);
    assert_eq!(queue.next_token, 43);

    drop(db);
    let _ = std::fs::remove_dir_all(state_dir);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn transfer_state_loader_rejects_oversized_state_file() {
    let state_dir = std::env::temp_dir().join(format!(
        "slskr-transfer-state-size-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&state_dir).expect("state dir");
    let state_path = crate::transfer_state_path(&state_dir);
    std::fs::write(
        &state_path,
        vec![b' '; (crate::MAX_TRANSFER_STATE_BYTES as usize) + 1],
    )
    .expect("oversized state");

    let error = crate::load_transfer_state(&state_path, 100).expect_err("oversized state");
    assert!(error.contains("transfer state file is too large"));

    let _ = std::fs::remove_dir_all(state_dir);
}

#[cfg(unix)]
#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn transfer_state_loader_rejects_fifo_without_blocking() {
    let state_dir = std::env::temp_dir().join(format!(
        "slskr-transfer-state-fifo-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&state_dir).expect("state dir");
    let state_path = crate::transfer_state_path(&state_dir);
    let status = std::process::Command::new("mkfifo")
        .arg(&state_path)
        .status()
        .expect("run mkfifo");
    assert!(status.success());

    let error = crate::load_transfer_state(&state_path, 100)
        .expect_err("FIFO transfer state path must be rejected");
    assert!(error.contains("must be a regular file"));

    let _ = std::fs::remove_dir_all(state_dir);
}
#[cfg(unix)]
#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn transfer_event_open_rejects_fifo_without_blocking() {
    let state_dir = std::env::temp_dir().join(format!(
        "slskr-transfer-events-fifo-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&state_dir).expect("state dir");
    let events_path = crate::transfer_events_path(&state_dir);
    let status = std::process::Command::new("mkfifo")
        .arg(&events_path)
        .status()
        .expect("run mkfifo");
    assert!(status.success());

    let error = crate::open_transfer_event_file(&events_path)
        .expect_err("FIFO transfer event path must be rejected");
    assert!(error.contains("transfer event open failed"));

    let _ = std::fs::remove_dir_all(state_dir);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn transfer_event_append_rotates_oversized_event_file() {
    let state_dir = std::env::temp_dir().join(format!(
        "slskr-transfer-events-size-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&state_dir).expect("state dir");
    let events_path = crate::transfer_events_path(&state_dir);
    std::fs::write(
        &events_path,
        vec![b' '; (crate::MAX_TRANSFER_EVENTS_BYTES as usize) + 1],
    )
    .expect("oversized transfer events");

    let entry = crate::TransferEntry {
        id: 7,
        direction: 0,
        token: 9,
        peer_username: Some("friend".to_owned()),
        filename: "Remote/Song.flac".to_owned(),
        local_path: None,
        batch_id: None,
        request_id: None,
        wishlist_item_id: None,
        request_name: None,
        destination_directory: None,
        bit_rate: None,
        sample_rate: None,
        bit_depth: None,
        length_seconds: None,
        artist: None,
        album: None,
        title: None,
        track_number: None,
        year: None,
        attempts: 1,
        auto_replace_attempts: 0,
        next_attempt_at: None,
        size: Some(100),
        bytes_transferred: 25,
        status: "in_progress".to_owned(),
        reason: None,
        requested_at: 11,
        started_at: Some(11),
        start_offset: 0,
        updated_at: 12,
        updated_at_ms: 12_000,
        previous_status: None,
    };
    crate::append_transfer_event(&events_path, &entry).expect("append rotated event");

    let body = std::fs::read_to_string(&events_path).expect("events body");
    assert!(body.starts_with("slskr-transfer-events-v2\n"));
    assert!(body.contains("7\t0\t9\t100\t25\tin_progress\t\tRemote/Song.flac\n"));
    assert!(events_path.with_extension("tsv.old").exists());
    assert!(
        std::fs::metadata(&events_path)
            .expect("events metadata")
            .len()
            < 1024
    );

    let _ = std::fs::remove_dir_all(state_dir);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn queued_backfill_transfer_reads_only_the_bounded_header() {
    let (state, _receiver) = test_state();
    let token = 77_u32;
    let mut header = vec![0_u8; 42];
    header[..4].copy_from_slice(b"fLaC");
    header[7] = 34;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let client = tokio::net::TcpStream::connect(address);
    let server = listener.accept();
    let (client, server) = tokio::join!(client, server);
    let (server, _) = server.unwrap();
    let (response, result) = tokio::sync::oneshot::channel();
    state.pending_backfill_transfers.write().await.insert(
        token,
        crate::PendingBackfillTransfer {
            expected_size: header.len() as u64,
            response,
        },
    );
    let task_state = Arc::clone(&state);
    let handler = tokio::spawn(async move {
        crate::handle_inbound_file_transfer(
            &task_state,
            slskr_client::file_transfer::FileTransferConnection::new(server),
            Some(token),
        )
        .await
    });
    let mut client = slskr_client::file_transfer::FileTransferConnection::new(client.unwrap());
    client.send_token(token).await.unwrap();
    assert_eq!(client.receive_offset().await.unwrap(), 0);
    client.write_chunk(&header).await.unwrap();
    assert_eq!(result.await.unwrap().unwrap(), header);
    handler.await.unwrap().unwrap();
}
