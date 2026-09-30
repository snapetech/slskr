use super::*;

pub(crate) async fn probe_peer_capability(
    state: &AppState,
    username: &str,
) -> Result<PeerCapabilityDescriptor, String> {
    let mesh_settings = state.advanced_networking.read().await.mesh.clone();
    if !mesh_settings.enabled
        || !mesh_settings.enable_soulseek_rendezvous
        || !mesh_settings.probe_soulseek_rendezvous_capabilities
        || !mesh_settings.enable_soulseek_capability_handshake
    {
        return Err("Soulseek mesh capability probing is disabled by configuration".to_owned());
    }
    let username = username.trim();
    if username.is_empty() {
        return Err("peer capability username is required".to_owned());
    }
    let address = request_peer_endpoint(state, username).await?;
    let nonce = uuid::Uuid::new_v4().simple().to_string();
    let hello = PeerCapabilityEnvelope::new(
        PeerCapabilityMessageType::Hello,
        nonce.clone(),
        local_capability_descriptor(state).await?,
    );
    let message = peer_capability_message(&hello)
        .map_err(|error| format!("peer capability hello failed: {error}"))?;
    let response = send_peer_message_request(state, &address, message).await?;
    let Some(mut acknowledgement) = decode_peer_capability_message(&response)
        .map_err(|error| format!("peer capability acknowledgement rejected: {error}"))?
    else {
        return Err(format!(
            "expected peer capability acknowledgement, got {}",
            peer_message_name(&response)
        ));
    };
    if acknowledgement.message_type != PeerCapabilityMessageType::Acknowledge {
        return Err("peer capability response was not an acknowledgement".to_owned());
    }
    if acknowledgement.nonce != nonce {
        return Err("peer capability acknowledgement nonce did not match".to_owned());
    }
    acknowledgement.descriptor.username = username.to_owned();
    let descriptor = acknowledgement.descriptor;
    state
        .mesh
        .write()
        .await
        .update_capability(descriptor.clone())?;
    update_listeners(state, |snapshot| {
        snapshot.last_event = Some(format!(
            "peer_capability_acknowledge:{}",
            redact_username(username)
        ));
        snapshot.last_error = None;
    })
    .await;
    Ok(descriptor)
}

pub(crate) async fn execute_accepted_file_transfer(
    state: &AppState,
    address: &PeerAddress,
    transfer: &TransferEntry,
) {
    if cancel_download_if_blocked_by_policy(state, transfer)
        .await
        .is_some()
    {
        return;
    }
    if transfer
        .local_path
        .as_deref()
        .unwrap_or_default()
        .is_empty()
    {
        return;
    }

    {
        let mut transfers = state.transfers.write().await;
        if transfers
            .entries
            .iter()
            .find(|entry| entry.id == transfer.id)
            .is_some_and(|entry| entry.status == "cancelled")
        {
            return;
        }
        transfers.update_status(transfer.id, "in_progress", None, None);
    }
    persist_transfer_durability(state).await;

    let result = if transfer.direction == 1 {
        upload_file_transfer(state, address, transfer).await
    } else {
        download_file_transfer_with_retry(state, address, transfer).await
    };
    if transfer_is_cancelled(state, transfer.id).await {
        return;
    }
    if let Err(error) = &result {
        record_expected_upload_failure(state, transfer, error).await;
    }
    let (status, bytes_transferred, size, reason) = match result {
        Ok((bytes_transferred, size)) => ("succeeded", bytes_transferred, Some(size), None),
        Err(error) => {
            if let Some(username) = transfer.peer_username.clone() {
                let indirect_pending = {
                    let mut transfers = state.transfers.write().await;
                    transfers.update_status(
                        transfer.id,
                        "indirect_pending",
                        None,
                        Some(format!("direct file-transfer failed: {error}")),
                    )
                };
                if let Some(indirect_pending) = indirect_pending {
                    persist_transfer_projection(state, &indirect_pending).await;
                }
                if let Err(error) = try_send_session_command(
                    state,
                    SessionCommand::IndirectTransfer {
                        id: transfer.id,
                        username,
                        token: transfer.token,
                    },
                ) {
                    let failed = {
                        let mut transfers = state.transfers.write().await;
                        transfers.update_status(
                            transfer.id,
                            "failed",
                            None,
                            Some(format!("indirect transfer dispatch failed: {error}")),
                        )
                    };
                    if let Some(failed) = failed {
                        persist_transfer_projection(state, &failed).await;
                    }
                    update_session(state, |snapshot| {
                        snapshot.last_error = Some(format!(
                            "indirect transfer {} dispatch failed: {error}",
                            transfer.id
                        ));
                    })
                    .await;
                }
                return;
            }
            (
                "failed",
                transfer.bytes_transferred,
                transfer.size,
                Some(error),
            )
        }
    };

    let updated = {
        let mut transfers = state.transfers.write().await;
        transfers.update_local_execution(transfer.id, status, bytes_transferred, size, reason)
    };
    if let Some(updated) = updated {
        let updated = apply_completed_download_permissions(state, updated).await;
        let updated = enrich_completed_audio_metadata(state, updated).await;
        persist_transfer_projection(state, &updated).await;
        issue_relay_download_tokens(state, &updated).await;
        maybe_import_lidarr_completed_download(state, &updated).await;
        maybe_upload_ftp_completed_download(state, &updated).await;
    }
    if transfer.direction == 1 {
        schedule_queued_uploads(state).await;
    } else {
        schedule_queued_downloads(state).await;
    }
}

