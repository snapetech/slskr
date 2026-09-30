use super::*;

pub(super) fn lidarr_map_import_path(path: &str, from_prefix: &str, to_prefix: &str) -> String {
    // Lidarr mappings may use POSIX virtual paths even when the daemon runs
    // on Windows; apply that mapping before host Path semantics rewrite them.
    let raw = path.replace('\\', "/");
    let from_virtual = from_prefix.trim_end_matches(['/', '\\']).replace('\\', "/");
    if !from_virtual.is_empty()
        && (raw == from_virtual
            || raw
                .strip_prefix(&from_virtual)
                .is_some_and(|suffix| suffix.starts_with('/')))
    {
        let relative = raw[from_virtual.len()..].trim_start_matches('/');
        let to = to_prefix.trim_end_matches(['/', '\\']);
        let windows_destination = to.contains('\\')
            || to
                .as_bytes()
                .get(1)
                .is_some_and(|separator| *separator == b':');
        return if relative.is_empty() {
            to.to_owned()
        } else if windows_destination {
            format!(r"{to}\{}", relative.replace('/', "\\"))
        } else {
            format!("{to}/{relative}")
        };
    }
    let absolute = if Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("/"))
            .join(path)
    };
    let normalized = absolute.to_string_lossy().to_string();
    if from_prefix.trim().is_empty() || to_prefix.trim().is_empty() {
        return normalized;
    }
    let from = from_prefix.trim_end_matches(['/', '\\']);
    let same = normalized == from;
    let child = normalized
        .strip_prefix(from)
        .is_some_and(|suffix| suffix.starts_with('/') || suffix.starts_with('\\'));
    if !same && !child {
        return normalized;
    }
    let relative = normalized[from.len()..].trim_start_matches(['/', '\\']);
    if to_prefix.contains('/') && !to_prefix.contains('\\') {
        let prefix = to_prefix.trim_end_matches(['/', '\\']);
        if relative.is_empty() {
            prefix.to_owned()
        } else {
            format!("{prefix}/{}", relative.replace('\\', "/"))
        }
    } else {
        Path::new(to_prefix)
            .join(relative)
            .to_string_lossy()
            .to_string()
    }
}

pub(super) fn lidarr_safe_import_candidate(candidate: &serde_json::Value) -> bool {
    let valid_rejections = candidate
        .get("rejections")
        .and_then(serde_json::Value::as_array)
        .is_none_or(|rejections| {
            rejections.iter().all(|rejection| {
                rejection
                    .get("reason")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_ascii_lowercase()
                    .contains("missing tracks")
            })
        });
    candidate
        .get("id")
        .and_then(serde_json::Value::as_i64)
        .is_some_and(|value| value > 0)
        && candidate
            .get("path")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| !value.trim().is_empty())
        && candidate
            .pointer("/artist/id")
            .and_then(serde_json::Value::as_i64)
            .is_some_and(|value| value > 0)
        && candidate
            .pointer("/album/id")
            .and_then(serde_json::Value::as_i64)
            .is_some_and(|value| value > 0)
        && candidate
            .get("albumReleaseId")
            .and_then(serde_json::Value::as_i64)
            .is_some_and(|value| value > 0)
        && candidate
            .get("tracks")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|value| !value.is_empty())
        && candidate
            .get("quality")
            .is_some_and(|value| !value.is_null())
        && !candidate
            .get("additionalFile")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
        && valid_rejections
}

pub(super) fn lidarr_portable_file_name(path: &str) -> String {
    let normalized = path.replace('\\', "/").trim_end_matches('/').to_owned();
    normalized.rsplit('/').next().unwrap_or_default().to_owned()
}

