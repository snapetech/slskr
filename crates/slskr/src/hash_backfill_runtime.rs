use super::*;

pub(super) async fn backfill_candidates(state: &AppState, limit: usize) -> Vec<BackfillCandidate> {
    let limit = limit.clamp(1, BACKFILL_MAX_CANDIDATES);
    let now = unix_timestamp();
    let searches = state.searches.read().await;
    let mut raw = searches
        .records
        .iter()
        .flat_map(|search| {
            search.results.iter().filter_map(move |result| {
                let peer_id = result.peer_username.as_deref()?.trim();
                (result.size > 0
                    && !result.locked
                    && result.extension.eq_ignore_ascii_case("flac")
                    && !peer_id.is_empty())
                .then(|| {
                    (
                        peer_id.to_owned(),
                        result.filename.clone(),
                        result.size,
                        search.created_at,
                    )
                })
            })
        })
        .collect::<Vec<_>>();
    drop(searches);
    raw.sort_by_key(|(_, _, _, discovered_at)| *discovered_at);
    let discovery = state.content_discovery.read().await;
    raw.retain(|(_, path, size, _)| {
        discovery
            .lookup_hash(&content_discovery::generate_flac_key(path, *size))
            .is_none()
    });
    drop(discovery);
    let users = state.users.read().await;
    let mesh = state.mesh.read().await;
    let endpoints = state.peer_endpoints.read().await;
    let mut backfill = state.backfill.write().await;
    let mut seen = HashSet::new();
    raw.into_iter()
        .filter(|(peer_id, path, size, _)| {
            seen.insert((
                peer_id.to_ascii_lowercase(),
                path.to_ascii_lowercase(),
                *size,
            ))
        })
        .take(limit)
        .map(|(peer_id, path, size, discovered_at)| {
            let is_peer_online = users.records.iter().any(|user| {
                user.username.eq_ignore_ascii_case(&peer_id)
                    && !matches!(
                        user.status.as_deref(),
                        None | Some("offline") | Some("Offline")
                    )
            }) || endpoints.contains_key(&peer_id)
                || state
                    .config
                    .test_user_endpoint_overrides
                    .contains_key(&peer_id);
            let is_peer_native = mesh.capability_records.iter().any(|record| {
                record.username.eq_ignore_ascii_case(&peer_id)
                    && record.features.iter().any(|feature| {
                        feature.eq_ignore_ascii_case(FEATURE_CAPABILITIES_V1)
                            || feature.eq_ignore_ascii_case("mesh_sync")
                    })
            });
            let peer_backfills_today = backfill.peer_count_today(&peer_id, now);
            BackfillCandidate {
                peer_id,
                path,
                size,
                discovered_at,
                peer_backfills_today,
                is_peer_online,
                is_peer_native,
            }
        })
        .collect()
}

pub(super) fn parse_flac_backfill_hash(header: &[u8]) -> Result<String, String> {
    use sha2::{Digest, Sha256};

    if header.len() < 42 || header.get(..4) != Some(b"fLaC") {
        return Err("failed to parse FLAC header".to_owned());
    }
    let block = header
        .get(4..8)
        .ok_or_else(|| "failed to parse FLAC header".to_owned())?;
    if block[0] & 0x7f != 0 {
        return Err("failed to parse FLAC header".to_owned());
    }
    let block_length =
        usize::from(block[1]) << 16 | usize::from(block[2]) << 8 | usize::from(block[3]);
    if block_length < 34 || 8_usize.saturating_add(block_length) > header.len() {
        return Err("failed to parse FLAC header".to_owned());
    }
    Ok(hex::encode(Sha256::digest(
        &header[..header.len().min(BACKFILL_HASH_BYTES)],
    )))
}