pub(crate) async fn project_indirect_transfer_response(
    state: &AppState,
    response: &ConnectToPeerResponse,
) {
    let Ok(kind) = ConnectionKind::try_from_connection_type(&response.connection_type) else {
        return;
    };
    if kind != ConnectionKind::FileTransfer {
        return;
    }
    let transfer = {
        let transfers = state.transfers.read().await;
        transfers.pending_indirect_transfer(&response.username, response.token)
    };
    let Some(transfer) = transfer else {
        return;
    };
    let in_progress = {
        let mut transfers = state.transfers.write().await;
        if transfers
            .entries
            .iter()
            .find(|entry| entry.id == transfer.id)
            .is_some_and(|entry| entry.status == "cancelled")
        {
            return;
        }
        transfers.update_status(transfer.id, "in_progress", None, None)
    };
    if let Some(in_progress) = in_progress {
        persist_transfer_projection(state, &in_progress).await;
    }

    let result = execute_indirect_file_transfer(state, response, &transfer).await;
    if transfer_is_cancelled(state, transfer.id).await {
        return;
    }
    if let Err(error) = &result {
        record_expected_upload_failure(state, &transfer, error).await;
    }
    let (status, bytes_transferred, size, reason) = match result {
        Ok((bytes_transferred, size)) => ("succeeded", bytes_transferred, Some(size), None),
        Err(error) => (
            "failed",
            transfer.bytes_transferred,
            transfer.size,
            Some(error),
        ),
    };
    let updated = {
        let mut transfers = state.transfers.write().await;
        transfers.update_local_execution(transfer.id, status, bytes_transferred, size, reason)
    };
    if let Some(updated) = updated {
        let updated = enrich_completed_audio_metadata(state, updated).await;
        persist_transfer_projection(state, &updated).await;
        issue_relay_download_tokens(state, &updated).await;
        maybe_import_lidarr_completed_download(state, &updated).await;
        maybe_upload_ftp_completed_download(state, &updated).await;
    }
    if transfer.direction == 1 {
        schedule_queued_uploads(state).await;
    } else {
        schedule_queued_downloads(state).await;
    }
}

