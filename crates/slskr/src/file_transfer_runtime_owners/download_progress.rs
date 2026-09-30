use super::*;

pub(super) async fn download_file_with_progress(
    state: &AppState,
    transfer: &TransferEntry,
    connection: &mut slskr_client::file_transfer::FileTransferConnection<TcpStream>,
    offset: u64,
    remaining: usize,
    file: &mut fs::File,
) -> Result<u64, String> {
    let token = time::timeout(
        state.config.soulseek_connection.timeout_transfer,
        connection.receive_token(),
    )
    .await
    .map_err(|_| "file download token receive timed out".to_owned())?
    .map_err(|error| format!("file download token receive failed: {error}"))?;
    if token != transfer.token {
        return Err(format!(
            "transfer token mismatch: expected {}, received {token}",
            transfer.token
        ));
    }
    time::timeout(
        state.config.soulseek_connection.timeout_transfer,
        connection.send_offset(offset),
    )
    .await
    .map_err(|_| "file download offset send timed out".to_owned())?
    .map_err(|error| format!("file download offset send failed: {error}"))?;

    let mut bytes_received = 0_usize;
    let pacing_started = Instant::now();
    while bytes_received < remaining {
        if transfer_is_cancelled(state, transfer.id).await {
            return Err("transfer cancelled".to_owned());
        }
        let next_len =
            (remaining - bytes_received).min(state.config.soulseek_connection.buffer_transfer);
        let chunk = time::timeout(
            state.config.soulseek_connection.timeout_transfer,
            connection.read_chunk(next_len),
        )
        .await
        .map_err(|_| "file download chunk receive timed out".to_owned())?
        .map_err(|error| format!("file download chunk receive failed: {error}"))?;
        bytes_received += chunk.len();
        let transferred = offset.saturating_add(u64::try_from(bytes_received).unwrap_or(u64::MAX));
        write_download_chunk_if_active(state, transfer.id, file, &chunk, transferred).await?;
        let speed_limit_kib = effective_download_pacing_limit(state).await;
        if speed_limit_kib < i32::MAX as u32 {
            let bytes_per_second = u64::from(speed_limit_kib).saturating_mul(1024).max(1);
            let expected_nanos = u128::from(bytes_received as u64)
                .saturating_mul(1_000_000_000)
                .checked_div(u128::from(bytes_per_second))
                .unwrap_or(u128::MAX)
                .min(u128::from(u64::MAX));
            let expected = Duration::from_nanos(expected_nanos as u64);
            let elapsed = pacing_started.elapsed();
            if expected > elapsed {
                time::sleep(expected - elapsed).await;
            }
        }
    }
    Ok(u64::try_from(bytes_received).unwrap_or(u64::MAX))
}

pub(crate) async fn effective_download_pacing_limit(state: &AppState) -> u32 {
    let configured = state
        .transfer_download_settings
        .read()
        .await
        .speed_limit_kib;
    if configured == i32::MAX as u32 {
        return configured;
    }
    let active = state
        .transfers
        .read()
        .await
        .entries
        .iter()
        .filter(|entry| entry.direction == 0 && is_active_transfer_status(&entry.status))
        .count()
        .max(1);
    configured
        .checked_div(u32::try_from(active).unwrap_or(u32::MAX))
        .unwrap_or(0)
        .max(1)
}

pub(crate) async fn write_download_chunk_if_active(
    state: &AppState,
    transfer_id: u64,
    file: &mut fs::File,
    chunk: &[u8],
    bytes_transferred: u64,
) -> Result<(), String> {
    use std::io::Write;

    let mut transfers = state.transfers.write().await;
    if transfers
        .entries
        .iter()
        .find(|entry| entry.id == transfer_id)
        .is_none_or(|entry| entry.status != "in_progress")
    {
        return Err("transfer cancelled".to_owned());
    }
    file.write_all(chunk)
        .map_err(|error| format!("download file write failed: {error}"))?;
    let updated = transfers.update_progress(transfer_id, bytes_transferred);
    let should_persist = updated
        .as_ref()
        .is_some_and(|entry| transfers.should_persist_progress(entry));
    drop(transfers);
    if let Some(entry) = updated {
        if should_persist {
            persist_transfer_progress_projection(state, &entry).await;
        } else {
            persist_transfer_durability(state).await;
            publish_transfer_hub_event(state, "progress", &entry);
        }
    }
    Ok(())
}

pub(crate) async fn update_transfer_progress(
    state: &AppState,
    transfer_id: u64,
    bytes_transferred: u64,
) {
    let mut transfers = state.transfers.write().await;
    if transfers
        .entries
        .iter()
        .find(|entry| entry.id == transfer_id)
        .is_some_and(|entry| entry.status == "cancelled")
    {
        return;
    }
    let updated = transfers.update_progress(transfer_id, bytes_transferred);
    let should_persist = updated
        .as_ref()
        .is_some_and(|entry| transfers.should_persist_progress(entry));
    drop(transfers);
    if let Some(entry) = updated {
        if should_persist {
            persist_transfer_progress_projection(state, &entry).await;
        } else {
            persist_transfer_durability(state).await;
            publish_transfer_hub_event(state, "progress", &entry);
        }
    }
}

pub(super) async fn persist_transfer_progress_projection(state: &AppState, entry: &TransferEntry) {
    persist_transfer_durability(state).await;
    if let Some(db) = state.db.as_ref() {
        let persistence_error = db
            .update_transfer_progress(
                &entry.id.to_string(),
                entry.bytes_transferred,
                entry.updated_at_ms,
            )
            .await
            .err()
            .map(|error| error.to_string());
        if let Some(error) = persistence_error {
            update_session(state, |snapshot| {
                snapshot.last_error = Some(format!(
                    "transfer {} progress persistence failed: {error}",
                    entry.id
                ));
            })
            .await;
        }
    }
    publish_transfer_hub_event(state, "progress", entry);
}

pub(crate) async fn transfer_is_cancelled(state: &AppState, transfer_id: u64) -> bool {
    state
        .transfers
        .read()
        .await
        .entries
        .iter()
        .find(|entry| entry.id == transfer_id)
        .is_some_and(|entry| entry.status == "cancelled")
}
