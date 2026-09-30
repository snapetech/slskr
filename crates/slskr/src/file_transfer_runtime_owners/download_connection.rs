use super::*;

pub(super) async fn download_file_transfer(
    state: &AppState,
    address: &PeerAddress,
    transfer: &TransferEntry,
) -> Result<(u64, u64), String> {
    let mut connection = connect_file_transfer_preferred(state, address).await?;
    download_file_transfer_with_connection(state, transfer, &mut connection).await
}

pub(super) async fn download_file_transfer_with_retry(
    state: &AppState,
    address: &PeerAddress,
    transfer: &TransferEntry,
) -> Result<(u64, u64), String> {
    if let Some(reason) = cancel_download_if_blocked_by_policy(state, transfer).await {
        return Err(reason);
    }
    if transfer_is_cancelled(state, transfer.id).await {
        return Err("transfer cancelled".to_owned());
    }
    let retry = state.transfer_download_settings.read().await.retry.clone();
    let mut last_error = None;
    for attempt in 0..retry.attempts.max(1) {
        if attempt > 0 {
            let attempt_number = attempt.saturating_add(1);
            let delay = download_retry_delay(&retry, attempt);
            let next_attempt_at = unix_timestamp().saturating_add(delay.as_secs());
            let scheduled = {
                let mut transfers = state.transfers.write().await;
                transfers.update_retry_state(
                    transfer.id,
                    attempt_number,
                    Some(next_attempt_at),
                    "queued",
                    Some(format!("retry scheduled for attempt {attempt_number}")),
                )
            };
            if let Some(scheduled) = scheduled {
                persist_transfer_projection(state, &scheduled).await;
            }
            time::sleep(delay).await;
        }
        if transfer_is_cancelled(state, transfer.id).await {
            return Err("transfer cancelled".to_owned());
        }
        if attempt > 0 {
            let attempt_number = attempt.saturating_add(1);
            let started = {
                let mut transfers = state.transfers.write().await;
                transfers.update_retry_state(transfer.id, attempt_number, None, "in_progress", None)
            };
            if let Some(started) = started {
                persist_transfer_projection(state, &started).await;
            }
        }
        match download_file_transfer(state, address, transfer).await {
            Ok(completed) => return Ok(completed),
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error.unwrap_or_else(|| "download retry attempts exhausted".to_owned()))
}

pub(crate) fn download_retry_delay(
    retry: &crate::config::TransferDownloadRetrySettings,
    attempt: u32,
) -> Duration {
    let multiplier = 1_u32
        .checked_shl(attempt.saturating_sub(1))
        .unwrap_or(u32::MAX);
    retry
        .delay
        .checked_mul(multiplier)
        .unwrap_or(retry.max_delay)
        .min(retry.max_delay)
}

pub(crate) fn prepare_incomplete_download_file(
    root: &Path,
    path: &Path,
    strategy: &str,
    size: u64,
) -> Result<(fs::File, u64), String> {
    let file = open_download_file(root, path)?;
    let file = if strategy.eq_ignore_ascii_case("overwrite") {
        #[cfg(not(unix))]
        {
            drop(file);
            fs::OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(path)
                .map_err(|error| format!("download partial overwrite failed: {error}"))?
        }
        #[cfg(unix)]
        {
            file.set_len(0)
                .map_err(|error| format!("download partial overwrite failed: {error}"))?;
            file
        }
    } else {
        file
    };
    let metadata = file
        .metadata()
        .map_err(|error| format!("download file metadata failed: {error}"))?;
    if !metadata.is_file() {
        return Err("download path is not a regular file".to_owned());
    }
    let offset = metadata.len();
    if offset > size {
        return Err(format!(
            "local resume offset {offset} exceeds transfer size {size}"
        ));
    }
    Ok((file, offset))
}

pub(crate) async fn download_file_transfer_with_connection(
    state: &AppState,
    transfer: &TransferEntry,
    connection: &mut slskr_client::file_transfer::FileTransferConnection<TcpStream>,
) -> Result<(u64, u64), String> {
    let local_path = transfer
        .local_path
        .as_deref()
        .ok_or_else(|| "local path is required".to_owned())?;
    let size = transfer
        .size
        .ok_or_else(|| "download size is required before file transfer".to_owned())?;
    validate_configured_path_policy(state, local_path).await?;
    let downloads_dir = effective_downloads_dir(state);
    let final_path = ensure_scoped_download_path(&downloads_dir, local_path)?;
    let incomplete_dir = effective_incomplete_dir(state);
    let incomplete_path = safe_download_path(
        &incomplete_dir,
        &format!("{}-{}.part", transfer.id, transfer.token),
    )?;
    let incomplete_path =
        ensure_scoped_download_path(&incomplete_dir, incomplete_path.to_string_lossy().as_ref())?;
    let overwrite_destination = state.config.controller_profile == ControllerProfile::Legacy
        && state
            .transfer_download_settings
            .read()
            .await
            .destination
            .exists
            .eq_ignore_ascii_case("overwrite");
    if !incomplete_path.exists()
        && final_path.exists()
        && !overwrite_destination
        && transfer.bytes_transferred > 0
    {
        fs::rename(&final_path, &incomplete_path)
            .map_err(|error| format!("download partial migration failed: {error}"))?;
    }
    let retry_strategy = state
        .transfer_download_settings
        .read()
        .await
        .retry
        .incomplete
        .clone();
    let (mut file, offset) =
        prepare_incomplete_download_file(&incomplete_dir, &incomplete_path, &retry_strategy, size)?;
    let remaining = usize::try_from(size - offset)
        .map_err(|_| "download remaining size is too large".to_owned())?;
    let bytes_received =
        download_file_with_progress(state, transfer, connection, offset, remaining, &mut file)
            .await?;
    file.sync_all()
        .map_err(|error| format!("download file sync failed: {error}"))?;
    drop(file);
    enforce_completed_download_content_safety(state, &incomplete_path, &final_path).await?;
    let relative_final = final_path
        .strip_prefix(&downloads_dir)
        .map_err(|_| "download destination is outside the download root".to_owned())?;
    let completed_path =
        configured_download_destination_path(state, relative_final.to_string_lossy().as_ref())
            .await?;
    if completed_path.exists() && overwrite_destination {
        fs::remove_file(&completed_path)
            .map_err(|error| format!("download destination overwrite failed: {error}"))?;
    }
    match fs::rename(&incomplete_path, &completed_path) {
        Ok(()) => {}
        Err(rename_error) => {
            fs::copy(&incomplete_path, &completed_path).map_err(|copy_error| {
                format!(
                    "download completion move failed: {rename_error}; copy failed: {copy_error}"
                )
            })?;
            fs::remove_file(&incomplete_path)
                .map_err(|error| format!("download incomplete cleanup failed: {error}"))?;
        }
    }
    if completed_path != final_path {
        {
            let mut transfers = state.transfers.write().await;
            transfers.update_local_path(transfer.id, completed_path.display().to_string());
        }
        persist_transfer_durability(state).await;
    }
    Ok((offset + bytes_received, size))
}
