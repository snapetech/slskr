use super::*;

pub(crate) async fn handle_incoming_soulseek_pod_message(
    state: &Arc<AppState>,
    username: &str,
    message: &str,
) -> bool {
    const POD_MESSAGE_PREFIX: &str = "PODMSG:";
    const MAX_MESSAGE_BYTES: usize = 16 * 1024;
    const MAX_JSON_BYTES: usize = 12 * 1024;
    if !message.starts_with(POD_MESSAGE_PREFIX) {
        return false;
    }
    if message.len() > MAX_MESSAGE_BYTES {
        return true;
    }
    let json_payload = &message[POD_MESSAGE_PREFIX.len()..];
    if json_payload.is_empty() || json_payload.len() > MAX_JSON_BYTES {
        return true;
    }
    let Ok(payload) = serde_json::from_str::<serde_json::Value>(json_payload) else {
        return true;
    };
    let string_field = |camel: &str, pascal: &str| {
        payload
            .get(camel)
            .or_else(|| payload.get(pascal))
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .unwrap_or_default()
    };
    let pod_id = string_field("podId", "PodId");
    let channel_id = string_field("channelId", "ChannelId");
    let sender_peer_id = string_field("senderPeerId", "SenderPeerId");
    let message_id = string_field("messageId", "MessageId");
    let body = string_field("body", "Body");
    let signature = string_field("signature", "Signature");
    let timestamp_unix_ms = payload
        .get("timestampUnixMs")
        .or_else(|| payload.get("TimestampUnixMs"))
        .and_then(serde_json::Value::as_i64)
        .unwrap_or_default();
    let sig_version = payload
        .get("sigVersion")
        .or_else(|| payload.get("SigVersion"))
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(1);
    if pod_id.is_empty()
        || channel_id.is_empty()
        || sender_peer_id.is_empty()
        || message_id.is_empty()
        || body.is_empty()
        || timestamp_unix_ms <= 0
        || sender_peer_id != format!("bridge:{username}")
        || body.len() > 10_000
    {
        return true;
    }
    let normalized = serde_json::json!({
        "messageId": message_id,
        "podId": pod_id,
        "channelId": channel_id,
        "senderPeerId": sender_peer_id,
        "body": body,
        "timestampUnixMs": timestamp_unix_ms,
        "sigVersion": sig_version,
    });
    let (canonical, canonical_pod_id) = pod_message_canonical_payload(&normalized);
    let signature_mode = state
        .advanced_networking
        .read()
        .await
        .pod_security_signature_mode;
    if !pod_verify_signature(
        &normalized,
        &canonical,
        &canonical_pod_id,
        signature,
        signature_mode,
        state,
    )
    .await
    {
        return true;
    }
    let (binding, stored) = {
        let mut channels = state.pod_channels.write().await;
        let pods = state.pods.read().await;
        if pods.get(pod_id).is_none()
            || !pods.channel_exists(pod_id, channel_id)
            || !pods.is_member(pod_id, sender_peer_id)
        {
            return true;
        }
        let binding = pods.soulseek_binding(pod_id, channel_id);
        let stored = channels.append_with_id(
            message_id.to_owned(),
            pod_id.to_owned(),
            channel_id.to_owned(),
            sender_peer_id.to_owned(),
            body.to_owned(),
            signature.to_owned(),
            u64::try_from(timestamp_unix_ms).unwrap_or_default(),
            u8::try_from(sig_version).unwrap_or(1),
        );
        (binding, stored)
    };
    let Ok(stored) = stored else {
        return true;
    };
    if let Some(binding) =
        binding.filter(|binding| binding.kind == "room" && binding.mode == "mirror")
    {
        let room = binding.identifier;
        if let Err(error) = try_send_session_command(
            state,
            SessionCommand::SayRoom {
                room: room.clone(),
                body: format!("[Pod:{}] {}", stored.sender_peer_id, stored.body),
            },
        ) {
            record_pod_room_mirror_failure(state, &room, &error).await;
        }
    }
    record_event(
        state,
        "pod.message.received",
        pod_id.to_owned(),
        Some(format!("channel={channel_id}")),
    )
    .await;
    true
}

pub(crate) async fn bridge_soulseek_room_message_to_pods(
    state: &AppState,
    room: &str,
    username: &str,
    message: &str,
) {
    let bindings = state.pods.read().await.room_bindings(room);
    for binding in bindings {
        let append_result = {
            let mut channels = state.pod_channels.write().await;
            let pods = state.pods.read().await;
            if pods.channel_exists(&binding.pod_id, &binding.channel_id)
                && pods
                    .soulseek_binding(&binding.pod_id, &binding.channel_id)
                    .as_ref()
                    == Some(&binding)
            {
                Some(channels.append(
                    binding.pod_id.clone(),
                    binding.channel_id.clone(),
                    format!("bridge:{username}"),
                    format!("[Soulseek:{username}] {message}"),
                    String::new(),
                    unix_timestamp_millis(),
                ))
            } else {
                None
            }
        };
        match append_result {
            None => continue,
            Some(Err(error)) => {
                update_session(state, |snapshot| {
                    snapshot.last_error =
                        Some(format!("pod room bridge persistence failed: {error}"));
                })
                .await;
            }
            Some(Ok(_)) => {
                record_event(
                    state,
                    "pod.message.bridged",
                    binding.pod_id,
                    Some(format!("channel={}", binding.channel_id)),
                )
                .await;
            }
        }
    }
}
