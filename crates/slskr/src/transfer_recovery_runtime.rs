// Transfer recovery and retry execution, kept beside its durable state model.
use super::*;

pub(super) async fn discover_mesh_range_sources(
    state: &AppState,
    expected_hash: &str,
    file_size: u64,
) -> Vec<multisource::RangeSource> {
    let (recording_ids, peer_ids) = {
        let discovery = state.content_discovery.read().await;
        let recording_ids = discovery.recording_ids_for_hash(expected_hash, file_size);
        let peer_ids = discovery.peer_ids_for_recordings(&recording_ids);
        (recording_ids, peer_ids)
    };
    if peer_ids.is_empty() {
        return Vec::new();
    }
    let peer_ids = peer_ids
        .into_iter()
        .map(|peer_id| peer_id.to_ascii_lowercase())
        .collect::<HashSet<_>>();
    let mut usernames = HashSet::new();
    let mut sources = state
        .config
        .trusted_mesh_peers
        .iter()
        .filter(|peer| peer_ids.contains(&peer.peer_id.to_ascii_lowercase()))
        .filter_map(|peer| {
            let endpoint = peer.range_url(
                expected_hash,
                file_size,
                recording_ids.first().map(String::as_str),
            )?;
            usernames
                .insert(peer.username.to_ascii_lowercase())
                .then_some(multisource::RangeSource {
                    username: peer.username.clone(),
                    url: endpoint,
                    authorization: None,
                })
        })
        .collect::<Vec<_>>();
    let mesh = state.mesh.read().await;
    sources.extend(
        mesh.capability_records
            .iter()
            .filter(|descriptor| {
                peer_ids.contains(&descriptor.peer_id.to_ascii_lowercase())
                    && MeshRendezvous::accepts_descriptor(descriptor)
            })
            .filter_map(|descriptor| {
                let endpoint = descriptor.endpoints.iter().find(|endpoint| {
                    endpoint.starts_with("https://") || endpoint.starts_with("http://")
                })?;
                usernames
                    .insert(descriptor.username.to_ascii_lowercase())
                    .then_some(multisource::RangeSource {
                        username: descriptor.username.clone(),
                        url: endpoint.clone(),
                        authorization: None,
                    })
            }),
    );
    sources.truncate(multisource::MAX_SOURCES);
    sources
}

pub(super) fn spawn_download_auto_retry(state: Arc<AppState>) {
    let task_state = Arc::clone(&state);
    state.spawn_managed_task(async move {
        let state = task_state;
        let mut tracker = AutoRetryTracker::default();
        loop {
            let settings = state.transfer_auto_retry_settings.read().await.clone();
            if settings.enabled && state.session.read().await.state == "connected" {
                if let Err(error) =
                    run_download_auto_retry_cycle_with_settings(&state, &mut tracker, &settings)
                        .await
                {
                    record_daemon_log(
                        &state,
                        logging::LogLevel::Warn,
                        "transfers",
                        format!("download auto-retry cycle failed: {error}"),
                    )
                    .await;
                }
            }
            time::sleep(settings.check_interval).await;
        }
    });
}

pub(super) fn auto_replace_retry_settings(
    base: &crate::config::TransferAutoRetrySettings,
    download: &crate::config::TransferDownloadSettings,
) -> crate::config::TransferAutoRetrySettings {
    let mut settings = base.clone();
    settings.enabled = download.auto_replace_stuck;
    settings.retry_delay = Duration::ZERO;
    settings.check_interval = download.auto_replace_interval;
    if let Some(max_retries) = download.auto_replace_max_retries {
        settings.max_attempts = max_retries;
    }
    settings.alternate_sources_enabled = true;
    settings.alternate_source_size_tolerance_percent = download.auto_replace_threshold_percent;
    settings
}