pub(super) async fn lidarr_already_owned_skip_reason(
    lidarr: &config::LidarrIntegrationSettings,
    directory: &str,
) -> Result<Option<String>, String> {
    let release_title = lidarr_portable_file_name(directory);
    if release_title.trim().is_empty() {
        return Ok(None);
    }
    let parsed = fetch_lidarr_json_get(
        lidarr,
        &format!("/api/v1/parse?title={}", url_encode(&release_title)),
        "release parse",
    )
    .await?;
    let artist_id = parsed
        .pointer("/artist/id")
        .and_then(serde_json::Value::as_i64)
        .filter(|value| *value > 0);
    let album_title = parsed
        .pointer("/parsedAlbumInfo/albumTitle")
        .or_else(|| parsed.pointer("/album/albumTitle"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let (Some(artist_id), Some(album_title)) = (artist_id, album_title) else {
        return Ok(None);
    };
    let albums = fetch_lidarr_json_get(
        lidarr,
        &format!("/api/v1/album?artistId={artist_id}"),
        "artist albums",
    )
    .await?;
    let albums = albums.as_array().cloned().unwrap_or_default();
    let owned = albums.into_iter().find(|album| {
        album
            .get("title")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|title| title.eq_ignore_ascii_case(album_title))
            && album
                .pointer("/statistics/totalTrackCount")
                .and_then(serde_json::Value::as_i64)
                .is_some_and(|total| {
                    total > 0
                        && album
                            .pointer("/statistics/trackFileCount")
                            .and_then(serde_json::Value::as_i64)
                            .is_some_and(|files| files >= total)
                })
    });
    Ok(owned.map(|_| format!("Already fully in Lidarr library ({album_title})")))
}

pub(super) fn lidarr_rejected_filenames(candidates: &[serde_json::Value]) -> Vec<String> {
    let mut seen = HashSet::new();
    candidates
        .iter()
        .filter(|candidate| !lidarr_safe_import_candidate(candidate))
        .filter_map(|candidate| candidate.get("path").and_then(serde_json::Value::as_str))
        .map(lidarr_portable_file_name)
        .filter(|filename| !filename.trim().is_empty())
        .filter(|filename| seen.insert(filename.to_lowercase()))
        .collect()
}

pub(super) fn delete_lidarr_rejected_files(
    directory: &str,
    filenames: &[String],
) -> Result<usize, String> {
    let directory = Path::new(directory);
    if !directory.is_dir() {
        return Ok(0);
    }
    let directory = fs::canonicalize(directory)
        .map_err(|error| format!("failed to resolve Lidarr download directory: {error}"))?;
    let mut deleted = 0;
    for filename in filenames {
        let path = directory.join(filename);
        if path.parent() != Some(directory.as_path()) || !path.is_file() {
            continue;
        }
        fs::remove_file(&path).map_err(|error| {
            format!(
                "failed to delete rejected Lidarr file {}: {error}",
                path.display()
            )
        })?;
        deleted += 1;
    }
    Ok(deleted)
}

pub(super) const LIDARR_IMPORT_HISTORY_PREFIX: &str = "integrations/lidarr/import-history/";

pub(super) fn lidarr_import_history_key(id: &str) -> String {
    format!("{LIDARR_IMPORT_HISTORY_PREFIX}{id}")
}

fn lidarr_import_history_status(result: &serde_json::Value) -> &'static str {
    if result
        .get("skippedReason")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|reason| !reason.trim().is_empty())
    {
        "Skipped"
    } else if result
        .get("commandId")
        .and_then(serde_json::Value::as_i64)
        .is_some_and(|command_id| command_id > 0)
    {
        "Successful"
    } else {
        "Skipped"
    }
}

pub(super) fn lidarr_import_history_record(
    id: &str,
    source_directory: &str,
    result: Option<&serde_json::Value>,
    error: Option<&str>,
    retry_of_id: Option<&str>,
) -> serde_json::Value {
    let now = chrono::Utc::now().to_rfc3339();
    let result = result.cloned().unwrap_or_default();
    let status = error
        .map(|_| "Failed")
        .unwrap_or_else(|| lidarr_import_history_status(&result));
    serde_json::json!({
        "id": id,
        "sourceDirectory": source_directory,
        "directory": result.get("directory").cloned().unwrap_or_else(|| serde_json::json!(source_directory)),
        "status": status,
        "errorMessage": error.unwrap_or_default(),
        "skippedReason": result.get("skippedReason").and_then(serde_json::Value::as_str).unwrap_or_default(),
        "candidateCount": result.get("candidateCount").and_then(serde_json::Value::as_u64).unwrap_or_default(),
        "safeCandidateCount": result.get("safeCandidateCount").and_then(serde_json::Value::as_u64).unwrap_or_default(),
        "rejectedCandidateCount": result.get("rejectedCandidateCount").and_then(serde_json::Value::as_u64).unwrap_or_default(),
        "commandId": result.get("commandId").cloned().unwrap_or(serde_json::Value::Null),
        "importMode": result.get("importMode").and_then(serde_json::Value::as_str).unwrap_or_default(),
        "startedAt": now,
        "completedAt": chrono::Utc::now().to_rfc3339(),
        "retryOfId": retry_of_id,
    })
}

