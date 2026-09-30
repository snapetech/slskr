// Incoming peer-message dispatch and upload-queue runtime.
use super::browse_wire::build_empty_browse_payload;
use super::session_runtime::handle_peer_message;
use super::*;

pub(super) async fn handle_plain_peer_messages(
    state: &AppState,
    peer: PeerMessageConnection<TcpStream>,
    peer_username: Option<String>,
) -> Result<(), String> {
    handle_plain_peer_messages_with_address(state, peer, peer_username, None).await
}

pub(super) async fn handle_plain_peer_messages_with_address(
    state: &AppState,
    mut peer: PeerMessageConnection<TcpStream>,
    peer_username: Option<String>,
    peer_address: Option<IpAddr>,
) -> Result<(), String> {
    for message_index in 0..4 {
        let message = if message_index == 0 {
            receive_plain_peer_message(state, &mut peer).await?
        } else {
            match time::timeout(Duration::from_secs(15), peer.receive()).await {
                Ok(Ok(message)) => message,
                Ok(Err(_)) | Err(_) => break,
            }
        };
        let keep_alive = handle_plain_peer_message_once(
            state,
            &mut peer,
            message,
            peer_username.as_deref(),
            peer_address,
        )
        .await?;
        if !keep_alive {
            break;
        }
    }
    Ok(())
}

async fn handle_plain_peer_message_once(
    state: &AppState,
    peer: &mut PeerMessageConnection<TcpStream>,
    message: PeerMessage,
    peer_username: Option<&str>,
    peer_address: Option<IpAddr>,
) -> Result<bool, String> {
    if state.managed_blacklist.write().await.is_blacklisted(
        peer_username,
        peer_address,
        unix_timestamp(),
    ) {
        handle_blacklisted_peer_message(state, peer, message).await?;
        return Ok(false);
    }
    if let PeerMessage::TransferRequest(request) = &message {
        if request.direction == 0 {
            if let Some(username) = peer_username {
                if handle_queued_upload_resume_request(state, peer, username, request).await? {
                    return Ok(false);
                }
                if handle_queued_download_request(state, peer, username, request).await? {
                    return Ok(false);
                }
            }
        } else if request.direction == 1 {
            if let Some(username) = peer_username {
                if handle_queued_upload_request(state, peer, username, request).await? {
                    return Ok(false);
                }
            }
        }
    }
    if let Some(username) = peer_username {
        match &message {
            PeerMessage::QueueUpload { filename } => {
                handle_queue_upload_message(state, peer, username, filename).await?;
                return Ok(false);
            }
            PeerMessage::PlaceInQueueRequest { filename } => {
                if let Some(place) = upload_queue_position(state, username, filename).await {
                    peer.send(&PeerMessage::PlaceInQueueResponse {
                        filename: filename.clone(),
                        place,
                    })
                    .await
                    .map_err(|error| format!("queue position response send failed: {error}"))?;
                }
                return Ok(false);
            }
            _ => {}
        }
    }
    if matches!(message, PeerMessage::GetShareFileList) {
        update_listeners(state, |snapshot| {
            snapshot.share_list_requests += 1;
            snapshot.last_event = Some("share_list_request".to_owned());
        })
        .await;
        let entries = {
            let shares = state.shares.read().await;
            shares.entries.clone()
        };
        peer.send(&PeerMessage::SharedFileListResponse(
            build_shared_file_list_payload(&entries)?,
        ))
        .await
        .map_err(|error| format!("peer response send failed: {error}"))?;
        update_listeners(state, |snapshot| {
            snapshot.share_list_responses += 1;
            snapshot.last_event = Some("share_list_response".to_owned());
            snapshot.last_error = None;
        })
        .await;
        return Ok(true);
    }
    handle_peer_message(state, message, peer_username, |response| async move {
        peer.send(&response)
            .await
            .map_err(|error| format!("peer response send failed: {error}"))
    })
    .await?;
    Ok(false)
}