pub(super) fn spawn_download_auto_replace(state: Arc<AppState>) {
    if state.config.controller_profile != ControllerProfile::Native {
        return;
    }
    let task_state = Arc::clone(&state);
    state.spawn_managed_task(async move {
        let state = task_state;
        let mut tracker = AutoRetryTracker::default();
        loop {
            let download = state.transfer_download_settings.read().await.clone();
            let base = state.transfer_auto_retry_settings.read().await.clone();
            let settings = auto_replace_retry_settings(&base, &download);
            if settings.enabled && state.session.read().await.state == "connected" {
                if let Err(error) =
                    run_download_auto_retry_cycle_with_settings(&state, &mut tracker, &settings)
                        .await
                {
                    record_daemon_log(
                        &state,
                        logging::LogLevel::Warn,
                        "transfers",
                        format!("download auto-replace cycle failed: {error}"),
                    )
                    .await;
                }
            }
            time::sleep(settings.check_interval).await;
        }
    });
}

pub(super) fn spawn_transfer_rescue(state: Arc<AppState>) {
    if !state.config.transfer_rescue.enabled {
        return;
    }
    let task_state = Arc::clone(&state);
    state.spawn_managed_task(async move {
        let state = task_state;
        let mut tracker = RescueTracker::default();
        let mut interval = time::interval(state.config.transfer_rescue.check_interval);
        interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            if state.session.read().await.state != "connected" {
                continue;
            }
            if let Err(error) = run_transfer_rescue_cycle(&state, &mut tracker).await {
                record_daemon_log(
                    &state,
                    logging::LogLevel::Warn,
                    "transfers",
                    format!("transfer rescue cycle failed: {error}"),
                )
                .await;
            }
        }
    });
}

pub(super) async fn create_rescue_search(
    state: &AppState,
    query: String,
) -> Result<SearchRecord, String> {
    let (previous_searches, mutated_searches, record, evicted, expired) = {
        let mut searches = state.searches.write().await;
        let previous_searches = searches.clone();
        let outcome = searches
            .create(
                None,
                query,
                "global",
                None,
                Vec::new(),
                DEFAULT_SEARCH_TTL_SECONDS,
            )
            .map_err(|error| format!("rescue alternative search failed: {error:?}"))?;
        let mutated_searches = searches.clone();
        (
            previous_searches,
            mutated_searches,
            outcome.record,
            outcome.evicted,
            outcome.expired,
        )
    };
    let mut upserts = expired.clone();
    upserts.push(record.clone());
    if let Err(error) = persist_search_transition(state, &upserts, &evicted).await {
        rollback_searches_if_unchanged(state, previous_searches, &mutated_searches).await;
        return Err(error);
    }
    for expired_record in &expired {
        publish_search_hub_event(state, "update", expired_record);
    }
    publish_search_hub_event(state, "create", &record);
    Ok(record)
}

