use super::*;

struct ShareScanCancellationGuard {
    cancellation: Arc<AtomicBool>,
}

impl Drop for ShareScanCancellationGuard {
    fn drop(&mut self) {
        self.cancellation.store(true, Ordering::Release);
    }
}

fn register_share_scan_cancellation(state: &AppState) -> ShareScanCancellationGuard {
    let cancellation = Arc::new(AtomicBool::new(false));
    *state
        .share_scan_cancellation
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(Arc::clone(&cancellation));
    ShareScanCancellationGuard { cancellation }
}

pub(super) fn cancel_active_share_scan(state: &AppState) {
    if let Some(cancellation) = state
        .share_scan_cancellation
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .as_ref()
    {
        cancellation.store(true, Ordering::Release);
    }
}

pub(super) async fn rebuild_share_index(state: &AppState) -> Result<ShareIndexSnapshot, String> {
    let scan_permit = Arc::clone(&state.share_scans)
        .try_acquire_owned()
        .map_err(|_| SHARE_SCAN_BUSY_ERROR.to_owned())?;
    rebuild_share_index_with_permit(state, scan_permit).await
}

pub(super) async fn add_runtime_share(
    state: &AppState,
    path: &str,
    alias: Option<&str>,
) -> Result<(crate::config::ShareDirectory, ShareIndexSnapshot), String> {
    let directory = crate::config::ShareDirectory::from_path(path, alias)?;
    let (previous_directories, mutated_directories) = {
        let _persistence_turn = state.share_index_persistence_lock.lock().await;
        let mut settings = state.share_settings.write().await;
        if settings
            .directories
            .iter()
            .any(|existing| existing.local_path == directory.local_path)
        {
            return Err("share path is already configured".to_owned());
        }
        if settings
            .directories
            .iter()
            .any(|existing| existing.alias == directory.alias)
        {
            return Err(format!(
                "share alias '{}' is already configured",
                directory.alias
            ));
        }
        let previous_directories = settings.directories.clone();
        settings.directories.push(directory.clone());
        settings.roots = settings
            .directories
            .iter()
            .filter(|directory| !directory.is_excluded)
            .map(|directory| directory.local_path.clone())
            .collect();
        state
            .share_settings_generation
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        (previous_directories, settings.directories.clone())
    };

    match rebuild_share_index(state).await {
        Ok(snapshot) => Ok((directory, snapshot)),
        Err(error) => {
            let _persistence_turn = state.share_index_persistence_lock.lock().await;
            let mut settings = state.share_settings.write().await;
            if settings.directories == mutated_directories {
                settings.directories = previous_directories;
                settings.roots = settings
                    .directories
                    .iter()
                    .filter(|directory| !directory.is_excluded)
                    .map(|directory| directory.local_path.clone())
                    .collect();
                state
                    .share_settings_generation
                    .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
            }
            Err(error)
        }
    }
}

pub(super) async fn rebuild_share_index_with_permit(
    state: &AppState,
    _scan_permit: OwnedSemaphorePermit,
) -> Result<ShareIndexSnapshot, String> {
    let cancellation_guard = register_share_scan_cancellation(state);
    {
        let mut lifecycle = state.share_lifecycle.write().await;
        lifecycle.scanning = true;
        lifecycle.faulted = false;
        lifecycle.cancelled = false;
        lifecycle.scan_progress = 0.0;
    }
    let (share_settings, share_settings_generation, filter_case_sensitive) = {
        let _persistence_turn = state.share_index_persistence_lock.lock().await;
        (
            state.share_settings.read().await.clone(),
            state
                .share_settings_generation
                .load(std::sync::atomic::Ordering::Acquire),
            *state
                .controller_case_sensitive_regex
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        )
    };
    let mut config = state.config.clone();
    config.share_settings = share_settings;
    config.controller_case_sensitive_regex = filter_case_sensitive;
    let cancellation = Arc::clone(&cancellation_guard.cancellation);
    let snapshot = tokio::task::spawn_blocking(move || {
        build_share_index_with_cancellation(&config, cancellation)
    })
    .await
    .map_err(|_| SHARE_SCAN_WORKER_ERROR.to_owned());
    let snapshot = match snapshot {
        Ok(Ok(snapshot)) => snapshot,
        Ok(Err(error)) if error == SHARE_SCAN_CANCELLED_ERROR => {
            let mut lifecycle = state.share_lifecycle.write().await;
            lifecycle.scanning = false;
            lifecycle.cancelled = true;
            lifecycle.faulted = false;
            return Err(error);
        }
        Ok(Err(error)) => {
            let mut lifecycle = state.share_lifecycle.write().await;
            lifecycle.scanning = false;
            lifecycle.faulted = true;
            return Err(error);
        }
        Err(error) => {
            let mut lifecycle = state.share_lifecycle.write().await;
            lifecycle.scanning = false;
            lifecycle.faulted = true;
            return Err(error);
        }
    };
    commit_share_index_snapshot_checked(state, &snapshot, share_settings_generation).await?;
    Ok(snapshot)
}

pub(super) fn share_rebuild_error_response(error: &str) -> HttpResponse {
    if error == SHARE_SCAN_BUSY_ERROR {
        routing::service_unavailable_response(SHARE_SCAN_BUSY_ERROR)
    } else {
        routing::service_unavailable_response("share index unavailable")
    }
}

async fn persist_share_index_checked(
    state: &AppState,
    snapshot: &ShareIndexSnapshot,
) -> Result<bool, String> {
    let Some(db) = state.db.as_ref() else {
        return Ok(false);
    };
    let records = persisted_share_file_records(snapshot);
    db.replace_share_files(&records)
        .await
        .map_err(|error| format!("share index persistence failed: {error}"))?;
    Ok(true)
}

pub(super) async fn commit_share_index_snapshot_checked(
    state: &AppState,
    snapshot: &ShareIndexSnapshot,
    scanned_settings_generation: u64,
) -> Result<(), String> {
    let _persistence_turn = state.share_index_persistence_lock.lock().await;
    if state
        .share_settings_generation
        .load(std::sync::atomic::Ordering::Acquire)
        != scanned_settings_generation
    {
        let mut lifecycle = state.share_lifecycle.write().await;
        lifecycle.scanning = false;
        lifecycle.cancelled = true;
        lifecycle.faulted = false;
        lifecycle.scan_pending = true;
        return Err(SHARE_SCAN_CANCELLED_ERROR.to_owned());
    }
    if let Err(error) = persist_share_index_checked(state, snapshot).await {
        let mut lifecycle = state.share_lifecycle.write().await;
        lifecycle.scanning = false;
        lifecycle.faulted = true;
        return Err(error);
    }
    *state.shares.write().await = snapshot.clone();
    *state.share_lifecycle.write().await = ShareLifecycleState::from_snapshot(snapshot);
    Ok(())
}