async fn handle_blacklisted_peer_message(
    state: &AppState,
    peer: &mut PeerMessageConnection<TcpStream>,
    message: PeerMessage,
) -> Result<(), String> {
    if let Some(response) = blacklisted_peer_response(state, message).await? {
        peer.send(&response)
            .await
            .map_err(|error| format!("blacklisted peer response send failed: {error}"))?;
    }
    Ok(())
}

async fn blacklisted_peer_response(
    state: &AppState,
    message: PeerMessage,
) -> Result<Option<PeerMessage>, String> {
    Ok(match message {
        PeerMessage::UserInfoRequest => Some(PeerMessage::UserInfoResponse(UserInfo {
            description: effective_user_info_description(state).await,
            picture: effective_user_info_picture(state).await,
            total_uploads: 0,
            queue_size: i32::MAX as u32,
            slots_free: false,
            upload_permissions: None,
        })),
        PeerMessage::GetShareFileList => Some(PeerMessage::SharedFileListResponse(
            build_empty_browse_payload()?,
        )),
        PeerMessage::FileSearchRequest { .. } => None,
        PeerMessage::FolderContentsRequest(request) => Some(PeerMessage::FolderContentsResponse(
            build_folder_contents_payload(
                &[],
                request.token,
                &request.folder,
                request.folder_encoding,
            )?,
        )),
        PeerMessage::TransferRequest(request) => {
            Some(PeerMessage::TransferResponse(TransferResponse::Rejected {
                token: request.token,
                reason: "File not shared.".to_owned(),
            }))
        }
        PeerMessage::PlaceInQueueRequest { .. }
        | PeerMessage::QueueUpload { .. }
        | PeerMessage::ExactFileSearchRequest(_)
        | PeerMessage::IndirectFileSearchRequest(_) => None,
        _ => None,
    })
}

async fn handle_queued_upload_request(
    state: &AppState,
    peer: &mut PeerMessageConnection<TcpStream>,
    username: &str,
    request: &TransferRequest,
) -> Result<bool, String> {
    if !state.config.transfer_allow_inbound || !transfer_capacity_available(state, None).await {
        return Ok(false);
    }
    let accepted = {
        let mut transfers = state.transfers.write().await;
        transfers.accept_pending_remote_upload_request(
            username,
            &request.filename,
            request.token,
            request.size,
        )
    };
    let Some(accepted) = accepted else {
        return Ok(false);
    };
    peer.send(&PeerMessage::TransferResponse(TransferResponse::Allowed {
        token: request.token,
        size: accepted.size,
    }))
    .await
    .map_err(|error| format!("queued upload response send failed: {error}"))?;
    update_listeners(state, |snapshot| {
        snapshot.last_event = Some("transfer_queued_upload_accepted".to_owned());
        snapshot.last_error = None;
    })
    .await;
    Ok(true)
}

async fn handle_queued_upload_resume_request(
    state: &AppState,
    peer: &mut PeerMessageConnection<TcpStream>,
    username: &str,
    request: &TransferRequest,
) -> Result<bool, String> {
    if !state.config.transfer_allow_inbound || !transfer_capacity_available(state, None).await {
        return Ok(false);
    }
    let accepted = {
        let mut transfers = state.transfers.write().await;
        transfers.accept_pending_remote_download_request(
            username,
            &request.filename,
            request.token,
            request.size,
        )
    };
    let Some(accepted) = accepted else {
        return Ok(false);
    };
    peer.send(&PeerMessage::TransferResponse(TransferResponse::Allowed {
        token: request.token,
        size: accepted.size,
    }))
    .await
    .map_err(|error| format!("queued upload resume response send failed: {error}"))?;
    update_listeners(state, |snapshot| {
        snapshot.last_event = Some("transfer_queued_upload_resume_accepted".to_owned());
        snapshot.last_error = None;
    })
    .await;
    Ok(true)
}