pub(super) fn public_lidarr_import_history_record(
    mut record: serde_json::Value,
) -> serde_json::Value {
    if let Some(object) = record.as_object_mut() {
        object.remove("sourceDirectory");
    }
    record
}

pub(super) async fn persist_lidarr_import_history(
    state: &AppState,
    record: serde_json::Value,
) -> Result<(), String> {
    let id = record
        .get("id")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "Lidarr import history record has no id".to_owned())?;
    state
        .controller_features
        .upsert(lidarr_import_history_key(id), record)
        .await
}

pub(super) async fn list_lidarr_import_history(
    state: &AppState,
    limit: usize,
) -> Vec<serde_json::Value> {
    let mut records = state
        .controller_features
        .read()
        .await
        .values_with_prefix(LIDARR_IMPORT_HISTORY_PREFIX);
    records.sort_by(|left, right| {
        right
            .get("startedAt")
            .and_then(serde_json::Value::as_str)
            .cmp(&left.get("startedAt").and_then(serde_json::Value::as_str))
    });
    records
        .into_iter()
        .take(limit)
        .map(public_lidarr_import_history_record)
        .collect()
}

pub(super) async fn run_lidarr_import_with_history(
    state: &AppState,
    lidarr: &config::LidarrIntegrationSettings,
    directory: &str,
    retry_of_id: Option<&str>,
) -> Result<serde_json::Value, String> {
    run_lidarr_import_with_history_mode(state, lidarr, directory, retry_of_id, false).await
}

pub(super) async fn run_lidarr_automatic_import_with_history(
    state: &AppState,
    lidarr: &config::LidarrIntegrationSettings,
    directory: &str,
) -> Result<serde_json::Value, String> {
    run_lidarr_import_with_history_mode(state, lidarr, directory, None, true).await
}

async fn run_lidarr_import_with_history_mode(
    state: &AppState,
    lidarr: &config::LidarrIntegrationSettings,
    directory: &str,
    retry_of_id: Option<&str>,
    automatic: bool,
) -> Result<serde_json::Value, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let result = if automatic {
        import_lidarr_completed_directory_automatic(state, lidarr, directory).await
    } else {
        import_lidarr_completed_directory(state, lidarr, directory).await
    };
    match result {
        Ok(result) => {
            let record =
                lidarr_import_history_record(&id, directory, Some(&result), None, retry_of_id);
            persist_lidarr_import_history(state, record).await?;
            Ok(result)
        }
        Err(error) => {
            let record =
                lidarr_import_history_record(&id, directory, None, Some(&error), retry_of_id);
            persist_lidarr_import_history(state, record).await?;
            Err(error)
        }
    }
}

