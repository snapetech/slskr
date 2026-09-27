use super::*;

pub(super) async fn database_stats_value(state: &AppState) -> serde_json::Value {
    let searches = state.searches.read().await;
    let projected_searches = searches.records.len();
    drop(searches);
    let transfers = state.transfers.read().await;
    let projected_transfers = transfers.entries.len();
    drop(transfers);
    let shares = state.shares.read().await;
    let projected_shares = shares.entries.len();
    drop(shares);
    let messages = state.messages.read().await;
    let projected_messages = messages.records.len();
    drop(messages);
    let users = state.users.read().await;
    let projected_users = users.records.len();
    drop(users);
    let browse = state.browse.read().await;
    let projected_browse = browse.records.len();
    drop(browse);
    let rooms = state.rooms.read().await;
    let projected_rooms = rooms.records.len();
    drop(rooms);
    let events = state.events.read().await;
    let projected_events = events.records.len();
    drop(events);
    let user_notes = state.user_notes.read().await;
    let projected_user_notes = user_notes.records.len();
    drop(user_notes);
    let interests = state.interests.read().await;
    let projected_interests = interests.liked.len() + interests.hated.len();
    drop(interests);
    let security = state.security.read().await;
    let projected_security_bans = security.bans.len();
    drop(security);
    let wishlist = state.wishlist.read().await;
    let projected_wishlist = wishlist.records.len();
    drop(wishlist);
    let contacts = state.contacts.read().await;
    let projected_contacts = contacts.records.len();
    drop(contacts);
    let share_grants = state.share_grants.read().await;
    let projected_share_grants = share_grants.records.len();
    drop(share_grants);
    let sharegroups = state.sharegroups.read().await;
    let projected_sharegroups = sharegroups.records.len();
    let projected_sharegroup_members = sharegroups
        .records
        .iter()
        .map(|record| record.members.len())
        .sum::<usize>();
    drop(sharegroups);
    let collections = state.collections.read().await;
    let projected_collections = collections.records.len();
    let projected_collection_items = collections
        .records
        .iter()
        .map(|record| record.items.len())
        .sum::<usize>();
    drop(collections);
    let library = state.library.read().await;
    let projected_library_items = library.records.len();
    drop(library);
    let destinations = state.destinations.read().await;
    let projected_destinations = destinations.records.len();
    drop(destinations);
    let now_playing = state.now_playing.read().await;
    let projected_now_playing = now_playing.records.len();
    drop(now_playing);
    let oauth_states = state.oauth_states.read().await;
    let projected_oauth_states = oauth_states.records.len();
    drop(oauth_states);
    let webhooks = state.webhooks.read().await;
    let projected_webhooks = webhooks.get_all().len();
    drop(webhooks);

    let persisted = if let Some(db) = state.db.as_ref() {
        match db.get_stats().await {
            Ok(stats) => serde_json::json!({
                "enabled": true,
                "healthy": true,
                "searches": stats.search_count,
                "searchResults": stats.search_result_count,
                "transfers": stats.transfer_count,
                "transferEvents": stats.transfer_event_count,
                "shares": stats.share_file_count,
                "events": stats.event_count,
                "messages": stats.message_count,
                "users": stats.user_projection_count,
                "userStats": stats.user_count,
                "browse": stats.browse_count,
                "rooms": stats.room_count,
                "userNotes": stats.user_note_count,
                "interests": stats.interest_count,
                "securityBans": stats.security_ban_count,
                "wishlist": stats.wishlist_count,
                "contacts": stats.contact_count,
                "shareGrants": stats.share_grant_count,
                "sharegroups": stats.share_group_count,
                "sharegroupMembers": stats.share_group_member_count,
                "collections": stats.collection_count,
                "collectionItems": stats.collection_item_count,
                "libraryItems": stats.library_item_count,
                "destinations": stats.destination_count,
                "nowPlaying": stats.now_playing_count,
                "runtimeState": stats.runtime_state_count,
                "oauthStates": stats.oauth_state_count,
                "webhooks": stats.webhook_count,
                "webhookLogs": stats.webhook_log_count,
            }),
            Err(error) => {
                eprintln!("database statistics failed: {error}");
                serde_json::json!({
                    "enabled": true,
                    "healthy": false,
                    "error": "database statistics unavailable",
                })
            }
        }
    } else {
        serde_json::json!({
            "enabled": false,
            "healthy": false,
            "searches": 0,
            "searchResults": 0,
            "transfers": 0,
            "transferEvents": 0,
            "shares": 0,
            "events": 0,
            "messages": 0,
            "users": 0,
            "userStats": 0,
            "browse": 0,
            "rooms": 0,
            "userNotes": 0,
            "interests": 0,
            "securityBans": 0,
            "wishlist": 0,
            "contacts": 0,
            "shareGrants": 0,
            "sharegroups": 0,
            "sharegroupMembers": 0,
            "collections": 0,
            "collectionItems": 0,
            "libraryItems": 0,
            "destinations": 0,
            "nowPlaying": 0,
            "runtimeState": 0,
            "oauthStates": 0,
            "webhooks": 0,
            "webhookLogs": 0,
        })
    };

    let mut projections = serde_json::Map::new();
    projections.insert("searches".to_owned(), serde_json::json!(projected_searches));
    projections.insert(
        "transfers".to_owned(),
        serde_json::json!(projected_transfers),
    );
    projections.insert("shares".to_owned(), serde_json::json!(projected_shares));
    projections.insert("messages".to_owned(), serde_json::json!(projected_messages));
    projections.insert("users".to_owned(), serde_json::json!(projected_users));
    projections.insert("browse".to_owned(), serde_json::json!(projected_browse));
    projections.insert("rooms".to_owned(), serde_json::json!(projected_rooms));
    projections.insert("events".to_owned(), serde_json::json!(projected_events));
    projections.insert(
        "userNotes".to_owned(),
        serde_json::json!(projected_user_notes),
    );
    projections.insert(
        "interests".to_owned(),
        serde_json::json!(projected_interests),
    );
    projections.insert(
        "securityBans".to_owned(),
        serde_json::json!(projected_security_bans),
    );
    projections.insert("wishlist".to_owned(), serde_json::json!(projected_wishlist));
    projections.insert("contacts".to_owned(), serde_json::json!(projected_contacts));
    projections.insert(
        "shareGrants".to_owned(),
        serde_json::json!(projected_share_grants),
    );
    projections.insert(
        "sharegroups".to_owned(),
        serde_json::json!(projected_sharegroups),
    );
    projections.insert(
        "sharegroupMembers".to_owned(),
        serde_json::json!(projected_sharegroup_members),
    );
    projections.insert(
        "collections".to_owned(),
        serde_json::json!(projected_collections),
    );
    projections.insert(
        "collectionItems".to_owned(),
        serde_json::json!(projected_collection_items),
    );
    projections.insert(
        "libraryItems".to_owned(),
        serde_json::json!(projected_library_items),
    );
    projections.insert(
        "destinations".to_owned(),
        serde_json::json!(projected_destinations),
    );
    projections.insert(
        "nowPlaying".to_owned(),
        serde_json::json!(projected_now_playing),
    );
    projections.insert(
        "oauthStates".to_owned(),
        serde_json::json!(projected_oauth_states),
    );
    projections.insert("webhooks".to_owned(), serde_json::json!(projected_webhooks));

    let mut value = serde_json::Map::new();
    value.insert(
        "connected".to_owned(),
        serde_json::json!(state.db.is_some()),
    );
    value.insert("enabled".to_owned(), serde_json::json!(state.db.is_some()));
    value.insert(
        "healthy".to_owned(),
        serde_json::json!(persisted["healthy"].as_bool().unwrap_or(false)),
    );
    for key in [
        "searches",
        "searchResults",
        "transfers",
        "transferEvents",
        "shares",
        "events",
        "messages",
        "users",
        "userStats",
        "browse",
        "rooms",
        "userNotes",
        "interests",
        "securityBans",
        "wishlist",
        "contacts",
        "shareGrants",
        "sharegroups",
        "sharegroupMembers",
        "collections",
        "collectionItems",
        "libraryItems",
        "destinations",
        "nowPlaying",
        "runtimeState",
        "oauthStates",
        "webhooks",
        "webhookLogs",
    ] {
        value.insert(key.to_owned(), persisted[key].clone());
    }
    value.insert("persisted".to_owned(), persisted);
    value.insert(
        "projections".to_owned(),
        serde_json::Value::Object(projections),
    );
    serde_json::Value::Object(value)
}