fn queued_upload_group_policy(
    groups: &crate::config::TransferGroupsSettings,
    group_name: &str,
    global_slots: u32,
) -> (u32, u32, crate::config::TransferQueueStrategy) {
    if group_name == "privileged" {
        return (
            0,
            global_slots,
            crate::config::TransferQueueStrategy::FirstInFirstOut,
        );
    }
    let settings =
        transfer_group_upload_settings(groups, group_name).unwrap_or(&groups.default.upload);
    (
        settings.priority,
        settings.slots.min(global_slots),
        settings.strategy,
    )
}

pub(super) fn next_queued_upload_id(
    transfers: &TransferQueue,
    upload: &crate::config::TransferUploadSettings,
    groups: &crate::config::TransferGroupsSettings,
    users: &UserStore,
) -> Option<u64> {
    let active = transfers
        .entries
        .iter()
        .filter(|entry| entry.direction == 1 && is_active_transfer_status(&entry.status))
        .collect::<Vec<_>>();
    if active.len() >= usize::try_from(upload.slots).unwrap_or(usize::MAX) {
        return None;
    }

    let mut candidates = BTreeMap::<String, Vec<&TransferEntry>>::new();
    for entry in transfers.entries.iter().filter(|entry| {
        entry.direction == 1
            && entry.status == "queued"
            && entry.local_path.is_some()
            && entry
                .reason
                .as_deref()
                .is_some_and(is_remote_queue_response)
    }) {
        let Some(username) = entry.peer_username.as_deref() else {
            continue;
        };
        candidates
            .entry(effective_transfer_group_from(groups, users, username))
            .or_default()
            .push(entry);
    }

    let mut eligible_groups = candidates
        .into_iter()
        .filter_map(|(name, entries)| {
            let (priority, slots, strategy) =
                queued_upload_group_policy(groups, &name, upload.slots);
            let used = active
                .iter()
                .filter(|entry| {
                    entry.peer_username.as_deref().is_some_and(|username| {
                        effective_transfer_group_from(groups, users, username) == name
                    })
                })
                .count();
            (used < usize::try_from(slots).unwrap_or(usize::MAX))
                .then_some((priority, name, strategy, entries))
        })
        .collect::<Vec<_>>();
    eligible_groups.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
    let (_, _, strategy, entries) = eligible_groups.into_iter().next()?;
    entries
        .into_iter()
        .min_by_key(|entry| match strategy {
            crate::config::TransferQueueStrategy::FirstInFirstOut => (entry.requested_at, entry.id),
            crate::config::TransferQueueStrategy::RoundRobin => (entry.updated_at_ms, entry.id),
        })
        .map(|entry| entry.id)
}

pub(super) async fn schedule_queued_uploads(state: &AppState) {
    loop {
        let upload = state.transfer_upload_settings.read().await.clone();
        let groups = state.transfer_groups_settings.read().await.clone();
        let users = state.users.read().await;
        let selected = {
            let mut transfers = state.transfers.write().await;
            let id = next_queued_upload_id(&transfers, &upload, &groups, &users);
            id.and_then(|id| transfers.update_status(id, "peer_lookup", None, None))
        };
        drop(users);
        let Some(selected) = selected else {
            return;
        };
        persist_transfer_projection(state, &selected).await;
        let Some(username) = selected.peer_username.clone() else {
            continue;
        };
        if let Err(error) =
            try_send_session_command(state, SessionCommand::RequestPeerEndpoint(username))
        {
            let failed = {
                let mut transfers = state.transfers.write().await;
                transfers.update_status(
                    selected.id,
                    "failed",
                    None,
                    Some(format!("peer endpoint dispatch failed: {error}")),
                )
            };
            if let Some(failed) = failed {
                persist_transfer_projection(state, &failed).await;
            }
        }
    }
}

