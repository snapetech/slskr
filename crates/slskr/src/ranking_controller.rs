use super::*;

pub(super) fn ranking_bad_request_response(message: &str) -> HttpResponse {
    HttpResponse {
        status: "400 Bad Request",
        content_type: "text/plain; charset=utf-8",
        body: message.to_owned(),
    }
}

pub(super) async fn ranking_storage_failure_response(state: &AppState) -> Option<HttpResponse> {
    let db = state.db.as_ref()?;
    if db.get_traffic_totals().await.is_err() {
        Some(routing::internal_server_error_response(
            "ranking storage unavailable",
        ))
    } else {
        None
    }
}

pub(super) fn ranking_history_counts(transfers: &TransferQueue, username: &str) -> (usize, usize) {
    let mut successes = 0;
    let mut failures = 0;
    for entry in transfers
        .entries
        .iter()
        .filter(|entry| entry.direction == 0 && entry.peer_username.as_deref() == Some(username))
    {
        match controller_transfer_state(&entry.status) {
            "Completed" => successes += 1,
            "Failed" => failures += 1,
            _ => {}
        }
    }
    (successes, failures)
}

pub(super) fn ranking_history_json(
    username: &str,
    successes: usize,
    failures: usize,
) -> serde_json::Value {
    let total = successes + failures;
    serde_json::json!({
        "username": username,
        "successes": successes,
        "failures": failures,
        "successRate": if total == 0 { 0.5 } else { successes as f64 / total as f64 },
    })
}