async fn prune_terminal_transfers_for_cleanup(state: &AppState) -> Result<usize, String> {
    let mut transfers = state.transfers.write().await;
    let previous = transfers.mutation_snapshot();
    let ids = transfers
        .entries
        .iter()
        .filter(|entry| is_terminal_transfer_status(&entry.status))
        .map(|entry| entry.id)
        .collect::<Vec<_>>();
    let removed = transfers.remove_entries(&ids);
    let mutated = transfers.mutation_snapshot();
    drop(transfers);

    if removed.is_empty() {
        return Ok(0);
    }

    if let Err(error) = delete_persisted_transfers(state, &removed).await {
        let rolled_back = rollback_transfer_mutation_if_unchanged(state, previous, mutated).await;
        return Err(format!(
            "failed to persist transfer cleanup: {error}; in-memory rollback={rolled_back}"
        ));
    }

    Ok(removed.len())
}

async fn cleanup_old_message_projection(
    state: &AppState,
    cutoff: i64,
) -> (usize, serde_json::Value) {
    let _message_persistence = state.message_persistence_lock.lock().await;
    let (previous, mutated, removed) = {
        let mut messages = state.messages.write().await;
        let previous = messages.clone();
        let removed = messages.remove_older_than(cutoff);
        let mutated = messages.clone();
        (previous, mutated, removed)
    };

    let Some(db) = state.db.as_ref() else {
        return (
            removed,
            serde_json::json!({
                "enabled": false,
                "healthy": false,
                "cleaned": 0,
            }),
        );
    };

    let cleanup_result = db
        .cleanup_old_messages_before(cutoff)
        .await
        .map_err(|error| error.to_string());
    match cleanup_result {
        Ok(cleaned) => (
            removed,
            serde_json::json!({
                "enabled": true,
                "healthy": true,
                "cleaned": cleaned,
            }),
        ),
        Err(error_message) => {
            let rolled_back = {
                let mut messages = state.messages.write().await;
                if *messages == mutated {
                    *messages = previous;
                    true
                } else {
                    false
                }
            };
            eprintln!(
                "database cleanup failed: {error_message}; in-memory message rollback={rolled_back}"
            );
            (
                if rolled_back { 0 } else { removed },
                serde_json::json!({
                    "enabled": true,
                    "healthy": false,
                    "cleaned": 0,
                    "error": "database cleanup unavailable",
                }),
            )
        }
    }
}