async fn import_lidarr_completed_directory_once(
    state: &AppState,
    lidarr: &config::LidarrIntegrationSettings,
    directory: &str,
    bypass_debounce: bool,
) -> Result<serde_json::Value, String> {
    let empty_result = |enabled: bool, auto_import_enabled: bool| {
        serde_json::json!({
            "enabled": enabled,
            "autoImportEnabled": auto_import_enabled,
            "directory": "",
            "candidateCount": 0,
            "safeCandidateCount": 0,
            "rejectedCandidateCount": 0,
            "rejectedFilenames": [],
            "commandId": 0,
            "importMode": "",
            "skippedReason": "",
        })
    };
    if !lidarr.enabled || !lidarr.auto_import_completed {
        return Ok(empty_result(lidarr.enabled, lidarr.auto_import_completed));
    }
    if directory.trim().is_empty() {
        let mut result = empty_result(true, true);
        result["skippedReason"] = serde_json::json!("Directory is empty");
        return Ok(result);
    }

    let mapped_directory =
        lidarr_map_import_path(directory, &lidarr.import_path_from, &lidarr.import_path_to);
    let now = unix_timestamp();
    if !bypass_debounce {
        let mut recent = state.lidarr_recent_imports.write().await;
        recent.retain(|_, processed_at| now.saturating_sub(*processed_at) <= 60 * 60);
        let previous = recent.insert(mapped_directory.clone(), now);
        if previous.is_some() {
            let mut result = empty_result(true, true);
            result["directory"] = serde_json::json!(mapped_directory);
            result["skippedReason"] = serde_json::json!("Recently processed");
            return Ok(result);
        }
    }

    if state.config.current_upstream_behavior
        && lidarr.skip_already_owned_albums
        && !lidarr.import_replace_existing_files
    {
        match lidarr_already_owned_skip_reason(lidarr, &mapped_directory).await {
            Ok(Some(reason)) => {
                let mut result = empty_result(true, true);
                result["directory"] = serde_json::json!(mapped_directory);
                result["skippedReason"] = serde_json::json!(reason);
                return Ok(result);
            }
            Ok(None) => {}
            Err(error) => {
                record_daemon_log(
                    state,
                    logging::LogLevel::Debug,
                    "lidarr",
                    format!("Lidarr ownership pre-check unavailable: {error}"),
                )
                .await;
            }
        }
    }

    let _permit = state
        .lidarr_import_gate
        .acquire()
        .await
        .map_err(|_| "Lidarr import gate is unavailable".to_owned())?;
    let candidates = fetch_lidarr_manual_import_candidates(lidarr, &mapped_directory).await?;
    let candidate_count = candidates.len();
    let rejected_filenames = lidarr_rejected_filenames(&candidates);
    let mut safe = candidates
        .into_iter()
        .filter(lidarr_safe_import_candidate)
        .collect::<Vec<_>>();
    for candidate in &mut safe {
        candidate["replaceExistingFiles"] = serde_json::json!(lidarr.import_replace_existing_files);
        if candidate
            .pointer("/artist/foreignArtistId")
            .and_then(serde_json::Value::as_str)
            .is_none()
        {
            candidate["artist"]["foreignArtistId"] = serde_json::Value::String(String::new());
        }
    }
    let safe_count = safe.len();
    let rejected_count = candidate_count.saturating_sub(safe_count);
    if safe.is_empty() {
        return Ok(serde_json::json!({
            "enabled": true,
            "autoImportEnabled": true,
            "directory": mapped_directory,
            "candidateCount": candidate_count,
            "safeCandidateCount": 0,
            "rejectedCandidateCount": rejected_count,
            "rejectedFilenames": rejected_filenames,
            "commandId": 0,
            "importMode": "",
            "skippedReason": if candidate_count == 0 {
                "Lidarr found no import candidates"
            } else {
                "Lidarr candidates had rejections or ambiguous matches"
            },
        }));
    }
    let import_mode = if lidarr.import_mode.eq_ignore_ascii_case("copy") {
        "Copy"
    } else {
        "Move"
    };
    let command_id = start_lidarr_manual_import(lidarr, safe, import_mode).await?;
    Ok(serde_json::json!({
        "enabled": true,
        "autoImportEnabled": true,
        "directory": mapped_directory,
        "candidateCount": candidate_count,
        "safeCandidateCount": safe_count,
        "rejectedCandidateCount": rejected_count,
        "rejectedFilenames": rejected_filenames,
        "commandId": command_id,
        "importMode": import_mode,
        "skippedReason": "",
    }))
}

pub(super) async fn import_lidarr_completed_directory(
    state: &AppState,
    lidarr: &config::LidarrIntegrationSettings,
    directory: &str,
) -> Result<serde_json::Value, String> {
    // The target applies the same per-directory debounce to manual and
    // automatic imports. Manual requests skip only the automatic delay/retry
    // wrapper; a repeated request still reports "Recently processed".
    import_lidarr_completed_directory_once(state, lidarr, directory, false).await
}

async fn import_lidarr_completed_directory_automatic(
    state: &AppState,
    lidarr: &config::LidarrIntegrationSettings,
    directory: &str,
) -> Result<serde_json::Value, String> {
    if lidarr.import_delay_seconds > 0 {
        time::sleep(std::time::Duration::from_secs(lidarr.import_delay_seconds)).await;
    }

    let max_attempts = lidarr.import_retry_max_attempts.saturating_add(1).max(1);
    let mut retry_delay = std::time::Duration::from_secs(lidarr.import_retry_delay_seconds);
    let mut last_error = None;
    for attempt in 0..max_attempts {
        // The first attempt participates in the completion debounce. Retries
        // are already part of that same history item and must not turn into a
        // false "recently processed" skip.
        match import_lidarr_completed_directory_once(state, lidarr, directory, attempt > 0).await {
            Ok(result) => return Ok(result),
            Err(error) => {
                last_error = Some(error);
                if attempt + 1 < max_attempts {
                    time::sleep(retry_delay).await;
                    retry_delay = retry_delay.saturating_mul(2);
                }
            }
        }
    }
    Err(last_error.unwrap_or_else(|| "Lidarr import retry loop exhausted".to_owned()))
}