async fn ranking_history_mutation_response(body: &str, state: &AppState) -> HttpResponse {
    if let Some(response) = ranking_storage_failure_response(state).await {
        return response;
    }
    let payload = match serde_json::from_str::<serde_json::Value>(body) {
        Ok(payload) => payload,
        Err(_) => return ranking_bad_request_response("At least one username is required"),
    };
    let Some(values) = payload.as_array() else {
        return ranking_bad_request_response("At least one username is required");
    };
    if values.is_empty() {
        return ranking_bad_request_response("At least one username is required");
    }
    if values.len() > MAX_RANKING_BATCH_ITEMS {
        return ranking_bad_request_response("at most 1000 usernames may be requested");
    }
    let mut usernames = Vec::new();
    let mut seen = HashSet::new();
    for value in values {
        let Some(username) = value
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        if seen.insert(username.to_owned()) {
            usernames.push(username.to_owned());
        }
    }
    if usernames.is_empty() {
        return ranking_bad_request_response("Each username must be non-empty");
    }
    let transfers = state.transfers.read().await;
    let histories = usernames
        .iter()
        .map(|username| {
            let (successes, failures) = ranking_history_counts(&transfers, username);
            (
                username.clone(),
                ranking_history_json(username, successes, failures),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    routing::ok_response(serde_json::Value::Object(histories).to_string())
}

fn ranking_candidate_value(value: &serde_json::Value) -> Result<serde_json::Value, ()> {
    let Some(object) = value.as_object() else {
        return Err(());
    };
    let username = object
        .get("username")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_owned();
    let filename = object
        .get("filename")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_owned();
    let size = object
        .get("size")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or_default();
    let has_free_upload_slot = object
        .get("hasFreeUploadSlot")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let queue_length = object
        .get("queueLength")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or_default();
    let upload_speed = object
        .get("uploadSpeed")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or_default();
    let size_diff_percent = object
        .get("sizeDiffPercent")
        .filter(|value| !value.is_null())
        .and_then(serde_json::Value::as_f64);
    let mut candidate = serde_json::Map::from_iter([
        ("username".to_owned(), serde_json::json!(username)),
        ("filename".to_owned(), serde_json::json!(filename)),
        ("size".to_owned(), serde_json::json!(size)),
        (
            "hasFreeUploadSlot".to_owned(),
            serde_json::json!(has_free_upload_slot),
        ),
        ("queueLength".to_owned(), serde_json::json!(queue_length)),
        ("uploadSpeed".to_owned(), serde_json::json!(upload_speed)),
    ]);
    if let Some(size_diff_percent) = size_diff_percent {
        candidate.insert(
            "sizeDiffPercent".to_owned(),
            serde_json::json!(size_diff_percent),
        );
    }
    for field in ["bitRate", "sampleRate", "bitDepth", "length"] {
        if let Some(value) = object.get(field).filter(|value| !value.is_null()) {
            candidate.insert(field.to_owned(), value.clone());
        }
    }
    Ok(serde_json::Value::Object(candidate))
}

async fn ranking_sources_mutation_response(body: &str, state: &AppState) -> HttpResponse {
    if let Some(response) = ranking_storage_failure_response(state).await {
        return response;
    }
    let payload = match serde_json::from_str::<serde_json::Value>(body) {
        Ok(payload) => payload,
        Err(_) => return ranking_bad_request_response("At least one source candidate is required"),
    };
    let Some(values) = payload.as_array() else {
        return ranking_bad_request_response("At least one source candidate is required");
    };
    if values.is_empty() {
        return ranking_bad_request_response("At least one source candidate is required");
    }
    if values.len() > MAX_RANKING_BATCH_ITEMS {
        return ranking_bad_request_response("at most 1000 source candidates may be ranked");
    }
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();
    for value in values {
        if value.is_null() {
            continue;
        }
        let candidate = match ranking_candidate_value(value) {
            Ok(candidate) => candidate,
            Err(()) => {
                return ranking_bad_request_response("At least one source candidate is required")
            }
        };
        let username = candidate["username"].as_str().unwrap_or_default();
        let filename = candidate["filename"].as_str().unwrap_or_default();
        let size = candidate["size"].as_i64().unwrap_or_default();
        if seen.insert((username.to_owned(), filename.to_owned(), size)) {
            candidates.push(candidate);
        }
    }
    if candidates.is_empty() {
        return ranking_bad_request_response("At least one source candidate is required");
    }
    if candidates.iter().any(|candidate| {
        candidate["username"]
            .as_str()
            .unwrap_or_default()
            .is_empty()
            || candidate["filename"]
                .as_str()
                .unwrap_or_default()
                .is_empty()
    }) {
        return ranking_bad_request_response(
            "Each source candidate requires a non-empty username and filename",
        );
    }
    let transfers = state.transfers.read().await;
    let mut ranked = candidates
        .into_iter()
        .map(|candidate| {
            let username = candidate["username"].as_str().unwrap_or_default();
            let free_slot = candidate["hasFreeUploadSlot"].as_bool().unwrap_or(false);
            let queue_length = candidate["queueLength"].as_i64().unwrap_or_default() as f64;
            let upload_speed = candidate["uploadSpeed"].as_i64().unwrap_or_default() as f64;
            let speed_score = (upload_speed / 10_000_000.0 * 40.0).min(40.0);
            let queue_score = (30.0 * (1.0 - queue_length / 100.0)).max(0.0);
            let free_slot_score = if free_slot { 15.0 } else { 0.0 };
            let (successes, failures) = ranking_history_counts(&transfers, username);
            let history_score = if successes + failures > 0 {
                ((successes as f64 / (successes + failures) as f64) - 0.5) * 2.0 * 15.0
            } else {
                0.0
            };
            let size_match_score = candidate["sizeDiffPercent"]
                .as_f64()
                .map(|value| (20.0 * (1.0 - value / 10.0)).max(0.0))
                .unwrap_or(0.0);
            let smart_score =
                speed_score + queue_score + free_slot_score + history_score + size_match_score;
            let mut ranked = match candidate {
                serde_json::Value::Object(candidate) => candidate,
                _ => unreachable!("ranking candidate is always an object"),
            };
            ranked.insert("smartScore".to_owned(), serde_json::json!(smart_score));
            ranked.insert("speedScore".to_owned(), serde_json::json!(speed_score));
            ranked.insert("queueScore".to_owned(), serde_json::json!(queue_score));
            ranked.insert(
                "freeSlotScore".to_owned(),
                serde_json::json!(free_slot_score),
            );
            ranked.insert("historyScore".to_owned(), serde_json::json!(history_score));
            ranked.insert(
                "sizeMatchScore".to_owned(),
                serde_json::json!(size_match_score),
            );
            serde_json::Value::Object(ranked)
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        right["smartScore"]
            .as_f64()
            .partial_cmp(&left["smartScore"].as_f64())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    routing::ok_response(serde_json::Value::Array(ranked).to_string())
}

pub(super) async fn ranking_mutation_response(
    path: &str,
    body: &str,
    state: &AppState,
) -> HttpResponse {
    if path.ends_with("/history") {
        ranking_history_mutation_response(body, state).await
    } else {
        ranking_sources_mutation_response(body, state).await
    }
}