pub(super) async fn run_transfer_rescue_cycle(
    state: &AppState,
    tracker: &mut RescueTracker,
) -> Result<usize, String> {
    if !state.config.transfer_allow_outbound {
        return Ok(0);
    }
    let now = unix_timestamp();
    let entries = state.transfers.read().await.entries.clone();
    let plan = {
        let searches = state.searches.read().await;
        create_rescue_plan(
            &entries,
            &searches,
            tracker,
            &state.config.transfer_rescue,
            now,
        )
    };
    let mut activated = 0usize;
    for candidate in plan {
        if let Some(replacement) = run_rescue_mesh_swarm(state, &candidate).await? {
            tracker.retry_after.insert(
                candidate.source.id,
                now.saturating_add(state.config.transfer_rescue.retry_cooldown.as_secs()),
            );
            activated += 1;
            record_event(
                state,
                "transfer.rescue.activated",
                candidate.source.id.to_string(),
                Some(format!(
                    "replacement={};reason={};source=mesh-swarm",
                    replacement.id,
                    candidate.reason.as_str()
                )),
            )
            .await;
            continue;
        }
        let permit = state
            .session_commands
            .reserve()
            .await
            .map_err(|_| "session manager is not running".to_owned())?;
        let alternative = match candidate.action {
            RescueAction::Search => {
                let query = virtual_basename(&candidate.source.filename).to_owned();
                let record = create_rescue_search(state, query.clone()).await?;
                permit.send(SessionCommand::Search {
                    token: record.token,
                    query,
                    target: SearchDispatchTarget::Global,
                });
                tracker.search_requested_at.insert(candidate.source.id, now);
                tracker.retry_after.insert(
                    candidate.source.id,
                    now.saturating_add(state.config.transfer_rescue.check_interval.as_secs()),
                );
                record_daemon_log(
                    state,
                    logging::LogLevel::Info,
                    "transfers",
                    format!(
                        "searching for rescue sources for underperforming transfer {} ({})",
                        candidate.source.id,
                        candidate.reason.as_str()
                    ),
                )
                .await;
                continue;
            }
            RescueAction::Replace(alternative) => alternative,
        };
        let Some(username) = alternative.peer_username.clone() else {
            continue;
        };
        let (replacement, persisted, previous, mutated) = {
            let mut transfers = state.transfers.write().await;
            let Some(current) = transfers
                .entries
                .iter()
                .find(|entry| entry.id == candidate.source.id)
                .cloned()
            else {
                continue;
            };
            if !matches!(current.status.as_str(), "queued" | "in_progress")
                || current.bytes_transferred != candidate.source.bytes_transferred
            {
                continue;
            }
            let previous = transfers.mutation_snapshot();
            let original = transfers
                .update_status(
                    current.id,
                    "cancelled",
                    Some(current.bytes_transferred),
                    Some(format!(
                        "rescued underperforming transfer: {}",
                        candidate.reason.as_str()
                    )),
                )
                .expect("current rescue transfer exists");
            let replacement = transfers.create_with_details(
                0,
                Some(username.clone()),
                alternative.filename.clone(),
                current.local_path.clone(),
                Some(alternative.size),
                current.batch_id.clone(),
                retry_request_details(&current),
            );
            let replacement = transfers
                .update_status(replacement.id, "peer_lookup", None, None)
                .unwrap_or(replacement);
            let persisted = [original, replacement.clone()];
            let mutated = transfers.mutation_snapshot();
            (replacement, persisted, previous, mutated)
        };
        if let Err(error) = persist_transfer_records(state, &persisted).await {
            rollback_transfer_mutation_if_unchanged(state, previous, mutated).await;
            return Err(error);
        };
        permit.send(SessionCommand::TransferPeer {
            id: replacement.id,
            username: username.clone(),
        });
        tracker.retry_after.insert(
            candidate.source.id,
            now.saturating_add(state.config.transfer_rescue.retry_cooldown.as_secs()),
        );
        activated += 1;
        record_event(
            state,
            "transfer.rescue.activated",
            candidate.source.id.to_string(),
            Some(format!(
                "replacement={};reason={};source={}",
                replacement.id,
                candidate.reason.as_str(),
                username
            )),
        )
        .await;
    }
    Ok(activated)
}

