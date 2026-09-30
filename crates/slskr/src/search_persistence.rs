use super::*;

pub(super) fn persisted_search_status(status: &str) -> &'static str {
    match status {
        "active" | "pending" => "active",
        "completed" | "complete" => "completed",
        "expired" => "expired",
        "cancelled" | "canceled" => "cancelled",
        "failed" => "failed",
        _ => "active",
    }
}

pub(super) fn normalize_search_status(status: &str) -> Option<&'static str> {
    match status {
        "active" | "pending" | "in_progress" | "InProgress" => Some("active"),
        "completed" | "complete" | "Completed" => Some("completed"),
        "expired" | "Expired" => Some("expired"),
        "cancelled" | "canceled" | "Cancelled" | "Canceled" => Some("cancelled"),
        "failed" | "Failed" => Some("failed"),
        _ => None,
    }
}

pub(super) fn search_state_for_status(status: &str) -> &'static str {
    match status {
        "active" => "InProgress",
        "failed" => "Failed",
        "cancelled" => "Cancelled",
        "expired" => "Expired",
        _ => "Completed",
    }
}

pub(super) fn search_ttl_seconds_from_body(body: &str) -> Result<u64, &'static str> {
    let ttl_seconds = extract_json_u64_field(body, "ttl_seconds")
        .or_else(|| extract_json_u64_field(body, "ttlSeconds"));
    match ttl_seconds {
        Some(0) => Err("search ttl_seconds must be greater than zero"),
        Some(ttl) => Ok(ttl.min(MAX_SEARCH_TTL_SECONDS)),
        None => Ok(DEFAULT_SEARCH_TTL_SECONDS),
    }
}

pub(super) fn persisted_target(target: Option<&str>) -> &'static str {
    match target {
        Some("user") => "user",
        Some("room") => "room",
        Some("wishlist") => "wishlist",
        _ => "global",
    }
}

pub(super) fn persisted_search_record(record: &SearchRecord) -> persistence::SearchRecord {
    let completed_at = if record.status == "active" {
        None
    } else {
        Some(record.updated_at as i64)
    };
    let (room, target) = match record.target {
        "room" => (record.target_name.clone(), Some("room".to_owned())),
        "user" => (None, Some("user".to_owned())),
        "wishlist" => (record.target_name.clone(), Some("wishlist".to_owned())),
        _ => (None, Some("global".to_owned())),
    };
    persistence::SearchRecord {
        id: record.token.to_string(),
        query: record.query.clone(),
        status: record.status.to_string(),
        result_count: record.results.len() as i64,
        created_at: record.created_at as i64,
        completed_at,
        room,
        target,
        fallback_attempts: i64::try_from(record.fallback_attempts).unwrap_or(i64::MAX),
    }
}

fn persisted_search_result_records_from_entries(
    search_id: &str,
    updated_at: u64,
    results: &[SearchResultEntry],
) -> Vec<persistence::SearchResultRecord> {
    let created_at = i64::try_from(updated_at).unwrap_or(i64::MAX);
    results
        .iter()
        .map(|result| persistence::SearchResultRecord {
            id: 0,
            search_id: search_id.to_owned(),
            peer_username: result.peer_username.clone(),
            filename: result.filename.clone(),
            size: i64::try_from(result.size).unwrap_or(i64::MAX),
            extension: result.extension.clone(),
            bit_rate: result.bit_rate.map(i64::from),
            sample_rate: result.sample_rate.map(i64::from),
            bit_depth: result.bit_depth.map(i64::from),
            length_seconds: result.length_seconds.map(i64::from),
            locked: result.locked,
            slot_free: result.slot_free,
            average_speed: result.average_speed.map(i64::from),
            queue_length: result.queue_length.map(i64::from),
            created_at,
        })
        .collect()
}

pub(super) fn persisted_search_result_records(
    record: &SearchRecord,
) -> Vec<persistence::SearchResultRecord> {
    persisted_search_result_records_from_entries(
        &record.token.to_string(),
        record.updated_at,
        &record.results,
    )
}

pub(super) async fn persist_search_record(
    state: &AppState,
    record: &SearchRecord,
) -> Result<(), String> {
    let Some(db) = state.db.as_ref() else {
        return Ok(());
    };
    let _persistence_turn = state.search_persistence_lock.lock().await;
    let snapshot = {
        let searches = state.searches.read().await;
        searches
            .get(record.token)
            .filter(|current| current.id == record.id)
    };
    let Some(snapshot) = snapshot else {
        return Ok(());
    };
    let persisted_record = persisted_search_record(&snapshot);
    let persisted_results = persisted_search_result_records(&snapshot);
    db.persist_search(&persisted_record, &snapshot.id, &persisted_results)
        .await
        .map_err(|error| format!("failed to persist search and results: {error}"))?;
    Ok(())
}

pub(super) async fn persist_search_result_delta(
    state: &AppState,
    record: &SearchRecord,
    appended: &[SearchResultEntry],
) -> Result<(), String> {
    let Some(db) = state.db.as_ref() else {
        return Ok(());
    };
    let _persistence_turn = state.search_persistence_lock.lock().await;
    let snapshot = {
        let searches = state.searches.read().await;
        searches
            .get(record.token)
            .filter(|current| current.id == record.id)
    };
    let Some(snapshot) = snapshot else {
        return Ok(());
    };
    let appended = appended
        .iter()
        .filter(|entry| snapshot.results.contains(*entry))
        .cloned()
        .collect::<Vec<_>>();
    let persisted_record = persisted_search_record(&snapshot);
    let persisted_results = persisted_search_result_records_from_entries(
        &snapshot.token.to_string(),
        snapshot.updated_at,
        &appended,
    );
    db.append_search_results(&persisted_record, &snapshot.id, &persisted_results)
        .await
        .map_err(|error| format!("failed to append search results: {error}"))?;
    Ok(())
}

