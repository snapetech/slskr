use super::*;

pub(super) async fn auto_download_completed_wishlist(
    state: &AppState,
    search: &SearchRecord,
) -> Result<usize, String> {
    const MAX_AUTOMATIC_DOWNLOADS_PER_SEARCH: usize = 50;
    let Some(item_id) = search.wishlist_item_id() else {
        return Ok(0);
    };
    let item = state.wishlist.read().await.get_item(item_id);
    let Some(item) = item else {
        return Ok(0);
    };
    if !item.auto_download || search.results.is_empty() {
        return Ok(0);
    }
    let remaining_downloads = item
        .max_downloads
        .map(|limit| limit.saturating_sub(item.total_download_count))
        .unwrap_or(u64::MAX);
    if remaining_downloads == 0 {
        return Ok(0);
    }
    if !state.config.transfer_allow_outbound {
        return Err("wishlist auto-download is blocked by outbound transfer policy".to_owned());
    }

    let edition_mode = state
        .integration_settings
        .read()
        .await
        .lidarr
        .edition_match_mode
        .clone();
    let edition_matching_enabled =
        item.lidarr_album_id.is_some() && !edition_mode.eq_ignore_ascii_case("off");
    let download_exclusions = effective_download_exclusions(state).await;
    let recently_completed = recently_completed_wishlist_track_keys(state).await;
    let wishlist_policy = state.wishlist.read().await.result_policy_for(item_id);

    let mut groups = BTreeMap::<(String, String), Vec<SearchResultEntry>>::new();
    for result in &search.results {
        let Some(username) = result.peer_username.as_deref() else {
            continue;
        };
        if result.locked {
            continue;
        }
        if download_filter::is_excluded(&result.filename, &download_exclusions) {
            continue;
        }
        if recently_completed.contains(&wishlist_track_identity(&result.filename)) {
            continue;
        }
        if wishlist_policy
            .as_ref()
            .is_some_and(|policy| !policy.filter.matches_entry(result))
        {
            continue;
        }
        groups
            .entry((
                username.to_owned(),
                wishlist_parent_directory(&result.filename),
            ))
            .or_default()
            .push(result.clone());
    }
    let remaining_limit = usize::try_from(remaining_downloads).unwrap_or(usize::MAX);
    let mut plans = Vec::new();
    for ((username, directory), candidates) in groups {
        let mut best_per_track = Vec::<SearchResultEntry>::new();
        for candidate in candidates {
            let identity = wishlist_group_track_identity(&candidate.filename);
            if let Some(existing) = best_per_track
                .iter_mut()
                .find(|existing| wishlist_group_track_identity(&existing.filename) == identity)
            {
                let candidate_quality = wishlist_policy
                    .as_ref()
                    .map(|policy| wishlist_quality_key(&candidate, &policy.filter))
                    .unwrap_or_default();
                let existing_quality = wishlist_policy
                    .as_ref()
                    .map(|policy| wishlist_quality_key(existing, &policy.filter))
                    .unwrap_or_default();
                if candidate_quality > existing_quality
                    || (candidate_quality == existing_quality
                        && candidate.filename < existing.filename)
                {
                    *existing = candidate;
                }
            } else {
                best_per_track.push(candidate);
            }
        }
        best_per_track.sort_by(|left, right| {
            let left_quality = wishlist_policy
                .as_ref()
                .map(|policy| wishlist_quality_key(left, &policy.filter))
                .unwrap_or_default();
            let right_quality = wishlist_policy
                .as_ref()
                .map(|policy| wishlist_quality_key(right, &policy.filter))
                .unwrap_or_default();
            right_quality
                .cmp(&left_quality)
                .then_with(|| left.filename.cmp(&right.filename))
        });
        let files = best_per_track
            .iter()
            .take(remaining_limit)
            .cloned()
            .collect::<Vec<_>>();
        if files.is_empty() {
            continue;
        }
        let edition_mismatch = edition_matching_enabled
            && wishlist_edition_mismatch(&item, &directory, &best_per_track);
        if edition_mode.eq_ignore_ascii_case("exclude") && edition_mismatch {
            continue;
        }
        let weakest_quality = files
            .iter()
            .map(|file| {
                wishlist_policy
                    .as_ref()
                    .map(|policy| wishlist_quality_key(file, &policy.filter))
                    .unwrap_or_default()
            })
            .min()
            .unwrap_or_default();
        let representative_quality = wishlist_policy
            .as_ref()
            .map(|policy| wishlist_quality_key(&files[0], &policy.filter))
            .unwrap_or_default();
        plans.push(WishlistAutoDownloadPlan {
            username,
            directory,
            coverage: best_per_track.len().min(remaining_limit),
            files,
            weakest_quality,
            representative_quality,
            edition_mismatch,
        });
    }

    let Some(best_plan) = plans.into_iter().max_by(|left, right| {
        (!left.edition_mismatch)
            .cmp(&(!right.edition_mismatch))
            .then_with(|| left.coverage.cmp(&right.coverage))
            .then_with(|| left.weakest_quality.cmp(&right.weakest_quality))
            .then_with(|| {
                left.representative_quality
                    .cmp(&right.representative_quality)
            })
            .then_with(|| {
                left.files[0]
                    .slot_free
                    .unwrap_or(true)
                    .cmp(&right.files[0].slot_free.unwrap_or(true))
            })
            .then_with(|| {
                left.files[0]
                    .average_speed
                    .unwrap_or(0)
                    .cmp(&right.files[0].average_speed.unwrap_or(0))
            })
            .then_with(|| {
                right.files[0]
                    .queue_length
                    .unwrap_or(u32::MAX)
                    .cmp(&left.files[0].queue_length.unwrap_or(u32::MAX))
            })
            .then_with(|| right.username.cmp(&left.username))
            .then_with(|| right.directory.cmp(&left.directory))
    }) else {
        return Ok(0);
    };
    if best_plan.files.len() > MAX_AUTOMATIC_DOWNLOADS_PER_SEARCH {
        record_event(
            state,
            "wishlist.auto_download_skipped",
            item_id.to_owned(),
            Some(format!(
                "candidate_count={} safety_limit={MAX_AUTOMATIC_DOWNLOADS_PER_SEARCH}",
                best_plan.files.len()
            )),
        )
        .await;
        return Ok(0);
    }

    let username = best_plan.username;
    let files = best_plan.files;

    // Re-read the mutable state after ranking. A user or integration may have
    // disabled automatic downloads while the network search was in flight.
    let latest_item = state.wishlist.read().await.get_item(item_id);
    let Some(latest_item) = latest_item else {
        return Ok(0);
    };
    if !latest_item.enabled || !latest_item.auto_download {
        return Ok(0);
    }
    let remaining_downloads = latest_item
        .max_downloads
        .map(|limit| limit.saturating_sub(latest_item.total_download_count))
        .unwrap_or(u64::MAX);
    if remaining_downloads == 0 {
        return Ok(0);
    }

    let available = {
        let transfers = state.transfers.read().await;
        state
            .config
            .transfer_max_active
            .saturating_sub(transfers.active_count_excluding(None))
    };
    let batch_id = (files.len() > 1).then(|| uuid::Uuid::new_v4().to_string());
    let mut staged = Vec::new();
    let batch_limit = available
        .min(MAX_AUTOMATIC_DOWNLOADS_PER_SEARCH)
        .min(usize::try_from(remaining_downloads).unwrap_or(usize::MAX));
    for file in files.into_iter().take(batch_limit) {
        let Ok(session_command_permit) = state.session_commands.try_reserve() else {
            break;
        };
        let local_path = configured_download_destination_path(state, &file.filename)
            .await?
            .display()
            .to_string();
        let entry = {
            let mut transfers = state.transfers.write().await;
            let entry = transfers.create_with_batch_for_wishlist(
                0,
                Some(username.clone()),
                file.filename,
                Some(local_path),
                Some(file.size),
                batch_id.clone(),
                item_id.to_owned(),
            );
            transfers
                .update_status(entry.id, "peer_lookup", None, None)
                .unwrap_or(entry)
        };
        staged.push((entry, session_command_permit));
    }

    let enqueued = staged.len();
    if enqueued > 0 {
        let staged_entries = staged
            .iter()
            .map(|(entry, _)| entry.clone())
            .collect::<Vec<_>>();
        if let Err(error) = persist_transfer_records(state, &staged_entries).await {
            return Err(with_auto_download_rollback(
                error,
                rollback_staged_auto_downloads(state, &staged_entries).await,
            ));
        }
        let _wishlist_search_persistence = state.wishlist_search_persistence_lock.lock().await;
        let mut wishlist = state.wishlist.write().await;
        let previous = wishlist.clone();
        let Some(item) = wishlist.record_auto_downloads(item_id, enqueued) else {
            drop(wishlist);
            rollback_staged_auto_downloads(state, &staged_entries).await?;
            return Ok(0);
        };
        let mutated = wishlist.clone();
        drop(wishlist);
        if let Err(error) = persist_wishlist_item_checked(state, &item).await {
            rollback_wishlist_if_unchanged(state, previous, &mutated).await;
            return Err(with_auto_download_rollback(
                error,
                rollback_staged_auto_downloads(state, &staged_entries).await,
            ));
        }
        drop(_wishlist_search_persistence);
        for (entry, permit) in staged {
            permit.send(SessionCommand::TransferPeer {
                id: entry.id,
                username: username.clone(),
            });
        }
        record_event(
            state,
            "wishlist.auto_downloaded",
            item_id.to_owned(),
            Some(format!("enqueued={enqueued}")),
        )
        .await;
    }
    Ok(enqueued)
}