pub(super) async fn database_cleanup_value(state: &AppState, body: &str) -> serde_json::Value {
    let days = extract_json_i32_field(body, "days").unwrap_or(30).max(0);
    let (pruned_transfers, transfer_cleanup_error) =
        match prune_terminal_transfers_for_cleanup(state).await {
            Ok(removed) => (removed, None),
            Err(error) => {
                eprintln!("transfer cleanup failed: {error}");
                (0, Some(error))
            }
        };
    let cutoff = i64::try_from(unix_timestamp())
        .unwrap_or(i64::MAX)
        .saturating_sub(i64::from(days).saturating_mul(86_400));
    let (pruned_messages, persisted_cleaned) = cleanup_old_message_projection(state, cutoff).await;
    let persistence_healthy =
        persisted_cleaned["healthy"].as_bool().unwrap_or(true) && transfer_cleanup_error.is_none();
    let status = if persistence_healthy {
        "ok"
    } else if persisted_cleaned["enabled"].as_bool().unwrap_or(false)
        || transfer_cleanup_error.is_some()
    {
        "error"
    } else {
        "skipped"
    };

    let mut transfer_cleanup = serde_json::json!({
        "enabled": state.db.is_some(),
        "healthy": transfer_cleanup_error.is_none(),
        "cleaned": pruned_transfers,
    });
    if transfer_cleanup_error.is_some() {
        transfer_cleanup["error"] = serde_json::json!("transfer cleanup unavailable");
    }

    serde_json::json!({
        "status": status,
        "days": days,
        "cleaned": persisted_cleaned["cleaned"],
        "persisted": persisted_cleaned,
        "transferCleanup": transfer_cleanup,
        "pruned_messages": pruned_messages,
        "pruned_transfers": pruned_transfers,
        "projectionCleanup": {
            "messages": pruned_messages,
            "transfers": pruned_transfers,
        },
    })
}