async fn read_remote_flac_header(
    state: &AppState,
    peer_id: &str,
    path: &str,
    expected_size: u64,
) -> Result<Vec<u8>, String> {
    if !state.config.transfer_allow_outbound {
        return Err("outbound transfers are disabled".to_owned());
    }
    let address = request_peer_endpoint(state, peer_id).await?;
    let transfer_token = state.transfers.write().await.allocate_token();
    let now = unix_timestamp();
    let transfer = TransferEntry {
        id: 0,
        direction: 0,
        token: transfer_token,
        peer_username: Some(peer_id.to_owned()),
        filename: path.to_owned(),
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
        size: Some(expected_size),
        bytes_transferred: 0,
        status: "peer_negotiating".to_owned(),
        reason: None,
        requested_at: now,
        started_at: None,
        start_offset: 0,
        updated_at: now,
        updated_at_ms: unix_timestamp_millis(),
        previous_status: None,
    };
    let (remote_size, queued_token) =
        match negotiate_peer_transfer(state, &address, &transfer).await? {
            PeerTransferNegotiation::Allowed { token, size } if token == transfer_token => {
                (size.unwrap_or(expected_size), None)
            }
            PeerTransferNegotiation::Allowed { .. } => {
                return Err("backfill negotiation token did not match".to_owned());
            }
            PeerTransferNegotiation::Rejected { reason, .. } => return Err(reason),
            PeerTransferNegotiation::QueuedInbound { token, size } => {
                (size.unwrap_or(expected_size), Some(token))
            }
        };
    if remote_size != expected_size {
        return Err("backfill source size did not match the candidate".to_owned());
    }
    if let Some(queued_token) = queued_token {
        let (sender, receiver) = oneshot::channel();
        {
            let mut pending = state.pending_backfill_transfers.write().await;
            if pending.len() >= 2 || pending.contains_key(&queued_token) {
                return Err("backfill queued-transfer capacity is full".to_owned());
            }
            pending.insert(
                queued_token,
                PendingBackfillTransfer {
                    expected_size,
                    response: sender,
                },
            );
        }
        let result =
            time::timeout(state.config.soulseek_connection.timeout_transfer, receiver).await;
        state
            .pending_backfill_transfers
            .write()
            .await
            .remove(&queued_token);
        return result
            .map_err(|_| "backfill queued transfer timed out".to_owned())?
            .map_err(|_| "backfill queued transfer was cancelled".to_owned())?;
    }
    let mut connection = connect_file_transfer_preferred(state, &address).await?;
    receive_backfill_header(state, &mut connection, transfer_token, remote_size).await
}

pub(super) async fn receive_backfill_header(
    state: &AppState,
    connection: &mut slskr_client::file_transfer::FileTransferConnection<TcpStream>,
    transfer_token: u32,
    remote_size: u64,
) -> Result<Vec<u8>, String> {
    let received_token = time::timeout(
        state.config.soulseek_connection.timeout_transfer,
        connection.receive_token(),
    )
    .await
    .map_err(|_| "backfill token receive timed out".to_owned())?
    .map_err(|error| format!("backfill token receive failed: {error}"))?;
    if received_token != transfer_token {
        return Err("backfill transfer token did not match".to_owned());
    }
    time::timeout(
        state.config.soulseek_connection.timeout_transfer,
        connection.send_offset(0),
    )
    .await
    .map_err(|_| "backfill offset send timed out".to_owned())?
    .map_err(|error| format!("backfill offset send failed: {error}"))?;
    let wanted = usize::try_from(remote_size.min(BACKFILL_MAX_HEADER_BYTES as u64))
        .map_err(|_| "backfill header size is invalid".to_owned())?;
    time::timeout(
        state.config.soulseek_connection.timeout_transfer,
        connection.read_chunk(wanted),
    )
    .await
    .map_err(|_| "backfill header receive timed out".to_owned())?
    .map_err(|error| format!("backfill header receive failed: {error}"))
}