fn upload_queue_position_from(
    transfers: &TransferQueue,
    groups: &crate::config::TransferGroupsSettings,
    users: &UserStore,
    username: &str,
    filename: &str,
) -> Option<u32> {
    let group_name = effective_transfer_group_from(groups, users, username);
    let (_, _, strategy) = queued_upload_group_policy(groups, &group_name, u32::MAX);
    let mut group_entries = transfers
        .entries
        .iter()
        .filter(|entry| {
            entry.direction == 1
                && !is_terminal_transfer_status(&entry.status)
                && entry.peer_username.as_deref().is_some_and(|peer| {
                    effective_transfer_group_from(groups, users, peer) == group_name
                })
        })
        .collect::<Vec<_>>();
    match strategy {
        crate::config::TransferQueueStrategy::FirstInFirstOut => {
            group_entries.sort_by_key(|entry| (entry.requested_at, entry.id));
            group_entries
                .iter()
                .position(|entry| {
                    entry.peer_username.as_deref() == Some(username) && entry.filename == filename
                })
                .and_then(|position| u32::try_from(position).ok())
        }
        crate::config::TransferQueueStrategy::RoundRobin => {
            let mut own = group_entries
                .iter()
                .copied()
                .filter(|entry| entry.peer_username.as_deref() == Some(username))
                .collect::<Vec<_>>();
            own.sort_by_key(|entry| (entry.requested_at, entry.id));
            let local_position = own.iter().position(|entry| entry.filename == filename)?;
            let mut per_user = BTreeMap::<&str, usize>::new();
            for entry in group_entries {
                if let Some(peer) = entry.peer_username.as_deref() {
                    *per_user.entry(peer).or_default() += 1;
                }
            }
            let position = per_user
                .into_iter()
                .filter(|(peer, _)| *peer != username)
                .fold(local_position, |position, (_, count)| {
                    position.saturating_add(local_position.min(count))
                });
            u32::try_from(position).ok()
        }
    }
}

pub(super) async fn upload_queue_position(
    state: &AppState,
    username: &str,
    filename: &str,
) -> Option<u32> {
    let transfers = state.transfers.read().await;
    let groups = state.transfer_groups_settings.read().await;
    let users = state.users.read().await;
    upload_queue_position_from(&transfers, &groups, &users, username, filename)
}

fn upload_queue_forecast_from(
    transfers: &TransferQueue,
    upload: &crate::config::TransferUploadSettings,
    groups: &crate::config::TransferGroupsSettings,
    users: &UserStore,
    username: &str,
) -> (u32, u32) {
    let group_name = effective_transfer_group_from(groups, users, username);
    let (_, group_slots, strategy) = queued_upload_group_policy(groups, &group_name, upload.slots);
    let active = transfers
        .entries
        .iter()
        .filter(|entry| entry.direction == 1 && is_active_transfer_status(&entry.status))
        .collect::<Vec<_>>();
    let group_active = active
        .iter()
        .filter(|entry| {
            entry.peer_username.as_deref().is_some_and(|peer| {
                effective_transfer_group_from(groups, users, peer) == group_name
            })
        })
        .count();
    if active.len() < usize::try_from(upload.slots).unwrap_or(usize::MAX)
        && group_active < usize::try_from(group_slots).unwrap_or(usize::MAX)
    {
        return (group_slots, 0);
    }
    let group_entries = transfers.entries.iter().filter(|entry| {
        entry.direction == 1
            && !is_terminal_transfer_status(&entry.status)
            && entry.peer_username.as_deref().is_some_and(|peer| {
                effective_transfer_group_from(groups, users, peer) == group_name
            })
    });
    let ahead = match strategy {
        crate::config::TransferQueueStrategy::FirstInFirstOut => group_entries.count(),
        crate::config::TransferQueueStrategy::RoundRobin => group_entries
            .filter_map(|entry| entry.peer_username.as_deref())
            .collect::<HashSet<_>>()
            .len(),
    };
    (
        group_slots,
        u32::try_from(ahead.saturating_add(1)).unwrap_or(u32::MAX),
    )
}

pub(super) async fn upload_queue_forecast(state: &AppState, username: &str) -> (u32, u32) {
    let transfers = state.transfers.read().await;
    let upload = state.transfer_upload_settings.read().await;
    let groups = state.transfer_groups_settings.read().await;
    let users = state.users.read().await;
    upload_queue_forecast_from(&transfers, &upload, &groups, &users, username)
}

