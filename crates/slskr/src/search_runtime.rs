use super::*;

pub(super) async fn build_file_search_response(
    state: &AppState,
    token: u32,
    query: &str,
) -> Option<FileSearchResponse> {
    if state
        .search_request_filters
        .read()
        .await
        .iter()
        .any(|filter| filter.is_match(query))
    {
        return None;
    }
    let mut results = {
        let shares = state.shares.read().await;
        search_shares(&shares.entries, query)
    };
    let response_file_limit = state
        .core_workflow_settings
        .read()
        .await
        .incoming_search
        .response_file_limit;
    results.truncate(response_file_limit);
    if results.is_empty() {
        return None;
    }

    Some(FileSearchResponse {
        username: state
            .config
            .username
            .clone()
            .unwrap_or_else(|| "slskr".to_owned()),
        token,
        results,
        slot_free: true,
        average_speed: 0,
        queue_length: 0,
        unknown: 0,
        private_results: Vec::new(),
    })
}

pub(super) fn search_shares(entries: &[FileEntry], query: &str) -> Vec<FileEntry> {
    let terms = query
        .split_whitespace()
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>();
    if terms.is_empty() {
        return Vec::new();
    }
    entries
        .iter()
        .filter(|entry| {
            let filename = entry.filename.to_ascii_lowercase();
            terms.iter().all(|term| filename.contains(term))
        })
        .cloned()
        .collect()
}

pub(super) fn spawn_search_expiry_scheduler(state: Arc<AppState>) {
    let task_state = Arc::clone(&state);
    state.spawn_managed_task(async move {
        let state = task_state;
        let interval_duration = Duration::from_secs(1);
        let mut interval = time::interval_at(Instant::now() + interval_duration, interval_duration);
        interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

        loop {
            interval.tick().await;

            let preserve_wishlist = {
                let wishlist_enabled = state.core_workflow_settings.read().await.wishlist.enabled;
                let session_connected = state.session.read().await.state == "connected";
                wishlist_enabled && session_connected
            };
            let (previous_searches, expired, mutated_searches) = {
                let mut searches = state.searches.write().await;
                let previous_searches = searches.clone();
                let expired =
                    searches.expire_due_excluding_target(preserve_wishlist.then_some("wishlist"));
                let mutated_searches = searches.clone();
                (previous_searches, expired, mutated_searches)
            };

            if let Err(error) = persist_expired_searches_with_rollback(
                &state,
                previous_searches,
                &mutated_searches,
                &expired,
            )
            .await
            {
                record_daemon_log(&state, logging::LogLevel::Warn, "search", error).await;
            }
        }
    });
}