fn retention_age_for_transfer(
    retention: crate::config::TransferRetentionSettings,
    status: &str,
) -> Option<u64> {
    match status {
        "succeeded" | "completed" => retention.succeeded_minutes,
        "cancelled" => retention.cancelled_minutes.or(retention.failed_minutes),
        "failed" | "errored" | "aborted" | "rejected" | "timed_out" => {
            retention.errored_minutes.or(retention.failed_minutes)
        }
        _ => None,
    }
}

fn prune_files_older_than(root: &Path, age_minutes: u64, now: SystemTime) -> Result<usize, String> {
    let cutoff = now
        .checked_sub(Duration::from_secs(age_minutes.saturating_mul(60)))
        .unwrap_or(SystemTime::UNIX_EPOCH);
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => {
            return Err(format!(
                "file retention directory scan failed for {}: {error}",
                root.display()
            ));
        }
    };
    let mut removed = 0_usize;
    let mut failures = 0_usize;
    let mut first_failure = None;
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                failures = failures.saturating_add(1);
                if first_failure.is_none() {
                    first_failure = Some(format!("directory entry read failed: {error}"));
                }
                continue;
            }
        };
        let path = entry.path();
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                failures = failures.saturating_add(1);
                if first_failure.is_none() {
                    first_failure = Some(format!("{}: metadata failed: {error}", path.display()));
                }
                continue;
            }
        };
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_dir() {
            if let Err(error) = prune_files_older_than(&path, age_minutes, now) {
                failures = failures.saturating_add(1);
                if first_failure.is_none() {
                    first_failure = Some(error);
                }
            }
            match fs::remove_dir(&path) {
                Ok(()) => removed = removed.saturating_add(1),
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::NotFound | std::io::ErrorKind::DirectoryNotEmpty
                    ) => {}
                Err(error) => {
                    failures = failures.saturating_add(1);
                    if first_failure.is_none() {
                        first_failure = Some(format!(
                            "{}: directory removal failed: {error}",
                            path.display()
                        ));
                    }
                }
            }
        } else if metadata.is_file() {
            let is_expired = match metadata.modified() {
                Ok(modified) => modified < cutoff,
                Err(error) => {
                    failures = failures.saturating_add(1);
                    if first_failure.is_none() {
                        first_failure = Some(format!(
                            "{}: modification time failed: {error}",
                            path.display()
                        ));
                    }
                    false
                }
            };
            if is_expired {
                match fs::remove_file(&path) {
                    Ok(()) => removed = removed.saturating_add(1),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => {
                        failures = failures.saturating_add(1);
                        if first_failure.is_none() {
                            first_failure =
                                Some(format!("{}: file removal failed: {error}", path.display()));
                        }
                    }
                }
            }
        }
    }
    if failures == 0 {
        Ok(removed)
    } else {
        Err(format!(
            "file retention cleanup under {} had {failures} failure(s) after removing {removed} item(s); first failure: {}",
            root.display(),
            first_failure.unwrap_or_else(|| "unknown failure".to_owned())
        ))
    }
}

