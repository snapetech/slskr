async fn route_dispatch_group_6_pods(
    context: &RouteDispatchContext<'_, '_>,
) -> RouteDispatchResult {
    let method = context.method;
    let normalized_path = context.normalized_path;
    let authorization = context.authorization;
    let body = context.body;
    let state = context.state;
    let route = context.route;
    let headers = context.headers;
    let extended_mutation = context.extended_mutation;
    let request_is_versioned_v0 = context.request_is_versioned_v0;
    match (method, normalized_path) {
        ("GET", path) if pod_channel_messages_path(path).is_some() => {
            let (pod_id, channel_id) = pod_channel_messages_path(path).unwrap_or_default();
            if pods::is_gold_star_club(&pod_id) && !gold_star_club_available(state) {
                return Ok(routing::not_found_response());
            }
            let since = match query_millis_parameter(route.query, "since") {
                Ok(value) => value,
                Err(error) => return Ok(routing::bad_request_response(&error)),
            };
            let peer_id = pod_request_peer_id(state).await;
            let pods = state.pods.read().await;
            if pods.get(&pod_id).is_none() || !pods.channel_exists(&pod_id, &channel_id) {
                return Ok(routing::ok_response("[]".to_owned()));
            }
            if peer_id
                .as_deref()
                .is_none_or(|peer_id| !pods.is_member(&pod_id, peer_id))
            {
                return Ok(routing::forbidden_response("Pod membership is required"));
            }
            let binding = pods.soulseek_binding(&pod_id, &channel_id);
            drop(pods);
            if let Err(error) = state.pod_channels.read().await.validate_storage() {
                eprintln!("pod channel message storage failed: {error}");
                return Ok(routing::internal_server_error_response(
                    "Failed to get messages",
                ));
            }
            if let Some(binding) = binding.filter(|binding| binding.kind == "dm") {
                let local_peer_id = peer_id.unwrap_or_default();
                let messages = state.messages.read().await;
                let projected = messages
                    .records
                    .iter()
                    .filter(|message| {
                        message.username.eq_ignore_ascii_case(&binding.identifier)
                            && since.is_none_or(|since| message.created_at_ms > since)
                    })
                    .map(|message| pod_channels::PodChannelMessage {
                        message_id: message.id.to_string(),
                        pod_id: pod_id.clone(),
                        channel_id: channel_id.clone(),
                        sender_peer_id: if message.direction == "inbound" {
                            format!("bridge:{}", message.username)
                        } else {
                            local_peer_id.clone()
                        },
                        body: message.body.clone(),
                        timestamp_unix_ms: message.created_at_ms,
                        signature: String::new(),
                        sig_version: 1,
                    })
                    .collect::<Vec<_>>();
                return Ok(routing::ok_response(
                    serde_json::to_string(&projected)
                        .map_err(|error| format!("pod message serialization failed: {error}"))?,
                ));
            }
            let channels = state.pod_channels.read().await;
            let messages = channels.list(&pod_id, &channel_id, since);
            drop(channels);
            Ok(routing::ok_response(
                serde_json::to_string(&messages)
                    .map_err(|error| format!("pod message serialization failed: {error}"))?,
            ))
        }

        ("POST", path) if pod_channel_messages_path(path).is_some() => {
            let (pod_id, channel_id) = pod_channel_messages_path(path).unwrap_or_default();
            let body_text = extract_json_string_field(body, "body")
                .unwrap_or_default()
                .trim()
                .to_owned();
            let sender_peer_id = extract_json_string_field(body, "senderPeerId")
                .unwrap_or_default()
                .trim()
                .to_owned();
            if body_text.is_empty() {
                return Ok(routing::bad_request_response("Message body is required"));
            }
            if sender_peer_id.is_empty() {
                return Ok(routing::bad_request_response("SenderPeerId is required"));
            }
            let authenticated_peer_id = pod_request_peer_id(state).await;
            let Some(authenticated_peer_id) = authenticated_peer_id else {
                return Ok(routing::forbidden_response(
                    "Authenticated peer identity is required",
                ));
            };
            if sender_peer_id != authenticated_peer_id {
                return Ok(routing::forbidden_response(
                    "SenderPeerId must match the authenticated peer identity",
                ));
            }
            let signature = extract_json_string_field(body, "signature")
                .unwrap_or_default()
                .trim()
                .to_owned();
            let signature_mode = state
                .advanced_networking
                .read()
                .await
                .pod_security_signature_mode;
            if signature_mode == PodSignatureMode::Enforce && signature.is_empty() {
                return Ok(routing::bad_request_response(
                    "Message signature is required when PodCore.Security.SignatureMode is Enforce",
                ));
            }
            if signature_mode == PodSignatureMode::Warn && signature.is_empty() {
                record_daemon_log(
                    state,
                    logging::LogLevel::Warn,
                    "podcore",
                    "accepted unsigned pod message in warn mode".to_owned(),
                )
                .await;
            }
            // Use the same channel-then-pod order as parent mutations. The
            // checks and append share a turn so a queued pod update cannot
            // remove the channel and then have this request recreate an
            // orphaned message from an earlier check.
            let (binding, append_result) = {
                let mut channels = state.pod_channels.write().await;
                let pods = state.pods.read().await;
                if pods.get(&pod_id).is_none() || !pods.channel_exists(&pod_id, &channel_id) {
                    return Ok(routing::not_found_response());
                }
                if !pods.is_member(&pod_id, &authenticated_peer_id) {
                    return Ok(routing::forbidden_response("Pod membership is required"));
                }
                let binding = pods.soulseek_binding(&pod_id, &channel_id);
                let append_result = if binding.as_ref().is_some_and(|binding| binding.kind == "dm")
                {
                    None
                } else {
                    Some(channels.append(
                        pod_id.clone(),
                        channel_id.clone(),
                        authenticated_peer_id.clone(),
                        body_text.clone(),
                        signature.clone(),
                        unix_timestamp_millis(),
                    ))
                };
                (binding, append_result)
            };
            if let Some(binding) = binding.as_ref().filter(|binding| binding.kind == "dm") {
                let session_command_permit = match state.session_commands.reserve().await {
                    Ok(permit) => permit,
                    Err(_) => {
                        return Ok(routing::service_unavailable_response(
                            "session manager is not running",
                        ));
                    }
                };
                let _message_persistence = state.message_persistence_lock.lock().await;
                let mut messages = state.messages.write().await;
                let previous = messages.clone();
                let record =
                    messages.add(binding.identifier.clone(), "outbound", body_text.clone());
                let mutated = messages.clone();
                drop(messages);
                if let Err(error) = persist_message_record_checked(state, &record).await {
                    rollback_messages_if_unchanged(state, previous, &mutated).await;
                    return Ok(routing::service_unavailable_response(&error));
                }
                drop(_message_persistence);
                session_command_permit.send(SessionCommand::MessageUser {
                    username: binding.identifier.clone(),
                    body: body_text,
                });
                return Ok(routing::ok_response(
                    serde_json::json!({
                        "messageId": record.id.to_string(),
                        "sent": true,
                    })
                    .to_string(),
                ));
            }
            let result = append_result.expect("non-DM pod message is appended under the lock");
            match result {
                Ok(message) => {
                    if let Some(binding) =
                        binding.filter(|binding| binding.kind == "room" && binding.mode == "mirror")
                    {
                        let room = binding.identifier;
                        if let Err(error) = try_send_session_command(
                            state,
                            SessionCommand::SayRoom {
                                room: room.clone(),
                                body: format!("[Pod:{}] {}", message.sender_peer_id, message.body),
                            },
                        ) {
                            record_pod_room_mirror_failure(state, &room, &error).await;
                        }
                    }
                    Ok(routing::ok_response(
                        serde_json::json!({
                            "messageId": message.message_id,
                            "sent": true,
                        })
                        .to_string(),
                    ))
                }
                Err(error) if error.contains("required") || error.contains("must be at most") => {
                    Ok(routing::bad_request_response(&error))
                }
                Err(error) => {
                    eprintln!("pod channel message persistence failed: {error}");
                    Ok(routing::internal_server_error_response(
                        "Failed to send message",
                    ))
                }
            }
        }

        ("POST", "/api/pods") => {
            let value = match serde_json::from_str::<serde_json::Value>(body) {
                Ok(value) => value,
                Err(error) => {
                    return Ok(routing::bad_request_response(&format!(
                        "Invalid pod request: {error}"
                    )));
                }
            };
            let Some(pod_value) = value.get("pod").filter(|pod| !pod.is_null()).cloned() else {
                return Ok(routing::bad_request_response("Pod data is required"));
            };
            let pod = match serde_json::from_value::<pods::PodRecord>(pod_value) {
                Ok(pod) => pod,
                Err(error) => {
                    return Ok(routing::bad_request_response(&format!(
                        "Invalid pod request: {error}"
                    )));
                }
            };
            let Some(creator) = pod_request_peer_id(state).await else {
                return Ok(routing::forbidden_response(
                    "Authenticated peer identity is required",
                ));
            };
            match state.pods.write().await.create(pod, creator) {
                Ok(pod) => Ok(routing::created_response(
                    serde_json::to_string(&pod)
                        .map_err(|error| format!("pod serialization failed: {error}"))?,
                )),
                Err(error) if error == "Pod already exists" => {
                    Ok(routing::conflict_response(&error))
                }
                Err(error) if error.contains("capacity is full") => {
                    Ok(routing::conflict_response(&error))
                }
                Err(error) if error.starts_with("pod state write failed") => {
                    eprintln!("pod persistence failed: {error}");
                    Ok(routing::internal_server_error_response(
                        "Failed to create pod",
                    ))
                }
                Err(error) => Ok(routing::bad_request_response(&error)),
            }
        }

        ("GET", path)
            if pod_resource_segments(path).is_some_and(|segments| segments.len() == 1) =>
        {
            let pod_id = pod_resource_segments(path).unwrap_or_default().remove(0);
            let peer_id = pod_request_peer_id(state).await;
            let pods = state.pods.read().await;
            if let Err(error) = pods.validate_storage() {
                eprintln!("pod storage failed: {error}");
                return Ok(routing::internal_server_error_response("Failed to get pod"));
            }
            if pods.get(&pod_id).is_some()
                && !pods.is_public(&pod_id)
                && peer_id
                    .as_deref()
                    .is_none_or(|peer_id| !pods.is_member(&pod_id, peer_id))
            {
                return Ok(routing::forbidden_response("Pod membership is required"));
            }
            Ok(pods
                .get(&pod_id)
                .map(|pod| {
                    routing::ok_response(
                        serde_json::to_string(&pod).unwrap_or_else(|_| "{}".to_owned()),
                    )
                })
                .unwrap_or_else(routing::not_found_response))
        }

        ("PUT", path)
            if pod_resource_segments(path).is_some_and(|segments| segments.len() == 1) =>
        {
            let pod_id = pod_resource_segments(path).unwrap_or_default().remove(0);
            let value = match serde_json::from_str::<serde_json::Value>(body) {
                Ok(value) => value,
                Err(error) => {
                    return Ok(routing::bad_request_response(&format!(
                        "Invalid pod request: {error}"
                    )));
                }
            };
            let Some(pod_value) = value.get("pod").filter(|pod| !pod.is_null()).cloned() else {
                return Ok(routing::bad_request_response("Pod data is required"));
            };
            let pod = match serde_json::from_value::<pods::PodRecord>(pod_value) {
                Ok(pod) => pod,
                Err(error) => {
                    return Ok(routing::bad_request_response(&format!(
                        "Invalid pod request: {error}"
                    )));
                }
            };
            if pod.pod_id != pod_id {
                return Ok(routing::bad_request_response(
                    "PodId in URL must match PodId in body",
                ));
            }
            let peer_id = pod_request_peer_id(state).await;
            {
                let pods = state.pods.read().await;
                if pods.get(&pod_id).is_some()
                    && peer_id
                        .as_deref()
                        .is_none_or(|peer_id| !pods.can_moderate(&pod_id, peer_id))
                {
                    return Ok(routing::forbidden_response(
                        "Pod moderator membership is required",
                    ));
                }
                if let Some(gateway_peer_id) = pods.gateway_peer_for_update(&pod_id, &pod) {
                    if peer_id.as_deref() != Some(gateway_peer_id.as_str()) {
                        return Ok(routing::forbidden_response(
                            "Only the designated gateway peer can modify private service policy",
                        ));
                    }
                }
            }
            let proposed_channel_ids = pod
                .channels
                .iter()
                .map(|channel| channel.channel_id.trim().to_owned())
                .collect::<HashSet<_>>();
            // Acquire the channel store before the pod store, matching the
            // message append paths. Waiting on channel persistence therefore
            // does not retain the pod write lock, and all compound mutations
            // use one consistent order.
            let mut channels = state.pod_channels.write().await;
            let mut pods = state.pods.write().await;
            if pods.get(&pod_id).is_some()
                && peer_id
                    .as_deref()
                    .is_none_or(|peer_id| !pods.can_moderate(&pod_id, peer_id))
            {
                return Ok(routing::forbidden_response(
                    "Pod moderator membership is required",
                ));
            }
            if let Some(gateway_peer_id) = pods.gateway_peer_for_update(&pod_id, &pod) {
                if peer_id.as_deref() != Some(gateway_peer_id.as_str()) {
                    return Ok(routing::forbidden_response(
                        "Only the designated gateway peer can modify private service policy",
                    ));
                }
            }
            let previous_pods = pods.clone();
            let removed_channel_ids = pods
                .get(&pod_id)
                .map(|existing| {
                    existing
                        .channels
                        .into_iter()
                        .map(|channel| channel.channel_id)
                        .filter(|channel_id| !proposed_channel_ids.contains(channel_id))
                        .collect::<HashSet<_>>()
                })
                .unwrap_or_default();
            let update_result = pods.update(&pod_id, pod);
            match update_result {
                Ok(Some(pod)) => {
                    let response_body = serde_json::to_string(&pod)
                        .map_err(|error| format!("pod serialization failed: {error}"))?;
                    match channels.delete_channels(&pod_id, &removed_channel_ids) {
                        Ok(_) => Ok(routing::ok_response(response_body)),
                        Err(error) => {
                            if let Err(rollback_error) = pods.restore_snapshot(&previous_pods) {
                                eprintln!("pod update rollback failed: {rollback_error}");
                            }
                            eprintln!("pod channel cleanup failed: {error}");
                            Ok(routing::internal_server_error_response(
                                "Failed to update pod",
                            ))
                        }
                    }
                }
                Ok(None) => Ok(routing::not_found_response()),
                Err(error) => {
                    if error.starts_with("pod state write failed") {
                        eprintln!("pod persistence failed: {error}");
                        Ok(routing::internal_server_error_response(
                            "Failed to update pod",
                        ))
                    } else {
                        Ok(routing::bad_request_response(&error))
                    }
                }
            }
        }

        ("DELETE", path)
            if pod_resource_segments(path).is_some_and(|segments| segments.len() == 1) =>
        {
            let pod_id = pod_resource_segments(path).unwrap_or_default().remove(0);
            let peer_id = pod_request_peer_id(state).await;
            {
                let pods = state.pods.read().await;
                if let Err(error) = pods.validate_storage() {
                    eprintln!("pod storage failed: {error}");
                    return Ok(routing::internal_server_error_response(
                        "Failed to delete pod",
                    ));
                }
                if pods.get(&pod_id).is_none() {
                    return Ok(routing::not_found_response());
                }
                if peer_id
                    .as_deref()
                    .is_none_or(|peer_id| !pods.can_moderate(&pod_id, peer_id))
                {
                    return Ok(routing::forbidden_response(
                        "Pod moderator membership is required",
                    ));
                }
            }
            // Match the channel-first order used by pod updates. In
            // particular, do not hold the pod store while waiting for an
            // active channel append to finish.
            let mut channels = state.pod_channels.write().await;
            let mut pods = state.pods.write().await;
            if let Err(error) = pods.validate_storage() {
                eprintln!("pod storage failed: {error}");
                return Ok(routing::internal_server_error_response(
                    "Failed to delete pod",
                ));
            }
            if pods.get(&pod_id).is_none() {
                return Ok(routing::not_found_response());
            }
            if peer_id
                .as_deref()
                .is_none_or(|peer_id| !pods.can_moderate(&pod_id, peer_id))
            {
                return Ok(routing::forbidden_response(
                    "Pod moderator membership is required",
                ));
            }
            let previous_pods = pods.clone();
            match pods.delete(&pod_id) {
                Ok(true) => match channels.delete_pod(&pod_id) {
                    Ok(_) => Ok(routing::no_content_response()),
                    Err(error) => {
                        if let Err(rollback_error) = pods.restore_snapshot(&previous_pods) {
                            eprintln!("pod delete rollback failed: {rollback_error}");
                        }
                        eprintln!("pod channel cleanup failed: {error}");
                        Ok(routing::internal_server_error_response(
                            "Failed to delete pod",
                        ))
                    }
                },
                Ok(false) => Ok(routing::not_found_response()),
                Err(error) => {
                    eprintln!("pod persistence failed: {error}");
                    Ok(routing::internal_server_error_response(
                        "Failed to delete pod",
                    ))
                }
            }
        }

        ("GET", path)
            if pod_resource_segments(path)
                .is_some_and(|segments| segments.len() == 2 && segments[1] == "members") =>
        {
            let pod_id = pod_resource_segments(path).unwrap_or_default().remove(0);
            let peer_id = pod_request_peer_id(state).await;
            let pods = state.pods.read().await;
            if let Err(error) = pods.validate_storage() {
                eprintln!("pod storage failed: {error}");
                return Ok(routing::internal_server_error_response(
                    "Failed to get pod members",
                ));
            }
            if pods.get(&pod_id).is_some()
                && !pods.is_public(&pod_id)
                && peer_id
                    .as_deref()
                    .is_none_or(|peer_id| !pods.is_member(&pod_id, peer_id))
            {
                return Ok(routing::forbidden_response("Pod membership is required"));
            }
            Ok(pods
                .members(&pod_id)
                .map(|members| {
                    routing::ok_response(
                        serde_json::to_string(&members).unwrap_or_else(|_| "[]".to_owned()),
                    )
                })
                .unwrap_or_else(routing::not_found_response))
        }

        ("POST", path)
            if pod_resource_segments(path).is_some_and(|segments| {
                segments.len() == 2 && matches!(segments[1].as_str(), "join" | "leave" | "ban")
            }) =>
        {
            let segments = pod_resource_segments(path).unwrap_or_default();
            let pod_id = &segments[0];
            let action = &segments[1];
            if pods::is_gold_star_club(pod_id) && !gold_star_club_available(state) {
                return Ok(routing::not_found_response());
            }
            let peer_id = if action == "ban" {
                extract_json_string_field(body, "peerId")
                    .unwrap_or_default()
                    .trim()
                    .to_owned()
            } else {
                pod_request_peer_id(state).await.unwrap_or_default()
            };
            if peer_id.is_empty() {
                return Ok(routing::bad_request_response("PeerId is required"));
            }
            let moderator = if action == "ban" {
                pod_request_peer_id(state).await
            } else {
                None
            };
            let mut pods = state.pods.write().await;
            if action == "ban"
                && moderator
                    .as_deref()
                    .is_none_or(|moderator| !pods.can_moderate(pod_id, moderator))
            {
                return Ok(routing::forbidden_response(
                    "Pod moderator membership is required",
                ));
            }
            let result = match action.as_str() {
                "join" => pods.join(pod_id, peer_id.clone()),
                "leave" => pods.leave(pod_id, &peer_id),
                _ => pods.ban(pod_id, &peer_id),
            };
            match result {
                Ok(Some(true)) => {
                    if action == "leave" && pods::is_gold_star_club(pod_id) {
                        if let Err(error) = pods::record_gold_star_club_revocation(
                            &state.config.state_dir,
                            &peer_id,
                        ) {
                            eprintln!("Gold Star Club revocation persistence failed: {error}");
                            return Ok(routing::service_unavailable_response(
                                "pod revocation storage is unavailable",
                            ));
                        }
                    }
                    let response_key = match action.as_str() {
                        "join" => "joined",
                        "leave" => "left",
                        _ => "banned",
                    };
                    Ok(routing::ok_response(
                        serde_json::json!({ (response_key): true }).to_string(),
                    ))
                }
                Ok(Some(false)) if action == "join" => Ok(routing::bad_request_response(
                    "Failed to join pod (may already be a member)",
                )),
                Ok(Some(false)) => Ok(routing::not_found_response()),
                Ok(None) => Ok(routing::not_found_response()),
                Err(error)
                    if error.contains("capacity")
                        || error.contains("banned")
                        || error.contains("approval")
                        || error.contains("last Pod moderator") =>
                {
                    Ok(routing::bad_request_response(&error))
                }
                Err(error) if error.contains("required") || error.contains("at most") => {
                    Ok(routing::bad_request_response(&error))
                }
                Err(error) => {
                    eprintln!("pod persistence failed: {error}");
                    let message = match action.as_str() {
                        "join" => "Failed to join pod",
                        "leave" => "Failed to leave pod",
                        _ => "Failed to ban member",
                    };
                    Ok(routing::internal_server_error_response(message))
                }
            }
        }

        ("POST", path)
            if pod_resource_segments(path).is_some_and(|segments| {
                segments.len() == 4
                    && segments[1] == "channels"
                    && matches!(segments[3].as_str(), "bind" | "unbind")
            }) =>
        {
            let segments = pod_resource_segments(path).unwrap_or_default();
            let pod_id = &segments[0];
            let channel_id = &segments[2];
            let action = &segments[3];
            if action == "bind" {
                let mode = extract_json_string_field(body, "mode")
                    .unwrap_or_else(|| "readonly".to_owned())
                    .trim()
                    .to_ascii_lowercase();
                if !matches!(mode.as_str(), "readonly" | "mirror") {
                    return Ok(routing::bad_request_response(
                        "Mode must be 'readonly' or 'mirror'",
                    ));
                }
            } else if state
                .pods
                .read()
                .await
                .soulseek_binding(pod_id, channel_id)
                .is_none()
            {
                return Ok(routing::not_found_response());
            }
            let peer_id = pod_request_peer_id(state).await;
            let pods = state.pods.read().await;
            let can_moderate = peer_id
                .as_deref()
                .is_some_and(|peer_id| pods.can_moderate(pod_id, peer_id));
            drop(pods);
            if !can_moderate {
                return Ok(routing::forbidden_response(
                    "Pod moderator membership is required",
                ));
            }
            let result = if action == "bind" {
                let room_name = extract_json_string_field(body, "roomName")
                    .unwrap_or_default()
                    .trim()
                    .to_owned();
                let mode = extract_json_string_field(body, "mode")
                    .unwrap_or_else(|| "readonly".to_owned())
                    .trim()
                    .to_ascii_lowercase();
                state
                    .pods
                    .write()
                    .await
                    .bind_room(pod_id, channel_id, room_name, mode)
            } else {
                state.pods.write().await.unbind_room(pod_id, channel_id)
            };
            match result {
                Ok(Some(true)) => {
                    if action == "bind" {
                        if let Some(binding) =
                            state.pods.read().await.soulseek_binding(pod_id, channel_id)
                        {
                            let room = binding.identifier;
                            if let Err(error) = try_send_session_command(
                                state,
                                SessionCommand::JoinRoom(room.clone()),
                            ) {
                                record_room_dispatch_failure(state, "join", &room, &error).await;
                            }
                        }
                    }
                    let response_key = if action == "bind" { "bound" } else { "unbound" };
                    Ok(routing::ok_response(
                        serde_json::json!({ (response_key): true }).to_string(),
                    ))
                }
                Ok(Some(false)) | Ok(None) => Ok(routing::not_found_response()),
                Err(error) if error.contains("required") || error.starts_with("Mode must") => {
                    Ok(routing::bad_request_response(&error))
                }
                Err(error) => {
                    eprintln!("pod binding persistence failed: {error}");
                    Ok(routing::internal_server_error_response(
                        if action == "bind" {
                            "Failed to bind room"
                        } else {
                            "Failed to unbind room"
                        },
                    ))
                }
            }
        }

        ("GET", "/api/pods") => {
            let peer_id = pod_request_peer_id(state).await;
            let pods = state.pods.read().await;
            if let Err(error) = pods.validate_storage() {
                eprintln!("pod storage failed: {error}");
                return Ok(routing::internal_server_error_response(
                    "Failed to list pods",
                ));
            }
            Ok(routing::ok_response(
                serde_json::to_string(&pods.list_visible(peer_id.as_deref()))
                    .map_err(|error| format!("pod serialization failed: {error}"))?,
            ))
        }

        _ => Err(ROUTE_NOT_HANDLED.to_owned()),
    }
}
