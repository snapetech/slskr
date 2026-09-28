use super::*;

pub(super) async fn upload_file_transfer(
    state: &AppState,
    address: &PeerAddress,
    transfer: &TransferEntry,
) -> Result<(u64, u64), String> {
    let mut connection = connect_file_transfer_preferred(state, address).await?;
    upload_file_transfer_with_connection(state, transfer, &mut connection, true).await
}

pub(crate) async fn upload_file_transfer_with_connection(
    state: &AppState,
    transfer: &TransferEntry,
    connection: &mut slskr_client::file_transfer::FileTransferConnection<TcpStream>,
    send_token: bool,
) -> Result<(u64, u64), String> {
    let local_path = transfer
        .local_path
        .as_deref()
        .ok_or_else(|| "local path is required".to_owned())?;
    let shared_file = find_shared_local_file(state, &transfer.filename)
        .await
        .ok_or_else(|| "upload filename is not available from local shares".to_owned())?;
    if Path::new(local_path) != shared_file.local_path {
        return Err("upload local path does not match the share index".to_owned());
    }
    let mut file = open_shared_local_file(state, &shared_file.local_path).await?;
    let metadata = file
        .metadata()
        .map_err(|error| format!("local file metadata failed: {error}"))?;
    if !metadata.is_file() {
        return Err("local path is not a file".to_owned());
    }
    if let Some(expected_size) = transfer.size.or(Some(shared_file.size)) {
        if metadata.len() != expected_size {
            return Err(format!(
                "local file size {} does not match expected transfer size {expected_size}",
                metadata.len()
            ));
        }
    }
    let size = metadata.len();
    let bytes_transferred =
        upload_file_with_progress(state, transfer, connection, &mut file, size, send_token).await?;
    Ok((bytes_transferred, size))
}

pub(super) async fn upload_file_with_progress(
    state: &AppState,
    transfer: &TransferEntry,
    connection: &mut slskr_client::file_transfer::FileTransferConnection<TcpStream>,
    file: &mut fs::File,
    size: u64,
    send_token: bool,
) -> Result<u64, String> {
    use std::io::{Read, Seek, SeekFrom};

    if send_token {
        time::timeout(
            state.config.soulseek_connection.timeout_transfer,
            connection.send_token(transfer.token),
        )
        .await
        .map_err(|_| "file upload token send timed out".to_owned())?
        .map_err(|error| format!("file upload token send failed: {error}"))?;
    }
    let offset = time::timeout(
        state.config.soulseek_connection.timeout_transfer,
        connection.receive_offset(),
    )
    .await
    .map_err(|_| "file upload offset receive timed out".to_owned())?
    .map_err(|error| format!("file upload offset receive failed: {error}"))?;
    let start = usize::try_from(offset)
        .map_err(|_| format!("transfer offset {offset} exceeds local file size {size}"))?;
    if u64::try_from(start).unwrap_or(u64::MAX) > size {
        return Err(format!(
            "transfer offset {offset} exceeds local file size {size}"
        ));
    }
    file.seek(SeekFrom::Start(offset))
        .map_err(|error| format!("local file seek failed: {error}"))?;

    // The peer's offset is already present on the remote side.  Report the
    // complete remote position to the transfer projection, while pacing only
    // the bytes sent by this attempt.
    let mut sent = offset;
    update_transfer_progress(state, transfer.id, sent).await;
    let pacing_started = Instant::now();
    let buffer_len = state
        .config
        .soulseek_connection
        .buffer_transfer
        .min(connection.max_write_chunk_len());
    let mut buffer = vec![0_u8; buffer_len];
    loop {
        if transfer_is_cancelled(state, transfer.id).await {
            return Err("transfer cancelled".to_owned());
        }
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("local file read failed: {error}"))?;
        if read == 0 {
            break;
        }
        let chunk = &buffer[..read];
        time::timeout(
            state.config.soulseek_connection.timeout_transfer,
            connection.write_chunk(chunk),
        )
        .await
        .map_err(|_| "file upload chunk send timed out".to_owned())?
        .map_err(|error| format!("file upload chunk send failed: {error}"))?;
        sent = sent.saturating_add(u64::try_from(chunk.len()).unwrap_or(u64::MAX));
        update_transfer_progress(state, transfer.id, sent).await;
        let speed_limit_kib = effective_upload_pacing_limit(
            state,
            transfer.peer_username.as_deref().unwrap_or_default(),
        )
        .await;
        if speed_limit_kib < i32::MAX as u32 {
            let bytes_per_second = u64::from(speed_limit_kib).saturating_mul(1024).max(1);
            let expected_nanos = u128::from(sent.saturating_sub(offset))
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
    Ok(sent)
}

pub(crate) async fn effective_upload_speed_limit(state: &AppState, username: &str) -> u32 {
    let global = state.transfer_upload_settings.read().await.speed_limit_kib;
    if username.is_empty() {
        return global;
    }
    let group_name = effective_transfer_group(state, username).await;
    if group_name == "privileged" {
        return global;
    }
    let groups = state.transfer_groups_settings.read().await;
    transfer_group_upload_settings(&groups, &group_name)
        .map_or(global, |group| global.min(group.speed_limit_kib))
}

pub(super) async fn effective_upload_pacing_limit(state: &AppState, username: &str) -> u32 {
    let configured = effective_upload_speed_limit(state, username).await;
    let global = state.transfer_upload_settings.read().await.speed_limit_kib;
    if configured == i32::MAX as u32 && global == i32::MAX as u32 {
        return configured;
    }
    let group_name = effective_transfer_group(state, username).await;
    let groups = state.transfer_groups_settings.read().await;
    let users = state.users.read().await;
    let transfers = state.transfers.read().await;
    let active = transfers
        .entries
        .iter()
        .filter(|entry| entry.direction == 1 && is_active_transfer_status(&entry.status))
        .collect::<Vec<_>>();
    let global_share = if global == i32::MAX as u32 {
        global
    } else {
        global
            .checked_div(u32::try_from(active.len().max(1)).unwrap_or(u32::MAX))
            .unwrap_or(0)
            .max(1)
    };
    let group_active = active
        .iter()
        .filter(|entry| {
            entry.peer_username.as_deref().is_some_and(|peer| {
                effective_transfer_group_from(&groups, &users, peer) == group_name
            })
        })
        .count();
    let group_share = if configured == i32::MAX as u32 {
        configured
    } else {
        configured
            .checked_div(u32::try_from(group_active.max(1)).unwrap_or(u32::MAX))
            .unwrap_or(0)
            .max(1)
    };
    global_share.min(group_share)
}