async fn run_retention_once(state: &AppState) {
    let now = unix_timestamp();
    let age_minutes = state.config.retention.search_minutes;
    let max_age_days = state.config.search_retention.max_age_days;
    let max_count = state.config.search_retention.max_count;
    let (previous_searches, mutated_searches, removed_searches) = {
        let mut searches = state.searches.write().await;
        let previous = searches.clone();
        let mut removed = Vec::new();
        searches.records.retain(|record| {
            if record.status == "active" {
                return true;
            }
            let expired_by_minutes = age_minutes.is_some_and(|minutes| {
                record.updated_at.saturating_add(minutes.saturating_mul(60)) <= now
            });
            let expired_by_days = max_age_days != 0
                && record
                    .updated_at
                    .saturating_add(max_age_days.saturating_mul(86_400))
                    <= now;
            if expired_by_minutes || expired_by_days {
                removed.push(record.clone());
                false
            } else {
                true
            }
        });
        if max_count != 0 {
            let mut completed = searches
                .records
                .iter()
                .filter(|record| record.status != "active")
                .map(|record| (record.updated_at, record.token))
                .collect::<Vec<_>>();
            completed.sort_unstable_by(|left, right| right.cmp(left));
            let excess = completed
                .into_iter()
                .skip(max_count)
                .map(|(_, token)| token)
                .collect::<HashSet<_>>();
            searches.records.retain(|record| {
                if excess.contains(&record.token) {
                    removed.push(record.clone());
                    false
                } else {
                    true
                }
            });
        }
        let mutated = searches.clone();
        (previous, mutated, removed)
    };
    if !removed_searches.is_empty() {
        if let Err(error) = delete_persisted_searches(state, &removed_searches).await {
            rollback_searches_if_unchanged(state, previous_searches, &mutated_searches).await;
            record_daemon_log(
                state,
                logging::LogLevel::Error,
                "retention",
                format!("failed to delete expired searches: {error}"),
            )
            .await;
        }
    }

    let (previous_transfer_entries, mutated_transfer_entries, removed_transfers) = {
        let mut transfers = state.transfers.write().await;
        let previous = transfers.mutation_snapshot();
        let ids = transfers
            .entries
            .iter()
            .filter(|entry| {
                let settings = if entry.direction == 1 {
                    state.config.retention.upload
                } else {
                    state.config.retention.download
                };
                retention_age_for_transfer(settings, &entry.status).is_some_and(|minutes| {
                    entry.updated_at.saturating_add(minutes.saturating_mul(60)) <= now
                })
            })
            .map(|entry| entry.id)
            .collect::<Vec<_>>();
        let removed = transfers.remove_entries(&ids);
        let mutated = transfers.mutation_snapshot();
        (previous, mutated, removed)
    };
    if !removed_transfers.is_empty() {
        if let Err(error) = delete_persisted_transfers(state, &removed_transfers).await {
            let rolled_back = rollback_transfer_mutation_if_unchanged(
                state,
                previous_transfer_entries,
                mutated_transfer_entries,
            )
            .await;
            record_daemon_log(
                state,
                logging::LogLevel::Error,
                "retention",
                format!(
                    "failed to delete expired transfers: {error}; in-memory rollback={rolled_back}"
                ),
            )
            .await;
        }
    }

    let complete = state.config.retention.files_complete_minutes;
    let incomplete = state.config.retention.files_incomplete_minutes;
    let downloads = effective_downloads_dir(state);
    let incomplete_dir = effective_incomplete_dir(state);
    let prune_result = match tokio::task::spawn_blocking(move || {
        let now = SystemTime::now();
        let mut failures = Vec::new();
        if let Some(age) = complete {
            if let Err(error) = prune_files_older_than(&downloads, age, now) {
                failures.push(error);
            }
        }
        if let Some(age) = incomplete {
            if let Err(error) = prune_files_older_than(&incomplete_dir, age, now) {
                failures.push(error);
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(failures.join("; "))
        }
    })
    .await
    {
        Ok(result) => result,
        Err(error) => Err(error.to_string()),
    };
    if let Err(error) = prune_result {
        record_daemon_log(
            state,
            logging::LogLevel::Error,
            "retention",
            format!("file retention worker failed: {error}"),
        )
        .await;
    }

    let log_cutoff = SystemTime::now()
        .checked_sub(Duration::from_secs(
            state.config.retention.logs_days.saturating_mul(86_400),
        ))
        .unwrap_or(SystemTime::UNIX_EPOCH);
    let log_directory = state.config.state_dir.join("logs");
    let mut log_cleanup_failure = None;
    match fs::read_dir(&log_directory) {
        Ok(entries) => {
            for entry in entries {
                let Ok(entry) = entry else {
                    log_cleanup_failure = Some("log directory entry read failed".to_owned());
                    continue;
                };
                let path = entry.path();
                let metadata = match entry.metadata() {
                    Ok(metadata) => metadata,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                    Err(error) => {
                        if log_cleanup_failure.is_none() {
                            log_cleanup_failure =
                                Some(format!("{}: metadata failed: {error}", path.display()));
                        }
                        continue;
                    }
                };
                let modified = match metadata.modified() {
                    Ok(modified) => modified,
                    Err(error) => {
                        if log_cleanup_failure.is_none() {
                            log_cleanup_failure = Some(format!(
                                "{}: modification time failed: {error}",
                                path.display()
                            ));
                        }
                        continue;
                    }
                };
                let expired = modified < log_cutoff;
                if expired {
                    match fs::remove_file(&path) {
                        Ok(()) => {}
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                        Err(error) if log_cleanup_failure.is_none() => {
                            log_cleanup_failure = Some(format!("{}: {error}", path.display()));
                        }
                        Err(_) => {}
                    }
                }
            }
        }
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
            log_cleanup_failure = Some(format!("{}: {error}", log_directory.display()));
        }
        Err(_) => {}
    }
    if let Some(error) = log_cleanup_failure {
        record_daemon_log(
            state,
            logging::LogLevel::Error,
            "retention",
            format!("log retention cleanup failed: {error}"),
        )
        .await;
    }
}

pub(super) fn spawn_retention_scheduler(state: Arc<AppState>) {
    let task_state = Arc::clone(&state);
    state.spawn_managed_task(async move {
        let state = task_state;
        run_retention_once(&state).await;
        let mut interval = time::interval(state.config.search_retention.cleanup_interval);
        interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);
        interval.tick().await;
        loop {
            interval.tick().await;
            run_retention_once(&state).await;
        }
    });
}

pub(super) async fn database_vacuum_value(state: &AppState) -> serde_json::Value {
    if let Some(db) = state.db.as_ref() {
        match db.vacuum().await {
            Ok(()) => serde_json::json!({
                "vacuumed": true,
                "enabled": true,
                "status": "ok",
            }),
            Err(error) => {
                eprintln!("database vacuum failed: {error}");
                serde_json::json!({
                    "vacuumed": false,
                    "enabled": true,
                    "status": "error",
                    "error": "database vacuum unavailable",
                })
            }
        }
    } else {
        serde_json::json!({
            "vacuumed": false,
            "enabled": false,
            "status": "skipped",
            "note": "database not initialized",
        })
    }
}