async fn run_rescue_mesh_swarm(
    state: &AppState,
    candidate: &RescueCandidate,
) -> Result<Option<TransferEntry>, String> {
    let Some(file_size) = candidate.source.size.filter(|size| *size > 0) else {
        return Ok(None);
    };
    let expected_hash = {
        state
            .content_discovery
            .read()
            .await
            .verified_file_hash(&candidate.source.filename, file_size)
    };
    let Some(expected_hash) = expected_hash else {
        return Ok(None);
    };
    let sources = discover_mesh_range_sources(state, &expected_hash, file_size).await;
    if sources.len() < 2 {
        return Ok(None);
    }

    let job_id = uuid::Uuid::new_v4().to_string();
    let relative_path = format!(
        "rescue/{}-{}-{}",
        candidate.source.id,
        job_id,
        virtual_basename(&candidate.source.filename)
    );
    let downloads_dir = effective_downloads_dir(state);
    let output_path = safe_download_path(&downloads_dir, &relative_path).and_then(|path| {
        ensure_scoped_download_path(&downloads_dir, path.to_string_lossy().as_ref())
    })?;
    let public_output_path = output_path
        .strip_prefix(&downloads_dir)
        .map(|path| path.to_string_lossy().replace('\\', "/"))
        .map_err(|_| "rescue swarm output path escaped the download root".to_owned())?;
    let mut request = multisource::SwarmRequest {
        filename: candidate.source.filename.clone(),
        file_size,
        expected_hash: Some(expected_hash),
        output_path: Some(public_output_path.clone()),
        chunk_size: multisource::DEFAULT_CHUNK_SIZE,
        sources,
    };
    if multisource::validate_request(&mut request).is_err() {
        return Ok(None);
    }
    let job = multisource::new_job(
        job_id.clone(),
        &request,
        public_output_path.clone(),
        unix_timestamp(),
    );
    state.multisource.write().await.insert(job);
    let result = multisource::execute(
        job_id.clone(),
        request,
        output_path.clone(),
        public_output_path,
        Arc::clone(&state.multisource),
    )
    .await;
    if !result.success {
        record_daemon_log(
            state,
            logging::LogLevel::Info,
            "transfers",
            format!(
                "verified mesh rescue for transfer {} did not complete; retaining the Soulseek attempt",
                candidate.source.id
            ),
        )
        .await;
        return Ok(None);
    }

    let (replacement, persisted, previous, mutated) = {
        let mut transfers = state.transfers.write().await;
        let Some(current) = transfers
            .entries
            .iter()
            .find(|entry| entry.id == candidate.source.id)
            .cloned()
        else {
            drop(transfers);
            discard_rescue_swarm_output(
                state,
                &job_id,
                &output_path,
                "verified rescue output was not promoted because the original transfer changed",
            )
            .await;
            return Ok(None);
        };
        if !matches!(current.status.as_str(), "queued" | "in_progress") {
            drop(transfers);
            discard_rescue_swarm_output(
                state,
                &job_id,
                &output_path,
                "verified rescue output was not promoted because the original transfer changed",
            )
            .await;
            return Ok(None);
        }
        let previous = transfers.mutation_snapshot();
        let original = transfers
            .update_status(
                current.id,
                "cancelled",
                Some(current.bytes_transferred),
                Some(format!(
                    "rescued by verified mesh swarm: {}",
                    candidate.reason.as_str()
                )),
            )
            .expect("current mesh rescue transfer exists");
        let replacement = transfers.create_with_details(
            0,
            Some("mesh-swarm".to_owned()),
            current.filename.clone(),
            Some(output_path.display().to_string()),
            Some(file_size),
            current.batch_id.clone(),
            retry_request_details(&current),
        );
        let replacement = transfers
            .update_local_execution(
                replacement.id,
                "succeeded",
                file_size,
                Some(file_size),
                None,
            )
            .unwrap_or(replacement);
        let mutated = transfers.mutation_snapshot();
        (
            replacement.clone(),
            [original, replacement],
            previous,
            mutated,
        )
    };
    if let Err(error) = persist_transfer_records(state, &persisted).await {
        rollback_transfer_mutation_if_unchanged(state, previous, mutated).await;
        discard_rescue_swarm_output(
            state,
            &job_id,
            &output_path,
            "verified rescue output was not promoted because transfer metadata could not be committed",
        )
        .await;
        return Err(error);
    };
    record_daemon_log(
        state,
        logging::LogLevel::Info,
        "transfers",
        format!(
            "promoted underperforming transfer {} to verified mesh swarm job {} as attempt {}",
            candidate.source.id, job_id, replacement.id
        ),
    )
    .await;
    Ok(Some(replacement))
}