async fn handle_queue_upload_message(
    state: &AppState,
    peer: &mut PeerMessageConnection<TcpStream>,
    username: &str,
    filename: &str,
) -> Result<(), String> {
    let Some(shared_file) = find_shared_local_file(state, filename).await else {
        peer.send(&PeerMessage::UploadDenied {
            filename: filename.to_owned(),
            reason: "File not shared.".to_owned(),
        })
        .await
        .map_err(|error| format!("upload denial send failed: {error}"))?;
        return Ok(());
    };
    let already_queued = {
        let transfers = state.transfers.read().await;
        transfers.entries.iter().any(|entry| {
            entry.direction == 1
                && entry.peer_username.as_deref() == Some(username)
                && entry.filename == filename
                && !is_terminal_transfer_status(&entry.status)
        })
    };
    if already_queued {
        schedule_queued_uploads(state).await;
        return Ok(());
    }
    match inbound_upload_policy(state, username, filename, shared_file.size).await {
        Ok(_) => {}
        Err(reason) if reason == "Queued" => {}
        Err(reason) => {
            peer.send(&PeerMessage::UploadDenied {
                filename: filename.to_owned(),
                reason,
            })
            .await
            .map_err(|error| format!("upload policy denial send failed: {error}"))?;
            return Ok(());
        }
    }
    let queued = {
        let mut transfers = state.transfers.write().await;
        let entry = transfers.create(
            1,
            Some(username.to_owned()),
            filename.to_owned(),
            Some(shared_file.local_path.display().to_string()),
            Some(shared_file.size),
        );
        transfers
            .update_status(entry.id, "queued", None, Some("Queued".to_owned()))
            .unwrap_or(entry)
    };
    persist_transfer_projection(state, &queued).await;
    schedule_queued_uploads(state).await;
    Ok(())
}

async fn handle_queued_download_request(
    state: &AppState,
    peer: &mut PeerMessageConnection<TcpStream>,
    username: &str,
    request: &TransferRequest,
) -> Result<bool, String> {
    let Some(shared_file) = find_shared_local_file(state, &request.filename).await else {
        return Ok(false);
    };
    if let Err(reason) =
        inbound_upload_policy(state, username, &request.filename, shared_file.size).await
    {
        if reason == "Queued" {
            let queued = {
                let mut transfers = state.transfers.write().await;
                let queued = transfers.create(
                    1,
                    Some(username.to_owned()),
                    request.filename.clone(),
                    Some(shared_file.local_path.display().to_string()),
                    Some(shared_file.size),
                );
                transfers.update_status(queued.id, "queued", None, Some(reason.clone()));
                queued
            };
            persist_transfer_projection(state, &queued).await;
        } else {
            record_transfer_rejection(
                state,
                request.direction,
                request.token,
                request.filename.clone(),
                request.size,
                reason.clone(),
            )
            .await;
        }
        peer.send(&PeerMessage::TransferResponse(TransferResponse::Rejected {
            token: request.token,
            reason,
        }))
        .await
        .map_err(|error| format!("queued transfer policy response send failed: {error}"))?;
        return Ok(true);
    }
    let entry = {
        let mut transfers = state.transfers.write().await;
        transfers.record_accepted_inbound_request(
            1,
            request.token,
            Some(username.to_owned()),
            request.filename.clone(),
            shared_file.local_path.display().to_string(),
            shared_file.size,
        )
    };
    persist_transfer_durability(state).await;
    peer.send(&PeerMessage::TransferResponse(TransferResponse::Rejected {
        token: request.token,
        reason: "Queued".to_owned(),
    }))
    .await
    .map_err(|error| format!("queued transfer response send failed: {error}"))?;
    peer.send(&PeerMessage::TransferRequest(TransferRequest {
        filename_encoding: request.filename_encoding,
        direction: 1,
        token: request.token,
        filename: request.filename.clone(),
        size: Some(shared_file.size),
    }))
    .await
    .map_err(|error| format!("queued upload start request send failed: {error}"))?;
    let response = time::timeout(
        state.config.soulseek_connection.timeout_inactivity,
        peer.receive(),
    )
    .await
    .map_err(|_| "queued upload start response timed out".to_owned())?
    .map_err(|error| format!("queued upload start response failed: {error}"))?;
    match response {
        PeerMessage::TransferResponse(TransferResponse::Allowed { token, .. })
            if token == request.token =>
        {
            let Some(address) = test_user_endpoint_peer_address(state, username) else {
                {
                    let mut transfers = state.transfers.write().await;
                    transfers.update_status(
                        entry.id,
                        "failed",
                        None,
                        Some("no endpoint available for queued upload transfer".to_owned()),
                    );
                }
                persist_transfer_durability(state).await;
                return Err("no endpoint available for queued upload transfer".to_owned());
            };
            execute_accepted_file_transfer(state, &address, &entry).await;
            update_listeners(state, |snapshot| {
                snapshot.last_event = Some("transfer_queued_upload_started".to_owned());
                snapshot.last_error = None;
            })
            .await;
            Ok(true)
        }
        PeerMessage::TransferResponse(TransferResponse::Rejected { token, reason })
            if token == request.token =>
        {
            {
                let mut transfers = state.transfers.write().await;
                transfers.update_status(entry.id, "failed", None, Some(reason.clone()));
            }
            persist_transfer_durability(state).await;
            Err(format!("queued upload rejected: {reason}"))
        }
        other => {
            let reason = format!(
                "expected queued upload TransferResponse, got {}",
                peer_message_name(&other)
            );
            {
                let mut transfers = state.transfers.write().await;
                transfers.update_status(entry.id, "failed", None, Some(reason.clone()));
            }
            persist_transfer_durability(state).await;
            Err(reason)
        }
    }
}