pub(super) async fn persist_search_transition(
    state: &AppState,
    upserts: &[SearchRecord],
    evicted: &[SearchRecord],
) -> Result<(), String> {
    let Some(db) = state.db.as_ref() else {
        return Ok(());
    };
    let _persistence_turn = state.search_persistence_lock.lock().await;
    let searches = state.searches.read().await;
    let writes = upserts
        .iter()
        .filter_map(|record| {
            searches
                .records
                .iter()
                .find(|current| current.token == record.token && current.id == record.id)
                .map(|current| persistence::SearchWrite {
                    record: persisted_search_record(current),
                    external_id: current.id.clone(),
                    results: persisted_search_result_records(current),
                })
        })
        .collect::<Vec<_>>();
    let deletes = evicted
        .iter()
        .filter(|record| {
            !searches
                .records
                .iter()
                .any(|current| current.token == record.token)
        })
        .map(|record| record.token.to_string())
        .collect::<Vec<_>>();
    drop(searches);
    db.persist_search_changes(&writes, &deletes)
        .await
        .map_err(|error| format!("failed to persist search transition: {error}"))?;
    Ok(())
}

async fn persist_search_records(state: &AppState, records: &[SearchRecord]) -> Result<(), String> {
    let Some(db) = state.db.as_ref() else {
        return Ok(());
    };
    let _persistence_turn = state.search_persistence_lock.lock().await;
    let searches = state.searches.read().await;
    let writes = records
        .iter()
        .filter_map(|record| {
            searches
                .records
                .iter()
                .find(|current| current.token == record.token && current.id == record.id)
                .map(|current| persistence::SearchWrite {
                    record: persisted_search_record(current),
                    external_id: current.id.clone(),
                    results: persisted_search_result_records(current),
                })
        })
        .collect::<Vec<_>>();
    drop(searches);
    db.persist_search_changes(&writes, &[])
        .await
        .map_err(|error| format!("failed to persist search records: {error}"))?;
    Ok(())
}

async fn persist_expired_searches(
    state: &AppState,
    records: &[SearchRecord],
) -> Result<(), String> {
    persist_search_records(state, records).await
}

pub(super) async fn delete_persisted_search(
    state: &AppState,
    record: &SearchRecord,
) -> Result<(), String> {
    let Some(db) = state.db.as_ref() else {
        return Ok(());
    };
    let _persistence_turn = state.search_persistence_lock.lock().await;
    if state
        .searches
        .read()
        .await
        .records
        .iter()
        .any(|current| current.token == record.token)
    {
        return Ok(());
    }
    db.delete_search(&record.token.to_string())
        .await
        .map_err(|error| format!("failed to delete persisted search: {error}"))?;
    Ok(())
}

pub(super) async fn delete_persisted_searches(
    state: &AppState,
    records: &[SearchRecord],
) -> Result<(), String> {
    let Some(db) = state.db.as_ref() else {
        return Ok(());
    };
    let _persistence_turn = state.search_persistence_lock.lock().await;
    let searches = state.searches.read().await;
    let ids = records
        .iter()
        .filter(|record| {
            !searches
                .records
                .iter()
                .any(|current| current.token == record.token)
        })
        .map(|record| record.token.to_string())
        .collect::<Vec<_>>();
    drop(searches);
    if !ids.is_empty() {
        db.delete_searches(&ids)
            .await
            .map_err(|error| format!("failed to delete persisted searches: {error}"))?;
    }
    Ok(())
}

pub(super) async fn clear_persisted_searches(
    state: &AppState,
    _persistence_turn: &tokio::sync::MutexGuard<'_, ()>,
) -> Result<(), String> {
    if let Some(db) = state.db.as_ref() {
        db.delete_all_searches()
            .await
            .map_err(|error| format!("failed to clear persisted searches: {error}"))?;
    }
    Ok(())
}

pub(super) async fn persist_expired_searches_with_rollback(
    state: &AppState,
    previous: SearchStore,
    mutated: &SearchStore,
    records: &[SearchRecord],
) -> Result<(), String> {
    if let Err(error) = persist_expired_searches(state, records).await {
        rollback_searches_if_unchanged(state, previous, mutated).await;
        return Err(error);
    }
    Ok(())
}

pub(super) async fn rollback_search_record_if_unchanged(
    state: &AppState,
    previous: &SearchRecord,
    mutated: &SearchRecord,
) {
    let mut searches = state.searches.write().await;
    let Some(current) = searches
        .records
        .iter_mut()
        .find(|record| record.token == mutated.token)
    else {
        return;
    };
    if *current == *mutated {
        *current = previous.clone();
    }
}

pub(super) async fn rollback_searches_if_unchanged(
    state: &AppState,
    previous: SearchStore,
    mutated: &SearchStore,
) {
    let mut searches = state.searches.write().await;
    if *searches == *mutated {
        *searches = previous;
    }
}