async fn discard_rescue_swarm_output(
    state: &AppState,
    job_id: &str,
    output_path: &Path,
    reason: &str,
) {
    let _ = fs::remove_file(output_path);
    state
        .multisource
        .write()
        .await
        .invalidate_completed(job_id, reason, unix_timestamp());
}

fn retry_request_details(source: &TransferEntry) -> TransferRequestDetails {
    TransferRequestDetails {
        request_id: source.request_id.clone(),
        wishlist_item_id: source.wishlist_item_id.clone(),
        request_name: source.request_name.clone(),
        destination_directory: source.destination_directory.clone(),
        bit_rate: source.bit_rate,
        sample_rate: source.sample_rate,
        bit_depth: source.bit_depth,
        length_seconds: source.length_seconds,
        artist: source.artist.clone(),
        album: source.album.clone(),
        title: source.title.clone(),
        track_number: source.track_number,
        year: source.year,
        attempts: source.attempts.saturating_add(1),
        auto_replace_attempts: source.auto_replace_attempts,
        next_attempt_at: None,
    }
}

#[cfg(any(test, feature = "bounded-differential"))]
#[allow(dead_code)]
pub(super) async fn run_download_auto_retry_cycle(
    state: &AppState,
    tracker: &mut AutoRetryTracker,
) -> Result<usize, String> {
    let settings = state.transfer_auto_retry_settings.read().await.clone();
    run_download_auto_retry_cycle_with_settings(state, tracker, &settings).await
}

pub(super) fn rollback_auto_retry_replacement(
    transfers: &mut TransferQueue,
    replacement: &TransferEntry,
    previous_next_id: u64,
    previous_next_token: u32,
    replacement_next_id: u64,
    replacement_next_token: u32,
) {
    let unchanged = transfers
        .entries
        .iter()
        .any(|entry| entry.id == replacement.id && entry == replacement);
    if !unchanged {
        return;
    }
    transfers.entries.retain(|entry| entry.id != replacement.id);
    transfers.progress_persisted_at.remove(&replacement.id);
    if transfers.next_id == replacement_next_id {
        transfers.next_id = previous_next_id;
    }
    if transfers.next_token == replacement_next_token {
        transfers.next_token = previous_next_token;
    }
    transfers.persist_state();
}