pub(super) async fn handle_obfuscated_peer_messages_with_address(
    state: &AppState,
    mut peer: ObfuscatedPeerMessageConnection<TcpStream>,
    peer_username: Option<String>,
    peer_address: Option<IpAddr>,
) -> Result<(), String> {
    for message_index in 0..4 {
        let message = if message_index == 0 {
            receive_obfuscated_peer_message(state, &mut peer).await?
        } else {
            match time::timeout(Duration::from_secs(15), peer.receive()).await {
                Ok(Ok(message)) => message,
                Ok(Err(_)) | Err(_) => break,
            }
        };
        if state.managed_blacklist.write().await.is_blacklisted(
            peer_username.as_deref(),
            peer_address,
            unix_timestamp(),
        ) {
            if let Some(response) = blacklisted_peer_response(state, message).await? {
                peer.send(&response).await.map_err(|error| {
                    format!("blacklisted obfuscated peer response send failed: {error}")
                })?;
            }
            break;
        }

        let peer_connection = &mut peer;
        handle_peer_message(
            state,
            message,
            peer_username.as_deref(),
            |response| async move {
                peer_connection
                    .send(&response)
                    .await
                    .map_err(|error| format!("obfuscated peer response send failed: {error}"))
            },
        )
        .await?;
    }
    Ok(())
}

async fn receive_plain_peer_message(
    state: &AppState,
    peer: &mut PeerMessageConnection<TcpStream>,
) -> Result<PeerMessage, String> {
    time::timeout(
        state.config.soulseek_connection.timeout_inactivity,
        peer.receive(),
    )
    .await
    .map_err(|_| "peer message receive timed out".to_owned())?
    .map_err(|error| format!("peer message receive failed: {error}"))
}

async fn receive_obfuscated_peer_message(
    state: &AppState,
    peer: &mut ObfuscatedPeerMessageConnection<TcpStream>,
) -> Result<PeerMessage, String> {
    time::timeout(
        state.config.soulseek_connection.timeout_inactivity,
        peer.receive(),
    )
    .await
    .map_err(|_| "obfuscated peer message receive timed out".to_owned())?
    .map_err(|error| format!("obfuscated peer message receive failed: {error}"))
}
