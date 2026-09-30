use super::*;

#[path = "podcore_mutations.rs"]
mod mutations;
#[path = "podcore_queries.rs"]
mod queries;
#[path = "podcore_stats_controller.rs"]
mod stats_controller;

pub(super) use self::mutations::podcore_mutation_response;
pub(super) use self::queries::podcore_dynamic_get_response;
pub(super) use self::stats_controller::podcore_stats_response;

pub(super) fn pod_channel_messages_path(path: &str) -> Option<(String, String)> {
    let rest = path.strip_prefix("/api/pods/")?;
    let (pod_id, rest) = rest.split_once("/channels/")?;
    let channel_id = rest.strip_suffix("/messages")?;
    if pod_id.is_empty()
        || channel_id.is_empty()
        || pod_id.contains('/')
        || channel_id.contains('/')
    {
        return None;
    }
    let pod_id = decoded_path_segment(pod_id).trim().to_owned();
    let channel_id = decoded_path_segment(channel_id).trim().to_owned();
    (!pod_id.is_empty()
        && !channel_id.is_empty()
        && !pod_id.contains('/')
        && !channel_id.contains('/'))
    .then_some((pod_id, channel_id))
}

pub(super) fn pod_resource_segments(path: &str) -> Option<Vec<String>> {
    let rest = path.strip_prefix("/api/pods/")?;
    let mut segments = Vec::new();
    for segment in rest.split('/') {
        if segment.is_empty() {
            return None;
        }
        let decoded = decoded_path_segment(segment).trim().to_owned();
        if decoded.is_empty() || decoded.contains('/') {
            return None;
        }
        segments.push(decoded);
    }
    (!segments.is_empty()).then_some(segments)
}

/// The frozen native profile controller receives whitespace-only route values and
/// validates them inside the action, returning 400. The compatibility
/// dispatcher normally rejects those values in its route-segment helper and
/// would otherwise expose a generic 404 before the controller contract runs.
pub(super) fn versioned_pods_blank_segment_response(
    method: &str,
    path: &str,
) -> Option<HttpResponse> {
    let rest = path.strip_prefix("/api/v0/pods/")?;
    let segments = rest
        .split('/')
        .map(decoded_path_segment)
        .map(|segment| segment.trim().to_owned())
        .collect::<Vec<_>>();

    let blank_pod = segments.first().is_some_and(String::is_empty);
    let blank_channel = segments.len() >= 3
        && segments.get(1).is_some_and(|segment| segment == "channels")
        && segments.get(2).is_some_and(String::is_empty);
    if !blank_pod && !blank_channel {
        return None;
    }

    let error = if blank_channel {
        "PodId and ChannelId are required"
    } else {
        "PodId is required"
    };
    let _ = method;
    Some(routing::bad_request_response(error))
}

/// Matches the oracle's `PodAffinityScorer.CalculateTrustScore`: a base
/// trust of 0.5, +0.3 for an owner or +0.2 for a mod, +0.2 for no ban,
/// capped at 1.0.
fn pod_member_trust_score(role: &str, is_banned: bool) -> f64 {
    let role_bonus = match role {
        "owner" => 0.3,
        "mod" => 0.2,
        _ => 0.0,
    };
    let clean_record_bonus = if is_banned { 0.0 } else { 0.2 };
    (0.5_f64 + role_bonus + clean_record_bonus).min(1.0)
}

/// Matches the oracle's `PodOpinionAggregator.CalculateAffinityScore`.
fn pod_member_affinity_score(
    message_count: usize,
    opinion_count: usize,
    membership_duration_seconds: i64,
    is_active: bool,
) -> f64 {
    let activity_score = ((message_count as f64 + opinion_count as f64 * 2.0) / 100.0).min(1.0);
    let duration_months = membership_duration_seconds as f64 / 86_400.0 / 30.0;
    let duration_bonus = (duration_months / 12.0).min(0.3);
    let activity_bonus = if is_active { 0.2 } else { 0.0 };
    let trust_component = (0.5 + activity_score * 0.5).min(1.0);
    (activity_score + duration_bonus + activity_bonus).min(1.0) * trust_component
}

