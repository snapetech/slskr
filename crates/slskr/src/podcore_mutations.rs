use super::*;

pub(crate) async fn podcore_mutation_response(
    method: &str,
    path: &str,
    query: Option<&str>,
    body: &str,
    state: &AppState,
    versioned_v0: bool,
) -> Option<HttpResponse> {
    let blank_route_value = |value: &str| decoded_path_segment(value).trim().is_empty();
    if let Some(rest) = path.strip_prefix("/api/podcore/") {
        let segments = rest.split('/').collect::<Vec<_>>();
        match (method, segments.as_slice()) {
            ("DELETE", [pod_id, "channels", channel_id])
            | ("PUT", [pod_id, "channels", channel_id])
                if blank_route_value(pod_id) || blank_route_value(channel_id) =>
            {
                return Some(routing::bad_request_response(
                    if blank_route_value(pod_id) {
                        "Pod ID is required"
                    } else {
                        "Channel ID is required"
                    },
                ));
            }
            ("DELETE", ["discovery", "unregister", pod_id]) if blank_route_value(pod_id) => {
                return Some(routing::bad_request_response("PodId is required"));
            }
            ("DELETE", ["membership", pod_id, peer_id])
            | ("DELETE", ["membership", "join", pod_id, peer_id])
            | ("DELETE", ["membership", "leave", pod_id, peer_id])
                if blank_route_value(pod_id) || blank_route_value(peer_id) =>
            {
                return Some(routing::bad_request_response(
                    "PodId and PeerId are required",
                ));
            }
            ("DELETE", ["messages", pod_id, channel_id, "cleanup"])
            | ("PUT", ["backfill", pod_id, channel_id, "last-seen"])
                if blank_route_value(pod_id) || blank_route_value(channel_id) =>
            {
                return Some(routing::bad_request_response(
                    if blank_route_value(pod_id) {
                        "Pod ID is required"
                    } else {
                        "Channel ID is required"
                    },
                ));
            }
            ("POST", ["membership", pod_id, peer_id, action])
                if matches!(*action, "ban" | "unban" | "role")
                    && (blank_route_value(pod_id) || blank_route_value(peer_id)) =>
            {
                return Some(routing::bad_request_response(
                    "PodId and PeerId are required",
                ));
            }
            ("POST", ["membership", pod_id, "members"]) if blank_route_value(pod_id) => {
                return Some(routing::bad_request_response(
                    "Valid podId and member with PeerId are required",
                ));
            }
            ("POST", [pod_id, "opinions", "refresh"])
            | ("POST", [pod_id, "opinions", "members", "affinity", "update"])
                if blank_route_value(pod_id) =>
            {
                return Some(routing::bad_request_response("Pod ID is required"));
            }
            ("PUT", ["membership", pod_id, "members", peer_id])
                if blank_route_value(pod_id) || blank_route_value(peer_id) =>
            {
                return Some(routing::bad_request_response(
                    "Valid podId, peerId, and member are required",
                ));
            }
            ("POST", ["routing", "seen", message_id, pod_id])
                if blank_route_value(message_id) || blank_route_value(pod_id) =>
            {
                return Some(routing::bad_request_response(
                    "MessageId and PodId are required",
                ));
            }
            _ => {}
        }
    }
    match (method, path) {
        ("POST", "/api/podcore/dht/refresh/") | ("DELETE", "/api/podcore/dht/unpublish/") => {
            return Some(routing::bad_request_response("Pod ID is required"));
        }
        _ => {}
    }
    let segments = decoded_segments_after(path, "/api/podcore/")?;
    match (method, segments.as_slice()) {
        ("POST", [pod_id, section]) if section == "opinions" => {
            let pod_id = pod_id.trim();
            if pod_id.is_empty() {
                return Some(routing::bad_request_response("Pod ID is required"));
            }
            let mut opinion = match serde_json::from_str::<serde_json::Value>(body) {
                Ok(opinion @ serde_json::Value::Object(_)) => opinion,
                Ok(_) => return Some(routing::bad_request_response("opinion must be an object")),
                Err(_) => return Some(routing::bad_request_response("invalid JSON body")),
            };
            let content_id = opinion
                .get("contentId")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .unwrap_or_default()
                .to_owned();
            let variant_hash = opinion
                .get("variantHash")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .unwrap_or_default()
                .to_owned();
            if content_id.is_empty() || variant_hash.is_empty() {
                return Some(routing::bad_request_response(
                    "Content ID and variant hash are required",
                ));
            }
            let signature_valid = opinion
                .get("signature")
                .and_then(serde_json::Value::as_str)
                .and_then(|signature| signature.strip_prefix("ed25519:"))
                .and_then(|signature| STANDARD.decode(signature.as_bytes()).ok())
                .is_some_and(|signature| signature.len() == 64);
            if !signature_valid {
                return Some(HttpResponse {
                    status: "400 Bad Request",
                    content_type: "application/json",
                    body: serde_json::json!("Opinion could not be published").to_string(),
                });
            }
            opinion["variantHash"] = serde_json::json!(variant_hash);
            let id = opinion
                .get("id")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
            opinion["id"] = serde_json::json!(id);
            opinion["podId"] = serde_json::json!(pod_id);
            opinion["contentId"] = serde_json::json!(content_id);
            opinion["createdAt"] = serde_json::json!(unix_timestamp());
            let key = format!("pod/opinion/{pod_id}/{content_id}/{id}");
            let published_opinion = serde_json::json!({
                "contentId": content_id,
                "variantHash": variant_hash,
                "score": opinion.get("score").cloned().unwrap_or_else(|| serde_json::json!(0.0)),
                "note": opinion.get("note").and_then(serde_json::Value::as_str).unwrap_or_default(),
                "senderPeerId": opinion
                    .get("senderPeerId")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default(),
                "signature": opinion
                    .get("signature")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default(),
            });
            Some(
                match state.controller_features.upsert(key, opinion.clone()).await {
                    Ok(()) => routing::ok_response(
                        serde_json::json!({
                            "success": true,
                            "podId": pod_id,
                            "contentId": content_id,
                            "variantHash": variant_hash,
                            "publishedOpinion": published_opinion,
                        })
                        .to_string(),
                    ),
                    Err(error) => podcore_storage_error_response(
                        state,
                        versioned_v0,
                        "An error occurred while publishing the opinion",
                        &error,
                    ),
                },
            )
        }
        ("POST", [pod_id, section, members, affinity, update])
            if section == "opinions"
                && members == "members"
                && affinity == "affinity"
                && update == "update" =>
        {
            // slskR computes affinities fresh on every GET rather than
            // caching them (see pod_member_affinities_json), so "update"
            // has no separate cache to invalidate; it real-computes them
            // once here and reports the real count, matching the oracle's
            // UpdateMemberAffinitiesAsync contract without a fake 0.
            let pod_id = pod_id.trim();
            if pod_id.is_empty() {
                return Some(routing::bad_request_response("Pod ID is required"));
            }
            if versioned_v0
                && state.config.controller_profile == ControllerProfile::Native
                && state
                    .controller_features
                    .read()
                    .await
                    .validate_storage()
                    .is_err()
            {
                return Some(routing::internal_server_error_response(
                    "An error occurred while updating member affinities",
                ));
            }
            let started_at = std::time::Instant::now();
            let affinities = pod_member_affinities_json(state, pod_id).await;
            let members_updated = affinities.as_object().map_or(0, serde_json::Map::len);
            Some(routing::ok_response(
                serde_json::json!({
                    "success": true,
                    "podId": pod_id,
                    "membersUpdated": members_updated,
                    "duration": format_timespan_hms(started_at.elapsed().as_secs() as i64),
                })
                .to_string(),
            ))
        }
        ("POST", [pod_id, section, refresh]) if section == "opinions" && refresh == "refresh" => {
            let pod_id = pod_id.trim();
            if pod_id.is_empty() {
                return Some(routing::bad_request_response("Pod ID is required"));
            }
            if versioned_v0
                && state.config.controller_profile == ControllerProfile::Native
                && state
                    .controller_features
                    .read()
                    .await
                    .validate_storage()
                    .is_err()
            {
                return Some(routing::internal_server_error_response(
                    "An error occurred while refreshing opinions",
                ));
            }
            let started_at = std::time::Instant::now();
            let opinions = state
                .controller_features
                .read()
                .await
                .values_with_prefix(&format!("pod/opinion/{pod_id}/"));
            let new_opinions = state
                .podcore_runtime_stats
                .record_opinion_refresh(pod_id, opinions.len());
            Some(routing::ok_response(
                serde_json::json!({
                    "success": true,
                    "podId": pod_id,
                    "opinionsRefreshed": opinions.len(),
                    "newOpinions": new_opinions,
                    "duration": format_timespan_hms(started_at.elapsed().as_secs() as i64),
                })
                .to_string(),
            ))
        }
        ("POST", [pod_id, section]) if section == "channels" => {
            // Matches the frozen native profile contract: creating a pod channel
            // requires the acting peer to moderate the pod.
            if !pod_local_peer_can_moderate(state, pod_id).await {
                return Some(routing::forbidden_response(
                    "only a pod moderator may create channels",
                ));
            }
            let mut value = match serde_json::from_str::<serde_json::Value>(body) {
                Ok(serde_json::Value::Object(value)) => serde_json::Value::Object(value),
                Ok(_) => {
                    return Some(routing::bad_request_response(
                        "channel body must be an object",
                    ));
                }
                Err(_) => return Some(routing::bad_request_response("invalid JSON body")),
            };
            let Some(channel_id) = value
                .get("channelId")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
            else {
                return Some(routing::bad_request_response("Channel ID is required"));
            };
            let Some(name) = value
                .get("name")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
            else {
                return Some(routing::bad_request_response("Channel name is required"));
            };
            value["channelId"] = serde_json::json!(channel_id);
            value["name"] = serde_json::json!(name);
            let channel = match serde_json::from_value::<pods::PodChannel>(value) {
                Ok(channel) => channel,
                Err(error) => {
                    return Some(routing::bad_request_response(&format!(
                        "invalid channel: {error}"
                    )));
                }
            };
            let response = state.pods.write().await.upsert_channel(pod_id, channel);
            Some(match response {
                Ok(Some(channel)) => {
                    routing::created_response(serde_json::json!(channel).to_string())
                }
                Ok(None) => routing::not_found_response(),
                Err(error) if error.starts_with("pod state write failed:") => {
                    routing::internal_server_error_response(
                        "An error occurred while creating the channel",
                    )
                }
                Err(error) => routing::bad_request_response(&error),
            })
        }
        ("PUT", [pod_id, section, channel_id]) if section == "channels" => {
            // Matches the frozen native profile contract: updating a pod channel
            // requires the acting peer to moderate the pod.
            if !pod_local_peer_can_moderate(state, pod_id).await {
                return Some(routing::forbidden_response(
                    "only a pod moderator may update channels",
                ));
            }
            let mut value = match serde_json::from_str::<serde_json::Value>(body) {
                Ok(serde_json::Value::Object(value)) => serde_json::Value::Object(value),
                Ok(_) => {
                    return Some(routing::bad_request_response(
                        "channel body must be an object",
                    ));
                }
                Err(_) => return Some(routing::bad_request_response("invalid JSON body")),
            };
            value["channelId"] = serde_json::json!(channel_id);
            let Some(name) = value
                .get("name")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
            else {
                return Some(routing::bad_request_response("Channel name is required"));
            };
            value["name"] = serde_json::json!(name);
            let channel = match serde_json::from_value::<pods::PodChannel>(value) {
                Ok(channel) => channel,
                Err(error) => {
                    return Some(routing::bad_request_response(&format!(
                        "invalid channel: {error}"
                    )));
                }
            };
            let existing = state.pods.read().await.get(pod_id);
            let Some(existing) = existing else {
                return Some(routing::not_found_response());
            };
            let Some(existing_channel) = existing
                .channels
                .iter()
                .find(|existing| existing.channel_id == *channel_id)
            else {
                return Some(routing::not_found_response());
            };
            if existing_channel.kind.as_i64() == Some(0)
                && existing_channel.name.eq_ignore_ascii_case("general")
                && (channel.kind.as_i64() != Some(0)
                    || !channel.name.eq_ignore_ascii_case("general"))
            {
                return Some(routing::bad_request_response("Invalid channel request"));
            }
            let response = state.pods.write().await.upsert_channel(pod_id, channel);
            Some(match response {
                Ok(Some(_channel)) => routing::ok_response(String::new()),
                Ok(None) => routing::not_found_response(),
                Err(error) if error.starts_with("pod state write failed:") => {
                    routing::internal_server_error_response(
                        "An error occurred while updating the channel",
                    )
                }
                Err(error) => routing::bad_request_response(&error),
            })
        }
        ("DELETE", [pod_id, section, channel_id]) if section == "channels" => {
            // Matches the frozen native profile contract: deleting a pod channel
            // requires the acting peer to moderate the pod.
            if !pod_local_peer_can_moderate(state, pod_id).await {
                return Some(routing::forbidden_response(
                    "only a pod moderator may delete channels",
                ));
            }
            let existing = state.pods.read().await.get(pod_id);
            let Some(existing) = existing else {
                return Some(routing::not_found_response());
            };
            let Some(existing_channel) = existing
                .channels
                .iter()
                .find(|existing| existing.channel_id == *channel_id)
            else {
                return Some(routing::not_found_response());
            };
            if existing_channel.kind.as_i64() == Some(0)
                && existing_channel.name.eq_ignore_ascii_case("general")
            {
                return Some(routing::bad_request_response("Invalid channel request"));
            }
            let response = state.pods.write().await.remove_channel(pod_id, channel_id);
            if matches!(response, Ok(Some(true))) {
                let mut channels = HashSet::new();
                channels.insert(channel_id.clone());
                if let Err(error) = state
                    .pod_channels
                    .write()
                    .await
                    .delete_channels(pod_id, &channels)
                {
                    return Some(routing::service_unavailable_response(&error));
                }
            }
            Some(match response {
                Ok(Some(true)) => routing::ok_response(String::new()),
                Ok(Some(false)) | Ok(None) => routing::not_found_response(),
                Err(_error) => routing::internal_server_error_response(
                    "An error occurred while deleting the channel",
                ),
            })
        }
        ("POST", [section, pod_id, peer_id, action])
            if section == "membership" && matches!(action.as_str(), "ban" | "unban" | "role") =>
        {
            // Matches the frozen native profile contract: BanMember/UnbanMember/
            // ChangeRole all require the acting peer to moderate the pod --
            // there is no self-service path for any of these three.
            if !pod_local_peer_can_moderate(state, pod_id).await {
                return Some(routing::forbidden_response(
                    "only a pod moderator may perform this action",
                ));
            }
            let response = {
                let mut pods = state.pods.write().await;
                match action.as_str() {
                    "ban" => pods.ban(pod_id, peer_id),
                    "unban" => pods.unban(pod_id, peer_id),
                    "role" => {
                        let role = extract_json_string_field(body, "role").unwrap_or_default();
                        if role.trim().is_empty() {
                            return Some(routing::bad_request_response("role is required"));
                        }
                        pods.set_member_role(pod_id, peer_id, &role)
                            .map(|value| value.map(|_| true))
                    }
                    _ => unreachable!(),
                }
            };
            Some(match response {
                Ok(Some(_)) => {
                    let member = state
                        .pods
                        .read()
                        .await
                        .member_for_verification(pod_id, peer_id);
                    let Some(member) = member else {
                        return Some(routing::internal_server_error_response(
                            "Failed to update membership",
                        ));
                    };
                    let now = unix_timestamp_millis();
                    let membership_action = if member.is_banned { "ban" } else { "join" };
                    let signed_record = pod_membership_signed_record(
                        state,
                        pod_id,
                        &member,
                        membership_action,
                        now,
                    );
                    let expires_at = chrono::DateTime::from_timestamp_millis(
                        i64::try_from(now).unwrap_or(i64::MAX),
                    )
                    .unwrap_or_else(chrono::Utc::now)
                        + chrono::Duration::hours(24);
                    let key = format!("pod/membership/{pod_id}/{peer_id}");
                    if let Err(error) = state
                        .controller_features
                        .upsert(key, signed_record.clone())
                        .await
                    {
                        return Some(routing::service_unavailable_response(&error));
                    }
                    routing::ok_response(
                        serde_json::json!({
                            "success": true,
                            "podId": pod_id,
                            "peerId": peer_id,
                            "dhtKey": format!("pod:{pod_id}:member:{peer_id}"),
                            "publishedAt": chrono::DateTime::from_timestamp_millis(
                                i64::try_from(now).unwrap_or(i64::MAX),
                            )
                            .unwrap_or_else(chrono::Utc::now)
                            .to_rfc3339(),
                            "expiresAt": expires_at.to_rfc3339(),
                        })
                        .to_string(),
                    )
                }
                Ok(None) => HttpResponse {
                    status: "500 Internal Server Error",
                    content_type: "application/json",
                    body: serde_json::json!({"error": "Failed to update membership"}).to_string(),
                },
                Err(_error) => routing::internal_server_error_response(match action.as_str() {
                    "ban" => "Failed to ban member",
                    "unban" => "Failed to unban member",
                    "role" => "Failed to change role",
                    _ => "Failed to update membership",
                }),
            })
        }
        ("POST", [section, pod_id, members]) if section == "membership" && members == "members" => {
            let mut member = match serde_json::from_str::<pods::PodMember>(body) {
                Ok(member) => member,
                Err(error) => {
                    return Some(routing::bad_request_response(&format!(
                        "invalid member: {error}"
                    )));
                }
            };
            // Matches the frozen native profile contract: a peer may only publish
            // membership for themselves, and role/ban state always come
            // from the existing record (or safe defaults), never the
            // request body -- otherwise any caller could self-assign a
            // moderator role or clear their own ban.
            let acting_peer_id = pod_request_peer_id(state).await;
            if acting_peer_id
                .as_deref()
                .is_none_or(|acting| !acting.eq_ignore_ascii_case(&member.peer_id))
            {
                return Some(routing::forbidden_response(
                    "a peer may only publish their own membership",
                ));
            }
            let existing = state
                .pods
                .read()
                .await
                .member_for_verification(pod_id, &member.peer_id);
            member.role = existing
                .as_ref()
                .map(|existing| existing.role.clone())
                .unwrap_or_else(|| "member".to_owned());
            member.is_banned = existing.as_ref().is_some_and(|existing| existing.is_banned);
            let response = state.pods.write().await.upsert_member(pod_id, member);
            Some(match response {
                Ok(Some(member)) => {
                    let now = unix_timestamp_millis();
                    let signed_record =
                        pod_membership_signed_record(state, pod_id, &member, "join", now);
                    if let Err(error) = state
                        .controller_features
                        .upsert(
                            format!("pod/membership/{pod_id}/{}", member.peer_id),
                            signed_record,
                        )
                        .await
                    {
                        return Some(routing::service_unavailable_response(&error));
                    }
                    routing::ok_response(
                        pod_membership_publish_result(pod_id, &member.peer_id, now).to_string(),
                    )
                }
                Ok(None) => routing::not_found_response(),
                Err(_error) => {
                    routing::internal_server_error_response("Failed to publish membership")
                }
            })
        }
        ("PUT", [section, pod_id, members, peer_id])
            if section == "membership" && members == "members" =>
        {
            let mut value = match serde_json::from_str::<serde_json::Value>(body) {
                Ok(serde_json::Value::Object(value)) => serde_json::Value::Object(value),
                Ok(_) => {
                    return Some(routing::bad_request_response(
                        "member body must be an object",
                    ));
                }
                Err(_) => return Some(routing::bad_request_response("invalid JSON body")),
            };
            value["peerId"] = serde_json::json!(peer_id);
            let mut member = match serde_json::from_value::<pods::PodMember>(value) {
                Ok(member) => member,
                Err(error) => {
                    return Some(routing::bad_request_response(&format!(
                        "invalid member: {error}"
                    )));
                }
            };
            // Matches the frozen native profile contract: updating a membership
            // record requires moderating the pod, or the member updating
            // their own record. A non-moderator's role/ban fields in the
            // body are ignored and pinned to the existing record (or safe
            // defaults) so a self-update can't grant a role or clear a ban.
            let acting_peer_id = pod_request_peer_id(state).await;
            let existing = state
                .pods
                .read()
                .await
                .member_for_verification(pod_id, peer_id);
            let can_moderate = pod_local_peer_can_moderate(state, pod_id).await;
            let is_self = acting_peer_id
                .as_deref()
                .is_some_and(|acting| acting.eq_ignore_ascii_case(peer_id));
            if !can_moderate && !is_self {
                return Some(routing::forbidden_response(
                    "only a pod moderator or the member themselves may update this membership",
                ));
            }
            if !can_moderate {
                member.role = existing
                    .as_ref()
                    .map(|existing| existing.role.clone())
                    .unwrap_or_else(|| "member".to_owned());
                member.is_banned = existing.as_ref().is_some_and(|existing| existing.is_banned);
            }
            let response = state.pods.write().await.upsert_member(pod_id, member);
            Some(match response {
                Ok(Some(member)) => {
                    let now = unix_timestamp_millis();
                    let signed_record =
                        pod_membership_signed_record(state, pod_id, &member, "join", now);
                    if let Err(error) = state
                        .controller_features
                        .upsert(
                            format!("pod/membership/{pod_id}/{}", member.peer_id),
                            signed_record,
                        )
                        .await
                    {
                        return Some(routing::service_unavailable_response(&error));
                    }
                    routing::ok_response(
                        pod_membership_publish_result(pod_id, &member.peer_id, now).to_string(),
                    )
                }
                Ok(None) => routing::not_found_response(),
                Err(_error) => {
                    routing::internal_server_error_response("Failed to update membership")
                }
            })
        }
        ("DELETE", [section, pod_id, peer_id]) if section == "membership" => {
            // Matches the frozen native profile contract: removing a membership
            // record requires either moderating the pod, or the acting
            // peer removing themselves. The acting peer is always this
            // instance's own configured Soulseek identity (as with every
            // other podcore mutation, see `pod_request_peer_id`) -- never a
            // client-supplied parameter, which would let any caller assert
            // an arbitrary identity and bypass the check entirely.
            let acting_peer_id = pod_request_peer_id(state).await;
            let is_self_leave = acting_peer_id
                .as_deref()
                .is_some_and(|acting| acting.eq_ignore_ascii_case(peer_id.as_str()));
            let can_moderate = pod_local_peer_can_moderate(state, pod_id).await;
            if !is_self_leave && !can_moderate {
                return Some(routing::forbidden_response(
                    "only a pod moderator or the member themselves may remove this membership",
                ));
            }
            if let Err(_error) = state.pods.write().await.leave(pod_id, peer_id) {
                return Some(routing::internal_server_error_response(
                    "Failed to remove membership",
                ));
            }
            let now = chrono::Utc::now();
            let value = serde_json::json!({
                "success": true,
                "podId": pod_id,
                "peerId": peer_id,
                "dhtKey": format!("pod:{pod_id}:member:{peer_id}"),
                "publishedAt": now.to_rfc3339(),
                "expiresAt": (now + chrono::Duration::hours(24)).to_rfc3339(),
            });
            Some(
                match state
                    .controller_features
                    .upsert(
                        format!("pod/membership-tombstone/{pod_id}/{peer_id}"),
                        value.clone(),
                    )
                    .await
                {
                    Ok(()) => routing::ok_response(value.to_string()),
                    Err(error) => routing::service_unavailable_response(&error),
                },
            )
        }
        ("POST", [section, cleanup]) if section == "membership" && cleanup == "cleanup" => {
            if state.pods.read().await.validate_storage().is_err() {
                return Some(routing::internal_server_error_response(
                    "Failed to cleanup expired memberships",
                ));
            }
            let removed = state.pod_membership_workflow.write().await.clear_pending();
            Some(routing::ok_response(
                serde_json::json!({
                    "recordsCleaned": removed,
                    "errorsEncountered": 0,
                    "completedAt": chrono::Utc::now().to_rfc3339(),
                })
                .to_string(),
            ))
        }
        ("DELETE", [section, cleanup]) if section == "messages" && cleanup == "cleanup" => {
            let older_than =
                query_parameter(query, "olderThan").and_then(|value| value.parse::<u64>().ok());
            let Some(older_than) = older_than.filter(|value| *value > 0) else {
                return Some(routing::bad_request_response(
                    "olderThan timestamp must be positive",
                ));
            };
            let response = state
                .pod_channels
                .write()
                .await
                .delete_older_than(older_than, None, None);
            Some(match response {
                Ok(removed) => routing::ok_response(removed.len().to_string()),
                Err(_error) => routing::internal_server_error_response(
                    "An error occurred while cleaning up messages",
                ),
            })
        }
        ("DELETE", [section, pod_id, channel_id, cleanup])
            if section == "messages" && cleanup == "cleanup" =>
        {
            let older_than =
                query_parameter(query, "olderThan").and_then(|value| value.parse::<u64>().ok());
            let Some(older_than) = older_than.filter(|value| *value > 0) else {
                return Some(routing::bad_request_response(
                    "olderThan timestamp must be positive",
                ));
            };
            let response = state.pod_channels.write().await.delete_older_than(
                older_than,
                Some(pod_id),
                Some(channel_id),
            );
            Some(match response {
                Ok(removed) => routing::ok_response(removed.len().to_string()),
                Err(_error) => routing::internal_server_error_response(
                    "An error occurred while cleaning up channel messages",
                ),
            })
        }
        ("PUT", [section, pod_id, channel_id, last_seen])
            if section == "backfill" && last_seen == "last-seen" =>
        {
            let timestamp = extract_json_u64_field(body, "lastSeen")
                .or_else(|| extract_json_u64_field(body, "timestamp"))
                .or_else(|| serde_json::from_str::<u64>(body).ok());
            let Some(timestamp) = timestamp else {
                return Some(routing::bad_request_response("lastSeen is required"));
            };
            let value = serde_json::json!({
                "podId": pod_id, "channelId": channel_id, "lastSeen": timestamp
            });
            let key = format!("pod/backfill/{pod_id}/{channel_id}");
            Some(match state.controller_features.upsert(key, value).await {
                Ok(()) => routing::HttpResponse {
                    status: "200 OK",
                    content_type: "",
                    body: String::new(),
                },
                Err(error) => podcore_storage_error_response(
                    state,
                    versioned_v0,
                    "An error occurred while updating last seen timestamp",
                    &error,
                ),
            })
        }
        ("POST", [section, pod_id, sync]) if section == "backfill" && sync == "sync" => {
            let last_seen = match serde_json::from_str::<serde_json::Value>(body) {
                Ok(serde_json::Value::Object(values)) if !values.is_empty() => values,
                _ => {
                    return Some(routing::bad_request_response(
                        "Last seen timestamps are required",
                    ));
                }
            };
            if last_seen.iter().any(|(channel_id, timestamp)| {
                channel_id.trim().is_empty()
                    || timestamp.as_i64().is_none_or(|timestamp| timestamp <= 0)
            }) {
                return Some(routing::bad_request_response(
                    "Each last seen timestamp requires a non-empty channel ID and positive timestamp",
                ));
            }
            let last_seen = last_seen
                .into_iter()
                .map(|(channel_id, timestamp)| (channel_id, timestamp.as_u64().unwrap_or_default()))
                .collect::<BTreeMap<_, _>>();
            if versioned_v0
                && state.config.controller_profile == ControllerProfile::Native
                && (state.pods.read().await.validate_storage().is_err()
                    || state.pod_channels.read().await.validate_storage().is_err())
            {
                return Some(routing::internal_server_error_response(
                    "An error occurred while syncing backfill",
                ));
            }
            let Some(result) = podcore_local_backfill_response(state, pod_id, &last_seen).await
            else {
                return Some(routing::not_found_response());
            };
            Some(routing::ok_response(result.to_string()))
        }
        ("POST", [section, sync_all]) if section == "backfill" && sync_all == "sync-all" => {
            if versioned_v0
                && state.config.controller_profile == ControllerProfile::Native
                && (state.pods.read().await.validate_storage().is_err()
                    || state.pod_channels.read().await.validate_storage().is_err())
            {
                return Some(routing::internal_server_error_response(
                    "An error occurred while syncing all pods",
                ));
            }
            let local_peer_id = pod_request_peer_id(state).await.unwrap_or_default();
            let pods_store = state.pods.read().await;
            let pods = pods_store.list_visible(Some(&local_peer_id));
            let features = state.controller_features.read().await;
            let work = pods
                .iter()
                .filter(|pod| pods_store.is_member(&pod.pod_id, &local_peer_id))
                .filter_map(|pod| {
                    let last_seen = pod
                        .channels
                        .iter()
                        .filter_map(|channel| {
                            features
                                .get(&format!(
                                    "pod/backfill/{}/{}",
                                    pod.pod_id, channel.channel_id
                                ))
                                .and_then(|value| value.get("lastSeen"))
                                .and_then(serde_json::Value::as_u64)
                                .filter(|timestamp| *timestamp > 0)
                                .map(|timestamp| (channel.channel_id.clone(), timestamp))
                        })
                        .collect::<BTreeMap<_, _>>();
                    (!last_seen.is_empty()).then(|| (pod.pod_id.clone(), last_seen))
                })
                .collect::<Vec<_>>();
            drop(features);
            drop(pods_store);
            let mut results = Vec::with_capacity(work.len());
            for (pod_id, last_seen) in work {
                if let Some(result) =
                    podcore_local_backfill_response(state, &pod_id, &last_seen).await
                {
                    results.push(result);
                }
            }
            Some(routing::ok_response(
                serde_json::Value::Array(results).to_string(),
            ))
        }
        ("POST", [section, action]) if section == "content" && action == "validate" => {
            let content_id = match serde_json::from_str::<String>(body) {
                Ok(content_id) => content_id.trim().to_owned(),
                Err(_) => {
                    return Some(routing::bad_request_response(
                        "Content ID must be a JSON string",
                    ));
                }
            };
            if content_id.is_empty() {
                return Some(routing::bad_request_response("Content ID is required"));
            }
            let parts = content_id.split(':').collect::<Vec<_>>();
            let valid = parts.len() == 4
                && parts[0].eq_ignore_ascii_case("content")
                && parts[1..].iter().all(|part| !part.trim().is_empty());
            if !valid {
                return Some(routing::ok_response(
                    serde_json::json!({
                        "isValid": false,
                        "contentId": content_id,
                        "errorMessage": "Invalid content ID format. Expected: content:<domain>:<type>:<id>",
                    })
                    .to_string(),
                ));
            }
            Some(routing::ok_response(
                serde_json::json!({
                    "isValid": true,
                    "contentId": content_id,
                    "metadata": {
                        "contentId": content_id,
                        "title": format!("{}: {}", parts[2], parts[3]),
                        "artist": "Unknown",
                        "type": parts[2],
                        "domain": parts[1],
                        "additionalInfo": {
                            "id": parts[3],
                            "domain": parts[1],
                            "type": parts[2],
                        },
                    },
                })
                .to_string(),
            ))
        }
        ("POST", [section, action]) if section == "content" && action == "create-pod" => {
            let payload = match serde_json::from_str::<serde_json::Value>(body) {
                Ok(payload @ serde_json::Value::Object(_)) => payload,
                Ok(_) => {
                    return Some(routing::bad_request_response(
                        "pod content must be an object",
                    ));
                }
                Err(_) => return Some(routing::bad_request_response("invalid JSON body")),
            };
            let wrapped_pod = payload.get("pod").is_some();
            let mut pod_value = payload
                .get("pod")
                .cloned()
                .unwrap_or_else(|| payload.clone());
            if !wrapped_pod {
                let pod_id = pod_value
                    .get("podId")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default();
                let valid_pod_id = pod_id.strip_prefix("pod:").is_some_and(|suffix| {
                    suffix.len() == 32
                        && suffix
                            .bytes()
                            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
                });
                if !valid_pod_id {
                    return Some(routing::bad_request_response(
                        "Invalid content-linked pod request",
                    ));
                }
                let content_id = pod_value
                    .get("contentId")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_owned();
                let valid_content_id = content_id.split(':').collect::<Vec<_>>();
                if valid_content_id.len() != 4
                    || !valid_content_id[0].eq_ignore_ascii_case("content")
                    || valid_content_id[1..]
                        .iter()
                        .any(|part| part.trim().is_empty())
                {
                    return Some(routing::bad_request_response(
                        "Invalid content-linked pod request",
                    ));
                }
                pod_value["focusContentId"] = serde_json::json!(content_id);
            }
            let pod = match serde_json::from_value::<pods::PodRecord>(pod_value) {
                Ok(pod) => pod,
                Err(error) => {
                    return Some(routing::bad_request_response(&format!(
                        "invalid pod: {error}"
                    )));
                }
            };
            if action == "validate" {
                return Some(routing::ok_response(
                    serde_json::json!({
                        "valid": !pod.pod_id.trim().is_empty() && !pod.name.trim().is_empty(),
                        "podId": pod.pod_id,
                        "channelCount": pod.channels.len(),
                    })
                    .to_string(),
                ));
            }
            let creator = payload
                .get("requestingPeerId")
                .or_else(|| payload.get("creatorPeerId"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
                .or(pod_request_peer_id(state).await)
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "local".to_owned());
            Some(match state.pods.write().await.create(pod, creator) {
                Ok(pod) => routing::created_response(serde_json::json!(pod).to_string()),
                Err(error) if error == "Pod already exists" => routing::conflict_response(&error),
                Err(error)
                    if versioned_v0
                        && state.config.controller_profile == ControllerProfile::Native
                        && error.starts_with("pod state write failed") =>
                {
                    routing::internal_server_error_response(
                        "An error occurred while creating the pod",
                    )
                }
                Err(error) => routing::bad_request_response(&error),
            })
        }
        ("POST", [section, action])
            if section == "messages" && matches!(action.as_str(), "rebuild-index" | "vacuum") =>
        {
            let result = {
                let mut messages = state.pod_channels.write().await;
                if action == "rebuild-index" {
                    messages.rebuild_search_index()
                } else {
                    messages.vacuum()
                }
            };
            Some(match result {
                Ok(success) => routing::ok_response(success.to_string()),
                Err(_error) if action == "rebuild-index" => {
                    routing::internal_server_error_response(
                        "An error occurred while rebuilding search index",
                    )
                }
                Err(_error) => routing::internal_server_error_response(
                    "An error occurred while vacuuming database",
                ),
            })
        }
        ("POST", [section, action])
            if section == "routing"
                && matches!(action.as_str(), "cleanup" | "route" | "route-to-peers") =>
        {
            if action == "cleanup" {
                const ROUTING_WINDOW_MILLIS: u64 = 24 * 60 * 60 * 1_000;
                let started_at = std::time::Instant::now();
                let cutoff = unix_timestamp_millis().saturating_sub(ROUTING_WINDOW_MILLIS);
                let prefix = "pod/routing-seen/";
                let entries = state
                    .controller_features
                    .read()
                    .await
                    .entries_with_prefix(prefix);
                let expired_keys = entries
                    .iter()
                    .filter_map(|(key, value)| {
                        let seen_at = value.get("seenAt").and_then(serde_json::Value::as_u64);
                        (seen_at.is_none_or(|seen_at| seen_at < cutoff)).then(|| key.clone())
                    })
                    .collect::<Vec<_>>();
                let retained_before_cleanup = entries.len().saturating_sub(expired_keys.len());
                let removed = match state.controller_features.remove_keys(&expired_keys).await {
                    Ok(removed) => removed,
                    Err(error) => {
                        return Some(podcore_storage_error_response(
                            state,
                            versioned_v0,
                            "Failed to cleanup seen messages",
                            &error,
                        ));
                    }
                };
                return Some(routing::ok_response(
                    serde_json::json!({
                        "messagesCleaned": removed,
                        "messagesRetained": retained_before_cleanup,
                        "cleanupDuration": format_timespan_hms(started_at.elapsed().as_secs() as i64),
                        "completedAt": chrono::Utc::now().to_rfc3339(),
                    })
                    .to_string(),
                ));
            }
            let payload = match serde_json::from_str::<serde_json::Value>(body) {
                Ok(payload @ serde_json::Value::Object(_)) => payload,
                Ok(_) => {
                    return Some(routing::bad_request_response(
                        "routing body must be an object",
                    ));
                }
                Err(_) => return Some(routing::bad_request_response("invalid JSON body")),
            };
            let started_at = std::time::Instant::now();

            if action == "route-to-peers" {
                let message = match payload.get("message") {
                    Some(message @ serde_json::Value::Object(_)) => message,
                    _ => {
                        return Some(routing::bad_request_response(
                            "Valid message and target peer IDs are required",
                        ));
                    }
                };
                let message_id = message
                    .get("messageId")
                    .and_then(serde_json::Value::as_str)
                    .map(str::trim)
                    .unwrap_or_default();
                let channel_id = message
                    .get("channelId")
                    .and_then(serde_json::Value::as_str)
                    .map(str::trim)
                    .unwrap_or_default();
                if message_id.is_empty() || channel_id.is_empty() {
                    return Some(routing::bad_request_response(
                        "Valid message and target peer IDs are required",
                    ));
                }
                if payload
                    .get("targetPeerIds")
                    .is_some_and(|peers| json_array_exceeds_limit(peers, MAX_ROUTING_TARGET_PEERS))
                {
                    return Some(routing::bad_request_response(
                        "targetPeerIds must contain at most 256 items",
                    ));
                }
                let peer_ids = payload
                    .get("targetPeerIds")
                    .and_then(serde_json::Value::as_array)
                    .map(|peers| {
                        let mut normalized = Vec::new();
                        let mut seen = HashSet::new();
                        for peer in peers.iter().filter_map(serde_json::Value::as_str) {
                            let peer = peer.trim();
                            if !peer.is_empty() && seen.insert(peer.to_ascii_lowercase()) {
                                normalized.push(peer.to_owned());
                            }
                        }
                        normalized
                    })
                    .unwrap_or_default();
                if peer_ids.is_empty() {
                    return Some(routing::bad_request_response(
                        "Valid message and target peer IDs are required",
                    ));
                }
                let pod_id = message
                    .get("podId")
                    .and_then(serde_json::Value::as_str)
                    .map(str::trim)
                    .unwrap_or_default();
                let mut successfully_routed = 0_usize;
                let mut failed_peer_ids = Vec::new();
                for peer_id in &peer_ids {
                    match route_pod_message_to_peer(state, message, peer_id).await {
                        Ok(_) => successfully_routed += 1,
                        Err(_) => failed_peer_ids.push(peer_id.clone()),
                    }
                }
                let duration = format_timespan_hms(started_at.elapsed().as_secs() as i64);
                return Some(routing::ok_response(
                    serde_json::json!({
                        "success": failed_peer_ids.is_empty(),
                        "messageId": message_id,
                        "podId": pod_id,
                        "targetPeerCount": peer_ids.len(),
                        "successfullyRoutedCount": successfully_routed,
                        "failedRoutingCount": failed_peer_ids.len(),
                        "routingDuration": duration,
                        "failedPeerIds": failed_peer_ids,
                    })
                    .to_string(),
                ));
            }

            let message_id = payload
                .get("messageId")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .unwrap_or_default();
            let pod_id = payload
                .get("podId")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .unwrap_or_default();
            let channel_id = payload
                .get("channelId")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .unwrap_or_default();
            if message_id.is_empty() || channel_id.is_empty() {
                return Some(routing::bad_request_response(
                    "Valid pod message with MessageId and ChannelId is required",
                ));
            }
            if pod_id.is_empty() {
                return Some(routing::internal_server_error_response(
                    "Failed to route message",
                ));
            }

            let (channel_exists, members) = {
                let pods = state.pods.read().await;
                let Some(pod) = pods.get(pod_id) else {
                    return Some(routing::internal_server_error_response(
                        "Failed to route message",
                    ));
                };
                (
                    pod.channels
                        .iter()
                        .any(|channel| channel.channel_id == channel_id),
                    pods.members(pod_id).unwrap_or_default(),
                )
            };
            if !channel_exists {
                return Some(routing::internal_server_error_response(
                    "Failed to route message",
                ));
            }

            let key = format!("pod/routing-seen/{pod_id}/{message_id}");
            if state.controller_features.read().await.get(&key).is_some() {
                return Some(routing::ok_response(
                    serde_json::json!({
                        "success": true,
                        "messageId": message_id,
                        "podId": pod_id,
                        "targetPeerCount": 0,
                        "successfullyRoutedCount": 0,
                        "failedRoutingCount": 0,
                        "routingDuration": format_timespan_hms(started_at.elapsed().as_secs() as i64),
                        "errorMessage": "Message already routed (duplicate)",
                        "failedPeerIds": [],
                    })
                    .to_string(),
                ));
            }

            let sender_peer_id = payload
                .get("senderPeerId")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .unwrap_or_default();
            let mut seen_peers = HashSet::new();
            let peer_ids = members
                .into_iter()
                .filter(|member| {
                    !member.is_banned
                        && (sender_peer_id.is_empty()
                            || !member.peer_id.eq_ignore_ascii_case(sender_peer_id))
                })
                .filter(|member| seen_peers.insert(member.peer_id.to_ascii_lowercase()))
                .map(|member| member.peer_id)
                .collect::<Vec<_>>();
            if let Err(error) = state
                .controller_features
                .upsert(
                    key,
                    serde_json::json!({
                        "messageId": message_id,
                        "podId": pod_id,
                        "seenAt": unix_timestamp_millis(),
                        "wasNewlyRegistered": true,
                    }),
                )
                .await
            {
                return Some(podcore_storage_error_response(
                    state,
                    versioned_v0,
                    "Failed to route message",
                    &error,
                ));
            }

            let target_count = peer_ids.len();
            let mut successfully_routed = 0_usize;
            let mut failed_peer_ids = Vec::new();
            for peer_id in &peer_ids {
                match route_pod_message_to_peer(state, &payload, peer_id).await {
                    Ok(_) => successfully_routed += 1,
                    Err(_) => failed_peer_ids.push(peer_id.clone()),
                }
            }
            if target_count > 0 {
                state.podcore_runtime_stats.record_routing(
                    pod_id,
                    target_count,
                    successfully_routed,
                    failed_peer_ids.len(),
                    started_at.elapsed().as_millis() as u64,
                );
            }
            if !failed_peer_ids.is_empty() {
                return Some(routing::internal_server_error_response(
                    "Failed to route message",
                ));
            }
            Some(routing::ok_response(
                serde_json::json!({
                    "success": true,
                    "messageId": message_id,
                    "podId": pod_id,
                    "targetPeerCount": target_count,
                    "successfullyRoutedCount": successfully_routed,
                    "failedRoutingCount": 0,
                    "routingDuration": format_timespan_hms(started_at.elapsed().as_secs() as i64),
                    "failedPeerIds": [],
                })
                .to_string(),
            ))
        }
        ("POST", [section, seen, message_id, pod_id]) if section == "routing" && seen == "seen" => {
            let key = format!("pod/routing-seen/{pod_id}/{message_id}");
            let was_new = state.controller_features.read().await.get(&key).is_none();
            let value = serde_json::json!({
                "messageId": message_id,
                "podId": pod_id,
                "seenAt": unix_timestamp_millis(),
                "wasNewlyRegistered": was_new,
            });
            Some(
                match state.controller_features.upsert(key, value.clone()).await {
                    Ok(()) => routing::ok_response(value.to_string()),
                    Err(error) => podcore_storage_error_response(
                        state,
                        versioned_v0,
                        "Failed to register message as seen",
                        &error,
                    ),
                },
            )
        }
        ("POST", [section, action])
            if section == "signing"
                && matches!(action.as_str(), "generate-keypair" | "sign" | "verify") =>
        {
            Some(pod_signing_response(action, body, state).await)
        }
        ("POST", [section, message]) if section == "verification" && message == "message" => {
            Some(pod_verification_message_response(body, state).await)
        }
        ("POST", [section, refresh, tail @ ..])
            if section == "dht" && refresh == "refresh" && !tail.is_empty() =>
        {
            let pod_id = tail.join("/");
            let publication_key = format!("pod/dht/{pod_id}");
            let Some(publication) = state
                .controller_features
                .read()
                .await
                .get(&publication_key)
                .cloned()
            else {
                return Some(routing::internal_server_error_response(
                    "Failed to refresh pod",
                ));
            };
            let now = chrono::Utc::now();
            let Some(expires_at) = publication
                .get("expiresAt")
                .and_then(serde_json::Value::as_str)
                .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
                .map(|value| value.with_timezone(&chrono::Utc))
            else {
                return Some(routing::internal_server_error_response(
                    "Failed to refresh pod",
                ));
            };
            let next_refresh = expires_at - chrono::Duration::hours(6);
            if expires_at - now >= chrono::Duration::hours(6) {
                return Some(routing::ok_response(
                    serde_json::json!({
                        "success": true,
                        "podId": pod_id,
                        "wasRepublished": false,
                        "nextRefresh": next_refresh.to_rfc3339(),
                    })
                    .to_string(),
                ));
            }

            let Some(pod) = state.pods.read().await.get(&pod_id) else {
                return Some(routing::internal_server_error_response(
                    "Failed to refresh pod",
                ));
            };
            let published_pod = serde_json::json!(pod);
            let canonical = serde_json::to_vec(&published_pod).unwrap_or_default();
            let signature = state.capability_signing_key.sign(&canonical);
            let published_at = chrono::Utc::now();
            let refreshed_expires_at = published_at + chrono::Duration::hours(24);
            let refreshed = serde_json::json!({
                "success": true,
                "podId": pod_id,
                "dhtKey": format!("pod:{pod_id}:meta"),
                "publishedAt": published_at.to_rfc3339(),
                "expiresAt": refreshed_expires_at.to_rfc3339(),
                "publishedPod": published_pod,
                "signature": STANDARD.encode(signature.to_bytes()),
                "publicKey": STANDARD.encode(state.capability_signing_key.verifying_key().as_bytes()),
            });
            if let Err(_error) = state
                .controller_features
                .upsert(publication_key, refreshed)
                .await
            {
                return Some(routing::internal_server_error_response(
                    "Failed to refresh pod",
                ));
            }
            state.podcore_runtime_stats.record_dht_refresh();
            Some(routing::ok_response(
                serde_json::json!({
                    "success": true,
                    "podId": pod_id,
                    "wasRepublished": true,
                    "nextRefresh": (refreshed_expires_at - chrono::Duration::hours(6)).to_rfc3339(),
                })
                .to_string(),
            ))
        }
        ("POST", [section, action]) if section == "discovery" && action == "refresh" => {
            Some(podcore_discovery_refresh_response(state).await)
        }
        ("POST", [section, action, tail @ ..])
            if matches!(section.as_str(), "dht" | "discovery") =>
        {
            let payload = serde_json::from_str::<serde_json::Value>(body).unwrap_or_default();
            let id = tail.first().cloned().unwrap_or_else(|| {
                payload
                    .get("pod")
                    .and_then(|pod| pod.get("podId"))
                    .or_else(|| payload.get("podId"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("all")
                    .to_owned()
            });
            let id = id.trim().to_owned();
            if pods::is_gold_star_club(&id) && !gold_star_club_available(state) {
                return Some(routing::not_found_response());
            }
            let canonical_dht_write = section == "dht"
                && matches!(action.as_str(), "publish" | "update")
                && tail.is_empty();
            if canonical_dht_write {
                if payload.get("pod").filter(|pod| pod.is_object()).is_none() {
                    return Some(routing::bad_request_response("Pod data is required"));
                }
                if id.is_empty() || id == "all" {
                    return Some(routing::bad_request_response("Pod ID is required"));
                }
                if state.pods.read().await.get(&id).is_none() {
                    return Some(routing::not_found_response());
                }
            }
            let key = format!("pod/{section}/{id}");
            let now = chrono::Utc::now();
            let request_pod = payload
                .get("pod")
                .filter(|pod| pod.is_object())
                .cloned()
                .or_else(|| {
                    payload
                        .get("podId")
                        .and_then(serde_json::Value::as_str)
                        .filter(|value| !value.trim().is_empty())
                        .map(|_| payload.clone())
                });
            let stored_pod = if canonical_dht_write {
                state
                    .pods
                    .read()
                    .await
                    .get(&id)
                    .map(|pod| serde_json::json!(pod))
            } else if request_pod.is_some() {
                request_pod.clone()
            } else {
                state
                    .pods
                    .read()
                    .await
                    .get(&id)
                    .map(|pod| serde_json::json!(pod))
            };
            let published_pod = (section == "dht"
                && matches!(action.as_str(), "publish" | "update"))
            .then(|| stored_pod.clone())
            .flatten();
            let (publication_signature, publication_public_key) =
                published_pod.as_ref().map_or((None, None), |pod| {
                    let canonical = serde_json::to_vec(pod).unwrap_or_default();
                    let signature = state.capability_signing_key.sign(&canonical);
                    (
                        Some(STANDARD.encode(signature.to_bytes())),
                        Some(
                            STANDARD
                                .encode(state.capability_signing_key.verifying_key().as_bytes()),
                        ),
                    )
                });
            let value = if section == "dht" && matches!(action.as_str(), "publish" | "update") {
                serde_json::json!({
                    "success": true,
                    "podId": id,
                    "dhtKey": format!("pod:{id}:meta"),
                    "publishedAt": now.to_rfc3339(),
                    "expiresAt": (now + chrono::Duration::hours(24)).to_rfc3339(),
                    "publishedPod": published_pod,
                    "signature": publication_signature,
                    "publicKey": publication_public_key,
                })
            } else if section == "discovery" && matches!(action.as_str(), "register" | "update") {
                let Some(pod_id) = payload
                    .get("podId")
                    .and_then(serde_json::Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                else {
                    return Some(routing::bad_request_response(
                        "Valid pod with PodId is required",
                    ));
                };
                let Some(visibility) = payload.get("visibility") else {
                    return Some(routing::internal_server_error_response(
                        "Failed to register pod",
                    ));
                };
                if !pod_visibility_is_listed(visibility) {
                    return Some(routing::internal_server_error_response(
                        "Failed to register pod",
                    ));
                }
                let name = payload
                    .get("name")
                    .and_then(serde_json::Value::as_str)
                    .map(str::trim)
                    .unwrap_or_default();
                if payload
                    .get("tags")
                    .is_some_and(|tags| json_array_exceeds_limit(tags, MAX_POD_DISCOVERY_TAGS))
                {
                    return Some(routing::bad_request_response(
                        "tags must contain at most 100 items",
                    ));
                }
                let mut keys = vec!["pod:discover:all".to_owned()];
                if !name.trim().is_empty() {
                    keys.push(format!(
                        "pod:discover:name:{}",
                        pod_discovery_name_slug(name)
                    ));
                }
                let tags = payload
                    .get("tags")
                    .and_then(serde_json::Value::as_array)
                    .map(|tags| {
                        tags.iter()
                            .filter_map(serde_json::Value::as_str)
                            .map(str::trim)
                            .filter(|tag| !tag.is_empty())
                            .map(ToOwned::to_owned)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                for tag in &tags {
                    let key = format!("pod:discover:tag:{}", tag.to_ascii_lowercase());
                    if !keys.contains(&key) {
                        keys.push(key);
                    }
                }
                if let Some(content_id) = payload
                    .get("focusContentId")
                    .and_then(serde_json::Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                {
                    keys.push(format!(
                        "pod:discover:content:{}",
                        content_id.to_ascii_lowercase()
                    ));
                }
                let mut registration = serde_json::json!({
                    "success": true,
                    "podId": pod_id,
                    "discoveryKeys": keys,
                    "registeredAt": now.to_rfc3339(),
                    "expiresAt": (now + chrono::Duration::hours(24)).to_rfc3339(),
                });
                registration["tags"] = serde_json::json!(tags);
                registration["pod"] = payload.clone();
                registration
            } else {
                serde_json::json!({
                    "podId": id,
                    "action": action,
                    "updatedAt": unix_timestamp(),
                    "payload": payload,
                })
            };
            if section == "discovery"
                && matches!(action.as_str(), "register" | "update")
                && state
                    .controller_features
                    .read()
                    .await
                    .validate_storage()
                    .is_err()
            {
                return Some(routing::internal_server_error_response(
                    if action == "register" {
                        "Failed to register pod"
                    } else {
                        "Failed to update pod discovery"
                    },
                ));
            }
            let is_dht_publish =
                section == "dht" && matches!(action.as_str(), "publish" | "update");
            let is_dht_update = section == "dht" && action == "update";
            let publication_started = std::time::Instant::now();
            if is_dht_update {
                // native profile implements UpdateAsync as UnpublishAsync followed by
                // PublishAsync, so the lifecycle counters retain the expired
                // publication even though the DHT key is overwritten.
                state.podcore_runtime_stats.record_dht_unpublish();
            }
            Some(
                match state.controller_features.upsert(key, value.clone()).await {
                    Ok(()) => {
                        if is_dht_publish {
                            state.pod_dht_publish_time_ms.fetch_add(
                                publication_started.elapsed().as_millis() as u64,
                                std::sync::atomic::Ordering::Relaxed,
                            );
                            state
                                .pod_dht_publish_count
                                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            state
                                .podcore_runtime_stats
                                .record_dht_publish(published_pod.as_ref());
                        }
                        let response = if is_dht_publish {
                            let mut response = value.clone();
                            if let Some(response) = response.as_object_mut() {
                                response.remove("publishedPod");
                                response.remove("signature");
                                response.remove("publicKey");
                                response.remove("errorMessage");
                            }
                            response
                        } else if section == "discovery"
                            && matches!(action.as_str(), "register" | "update")
                        {
                            let mut response = value.clone();
                            if let Some(response) = response.as_object_mut() {
                                response.remove("tags");
                                response.remove("pod");
                            }
                            response
                        } else {
                            value.clone()
                        };
                        routing::ok_response(response.to_string())
                    }
                    Err(error) => {
                        if is_dht_publish {
                            state
                                .pod_dht_failed_publish_count
                                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            state.pod_dht_publish_time_ms.fetch_add(
                                publication_started.elapsed().as_millis() as u64,
                                std::sync::atomic::Ordering::Relaxed,
                            );
                        }
                        if section == "discovery" && action == "register" {
                            routing::internal_server_error_response("Failed to register pod")
                        } else if section == "discovery" && action == "update" {
                            routing::internal_server_error_response(
                                "Failed to update pod discovery",
                            )
                        } else if section == "dht" && action == "publish" {
                            podcore_storage_error_response(
                                state,
                                versioned_v0,
                                "Failed to publish pod",
                                &error,
                            )
                        } else if section == "dht" && action == "update" {
                            podcore_storage_error_response(
                                state,
                                versioned_v0,
                                "Failed to update pod",
                                &error,
                            )
                        } else {
                            routing::service_unavailable_response(&error)
                        }
                    }
                },
            )
        }
        ("DELETE", [section, action, pod_id])
            if (section == "dht" && action == "unpublish")
                || (section == "discovery" && action == "unregister") =>
        {
            if section == "discovery"
                && state
                    .controller_features
                    .read()
                    .await
                    .validate_storage()
                    .is_err()
            {
                return Some(routing::internal_server_error_response(
                    "Failed to unregister pod",
                ));
            }
            let key = format!("pod/{section}/{pod_id}");
            Some(match state.controller_features.remove(&key).await {
                Ok(_removed) if section == "dht" => {
                    state.podcore_runtime_stats.record_dht_unpublish();
                    routing::ok_response(
                        serde_json::json!({
                            "success": true,
                            "podId": pod_id,
                            "dhtKey": format!("pod:{pod_id}:meta"),
                        })
                        .to_string(),
                    )
                }
                Ok(Some(removed)) => {
                    let removed_keys = removed
                        .get("discoveryKeys")
                        .cloned()
                        .unwrap_or_else(|| serde_json::json!([]));
                    routing::ok_response(
                        serde_json::json!({
                            "success": true,
                            "podId": pod_id,
                            "removedKeys": removed_keys,
                        })
                        .to_string(),
                    )
                }
                Ok(None) => routing::internal_server_error_response("Failed to unregister pod"),
                Err(_error) if section == "discovery" => {
                    routing::internal_server_error_response("Failed to unregister pod")
                }
                Err(error) => podcore_storage_error_response(
                    state,
                    versioned_v0,
                    "Failed to unpublish pod",
                    &error,
                ),
            })
        }
        _ => None,
    }
}
