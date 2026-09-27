use super::*;

fn hashdb_backfill_flac_candidates(records: &[SearchRecord]) -> Vec<(String, String, u64)> {
    records
        .iter()
        .flat_map(|record| {
            record.results.iter().filter_map(|result| {
                let peer_id = result.peer_username.as_deref()?.trim();
                let path = result.filename.trim();
                (path.to_ascii_lowercase().ends_with(".flac")
                    && result.size >= HASHDB_BACKFILL_MIN_FILE_SIZE
                    && !peer_id.is_empty()
                    && !path.is_empty())
                .then(|| (peer_id.to_owned(), path.to_owned(), result.size))
            })
        })
        .collect()
}

pub(super) async fn hashdb_backfill_from_history_response(
    query: Option<&str>,
    state: &AppState,
) -> HttpResponse {
    let batch_size = match query_parameter(query, "batchSize") {
        None => 50,
        Some(value) => match value.parse::<usize>() {
            Ok(value) => value.clamp(10, 500),
            Err(_) => return routing::bad_request_response("batchSize must be an integer"),
        },
    };
    let reset = match query_parameter(query, "reset") {
        None => false,
        Some(value) => match parse_bool_value(&value) {
            Some(value) => value,
            None => return routing::bad_request_response("reset must be a boolean"),
        },
    };

    let persistence_turn = hash_db_persistence_turn().await;
    let last_processed_at = if reset {
        None
    } else {
        match read_hash_db_state_json(state, HASHDB_BACKFILL_PROGRESS_KEY).await {
            Ok(value) => value
                .as_ref()
                .and_then(|value| value.get("lastProcessedAt"))
                .and_then(serde_json::Value::as_u64),
            Err(error) => return routing::internal_server_error_response(&error),
        }
    };
    let mut records = state.searches.read().await.records.clone();
    let total_searches = records.len();
    let remaining_searches = last_processed_at.map_or(total_searches, |last_processed_at| {
        records
            .iter()
            .filter(|record| record.created_at < last_processed_at)
            .count()
    });

    if remaining_searches == 0 && !reset {
        return routing::ok_response(
            serde_json::json!({
                "searchesProcessed": 0,
                "flacsDiscovered": 0,
                "totalSearches": total_searches,
                "remainingSearches": 0,
                "complete": true,
                "message": "Backfill complete - all searches have been processed. Use reset=true to start over.",
            })
            .to_string(),
        );
    }

    records.retain(|record| {
        last_processed_at.is_none_or(|last_processed_at| record.created_at < last_processed_at)
    });
    records.sort_by_key(|record| std::cmp::Reverse(record.created_at));
    records.truncate(batch_size);
    let searches_processed = records.len();
    let candidates = hashdb_backfill_flac_candidates(&records);
    let flacs_discovered = candidates.len();
    let oldest_processed = records.last().map(|record| record.created_at);

    let store_result = state
        .controller_features
        .mutate(move |features| {
            if reset {
                features.remove(HASHDB_BACKFILL_PROGRESS_KEY)?;
            }
            for (peer_id, path, size) in candidates {
                let key = hashdb_flac_inventory_key(&peer_id, &path, size);
                features.upsert(key, hashdb_flac_inventory_record(&peer_id, &path, size))?;
            }
            if let Some(oldest_processed) = oldest_processed {
                features.upsert(
                    HASHDB_BACKFILL_PROGRESS_KEY.to_owned(),
                    serde_json::json!({"lastProcessedAt": oldest_processed}),
                )?;
            }
            Ok::<(), String>(())
        })
        .await;
    if let Err(error) = store_result {
        return routing::service_unavailable_response(&error);
    }
    let progress = oldest_processed
        .map(|oldest_processed| serde_json::json!({"lastProcessedAt": oldest_processed}));
    if let Err(error) = write_hash_db_state_json(
        state,
        HASHDB_BACKFILL_PROGRESS_KEY,
        if reset && progress.is_none() {
            None
        } else {
            progress
        },
        &persistence_turn,
    )
    .await
    {
        return routing::internal_server_error_response(&error);
    }

    let new_remaining = remaining_searches.saturating_sub(searches_processed);
    let complete = new_remaining == 0;
    let message = if complete {
        format!(
            "Backfill complete! Processed {searches_processed} searches, discovered {flacs_discovered} FLACs."
        )
    } else {
        format!(
            "Processed {searches_processed} searches, discovered {flacs_discovered} FLACs. {new_remaining} searches remaining - click again to continue."
        )
    };
    routing::ok_response(
        serde_json::json!({
            "searchesProcessed": searches_processed,
            "flacsDiscovered": flacs_discovered,
            "totalSearches": total_searches,
            "remainingSearches": new_remaining,
            "complete": complete,
            "message": message,
        })
        .to_string(),
    )
}