pub(super) fn trusted_mesh_peer_for(state: &AppState, identity: &str) -> Option<TrustedMeshPeer> {
    let identity = identity.trim();
    if identity.is_empty() {
        return None;
    }
    state
        .config
        .trusted_mesh_peers
        .iter()
        .find(|peer| peer.matches(identity))
        .cloned()
}

fn configured_quic_certificate_pin(state: &AppState, peer: &TrustedMeshPeer) -> Option<[u8; 32]> {
    state
        .config
        .advanced_networking
        .overlay
        .trusted_certificate_pins
        .get(&peer.overlay_endpoint.to_string())?
        .iter()
        .filter_map(|pin| STANDARD.decode(pin.trim()).ok())
        .find_map(|pin| pin.try_into().ok())
}

pub(super) async fn route_pod_message_to_peer(
    state: &AppState,
    message: &serde_json::Value,
    peer_identity: &str,
) -> Result<String, String> {
    let peer = trusted_mesh_peer_for(state, peer_identity).ok_or_else(|| {
        format!(
            "no authenticated mesh endpoint is configured for peer {}",
            peer_identity.trim()
        )
    })?;
    let local_username = pod_request_peer_id(state)
        .await
        .or_else(|| state.config.username.clone())
        .unwrap_or_else(|| "slskr".to_owned());
    let pod_id = message
        .get("podId")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let channel_id = message
        .get("channelId")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let body = message
        .get("body")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let signature = message
        .get("signature")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let message_id = message
        .get("messageId")
        .or_else(|| message.get("MessageId"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let timestamp_unix_ms = message
        .get("timestampUnixMs")
        .or_else(|| message.get("TimestampUnixMs"))
        .and_then(serde_json::Value::as_i64)
        .unwrap_or_default();
    let sig_version = message
        .get("sigVersion")
        .or_else(|| message.get("SigVersion"))
        .and_then(serde_json::Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
        .unwrap_or(1);
    let use_native_control = state.config.controller_profile == ControllerProfile::Native
        && state
            .private_gateway
            .as_ref()
            .is_none_or(|gateway| gateway.bind() != peer.overlay_endpoint);
    if use_native_control {
        let request = mesh_services::PodMessageControlRequest {
            message_id,
            pod_id,
            channel_id,
            body,
            timestamp_unix_ms,
            signature,
            sig_version,
        };
        if state.config.advanced_networking.overlay.enable_quic
            && !state
                .config
                .advanced_networking
                .overlay
                .share_quic_with_dht_port
        {
            if let Some(certificate_pin) = configured_quic_certificate_pin(state, &peer) {
                return mesh_services::post_pod_message_quic(
                    &peer,
                    &local_username,
                    &state.capability_signing_key,
                    &request,
                    certificate_pin,
                )
                .await;
            }
        }
        return mesh_services::post_pod_message_control(
            &peer,
            &local_username,
            &state.capability_signing_key,
            &request,
        )
        .await;
    }
    mesh_services::post_pod_message(
        state.private_gateway.as_ref(),
        &peer,
        &local_username,
        &state.capability_signing_key,
        pod_id,
        channel_id,
        body,
        signature,
    )
    .await
}

/// Matches the oracle's canonical payload exactly:
/// `sigVersion|podId|channelId|messageId|senderPeerId|timestampUnixMs|base64(sha256(body))`.
/// `podId` falls back to the segment before the first `:` in `channelId`
/// when absent, matching `MessageSigner.GetPodId`.
pub(super) fn pod_message_canonical_payload(message: &serde_json::Value) -> (String, String) {
    let sig_version = message
        .get("sigVersion")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(1);
    let explicit_pod_id = message
        .get("podId")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let channel_id = message
        .get("channelId")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let pod_id = if !explicit_pod_id.is_empty() {
        explicit_pod_id.to_owned()
    } else {
        channel_id
            .split_once(':')
            .map_or_else(String::new, |(prefix, _)| prefix.to_owned())
    };
    let message_id = message
        .get("messageId")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let sender_peer_id = message
        .get("senderPeerId")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let timestamp_unix_ms = message
        .get("timestampUnixMs")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(0);
    let body_text = message
        .get("body")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let body_sha256 = STANDARD.encode(Sha256::digest(body_text.as_bytes()));
    let canonical = format!(
        "{sig_version}|{pod_id}|{channel_id}|{message_id}|{sender_peer_id}|{timestamp_unix_ms}|{body_sha256}"
    );
    (canonical, pod_id)
}

async fn pod_signing_response(action: &str, body: &str, state: &AppState) -> HttpResponse {
    if action == "generate-keypair" {
        let mut secret = [0_u8; 32];
        if SysRng.try_fill_bytes(&mut secret).is_err() {
            return routing::service_unavailable_response("secure randomness is unavailable");
        }
        let signing_key = SigningKey::from_bytes(&secret);
        return routing::ok_response(
            serde_json::json!({
                "privateKey": STANDARD.encode(signing_key.to_bytes()),
                "publicKey": STANDARD.encode(signing_key.verifying_key().to_bytes()),
                "algorithm": "Ed25519",
            })
            .to_string(),
        );
    }
    let payload = match serde_json::from_str::<serde_json::Value>(body) {
        Ok(payload @ serde_json::Value::Object(_)) => payload,
        Ok(_) => return routing::bad_request_response("signing body must be an object"),
        Err(_) => return routing::bad_request_response("invalid JSON body"),
    };
    let message = payload
        .get("message")
        .cloned()
        .unwrap_or_else(|| payload.clone());
    let (canonical, pod_id) = pod_message_canonical_payload(&message);
    if action == "sign" {
        let private_key = payload
            .get("privateKey")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        if private_key.trim().is_empty() {
            return routing::bad_request_response("Valid message and private key are required");
        }
        let secret: [u8; 32] = match STANDARD
            .decode(private_key.as_bytes())
            .ok()
            .and_then(|bytes| bytes.try_into().ok())
        {
            Some(secret) => secret,
            None => {
                return routing::bad_request_response("privateKey must be a base64 Ed25519 key")
            }
        };
        let signing_key = SigningKey::from_bytes(&secret);
        let signing_started_at = std::time::Instant::now();
        let signature = signing_key.sign(canonical.as_bytes());
        state
            .pod_signature_stats
            .record_sign(signing_started_at.elapsed().as_millis() as u64);
        return routing::ok_response(
            serde_json::json!({
                "message": message,
                "signature": format!("ed25519:{}", STANDARD.encode(signature.to_bytes())),
                "publicKey": STANDARD.encode(signing_key.verifying_key().to_bytes()),
                "sigVersion": message.get("sigVersion").and_then(serde_json::Value::as_i64).unwrap_or(1),
            })
            .to_string(),
        );
    }
    if message
        .get("messageId")
        .and_then(serde_json::Value::as_str)
        .is_none_or(|message_id| message_id.trim().is_empty())
    {
        return routing::bad_request_response("Valid message is required");
    }
    // Matches the oracle's MessageSigner.VerifyMessageAsync: the sender's
    // public key is always resolved from real pod membership, never taken
    // from a client-supplied field -- accepting a caller-supplied public
    // key would let anyone "verify" a signature they made up themselves
    // against a key they also made up, proving nothing about the actual
    // sender. A missing or non-ed25519 signature is honored as valid only
    // when the pod's configured signature mode isn't Enforce, matching the
    // same mode already enforced on real message posting.
    let signature = payload
        .get("signature")
        .or_else(|| message.get("signature"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let mode = state
        .advanced_networking
        .read()
        .await
        .pod_security_signature_mode;
    let verification_started_at = std::time::Instant::now();
    let is_valid =
        pod_verify_signature(&message, &canonical, &pod_id, signature, mode, state).await;
    state.pod_signature_stats.record_verify(
        verification_started_at.elapsed().as_millis() as u64,
        is_valid,
    );
    routing::ok_response(serde_json::json!({"isValid": is_valid}).to_string())
}

/// The real signature-check body of `pod_signing_response`'s "verify"
/// action, split out so it can also be reused by the pod membership
/// verification endpoint, whose `hasValidSignature` field runs the exact
/// same check (both wrap the oracle's single `MessageSigner.
/// VerifyMessageAsync`). Matches the oracle exactly: a malformed
/// signature is a verification failure (`false`), never a 400 -- the
/// oracle's own implementation returns `false` for malformed base64
/// rather than raising an error.
pub(super) async fn pod_verify_signature(
    message: &serde_json::Value,
    canonical: &str,
    pod_id: &str,
    signature: &str,
    mode: PodSignatureMode,
    state: &AppState,
) -> bool {
    if signature.is_empty() || !signature.starts_with("ed25519:") {
        return mode != PodSignatureMode::Enforce;
    }
    let stripped = signature.strip_prefix("ed25519:").unwrap_or(signature);
    let signature_bytes: [u8; 64] = match STANDARD
        .decode(stripped.as_bytes())
        .ok()
        .and_then(|bytes| bytes.try_into().ok())
    {
        Some(signature) => signature,
        None => return false,
    };
    let timestamp_unix_ms = message
        .get("timestampUnixMs")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(0);
    let now_ms = i64::try_from(unix_timestamp())
        .unwrap_or(i64::MAX)
        .saturating_mul(1000);
    if (now_ms - timestamp_unix_ms).abs() > 5 * 60 * 1000 {
        return false;
    }
    let sender_peer_id = message
        .get("senderPeerId")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if pod_id.is_empty() || sender_peer_id.is_empty() {
        return false;
    }
    let sender_public_key = state
        .pods
        .read()
        .await
        .members(pod_id)
        .and_then(|members| {
            members
                .into_iter()
                .find(|member| member.peer_id.eq_ignore_ascii_case(sender_peer_id))
        })
        .and_then(|member| member.public_key);
    let Some(sender_public_key) = sender_public_key else {
        return false;
    };
    let public_key: [u8; 32] = match STANDARD
        .decode(sender_public_key.as_bytes())
        .ok()
        .and_then(|bytes| bytes.try_into().ok())
    {
        Some(public_key) => public_key,
        None => return false,
    };
    VerifyingKey::from_bytes(&public_key)
        .ok()
        .is_some_and(|key| {
            key.verify(
                canonical.as_bytes(),
                &Signature::from_bytes(&signature_bytes),
            )
            .is_ok()
        })
}

/// Matches the oracle's `PodMembershipVerifier.VerifyMessageAsync`: real
/// pod-membership + signature checks, not the always-false stub this
/// endpoint used to return regardless of input.
async fn pod_verification_message_response(body: &str, state: &AppState) -> HttpResponse {
    const MAX_FIELD_LEN: usize = 512;
    let payload = match serde_json::from_str::<serde_json::Value>(body) {
        Ok(payload @ serde_json::Value::Object(_)) => payload,
        _ => return routing::bad_request_response("Message is required"),
    };
    let message_id = payload
        .get("messageId")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim();
    let pod_id = payload
        .get("podId")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim();
    let channel_id = payload
        .get("channelId")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim();
    let sender_peer_id = payload
        .get("senderPeerId")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim();
    let signature = payload
        .get("signature")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim();
    if message_id.is_empty()
        || pod_id.is_empty()
        || message_id.len() > MAX_FIELD_LEN
        || pod_id.len() > MAX_FIELD_LEN
        || channel_id.len() > MAX_FIELD_LEN
        || sender_peer_id.len() > MAX_FIELD_LEN
        || signature.len() > MAX_FIELD_LEN
    {
        return routing::bad_request_response(
            "Message fields are required and must be within length limits",
        );
    }

    // The membership check's podId comes from splitting channelId as
    // "podId:channelId", independent of the message's own explicit
    // `podId` field (used below for signature verification) -- this
    // mirrors the oracle's real (if slightly inconsistent) behavior
    // rather than unifying the two for tidiness.
    let Some((channel_pod_id, _)) = channel_id.split_once(':') else {
        return routing::ok_response(
            serde_json::json!({
                "isValid": false,
                "isFromValidMember": false,
                "hasValidSignature": false,
                "isNotBanned": false,
                "errorMessage": "Invalid channel ID format",
            })
            .to_string(),
        );
    };

    let member = state
        .pods
        .read()
        .await
        .members(channel_pod_id)
        .and_then(|members| {
            members
                .into_iter()
                .find(|candidate| candidate.peer_id.eq_ignore_ascii_case(sender_peer_id))
        });
    let is_from_valid_member = member.is_some();
    let is_not_banned = !member.as_ref().is_some_and(|member| member.is_banned);

    let mode = state
        .advanced_networking
        .read()
        .await
        .pod_security_signature_mode;
    let (canonical, signature_pod_id) = pod_message_canonical_payload(&payload);
    let started_at = std::time::Instant::now();
    let has_valid_signature = pod_verify_signature(
        &payload,
        &canonical,
        &signature_pod_id,
        signature,
        mode,
        state,
    )
    .await;
    let is_valid = is_from_valid_member && is_not_banned && has_valid_signature;

    state.pod_verification_stats.record(
        started_at.elapsed().as_millis() as u64,
        is_from_valid_member,
        is_not_banned,
        has_valid_signature,
        is_valid,
    );

    routing::ok_response(
        serde_json::json!({
            "isValid": is_valid,
            "isFromValidMember": is_from_valid_member,
            "hasValidSignature": has_valid_signature,
            "isNotBanned": is_not_banned,
        })
        .to_string(),
    )
}

/// Matches the oracle's `PodOpinionAggregator.GetMemberAffinitiesAsync`:
/// real per-member engagement/trust scoring from actual channel messages,
/// stored opinions, and membership records -- not the hardcoded zeros the
/// prior handler returned for every member regardless of real activity.
/// One honest simplification: the oracle scopes `opinionCount` to opinions
/// about this pod's own known content ids; slskR's opinion store has no
/// pod-to-content association to filter on, so this counts all opinions
/// issued by the member globally instead (documented here, not silently
/// invented).
async fn pod_member_affinities_json(state: &AppState, pod_id: &str) -> serde_json::Value {
    let (pod_members, channel_ids) = {
        let pods = state.pods.read().await;
        let members = pods.members(pod_id).unwrap_or_default();
        let channel_ids = pods
            .get(pod_id)
            .map(|pod| {
                pod.channels
                    .into_iter()
                    .map(|channel| channel.channel_id)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        (members, channel_ids)
    };
    let now = chrono::Utc::now();
    let activity_window_start_millis = (now - chrono::Duration::days(30)).timestamp_millis();
    let messages = state.pod_channels.read().await;
    let pod_messages = channel_ids
        .iter()
        .flat_map(|channel_id| messages.list(pod_id, channel_id, None))
        .collect::<Vec<_>>();
    drop(messages);
    let opinions = state
        .controller_features
        .read()
        .await
        .values_with_prefix("opinion/");

    let affinities = pod_members
        .into_iter()
        .map(|member| {
            let member_messages = pod_messages
                .iter()
                .filter(|message| message.sender_peer_id.eq_ignore_ascii_case(&member.peer_id));
            let recent_message_count = member_messages
                .clone()
                .filter(|message| message.timestamp_unix_ms as i64 >= activity_window_start_millis)
                .count();
            let last_message_millis = member_messages
                .map(|message| message.timestamp_unix_ms)
                .max();
            let opinion_count = opinions
                .iter()
                .filter(|opinion| {
                    opinion
                        .get("issuer")
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(|issuer| issuer.eq_ignore_ascii_case(&member.peer_id))
                })
                .count();
            let joined_at_millis = member
                .joined_at
                .as_deref()
                .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
                .map(|value| value.timestamp_millis());
            let membership_duration_seconds = joined_at_millis
                .map(|joined_at_millis| (now.timestamp_millis() - joined_at_millis) / 1000)
                .unwrap_or(0)
                .max(0);
            let last_seen_millis = member
                .last_seen
                .as_deref()
                .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
                .map(|value| value.timestamp_millis());
            let last_activity_millis = [
                last_seen_millis,
                last_message_millis.map(|value| value as i64),
            ]
            .into_iter()
            .flatten()
            .max();
            let is_active =
                last_activity_millis.is_some_and(|value| value >= activity_window_start_millis);
            let affinity_score = pod_member_affinity_score(
                recent_message_count,
                opinion_count,
                membership_duration_seconds,
                is_active,
            );
            let trust_score = pod_member_trust_score(&member.role, member.is_banned);
            let mut recent_activity = Vec::new();
            if recent_message_count > 0 {
                recent_activity.push(format!(
                    "{recent_message_count} messages in the last 30 days"
                ));
            }
            if opinion_count > 0 {
                recent_activity.push(format!("{opinion_count} opinions expressed"));
            }
            let last_activity = last_activity_millis
                .and_then(chrono::DateTime::from_timestamp_millis)
                .unwrap_or(now)
                .to_rfc3339();

            (
                member.peer_id.clone(),
                serde_json::json!({
                    "peerId": member.peer_id,
                    "affinityScore": affinity_score,
                    "messageCount": recent_message_count,
                    "opinionCount": opinion_count,
                    "membershipDuration": format_timespan_hms(membership_duration_seconds),
                    "lastActivity": last_activity,
                    "trustScore": trust_score,
                    "recentActivity": recent_activity,
                }),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    serde_json::Value::Object(affinities)
}

fn pod_membership_signed_record(
    state: &AppState,
    pod_id: &str,
    member: &pods::PodMember,
    action: &str,
    timestamp_unix_ms: u64,
) -> serde_json::Value {
    let canonical = format!(
        "{pod_id}|{}|{}|{action}|{timestamp_unix_ms}",
        member.peer_id, member.role,
    );
    let signature = state.capability_signing_key.sign(canonical.as_bytes());
    serde_json::json!({
        "podId": pod_id,
        "peerId": member.peer_id,
        "role": member.role,
        "action": action,
        "timestampUnixMs": timestamp_unix_ms,
        "publicKey": STANDARD.encode(state.capability_signing_key.verifying_key().as_bytes()),
        "signature": STANDARD.encode(signature.to_bytes()),
    })
}

fn pod_membership_publish_result(
    pod_id: &str,
    peer_id: &str,
    timestamp_unix_ms: u64,
) -> serde_json::Value {
    let published_at = chrono::DateTime::from_timestamp_millis(
        i64::try_from(timestamp_unix_ms).unwrap_or(i64::MAX),
    )
    .unwrap_or_else(chrono::Utc::now);
    serde_json::json!({
        "success": true,
        "podId": pod_id,
        "peerId": peer_id,
        "dhtKey": format!("pod:{pod_id}:member:{peer_id}"),
        "publishedAt": published_at.to_rfc3339(),
        "expiresAt": (published_at + chrono::Duration::hours(24)).to_rfc3339(),
    })
}

async fn podcore_local_backfill_response(
    state: &AppState,
    pod_id: &str,
    last_seen: &BTreeMap<String, u64>,
) -> Option<serde_json::Value> {
    let started_at = std::time::Instant::now();
    let pod = state.pods.read().await.get(pod_id)?;
    let messages = state.pod_channels.read().await;
    let mut synchronized = 0_usize;
    let mut bytes_transferred = 0_usize;
    let mut channels_requested = 0_usize;

    for channel in &pod.channels {
        let Some(last_seen_timestamp) = last_seen.get(&channel.channel_id) else {
            continue;
        };
        let mut channel_synchronized = false;
        for message in messages.list(pod_id, &channel.channel_id, None) {
            if message.timestamp_unix_ms > *last_seen_timestamp {
                synchronized = synchronized.saturating_add(1);
                channel_synchronized = true;
                // Match native profile's PodMessageBackfill.EstimateMessageSize:
                // message ID (50) + sender ID (50) + body + signature + metadata (100).
                bytes_transferred = bytes_transferred.saturating_add(
                    50usize
                        .saturating_add(50)
                        .saturating_add(message.body.len())
                        .saturating_add(message.signature.len())
                        .saturating_add(100),
                );
            }
        }
        if channel_synchronized {
            channels_requested += 1;
        }
    }
    drop(messages);

    let elapsed_ms = started_at.elapsed().as_millis() as u64;
    state.podcore_runtime_stats.record_backfill(
        pod_id,
        synchronized,
        bytes_transferred,
        elapsed_ms,
        true,
    );
    Some(serde_json::json!({
        "success": true,
        "podId": pod_id,
        "channelsRequested": channels_requested,
        "totalMessagesReceived": synchronized,
        "duration": format_timespan_hms(started_at.elapsed().as_secs() as i64),
    }))
}

fn podcore_storage_error_response(
    state: &AppState,
    versioned_v0: bool,
    frozen_message: &str,
    storage_error: &str,
) -> HttpResponse {
    if versioned_v0 && state.config.controller_profile == ControllerProfile::Native {
        routing::internal_server_error_response(frozen_message)
    } else {
        routing::service_unavailable_response(storage_error)
    }
}

fn pod_visibility_is_listed(value: &serde_json::Value) -> bool {
    value
        .as_str()
        .is_some_and(|value| value.eq_ignore_ascii_case("listed"))
        || value.as_i64() == Some(0)
}

pub(super) fn pod_visibility_name(value: &serde_json::Value) -> String {
    if let Some(value) = value.as_str() {
        return match value.trim().to_ascii_lowercase().as_str() {
            "listed" | "public" => "Listed".to_owned(),
            "unlisted" => "Unlisted".to_owned(),
            "private" => "Private".to_owned(),
            _ => value.trim().to_owned(),
        };
    }
    match value.as_i64() {
        Some(0) => "Listed".to_owned(),
        Some(1) => "Unlisted".to_owned(),
        Some(2) => "Private".to_owned(),
        _ => "Unknown".to_owned(),
    }
}

fn pod_discovery_name_slug(name: &str) -> String {
    name.trim().replace(' ', "-").to_ascii_lowercase()
}

fn pod_discovery_metadata_value(pod: &serde_json::Value) -> serde_json::Value {
    let default_visibility = serde_json::json!(1);
    let tags = pod
        .get("tags")
        .filter(|tags| tags.is_array())
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));
    let channel_count = pod
        .get("channels")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len);
    serde_json::json!({
        "podId": pod.get("podId").and_then(serde_json::Value::as_str).unwrap_or_default(),
        "name": pod.get("name").and_then(serde_json::Value::as_str).unwrap_or_default(),
        "visibility": pod_visibility_name(pod.get("visibility").unwrap_or(&default_visibility)),
        "focusContentId": pod.get("focusContentId").cloned().unwrap_or(serde_json::Value::Null),
        "tags": tags,
        "channelCount": channel_count,
        "publishedAt": unix_timestamp_millis(),
    })
}

fn pod_discovery_record_is_active(
    record: &serde_json::Value,
    now: chrono::DateTime<chrono::Utc>,
) -> bool {
    record
        .get("expiresAt")
        .and_then(serde_json::Value::as_str)
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.with_timezone(&chrono::Utc) > now)
        .unwrap_or(false)
}

fn pod_discovery_record_has_keys(record: &serde_json::Value, required_keys: &[String]) -> bool {
    let keys = record
        .get("discoveryKeys")
        .and_then(serde_json::Value::as_array);
    required_keys.iter().all(|required| {
        keys.is_some_and(|keys| {
            keys.iter()
                .filter_map(serde_json::Value::as_str)
                .any(|key| key == required)
        })
    })
}

fn pod_discovery_rows(
    records: &[serde_json::Value],
    pods: &pods::PodStore,
    required_keys: &[String],
) -> Vec<serde_json::Value> {
    let mut seen = HashSet::new();
    records
        .iter()
        .filter(|record| pod_discovery_record_has_keys(record, required_keys))
        .filter_map(|record| {
            record
                .get("pod")
                .filter(|pod| pod.is_object())
                .cloned()
                .or_else(|| {
                    record
                        .get("podId")
                        .and_then(serde_json::Value::as_str)
                        .and_then(|pod_id| pods.get(pod_id).map(|pod| serde_json::json!(pod)))
                })
        })
        .map(|pod| pod_discovery_metadata_value(&pod))
        .filter(|metadata| {
            metadata
                .get("podId")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|pod_id| seen.insert(pod_id.to_owned()))
        })
        .collect()
}

async fn podcore_discovery_refresh_response(state: &AppState) -> HttpResponse {
    if state
        .controller_features
        .read()
        .await
        .validate_storage()
        .is_err()
    {
        return routing::internal_server_error_response("Failed to refresh discovery");
    }
    let now = chrono::Utc::now();
    let refresh_before = now + chrono::Duration::hours(6);
    let entries = state
        .controller_features
        .read()
        .await
        .entries_with_prefix("pod/discovery/");
    let mut refreshed = 0_u64;
    let mut errors = 0_u64;

    for (key, mut record) in entries {
        let Some(expires_at) = record
            .get("expiresAt")
            .and_then(serde_json::Value::as_str)
            .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
            .map(|value| value.with_timezone(&chrono::Utc))
        else {
            errors += 1;
            continue;
        };
        if expires_at >= refresh_before {
            continue;
        }

        let Some(pod_id) = record
            .get("podId")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
        else {
            errors += 1;
            continue;
        };

        let pod_exists = state.pods.read().await.get(&pod_id).is_some();
        let result = if pod_exists {
            record["registeredAt"] = serde_json::json!(now.to_rfc3339());
            record["expiresAt"] =
                serde_json::json!((now + chrono::Duration::hours(24)).to_rfc3339());
            state.controller_features.upsert(key, record).await
        } else {
            state.controller_features.remove(&key).await.map(|_| ())
        };
        match result {
            Ok(()) => {
                if pod_exists {
                    refreshed += 1;
                }
            }
            Err(_) => errors += 1,
        }
    }

    let mut response = serde_json::json!({
        "success": errors == 0,
        "podId": "all",
        "wasRepublished": refreshed > 0,
        "nextRefresh": (chrono::Utc::now() + chrono::Duration::hours(1)).to_rfc3339(),
    });
    if errors > 0 {
        response["errorMessage"] =
            serde_json::json!(format!("{errors} errors occurred during refresh"));
    }
    routing::ok_response(response.to_string())
}

pub(super) fn dynamic_podcore_get_route(path: &str) -> bool {
    let rest = path.trim_start_matches("/api/podcore/");
    let segments = rest.split('/').collect::<Vec<_>>();
    matches!(
        segments.as_slice(),
        [_, "channels"]
            | [_, "channels", _]
            | [_, "opinions", "content", _]
            | [_, "opinions", "content", _, "aggregated"]
            | [_, "opinions", "content", _, "recommendations"]
            | [_, "opinions", "content", _, "stats"]
            | [_, "opinions", "content", _, "variant", _]
            | [_, "opinions", "members", "affinity"]
            | ["backfill", _, "last-seen"]
            | ["dht", "metadata", _]
            | ["discovery", "content", _]
            | ["discovery", "name", _]
            | ["discovery", "tag", _]
            | ["discovery", "tags", _]
            | ["membership", _, _]
            | ["membership", _, _, "verify"]
            | ["messages", _, _, "count"]
            | ["messages", _, "search"]
            | ["routing", "seen", _, _]
            | ["verification", "membership", _, _]
            | ["verification", "role", _, _, _]
    ) || (segments.len() >= 3 && segments[0] == "discovery" && segments[1] == "content")
        || (segments.len() >= 3 && segments[0] == "dht" && segments[1] == "metadata")
}

fn verify_pod_dht_publication(
    published_pod: &serde_json::Value,
    signature: &str,
    public_key: &str,
) -> bool {
    let Ok(signature_bytes) = STANDARD.decode(signature.as_bytes()) else {
        return false;
    };
    let Ok(signature_bytes) = <[u8; 64]>::try_from(signature_bytes.as_slice()) else {
        return false;
    };
    let Ok(public_key_bytes) = STANDARD.decode(public_key.as_bytes()) else {
        return false;
    };
    let Ok(public_key_bytes) = <[u8; 32]>::try_from(public_key_bytes.as_slice()) else {
        return false;
    };
    let Ok(public_key) = VerifyingKey::from_bytes(&public_key_bytes) else {
        return false;
    };
    let Ok(canonical) = serde_json::to_vec(published_pod) else {
        return false;
    };
    public_key
        .verify(&canonical, &Signature::from_bytes(&signature_bytes))
        .is_ok()
}
