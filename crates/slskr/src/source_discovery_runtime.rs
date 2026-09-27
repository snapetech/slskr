use super::*;

pub(super) async fn dispatch_source_discovery_search(
    state: &AppState,
    query: String,
) -> Result<SearchRecord, String> {
    let permit = state
        .session_commands
        .reserve()
        .await
        .map_err(|_| "session manager is not running".to_owned())?;
    let record = create_rescue_search(state, query.clone()).await?;
    permit.send(SessionCommand::Search {
        token: record.token,
        query,
        target: SearchDispatchTarget::Global,
    });
    Ok(record)
}

pub(super) fn source_discovery_sources(
    discovery: &SourceDiscoveryState,
    searches: &SearchStore,
) -> Vec<serde_json::Value> {
    let tokens = discovery
        .search_tokens
        .iter()
        .copied()
        .collect::<HashSet<_>>();
    let mut seen = HashSet::new();
    let mut sources = Vec::new();
    for search in searches
        .records
        .iter()
        .filter(|search| tokens.contains(&search.token))
    {
        for result in &search.results {
            let username = result.peer_username.as_deref().unwrap_or_default();
            if username.is_empty()
                || !seen.insert((
                    username.to_ascii_lowercase(),
                    result.filename.clone(),
                    result.size,
                ))
            {
                continue;
            }
            sources.push(serde_json::json!({
                "username": username,
                "filename": result.filename,
                "size": result.size,
                "hash": null,
                "uploadSpeed": result.average_speed.unwrap_or(0),
                "firstSeenUnix": search.created_at,
                "lastSeenUnix": search.updated_at,
            }));
        }
    }
    sources
}

pub(super) fn spawn_source_discovery(state: Arc<AppState>) {
    let task_state = Arc::clone(&state);
    state.spawn_managed_task(async move {
        let state = task_state;
        let mut interval = time::interval(Duration::from_secs(SOURCE_DISCOVERY_CYCLE_SECONDS));
        interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);
        interval.tick().await;
        loop {
            interval.tick().await;
            let query = {
                let discovery = state.source_discovery.read().await;
                discovery
                    .running
                    .then(|| (discovery.search_term.clone(), discovery.generation))
            };
            let Some((query, generation)) = query else {
                continue;
            };
            match dispatch_source_discovery_search(&state, query).await {
                Ok(record) => {
                    state
                        .source_discovery
                        .write()
                        .await
                        .record_dispatch_if_current(generation, record.token);
                }
                Err(error) => {
                    record_daemon_log(
                        &state,
                        logging::LogLevel::Warn,
                        "source-discovery",
                        format!("source discovery cycle failed: {error}"),
                    )
                    .await;
                }
            }
        }
    });
}