async fn recently_completed_wishlist_track_keys(state: &AppState) -> HashSet<String> {
    state
        .transfers
        .read()
        .await
        .entries
        .iter()
        .filter(|entry| entry.direction == 0 && is_successful_transfer_status(&entry.status))
        .filter_map(|entry| {
            let identity = entry
                .title
                .as_deref()
                .map(wishlist_track_identity)
                .filter(|identity| !identity.is_empty())
                .unwrap_or_else(|| wishlist_track_identity(&entry.filename));
            (!identity.is_empty()).then_some(identity)
        })
        .collect()
}

fn with_auto_download_rollback(error: String, rollback: Result<(), String>) -> String {
    match rollback {
        Ok(()) => error,
        Err(rollback_error) => {
            format!("{error}; staged transfer rollback failed: {rollback_error}")
        }
    }
}

async fn rollback_staged_auto_downloads(
    state: &AppState,
    entries: &[TransferEntry],
) -> Result<(), String> {
    let removed = remove_transfer_entries_if_unchanged(state, entries).await;
    if removed.is_empty() {
        return Ok(());
    }
    let Some(db) = state.db.as_ref() else {
        return Ok(());
    };
    let now = unix_timestamp_millis();
    let records = removed
        .iter()
        .map(|entry| {
            (
                entry.id.to_string(),
                database_i64(now.max(entry.updated_at_ms.saturating_add(1))),
            )
        })
        .collect::<Vec<_>>();
    db.rollback_staged_transfers(&records)
        .await
        .map_err(|error| format!("failed to remove staged transfers: {error}"))
}