pub(crate) async fn project_peer_transfer_response(state: &AppState, address: &PeerAddress) {
    let transfer = {
        let transfers = state.transfers.read().await;
        transfers.pending_peer_transfer(&address.username)
    };
    let Some(transfer) = transfer else {
        return;
    };
    if cancel_download_if_blocked_by_policy(state, &transfer)
        .await
        .is_some()
    {
        return;
    }
    if transfer.direction == 0 && !download_capacity_available(state, Some(transfer.id)).await {
        let queued = {
            let mut transfers = state.transfers.write().await;
            transfers.update_status(
                transfer.id,
                "queued",
                None,
                Some("Local download slots exhausted".to_owned()),
            )
        };
        if let Some(queued) = queued {
            persist_transfer_projection(state, &queued).await;
        }
        return;
    }

    let negotiating = {
        let mut transfers = state.transfers.write().await;
        transfers.update_status(transfer.id, "peer_negotiating", None, None)
    };
    if let Some(negotiating) = negotiating {
        persist_transfer_projection(state, &negotiating).await;
    }

    let result = negotiate_peer_transfer(state, address, &transfer).await;
    let (status, bytes_transferred, reason) = match result {
        Ok(PeerTransferNegotiation::Allowed { token, size }) if token == transfer.token => {
            let transferred = transfer.bytes_transferred;
            let size = size.or(transfer.size);
            let accepted = {
                let mut transfers = state.transfers.write().await;
                transfers.update_local_execution(transfer.id, "accepted", transferred, size, None)
            };
            if let Some(accepted) = accepted {
                persist_transfer_projection(state, &accepted).await;
                execute_accepted_file_transfer(state, address, &accepted).await;
            }
            return;
        }
        Ok(PeerTransferNegotiation::QueuedInbound { token, size }) => {
            let accepted_result = {
                let mut transfers = state.transfers.write().await;
                transfers.accept_queued_inbound_negotiation(transfer.id, token, size)
            };
            let accepted = match accepted_result {
                Ok(accepted) => accepted,
                Err(error) => {
                    let failed = {
                        let mut transfers = state.transfers.write().await;
                        transfers.update_status(transfer.id, "failed", None, Some(error))
                    };
                    if let Some(failed) = failed {
                        persist_transfer_projection(state, &failed).await;
                    }
                    return;
                }
            };
            if let Some(accepted) = accepted {
                persist_transfer_projection(state, &accepted).await;
            } else {
                let failed = {
                    let mut transfers = state.transfers.write().await;
                    transfers.update_status(
                        transfer.id,
                        "failed",
                        None,
                        Some("queued transfer is no longer pending".to_owned()),
                    )
                };
                if let Some(failed) = failed {
                    persist_transfer_projection(state, &failed).await;
                }
            }
            return;
        }
        Ok(PeerTransferNegotiation::Allowed { token, .. }) => (
            "failed",
            None,
            Some(format!(
                "transfer token mismatch: expected {}, received {token}",
                transfer.token
            )),
        ),
        Ok(PeerTransferNegotiation::Rejected { token, reason }) if token == transfer.token => {
            if is_remote_queue_response(&reason) {
                ("queued", None, Some(reason))
            } else {
                ("failed", None, Some(reason))
            }
        }
        Ok(PeerTransferNegotiation::Rejected { token, .. }) => (
            "failed",
            None,
            Some(format!(
                "transfer token mismatch: expected {}, received {token}",
                transfer.token
            )),
        ),
        Err(error) => ("failed", None, Some(error)),
    };

    let updated = {
        let mut transfers = state.transfers.write().await;
        transfers.update_status(transfer.id, status, bytes_transferred, reason)
    };
    if let Some(updated) = updated {
        persist_transfer_projection(state, &updated).await;
    }
    if transfer.direction == 1 && status != "queued" {
        schedule_queued_uploads(state).await;
    } else if transfer.direction == 0 && status != "queued" {
        schedule_queued_downloads(state).await;
    }
}

pub(crate) async fn schedule_queued_downloads(state: &AppState) {
    loop {
        if !download_capacity_available(state, None).await {
            return;
        }
        let queued = {
            let mut transfers = state.transfers.write().await;
            let next = transfers
                .entries
                .iter()
                .filter(|entry| {
                    entry.direction == 0
                        && entry.status == "queued"
                        && entry.reason.as_deref() == Some("Local download slots exhausted")
                })
                .min_by_key(|entry| (entry.requested_at, entry.id))
                .cloned();
            next.and_then(|entry| transfers.update_status(entry.id, "peer_lookup", None, None))
        };
        let Some(queued) = queued else {
            return;
        };
        persist_transfer_projection(state, &queued).await;
        let Some(username) = queued.peer_username.as_deref() else {
            continue;
        };
        if let Some(address) = cached_peer_endpoint(state, username).await {
            Box::pin(project_peer_transfer_response(state, &address)).await;
        } else if try_send_session_command(
            state,
            SessionCommand::TransferPeer {
                id: queued.id,
                username: username.to_owned(),
            },
        )
        .is_err()
        {
            let failed = state.transfers.write().await.update_status(
                queued.id,
                "failed",
                None,
                Some("download peer lookup dispatch failed".to_owned()),
            );
            if let Some(failed) = failed {
                persist_transfer_projection(state, &failed).await;
            }
        }
    }
}