pub(super) async fn run_download_auto_retry_cycle_with_settings(
    state: &AppState,
    tracker: &mut AutoRetryTracker,
    settings: &crate::config::TransferAutoRetrySettings,
) -> Result<usize, String> {
    if !state.config.transfer_allow_outbound {
        return Ok(0);
    }
    let now = unix_timestamp();
    let download_slots = state.transfer_download_settings.read().await.slots;
    let (entries, available_slots) = {
        let transfers = state.transfers.read().await;
        let active_downloads = transfers
            .entries
            .iter()
            .filter(|entry| entry.direction == 0 && is_active_transfer_status(&entry.status))
            .count();
        (
            transfers.entries.clone(),
            state
                .config
                .transfer_max_active
                .saturating_sub(transfers.active_count_excluding(None))
                .min(
                    usize::try_from(download_slots)
                        .unwrap_or(usize::MAX)
                        .saturating_sub(active_downloads),
                ),
        )
    };
    prune_auto_retry_tracker(tracker, &entries, now);
    let plan = {
        let searches = state.searches.read().await;
        create_auto_retry_plan(&entries, &searches, tracker, settings, now, available_slots)
    };
    let mut queued = 0usize;
    for candidate in plan {
        let permit = state
            .session_commands
            .reserve()
            .await
            .map_err(|_| "session manager is not running".to_owned())?;
        if candidate.source_kind == "network-search" {
            let query = virtual_basename(&candidate.source.filename).to_owned();
            let (previous_searches, mutated_searches, record, evicted, expired) = {
                let mut searches = state.searches.write().await;
                let previous_searches = searches.clone();
                let outcome = searches
                    .create(
                        None,
                        query.clone(),
                        "global",
                        None,
                        Vec::new(),
                        DEFAULT_SEARCH_TTL_SECONDS,
                    )
                    .map_err(|error| format!("auto-retry alternative search failed: {error:?}"))?;
                let record = outcome.record;
                let evicted = outcome.evicted;
                let expired = outcome.expired;
                let mutated_searches = searches.clone();
                (
                    previous_searches,
                    mutated_searches,
                    record,
                    evicted,
                    expired,
                )
            };
            let mut upserts = expired.clone();
            upserts.push(record.clone());
            if let Err(error) = persist_search_transition(state, &upserts, &evicted).await {
                rollback_searches_if_unchanged(state, previous_searches, &mutated_searches).await;
                return Err(error);
            }
            for expired_record in &expired {
                publish_search_hub_event(state, "update", expired_record);
            }
            publish_search_hub_event(state, "create", &record);
            permit.send(SessionCommand::Search {
                token: record.token,
                query,
                target: SearchDispatchTarget::Global,
            });
            tracker
                .alternate_search_requested_at
                .insert(candidate.source.id, now);
            record_daemon_log(
                state,
                logging::LogLevel::Info,
                "transfers",
                format!(
                    "searching for alternate sources before auto-retrying transfer {}",
                    candidate.source.id
                ),
            )
            .await;
            continue;
        }
        let (
            replacement,
            previous_next_id,
            previous_next_token,
            replacement_next_id,
            replacement_next_token,
        ) = {
            let mut transfers = state.transfers.write().await;
            if transfers.active_count_excluding(None) >= state.config.transfer_max_active
                || transfers
                    .entries
                    .iter()
                    .filter(|entry| {
                        entry.direction == 0 && is_active_transfer_status(&entry.status)
                    })
                    .count()
                    >= usize::try_from(download_slots).unwrap_or(usize::MAX)
                || !transfers.entries.iter().any(|entry| {
                    entry.id == candidate.source.id && is_failed_transfer_status(&entry.status)
                })
            {
                continue;
            }
            let previous_next_id = transfers.next_id;
            let previous_next_token = transfers.next_token;
            let replacement = transfers.create_with_details(
                0,
                Some(candidate.username.clone()),
                candidate.filename.clone(),
                candidate.source.local_path.clone(),
                candidate.size,
                candidate.source.batch_id.clone(),
                retry_request_details(&candidate.source),
            );
            let replacement = transfers
                .update_status(replacement.id, "peer_lookup", None, None)
                .unwrap_or(replacement);
            let replacement_next_id = transfers.next_id;
            let replacement_next_token = transfers.next_token;
            (
                replacement,
                previous_next_id,
                previous_next_token,
                replacement_next_id,
                replacement_next_token,
            )
        };
        if let Err(error) = persist_transfer_record(state, &replacement).await {
            {
                let mut transfers = state.transfers.write().await;
                rollback_auto_retry_replacement(
                    &mut transfers,
                    &replacement,
                    previous_next_id,
                    previous_next_token,
                    replacement_next_id,
                    replacement_next_token,
                );
            }
            persist_transfer_durability(state).await;
            return Err(error);
        }
        permit.send(SessionCommand::TransferPeer {
            id: replacement.id,
            username: candidate.username.clone(),
        });
        tracker.retried_ids.insert(candidate.source.id);
        *tracker
            .retry_counts
            .entry(auto_retry_key(&candidate.username, &candidate.filename))
            .or_default() += 1;
        tracker.peer_retry_after.insert(
            candidate.username.to_ascii_lowercase(),
            now.saturating_add(settings.peer_cooldown.as_secs()),
        );
        queued += 1;
        record_daemon_log(
            state,
            logging::LogLevel::Info,
            "transfers",
            format!(
                "auto-retried transfer {} as {} from {} via {}",
                candidate.source.id, replacement.id, candidate.username, candidate.source_kind
            ),
        )
        .await;
    }
    Ok(queued)
}
