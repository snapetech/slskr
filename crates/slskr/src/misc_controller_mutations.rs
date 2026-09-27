use super::*;

/// Matches the oracle's `ListeningPartyService.Normalize` exactly: a
/// real, restricted `play|pause|seek|stop` action vocabulary (not
/// slskR's previously-invented `{kind, action}` pair), a required
/// `contentId` for playback events, server-derived `partyId`/
/// `hostPeerId`/`serverTimeUnixMs`, and bounded/deduplicated tags.
/// `sequence` is the millisecond-resolution wall clock at call time,
/// since slskR has no real per-process monotonic event counter for
/// this feature the way the oracle does.
fn listening_party_normalize(
    payload: &serde_json::Value,
    pod_id: &str,
    channel_id: &str,
    host_peer_id: &str,
) -> Result<serde_json::Value, &'static str> {
    let now_ms = unix_timestamp_millis();
    let action = payload
        .get("action")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    if !matches!(action.as_str(), "play" | "pause" | "seek" | "stop") {
        return Err("Listen-along event is invalid.");
    }
    let content_id = if action == "stop" {
        String::new()
    } else {
        payload
            .get("contentId")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_owned()
    };
    if action != "stop" && content_id.is_empty() {
        return Err("Listen-along event is invalid.");
    }
    let party_id = payload
        .get("partyId")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| format!("party:{}", uuid::Uuid::new_v4().simple()));
    let position_seconds = payload
        .get("positionSeconds")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0)
        .max(0.0);
    let mut seen_tags = std::collections::BTreeSet::new();
    let mut tags = payload
        .get("tags")
        .and_then(serde_json::Value::as_array)
        .map(|tags| {
            tags.iter()
                .take(MAX_LISTENING_PARTY_TAGS)
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
        .into_iter()
        .filter_map(|tag| {
            tag.as_str()
                .map(str::trim)
                .filter(|tag| !tag.is_empty())
                .map(str::to_owned)
        })
        .collect::<Vec<_>>();
    tags.retain(|tag| seen_tags.insert(tag.to_ascii_lowercase()));
    tags.truncate(MAX_LISTENING_PARTY_TAGS);
    let album = payload
        .get("album")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    Ok(serde_json::json!({
        "partyId": party_id,
        "kind": "slskdn.listenAlong.v1",
        "podId": pod_id,
        "channelId": channel_id,
        "hostPeerId": host_peer_id,
        "action": action,
        "contentId": content_id,
        "title": payload.get("title").and_then(serde_json::Value::as_str).unwrap_or_default().trim(),
        "artist": payload.get("artist").and_then(serde_json::Value::as_str).unwrap_or_default().trim(),
        "album": album,
        "positionSeconds": position_seconds,
        "serverTimeUnixMs": now_ms,
        "sequence": now_ms,
        "listed": payload.get("listed").and_then(serde_json::Value::as_bool).unwrap_or(false),
        "allowMeshStreaming": payload.get("allowMeshStreaming").and_then(serde_json::Value::as_bool).unwrap_or(false),
        "description": payload.get("description").and_then(serde_json::Value::as_str).unwrap_or_default().trim(),
        "tags": tags,
    }))
}

pub(super) async fn misc_controller_mutation_response(
    method: &str,
    path: &str,
    query: Option<&str>,
    body: &str,
    state: &AppState,
    is_versioned_v0: bool,
    headers: &RequestSecurityHeaders,
) -> Option<HttpResponse> {
    if method == "POST"
        && path.starts_with("/actors/")
        && (path.ends_with("/inbox") || path.ends_with("/outbox"))
    {
        let segments = decoded_segments_after(path, "/actors/")?;
        let [actor, direction] = segments.as_slice() else {
            return Some(routing::not_found_response());
        };
        if !activitypub_actor_exists(actor, state).await {
            return Some(routing::not_found_response());
        }
        let verified_key_id = if *direction == "inbox" {
            match Box::pin(verify_activitypub_inbox_signature(
                method, path, query, body, headers,
            ))
            .await
            {
                Ok(key_id) => Some(key_id),
                Err(_) => return Some(routing::unauthorized_response()),
            }
        } else {
            None
        };
        let activity = match serde_json::from_str::<serde_json::Value>(body) {
            Ok(activity @ serde_json::Value::Object(_)) => activity,
            Ok(_) => return Some(routing::bad_request_response("activity must be an object")),
            Err(_) => return Some(routing::bad_request_response("invalid ActivityPub JSON")),
        };
        if let Some(verified_key_id) = verified_key_id.as_deref() {
            if !activitypub_actor_bound_to_signature(&activity, verified_key_id) {
                return Some(routing::unauthorized_response());
            }
        }
        let Some(activity_type) = activity
            .get("type")
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.trim().is_empty())
        else {
            return Some(routing::bad_request_response("activity type is required"));
        };
        let id = activity
            .get("id")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let response_activity = activity.clone();
        let record = serde_json::json!({
            "id": id,
            "actorName": actor,
            "direction": direction,
            "type": activity_type,
            "activity": activity,
            "receivedAt": unix_timestamp(),
        });
        let key = format!("activitypub/{actor}/{direction}/{id}");
        let activity_store_result = { state.controller_features.upsert(key, record).await };
        return Some(match activity_store_result {
            Ok(()) => match activitypub_apply_relationship(
                actor,
                direction,
                activity_type,
                &response_activity,
                state,
            )
            .await
            {
                Ok(()) if direction == "outbox" => {
                    routing::ok_response(response_activity.to_string())
                }
                Ok(()) => routing::accepted_response(String::new()),
                Err(error) => routing::service_unavailable_response(&error),
            },
            Err(error) => routing::service_unavailable_response(&error),
        });
    }
    if method == "POST" && path == "/api/audio/analyzers/migrate" {
        if query_parameter(query, "targetVersion").is_some_and(|value| value.trim().is_empty()) {
            return Some(routing::bad_request_response("TargetVersion is required."));
        }
        if query_parameter(query, "force").is_some_and(|value| {
            !matches!(value.trim().to_ascii_lowercase().as_str(), "true" | "false")
        }) {
            return Some(routing::bad_request_response(
                "The query value is not valid.",
            ));
        }
        if let Some(db) = state.db.as_ref() {
            if db.get_traffic_totals().await.is_err() {
                return Some(routing::service_unavailable_response(
                    "audio analyzer storage unavailable",
                ));
            }
        }
        let discovery = state.content_discovery.read().await;
        return Some(routing::ok_response(
            serde_json::json!({
                "updated": discovery.hash_entries().len(),
            })
            .to_string(),
        ));
    }
    if method == "POST" && path == "/api/jobs/label-crate" {
        let payload = match serde_json::from_str::<serde_json::Value>(body) {
            Ok(payload @ serde_json::Value::Object(_)) => payload,
            Ok(_) => return Some(routing::bad_request_response("job body must be an object")),
            Err(_) => return Some(routing::bad_request_response("invalid JSON body")),
        };
        let Some(label) = payload
            .get("label_name")
            .or_else(|| payload.get("label_id"))
            .or_else(|| payload.get("label"))
            .or_else(|| payload.get("name"))
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.trim().is_empty())
        else {
            return Some(routing::bad_request_response(
                "label_id or label_name is required",
            ));
        };
        let label_id = payload
            .get("label_id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        let limit = payload
            .get("limit")
            .and_then(serde_json::Value::as_i64)
            .and_then(|value| u32::try_from(value.max(0)).ok())
            .filter(|value| *value > 0)
            .unwrap_or(10);
        let id = uuid::Uuid::new_v4().to_string();
        let record = serde_json::json!({
            "id": id,
            "jobId": id,
            "labelId": label_id,
            "labelName": label,
            "limit": limit,
            "releaseIds": [],
            "totalReleases": 0,
            "completedReleases": 0,
            "failedReleases": 0,
            "label": label,
            "type": "label_crate",
            "status": "Pending",
            "input": payload,
            "createdAt": unix_timestamp(),
        });
        return Some(
            match state
                .controller_features
                .upsert(format!("job/label-crate/{id}"), record.clone())
                .await
            {
                Ok(()) if is_versioned_v0 => routing::ok_response(record.to_string()),
                Ok(()) => routing::created_response(record.to_string()),
                Err(error) => routing::service_unavailable_response(&error),
            },
        );
    }
    if method == "POST" && matches!(path, "/api/dht/announce" | "/api/dht/discover") {
        if body.trim().is_empty() {
            // Matches the oracle's real DhtRendezvousController.Announce/
            // Discover: both force a real DHT operation (the oracle's own
            // AnnounceAsync/DiscoverPeersAsync), not merely report whether
            // DHT is configured. slskR's Rendezvous::refresh() performs
            // both the real announce and the real peer-discovery lookup
            // together (it has no separate announce-only operation), so
            // both routes drive the same real refresh.
            return Some(if path.ends_with("/announce") {
                match state.dht.as_ref() {
                    Some(dht) if dht.is_beacon_capable() => {
                        dht.refresh().await;
                        routing::ok_response(
                            serde_json::json!({"message": "Announced"}).to_string(),
                        )
                    }
                    _ => routing::bad_request_response("Not beacon capable"),
                }
            } else {
                let Some(dht) = state.dht.as_ref() else {
                    return Some(routing::ok_response(
                        serde_json::json!({
                            "newConnectionsMade": 0,
                            "totalMeshConnections": 0,
                        })
                        .to_string(),
                    ));
                };
                let before = dht
                    .peers()
                    .await
                    .into_iter()
                    .collect::<std::collections::BTreeSet<_>>();
                dht.refresh().await;
                let after = dht.peers().await;
                let new_connections = after.iter().filter(|peer| !before.contains(peer)).count();
                routing::ok_response(
                    serde_json::json!({
                        "newConnectionsMade": new_connections,
                        "totalMeshConnections": after.len(),
                    })
                    .to_string(),
                )
            });
        }
        let payload = match serde_json::from_str::<serde_json::Value>(body) {
            Ok(payload @ serde_json::Value::Object(_)) => payload,
            Ok(_) => return Some(routing::bad_request_response("DHT body must be an object")),
            Err(_) => return Some(routing::bad_request_response("invalid JSON body")),
        };
        let Some(content_id) = payload
            .get("contentId")
            .or_else(|| payload.get("key"))
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.trim().is_empty())
        else {
            return Some(routing::bad_request_response("contentId/key is required"));
        };
        if path.ends_with("/announce") {
            let record = serde_json::json!({
                "contentId": content_id,
                "announcement": payload,
                "announcedAt": unix_timestamp(),
                "dhtEnabled": state.dht.is_some(),
            });
            return Some(
                match state
                    .controller_features
                    .upsert(format!("dht/announcement/{content_id}"), record.clone())
                    .await
                {
                    Ok(()) => routing::ok_response(record.to_string()),
                    Err(error) => routing::service_unavailable_response(&error),
                },
            );
        }
        let features = state.controller_features.read().await;
        let announcement = features
            .get(&format!("dht/announcement/{content_id}"))
            .cloned();
        drop(features);
        let discovery = state.content_discovery.read().await;
        let shadow = discovery
            .shadow_records()
            .iter()
            .find(|record| record.recording_id.eq_ignore_ascii_case(content_id));
        return Some(routing::ok_response(
            serde_json::json!({
                "contentId": content_id,
                "announcement": announcement,
                "peerIds": shadow.map(|record| record.peer_ids.clone()).unwrap_or_default(),
                "found": announcement.is_some() || shadow.is_some(),
            })
            .to_string(),
        ));
    }
    if method == "POST" && path.starts_with("/api/hashdb/optimize/") {
        let operation = path.rsplit('/').next().unwrap_or_default();
        if operation == "indexes" {
            return Some(routing::ok_response(
                serde_json::json!({"message": "Index optimization completed"}).to_string(),
            ));
        }
        if operation == "vacuum" {
            return Some(routing::ok_response(
                serde_json::json!({"message": "VACUUM and ANALYZE completed"}).to_string(),
            ));
        }
        if operation == "profile" {
            let query = extract_json_string_field(body, "query").unwrap_or_default();
            let normalized = query.trim().to_ascii_lowercase();
            if !normalized.starts_with("select ") && !normalized.starts_with("with ") {
                return Some(routing::bad_request_response(
                    "Query is not allowed for profiling",
                ));
            }
            let discovery = state.content_discovery.read().await;
            let profile = discovery.profile_query(&query);
            drop(discovery);
            return Some(routing::ok_response(serde_json::json!(profile).to_string()));
        }
        return Some(routing::not_found_response());
    }
    if method == "POST" && path.starts_with("/api/listening-party/") {
        let segments = decoded_segments_after(path, "/api/listening-party/")?;
        let [pod_id, channel_id] = segments.as_slice() else {
            return Some(routing::not_found_response());
        };
        let pods = state.pods.read().await;
        if pods.get(pod_id).is_none() || !pods.channel_exists(pod_id, channel_id) {
            return Some(routing::not_found_response());
        }
        // Matches the oracle's ListeningPartyController.Publish: the
        // caller must be a real pod member, and the host peer id is
        // always the authenticated local identity, never a
        // client-supplied field.
        let Some(host_peer_id) = pod_request_peer_id(state).await else {
            drop(pods);
            return Some(routing::forbidden_response(
                "Authenticated peer identity is required",
            ));
        };
        if !pods.is_member(pod_id, &host_peer_id) {
            drop(pods);
            return Some(routing::forbidden_response("Pod membership is required"));
        }
        drop(pods);
        let payload = match serde_json::from_str::<serde_json::Value>(body) {
            Ok(payload @ serde_json::Value::Object(_)) => payload,
            Ok(_) => {
                return Some(routing::bad_request_response(
                    "party body must be an object",
                ))
            }
            Err(_) => return Some(routing::bad_request_response("invalid JSON body")),
        };
        let event = match listening_party_normalize(&payload, pod_id, channel_id, &host_peer_id) {
            Ok(event) => event,
            Err(error) => {
                return Some(HttpResponse {
                    status: "400 Bad Request",
                    content_type: "application/json",
                    body: serde_json::json!(error).to_string(),
                })
            }
        };
        let key = format!("listening-party/{pod_id}/{channel_id}");
        // Matches the oracle's PublishAsync: a "stop" event clears the
        // stored state entirely rather than persisting a "stopped"
        // snapshot -- there is no active listen-along after a stop.
        let update_key = key.clone();
        let update_event = event.clone();
        let update_result = state
            .controller_features
            .mutate(move |features| {
                if update_event["action"] == "stop" {
                    let previous_party_id = features
                        .get(&update_key)
                        .and_then(|stored| stored.get("partyId"))
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_owned);
                    let requested_party_id = update_event
                        .get("partyId")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_owned);
                    features.remove(&update_key)?;
                    let revoke_party_ids = [previous_party_id, requested_party_id]
                        .into_iter()
                        .flatten()
                        .filter(|party_id| !party_id.is_empty())
                        .fold(Vec::new(), |mut ids, party_id| {
                            if !ids.iter().any(|existing| existing == &party_id) {
                                ids.push(party_id);
                            }
                            ids
                        });
                    Ok(revoke_party_ids)
                } else {
                    features.upsert(update_key, update_event)?;
                    Ok(Vec::new())
                }
            })
            .await;
        let (result, revoke_party_ids) = match update_result {
            Ok(revoke_party_ids) => (Ok(()), revoke_party_ids),
            Err(error) => (Err(error), Vec::new()),
        };
        let succeeded = result.is_ok();
        if succeeded && !revoke_party_ids.is_empty() {
            let mut tickets = state.stream_tickets.write().await;
            for party_id in revoke_party_ids {
                tickets.revoke_source(&format!("listening-party:{party_id}"));
            }
        }
        let response = match result {
            Ok(()) => routing::ok_response(event.to_string()),
            Err(error) => routing::service_unavailable_response(&error),
        };
        if succeeded {
            // The frozen native profile service publishes the normalized event to the
            // listening-party SignalR group after persistence. Reuse the
            // bounded event bus so the compatibility hub can apply the same
            // per-connection group filter without another unbounded channel.
            record_event(
                state,
                "listening_party.updated",
                key,
                Some(event.to_string()),
            )
            .await;
        }
        return Some(response);
    }
    if method == "POST" && path == "/api/mesh/nat/detect" {
        let mesh_settings = state.advanced_networking.read().await.mesh.clone();
        if !mesh_settings.enabled || !mesh_settings.enable_stun {
            return Some(controller_swagger_not_found_response());
        }
        let (nat_type, detected) = detect_nat_type(&STUN_SERVERS).await;
        return Some(routing::ok_response(
            serde_json::json!({
                "type": nat_type,
                "detected": detected,
            })
            .to_string(),
        ));
    }
    if method == "POST" && path == "/api/mesh/publish" {
        // Matches the oracle's real PublishHash/PublishHashRequest: only
        // flacKey (never "key"/"contentId" aliases -- those let a
        // payload skip the byteHash/size validation entirely), and the
        // entry is stored in the same real hash database GET
        // /api/mesh/lookup/{flacKey} already reads from. Previously this
        // wrote into an unrelated "mesh/published/{key}" feature-state
        // key that lookup never consulted, so a publish→lookup round
        // trip always reported "not found".
        let payload = match serde_json::from_str::<serde_json::Value>(body) {
            Ok(payload @ serde_json::Value::Object(_)) => payload,
            _ => {
                return Some(routing::bad_request_response(
                    "flacKey, byteHash, and size are required",
                ))
            }
        };
        let flac_key = payload
            .get("flacKey")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .unwrap_or_default();
        let byte_hash = payload
            .get("byteHash")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .unwrap_or_default();
        let size = payload.get("size").and_then(serde_json::Value::as_i64);
        if flac_key.is_empty() || byte_hash.is_empty() || size.is_none_or(|size| size <= 0) {
            return Some(routing::bad_request_response(
                "flacKey, byteHash, and size are required",
            ));
        }
        let entry = content_discovery::HashDbEntry {
            flac_key: flac_key.to_owned(),
            byte_hash: byte_hash.to_owned(),
            size: size.unwrap_or_default() as u64,
            ..Default::default()
        };
        let persistence_turn = hash_db_persistence_turn().await;
        let (
            merge_result,
            previous_entries,
            previous_latest_seq,
            mutated_entries,
            mutated_latest_seq,
        ) = {
            let mut discovery = state.content_discovery.write().await;
            let previous_entries = discovery.hash_entries().to_vec();
            let previous_latest_seq = discovery.latest_seq();
            let merge_result = discovery.merge_hash_entries(vec![entry]);
            let mutated_entries = discovery.hash_entries().to_vec();
            let mutated_latest_seq = discovery.latest_seq();
            (
                merge_result,
                previous_entries,
                previous_latest_seq,
                mutated_entries,
                mutated_latest_seq,
            )
        };
        return Some(match merge_result {
            Ok(_) => match persist_current_hash_db_snapshot(state, &persistence_turn).await {
                Ok(()) => routing::ok_response(serde_json::json!({"published": true}).to_string()),
                Err(error) => {
                    rollback_hash_db_entries_if_unchanged(
                        state,
                        previous_entries,
                        previous_latest_seq,
                        &mutated_entries,
                        mutated_latest_seq,
                    )
                    .await;
                    drop(persistence_turn);
                    content_discovery_error_response(state, error).await
                }
            },
            Err(error) => {
                drop(persistence_turn);
                content_discovery_error_response(state, error).await
            }
        });
    }
    if method == "POST" && path == "/api/mesh/message" {
        if is_versioned_v0 {
            if body.trim().is_empty() {
                return Some(HttpResponse {
                    status: "415 Unsupported Media Type",
                    content_type: "application/json",
                    body: String::new(),
                });
            }
            let Some(from_user) = query_parameter(query, "fromUser")
                .map(|value| value.trim().to_owned())
                .filter(|value| !value.is_empty())
            else {
                return Some(routing::bad_request_response(
                    "fromUser query parameter required",
                ));
            };
            let payload = match serde_json::from_str::<serde_json::Value>(body) {
                Ok(serde_json::Value::Object(payload)) => payload,
                Ok(_) | Err(_) => {
                    return Some(routing::bad_request_response("invalid mesh message"))
                }
            };
            let Some(message_type) = payload.get("type").and_then(serde_json::Value::as_i64) else {
                return Some(routing::bad_request_response(
                    "Message must have 'type' property",
                ));
            };
            if !matches!(message_type, 1 | 2 | 3 | 4 | 7) {
                return Some(routing::bad_request_response(
                    "Unknown or invalid message type",
                ));
            }
            // The controller delegates supported typed messages to the real
            // MeshSyncService dispatcher. Invalid signatures/DTOs and valid
            // response-only messages intentionally produce a null response.
            let response =
                match slskr_client::mesh_sync::MeshSyncMessage::decode_json(body.as_bytes()) {
                    Ok(message) => {
                        mesh_sync::handle_signed_message(state, &from_user, message).await
                    }
                    Err(_) => None,
                };
            let response = response
                .and_then(|message| message.encode_json().ok())
                .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
                .unwrap_or(serde_json::Value::Null);
            return Some(routing::ok_response(
                serde_json::json!({"handled": true, "response": response}).to_string(),
            ));
        }
        let username = extract_json_string_field(body, "username")
            .or_else(|| extract_json_string_field(body, "peerId"));
        let message = extract_json_string_field(body, "message")
            .or_else(|| extract_json_string_field(body, "body"));
        let (Some(username), Some(message)) = (username, message) else {
            return Some(routing::bad_request_response(
                "username/peerId and message are required",
            ));
        };
        let permit = match state.session_commands.reserve().await {
            Ok(permit) => permit,
            Err(_) => {
                return Some(routing::service_unavailable_response(
                    "session manager is not running",
                ))
            }
        };
        permit.send(SessionCommand::MessageUser {
            username: username.clone(),
            body: message,
        });
        return Some(routing::accepted_response(
            serde_json::json!({"username": username, "queued": true}).to_string(),
        ));
    }
    if method == "POST" && path == "/api/overlay/connect" {
        let Some(username) = extract_json_string_field(body, "username")
            .or_else(|| extract_json_string_field(body, "peerId"))
            .filter(|value| !value.trim().is_empty())
        else {
            return Some(routing::bad_request_response("username/peerId is required"));
        };
        let permit = match state.session_commands.reserve().await {
            Ok(permit) => permit,
            Err(_) => {
                return Some(routing::service_unavailable_response(
                    "session manager is not running",
                ))
            }
        };
        permit.send(SessionCommand::ProbePeerCapability(username.clone()));
        return Some(routing::accepted_response(
            serde_json::json!({"username": username, "status": "probing"}).to_string(),
        ));
    }
    None
}