pub(super) async fn backfill_file(
    state: &AppState,
    peer_id: &str,
    path: &str,
    size: u64,
) -> serde_json::Value {
    let started = Instant::now();
    let permit = match Arc::clone(&state.backfill_connections).try_acquire_owned() {
        Ok(permit) => permit,
        Err(_) => {
            return serde_json::json!({
                "success": false,
                "peerId": peer_id,
                "path": path,
                "hash": "",
                "flacKey": "",
                "error": "backfill concurrency limit reached",
                "durationMs": 0,
                "bytesRead": 0,
            });
        }
    };
    {
        let mut backfill = state.backfill.write().await;
        backfill.active = backfill.active.saturating_add(1);
        backfill.total_attempts = backfill.total_attempts.saturating_add(1);
    }
    let result = read_remote_flac_header(state, peer_id, path, size).await;
    let (mut success, hash, mut error, bytes_read) = match result {
        Ok(header) => match parse_flac_backfill_hash(&header) {
            Ok(hash) => {
                let flac_key = content_discovery::generate_flac_key(path, size);
                let persistence_turn = hash_db_persistence_turn().await;
                let (
                    stored,
                    previous_entries,
                    previous_latest_seq,
                    mutated_entries,
                    mutated_latest_seq,
                ) = {
                    let mut discovery = state.content_discovery.write().await;
                    let previous_entries = discovery.hash_entries().to_vec();
                    let previous_latest_seq = discovery.latest_seq();
                    let stored =
                        discovery.merge_hash_entries(vec![content_discovery::HashDbEntry {
                            flac_key,
                            byte_hash: hash.clone(),
                            size,
                            ..content_discovery::HashDbEntry::default()
                        }]);
                    let mutated_entries = discovery.hash_entries().to_vec();
                    let mutated_latest_seq = discovery.latest_seq();
                    (
                        stored,
                        previous_entries,
                        previous_latest_seq,
                        mutated_entries,
                        mutated_latest_seq,
                    )
                };
                match stored {
                    Ok(_) => match persist_current_hash_db_snapshot(state, &persistence_turn).await
                    {
                        Ok(()) => (true, hash, String::new(), header.len()),
                        Err(error) => {
                            rollback_hash_db_entries_if_unchanged(
                                state,
                                previous_entries,
                                previous_latest_seq,
                                &mutated_entries,
                                mutated_latest_seq,
                            )
                            .await;
                            (false, String::new(), error, header.len())
                        }
                    },
                    Err(error) => (false, String::new(), error, header.len()),
                }
            }
            Err(error) => (false, String::new(), error, header.len()),
        },
        Err(error) => (false, String::new(), error, 0),
    };
    drop(permit);
    let now = unix_timestamp();
    {
        let mut backfill = state.backfill.write().await;
        backfill.active = backfill.active.saturating_sub(1);
        if success {
            backfill.record_peer_success(peer_id, now);
            if let Err(persist_error) = backfill.persist() {
                success = false;
                error = persist_error;
            }
        }
        if success {
            backfill.successful = backfill.successful.saturating_add(1);
            backfill.hashes_discovered = backfill.hashes_discovered.saturating_add(1);
        } else {
            backfill.failed = backfill.failed.saturating_add(1);
        }
    }
    serde_json::json!({
        "success": success,
        "peerId": peer_id,
        "path": path,
        "hash": hash,
        "flacKey": if success {
            content_discovery::generate_flac_key(path, size)
        } else {
            String::new()
        },
        "error": error,
        "durationMs": u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        "bytesRead": bytes_read,
    })
}

pub(super) fn spawn_backfill_scheduler(state: Arc<AppState>) {
    let task_state = Arc::clone(&state);
    state.spawn_managed_task(async move {
        let state = task_state;
        time::sleep(Duration::from_secs(120)).await;
        let mut interval = time::interval(Duration::from_secs(BACKFILL_RUN_INTERVAL_SECONDS));
        interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            let should_run = {
                let backfill = state.backfill.read().await;
                backfill.enabled
                    && backfill.is_idle
                    && backfill.idle_since.is_some_and(|since| {
                        unix_timestamp().saturating_sub(since) >= BACKFILL_MIN_IDLE_SECONDS
                    })
            };
            if should_run {
                let _ = run_backfill_cycle(&state).await;
            }
        }
    });
}

pub(super) async fn run_backfill_cycle(state: &AppState) -> serde_json::Value {
    let started = Instant::now();
    let candidates = backfill_candidates(state, BACKFILL_DEFAULT_CANDIDATES).await;
    let candidates_evaluated = candidates.len();
    let mut attempted = 0_u64;
    let mut successful = 0_u64;
    let mut failed = 0_u64;
    let mut rate_limited = 0_u64;
    let mut results = Vec::new();
    for candidate in candidates {
        if candidate.peer_backfills_today >= BACKFILL_MAX_PER_PEER_PER_DAY {
            rate_limited = rate_limited.saturating_add(1);
            continue;
        }
        if candidate.is_peer_native || !candidate.is_peer_online {
            continue;
        }
        attempted = attempted.saturating_add(1);
        let result =
            backfill_file(state, &candidate.peer_id, &candidate.path, candidate.size).await;
        if result["success"].as_bool().unwrap_or(false) {
            successful = successful.saturating_add(1);
        } else {
            failed = failed.saturating_add(1);
        }
        results.push(result);
    }
    let now = unix_timestamp();
    {
        let mut backfill = state.backfill.write().await;
        backfill.rate_limited = backfill.rate_limited.saturating_add(rate_limited);
        backfill.last_cycle_time = Some(now);
        backfill.next_cycle_time = Some(now.saturating_add(BACKFILL_RUN_INTERVAL_SECONDS));
    }
    serde_json::json!({
        "candidatesEvaluated": candidates_evaluated,
        "backfillsAttempted": attempted,
        "successful": successful,
        "failed": failed,
        "rateLimited": rate_limited,
        "durationMs": u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        "results": results,
    })
}
