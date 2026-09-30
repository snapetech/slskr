use super::*;

pub(crate) async fn handle_peer_message<F, Fut>(
    state: &AppState,
    message: PeerMessage,
    peer_username: Option<&str>,
    send_response: F,
) -> Result<(), String>
where
    F: FnOnce(PeerMessage) -> Fut,
    Fut: std::future::Future<Output = Result<(), String>>,
{
    let capability = match decode_peer_capability_message(&message) {
        Ok(capability) => capability,
        Err(error) => {
            if let Some(username) = peer_username {
                record_peer_security_violation(state, username).await;
            }
            return Err(format!("peer capability message rejected: {error}"));
        }
    };
    if let Some(mut envelope) = capability {
        if !state
            .advanced_networking
            .read()
            .await
            .mesh
            .enable_soulseek_capability_handshake
        {
            return Err("peer capability handshake is disabled by configuration".to_owned());
        }
        let peer_username = peer_username
            .map(str::trim)
            .filter(|username| !username.is_empty())
            .ok_or_else(|| {
                "peer capability message rejected: authenticated peer username is unavailable"
                    .to_owned()
            })?;
        envelope.descriptor.username = peer_username.to_owned();
        let message_type = envelope.message_type;
        let nonce = envelope.nonce;
        let username = envelope.descriptor.username.clone();
        let projection = {
            let mut mesh = state.mesh.write().await;
            if let Err(error) = mesh.update_capability(envelope.descriptor) {
                Err(error)
            } else {
                Ok(mesh.persisted_capability_projection())
            }
        };
        let projection = match projection {
            Ok(projection) => projection,
            Err(error) => {
                record_peer_security_violation(state, peer_username).await;
                return Err(error);
            }
        };
        let feature_result = state
            .controller_features
            .upsert(
                "hashdb/peers".to_owned(),
                serde_json::json!({"peers": projection}),
            )
            .await;
        if let Err(error) = feature_result {
            record_daemon_log(
                state,
                logging::LogLevel::Error,
                "distributed",
                format!("peer capability projection persistence failed: {error}"),
            )
            .await;
        }
        if message_type == PeerCapabilityMessageType::Hello {
            let acknowledgement = PeerCapabilityEnvelope::new(
                PeerCapabilityMessageType::Acknowledge,
                nonce,
                local_capability_descriptor(state).await?,
            );
            let response = peer_capability_message(&acknowledgement)
                .map_err(|error| format!("peer capability acknowledgement failed: {error}"))?;
            send_response(response).await?;
        }
        update_listeners(state, |snapshot| {
            snapshot.last_event = Some(format!(
                "peer_capability_{}:{}",
                if message_type == PeerCapabilityMessageType::Hello {
                    "hello"
                } else {
                    "acknowledge"
                },
                redact_username(&username)
            ));
            snapshot.last_error = None;
        })
        .await;
        return Ok(());
    }
    match message {
        PeerMessage::UserInfoRequest => {
            update_listeners(state, |snapshot| {
                snapshot.user_info_requests += 1;
                snapshot.last_event = Some("user_info_request".to_owned());
            })
            .await;
            let (upload_slots, queue_position) =
                upload_queue_forecast(state, peer_username.unwrap_or_default()).await;
            send_response(PeerMessage::UserInfoResponse(UserInfo {
                description: effective_user_info_description(state).await,
                picture: effective_user_info_picture(state).await,
                total_uploads: upload_slots,
                queue_size: queue_position,
                slots_free: queue_position == 0,
                upload_permissions: None,
            }))
            .await?;
            update_listeners(state, |snapshot| {
                snapshot.user_info_responses += 1;
                snapshot.last_event = Some("user_info_response".to_owned());
                snapshot.last_error = None;
            })
            .await;
        }
        PeerMessage::GetShareFileList => {
            update_listeners(state, |snapshot| {
                snapshot.share_list_requests += 1;
                snapshot.last_event = Some("share_list_request".to_owned());
            })
            .await;
            let entries = {
                let shares = state.shares.read().await;
                shares.entries.clone()
            };
            send_response(PeerMessage::SharedFileListResponse(
                build_shared_file_list_payload(&entries)?,
            ))
            .await?;
            update_listeners(state, |snapshot| {
                snapshot.share_list_responses += 1;
                snapshot.last_event = Some("share_list_response".to_owned());
                snapshot.last_error = None;
            })
            .await;
        }
        PeerMessage::FileSearchRequest { token, query } => {
            update_listeners(state, |snapshot| {
                snapshot.file_search_requests += 1;
                snapshot.last_event = Some("file_search_request".to_owned());
            })
            .await;
            if let Some(mut response) = build_file_search_response(state, token, &query).await {
                if let Some(username) = peer_username
                    .map(str::trim)
                    .filter(|username| !username.is_empty())
                {
                    if state.config.controller_profile == ControllerProfile::Native
                        && state
                            .managed_blacklist
                            .read()
                            .await
                            .username_is_blacklisted(username)
                    {
                        return Ok(());
                    }
                    if matches!(
                        state.config.controller_profile,
                        ControllerProfile::Legacy | ControllerProfile::Native
                    ) && state.session.read().await.state == "connected"
                        && request_peer_endpoint(state, username).await.is_err()
                    {
                        // Frozen controllers resolve the requester's endpoint
                        // before accepting an incoming search response. If the
                        // server cannot resolve that peer, they drop it.
                        return Ok(());
                    }
                    let (_, queue_position) = upload_queue_forecast(state, username).await;
                    response.slot_free = queue_position == 0;
                    response.queue_length = queue_position;
                }
                send_response(PeerMessage::FileSearchResponse(response)).await?;
                update_listeners(state, |snapshot| {
                    snapshot.file_search_responses += 1;
                    snapshot.last_event = Some("file_search_response".to_owned());
                    snapshot.last_error = None;
                })
                .await;
            }
        }
        PeerMessage::FileSearchResponse(response) => {
            state
                .remote_path_encodings
                .write()
                .await
                .remember_search_response(&response);
            let wishlist_item_id = {
                let searches = state.searches.read().await;
                searches
                    .get(response.token)
                    .and_then(|record| record.wishlist_item_id().map(str::to_owned))
            };
            let _wishlist_search_persistence = if wishlist_item_id.is_some() {
                Some(state.wishlist_search_persistence_lock.lock().await)
            } else {
                None
            };
            let (ignored_results, wishlist_policy) =
                if let Some(item_id) = wishlist_item_id.as_deref() {
                    let wishlist = state.wishlist.read().await;
                    (
                        wishlist.ignored_results_for(item_id),
                        wishlist.result_policy_for(item_id),
                    )
                } else {
                    (Vec::new(), None)
                };
            let updated_record = {
                let mut searches = state.searches.write().await;
                searches.add_peer_response_filtered(
                    &response,
                    &ignored_results,
                    wishlist_policy.as_ref(),
                )
            };
            let accepted = updated_record.is_some();
            if let Some((record, appended)) = updated_record.as_ref() {
                persist_search_result_delta(state, record, appended).await?;
                publish_search_hub_event(state, "update", record);
            };
            update_listeners(state, |snapshot| {
                snapshot.file_search_responses += 1;
                snapshot.last_event = Some(if accepted {
                    "file_search_response".to_owned()
                } else {
                    "file_search_response_unmatched".to_owned()
                });
                snapshot.last_error = None;
            })
            .await;
        }
        PeerMessage::FolderContentsRequest(request) => {
            let entries = {
                let shares = state.shares.read().await;
                shares.entries.clone()
            };
            send_response(PeerMessage::FolderContentsResponse(
                build_folder_contents_payload(
                    &entries,
                    request.token,
                    &request.folder,
                    request.folder_encoding,
                )?,
            ))
            .await?;
        }
        PeerMessage::TransferRequest(request) => {
            if !state.config.transfer_allow_inbound {
                let reason = "inbound transfers are disabled".to_owned();
                record_transfer_rejection(
                    state,
                    request.direction,
                    request.token,
                    request.filename.clone(),
                    request.size,
                    reason.clone(),
                )
                .await;
                send_response(PeerMessage::TransferResponse(TransferResponse::Rejected {
                    token: request.token,
                    reason,
                }))
                .await?;
                update_listeners(state, |snapshot| {
                    snapshot.transfer_rejections += 1;
                    snapshot.last_event = Some("transfer_rejected_policy".to_owned());
                    snapshot.last_error = None;
                })
                .await;
            } else if !transfer_capacity_available(state, None).await {
                let reason = "transfer limit reached".to_owned();
                record_transfer_rejection(
                    state,
                    request.direction,
                    request.token,
                    request.filename.clone(),
                    request.size,
                    reason.clone(),
                )
                .await;
                send_response(PeerMessage::TransferResponse(TransferResponse::Rejected {
                    token: request.token,
                    reason,
                }))
                .await?;
                update_listeners(state, |snapshot| {
                    snapshot.transfer_rejections += 1;
                    snapshot.last_event = Some("transfer_rejected_limit".to_owned());
                    snapshot.last_error = None;
                })
                .await;
            } else if let Some(shared_file) = find_shared_local_file(state, &request.filename).await
            {
                if request.direction == 0 {
                    if let Some(username) = peer_username {
                        if let Err(reason) = inbound_upload_policy(
                            state,
                            username,
                            &request.filename,
                            shared_file.size,
                        )
                        .await
                        {
                            record_transfer_rejection(
                                state,
                                request.direction,
                                request.token,
                                request.filename.clone(),
                                request.size,
                                reason.clone(),
                            )
                            .await;
                            send_response(PeerMessage::TransferResponse(
                                TransferResponse::Rejected {
                                    token: request.token,
                                    reason,
                                },
                            ))
                            .await?;
                            update_listeners(state, |snapshot| {
                                snapshot.transfer_rejections += 1;
                                snapshot.last_event =
                                    Some("transfer_rejected_group_policy".to_owned());
                                snapshot.last_error = None;
                            })
                            .await;
                            return Ok(());
                        }
                    }
                }
                {
                    let mut transfers = state.transfers.write().await;
                    transfers.record_accepted_inbound_request(
                        1,
                        request.token,
                        peer_username.map(str::to_owned),
                        request.filename.clone(),
                        shared_file.local_path.display().to_string(),
                        shared_file.size,
                    );
                }
                persist_transfer_durability(state).await;
                send_response(PeerMessage::TransferResponse(TransferResponse::Allowed {
                    token: request.token,
                    size: Some(shared_file.size),
                }))
                .await?;
                update_listeners(state, |snapshot| {
                    snapshot.last_event = Some("transfer_accepted".to_owned());
                    snapshot.last_error = None;
                })
                .await;
            } else {
                let reason = "requested file is not available from local shares".to_owned();
                record_transfer_rejection(
                    state,
                    request.direction,
                    request.token,
                    request.filename.clone(),
                    request.size,
                    reason.clone(),
                )
                .await;
                send_response(PeerMessage::TransferResponse(TransferResponse::Rejected {
                    token: request.token,
                    reason,
                }))
                .await?;
                update_listeners(state, |snapshot| {
                    snapshot.transfer_rejections += 1;
                    snapshot.last_event = Some("transfer_rejected".to_owned());
                    snapshot.last_error = None;
                })
                .await;
            }
        }
        other => {
            update_listeners(state, |snapshot| {
                snapshot.unsupported_peer_messages += 1;
                snapshot.last_event = Some(format!(
                    "unsupported_peer_message:{}",
                    peer_message_name(&other)
                ));
            })
            .await;
        }
    }
    Ok(())
}
