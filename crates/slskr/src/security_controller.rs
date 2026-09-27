use super::*;

pub(super) async fn security_extended_response(
    path: &str,
    query: Option<&str>,
    state: &AppState,
) -> HttpResponse {
    let list_limit = if matches!(
        path,
        "/api/security/events"
            | "/api/security/anomalies"
            | "/api/security/reputation/suspicious"
            | "/api/security/reputation/trusted"
            | "/api/security/network/top"
    ) {
        let (name, error_message) =
            if matches!(path, "/api/security/events" | "/api/security/anomalies") {
                ("count", "count must be positive")
            } else {
                ("limit", "limit must be positive")
            };
        match query_bounded_usize(query, name, 1, 1_000) {
            Ok(Some(value)) => value,
            Ok(None) => 100,
            Err(()) => return routing::bad_request_response(error_message),
        }
    } else {
        1_000
    };
    match path {
        "/api/security/bans" => {
            let security = state.security.read().await;
            routing::ok_response(security.json_value().to_string())
        }
        "/api/security/events" | "/api/security/anomalies" => {
            let events = state.events.read().await;
            let rows = events
                .records
                .iter()
                .rev()
                .take(list_limit)
                .map(|event| {
                    serde_json::json!({
                        "eventId": event.id,
                        "eventType": event.kind,
                        "severity": if event.kind.contains("failed") || event.kind.contains("rejected") { "Warning" } else { "Info" },
                        "description": event.detail,
                        "source": event.resource,
                        "timestamp": event.created_at,
                    })
                })
                .collect::<Vec<_>>();
            routing::ok_response(serde_json::Value::Array(rows).to_string())
        }
        "/api/security/reputation/suspicious" | "/api/security/reputation/trusted" => {
            // Matches the oracle's real GetSuspiciousPeers/GetTrustedPeers:
            // real per-peer scores (SecurityState.reputation), sorted
            // worst/best-first, filtered by the oracle's real
            // Untrusted/Trusted thresholds -- not watch/online status,
            // which has nothing to do with reputation.
            let trusted = path.ends_with("/trusted");
            let security = state.security.read().await;
            let mut peers = security
                .reputation
                .iter()
                .map(|(username, score)| (username.clone(), *score))
                .collect::<Vec<_>>();
            peers.retain(|(_, score)| {
                if trusted {
                    *score >= SECURITY_REPUTATION_TRUSTED_THRESHOLD
                } else {
                    *score < SECURITY_REPUTATION_DEFAULT_SCORE
                }
            });
            if trusted {
                peers.sort_by_key(|peer| std::cmp::Reverse(peer.1));
            } else {
                peers.sort_by_key(|peer| peer.1);
            }
            peers.truncate(list_limit);
            let rows = peers
                .into_iter()
                .map(|(username, score)| {
                    let profile = security
                        .reputation_profiles
                        .get(&username)
                        .cloned()
                        .unwrap_or_else(|| {
                            SecurityReputationProfile::new(&username, unix_timestamp())
                        });
                    security_reputation_profile_value(&profile, score)
                })
                .collect::<Vec<_>>();
            routing::ok_response(serde_json::Value::Array(rows).to_string())
        }
        "/api/security/scanners" | "/api/security/threats" => {
            let feature_key = if path.ends_with("/scanners") {
                "security/profile/security/scanners"
            } else {
                "security/profile/security/threats"
            };
            let configured = state
                .controller_features
                .read()
                .await
                .get(feature_key)
                .and_then(|value| value.get("items"))
                .cloned();
            routing::ok_response(
                configured
                    .unwrap_or_else(|| serde_json::json!([]))
                    .to_string(),
            )
        }
        "/api/security/canaries" => {
            let configured = state
                .controller_features
                .read()
                .await
                .get("security/profile/security/canaries")
                .and_then(|value| value.get("settings"))
                .cloned();
            routing::ok_response(
                configured
                    .unwrap_or_else(|| {
                        serde_json::json!({
                            "totalAccesses": 0,
                            "uniqueAttackers": 0,
                            "activeTraps": 0,
                            "triggeredTraps": 0,
                            "trapsByType": {},
                        })
                    })
                    .to_string(),
            )
        }
        "/api/security/network" => {
            let listeners = state.listeners.read().await;
            let total_connections = listeners.regular_accepts + listeners.obfuscated_accepts;
            let policy = state
                .advanced_networking
                .read()
                .await
                .security
                .network_guard
                .clone();
            let (active_connections, tracked_ips) = {
                let active = state
                    .incoming_connection_ips
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                (active.values().copied().sum::<usize>(), active.len())
            };
            routing::ok_response(
                serde_json::json!({
                    "activeConnections": active_connections,
                    "globalConnections": active_connections,
                    "trackedIps": tracked_ips,
                    "totalConnections": total_connections,
                    "totalMessages": state.events.read().await.records.len(),
                    "rateLimitHits": 0,
                    "maxConnectionsPerIp": policy.max_connections_per_ip,
                    "maxGlobalConnections": policy.max_global_connections,
                    "maxMessagesPerMinute": policy.max_messages_per_minute,
                    "blockedConnections": listeners.transfer_rejections,
                    "rateLimitedConnections": 0,
                    "errors": listeners.errors,
                    "updatedAt": listeners.updated_at,
                })
                .to_string(),
            )
        }
        "/api/security/network/top" | "/api/security/peers" => {
            let users = state.users.read().await;
            let rows = users
                .records
                .iter()
                .take(list_limit)
                .map(|user| {
                    serde_json::json!({
                        "username": user.username,
                        "status": user.status,
                        "watched": user.watched,
                        "averageSpeed": user.average_speed.unwrap_or(0),
                        "fileCount": user.file_count.unwrap_or(0),
                        "updatedAt": user.updated_at,
                    })
                })
                .collect::<Vec<_>>();
            routing::ok_response(serde_json::Value::Array(rows).to_string())
        }
        "/api/security/adversarial" => {
            let configured = state
                .controller_features
                .read()
                .await
                .get("security/profile/security/adversarial")
                .and_then(|value| value.get("settings"))
                .cloned();
            routing::ok_response(
                configured
                    .unwrap_or_else(|| {
                        serde_json::json!({
                            "enabled": false,
                            "injectLatency": false,
                            "dropMessages": false,
                            "corruptMessages": false,
                            "reorderMessages": false,
                            "partitionNetwork": false,
                        })
                    })
                    .to_string(),
            )
        }
        "/api/security/adversarial/stats" => routing::ok_response(
            serde_json::json!({
                "enabled": false,
                "profile": "Standard",
                "privacyEnabled": false,
                "anonymityEnabled": false,
                "transportEnabled": state.private_gateway.is_some(),
                "plausibleDeniabilityEnabled": false,
                "onionRoutingEnabled": false,
                "censorshipResistanceEnabled": false,
                "messagesProcessed": 0,
                "messagesDropped": 0,
                "messagesCorrupted": 0,
                "messagesReordered": 0,
                "latencyInjected": 0,
                "partitionsCreated": 0,
            })
            .to_string(),
        ),
        "/api/security/tor/status" => {
            let configured = state
                .controller_features
                .read()
                .await
                .get("security/profile/security/tor")
                .and_then(|value| value.get("settings"))
                .cloned();
            routing::ok_response(
                configured
                    .unwrap_or_else(|| {
                        serde_json::json!({
                            "type": "Tor",
                            "available": false,
                            "connected": false,
                            "healthy": false,
                            "errorMessage": "Tor transport is not configured",
                        })
                    })
                    .to_string(),
            )
        }
        "/api/security/transports/status" => {
            let mesh_enabled = state.private_gateway.is_some();
            routing::ok_response(
                serde_json::json!({
                    "selectedTransport": if mesh_enabled { "Direct" } else { "None" },
                    "availableTransports": if mesh_enabled { vec!["Direct"] } else { Vec::<&str>::new() },
                    "healthy": mesh_enabled,
                })
                .to_string(),
            )
        }
        "/api/security/transports" => {
            let mesh_enabled = state.private_gateway.is_some();
            let configured = state
                .controller_features
                .read()
                .await
                .get("security/profile/security/transports")
                .and_then(|value| value.get("items"))
                .cloned();
            routing::ok_response(
                configured
                    .unwrap_or_else(|| {
                        serde_json::json!({
                            "Direct": {
                                "type": "Direct",
                                "available": mesh_enabled,
                                "connected": mesh_enabled,
                                "healthy": mesh_enabled,
                            },
                            "Tor": {
                                "type": "Tor",
                                "available": false,
                                "connected": false,
                                "healthy": false,
                            }
                        })
                    })
                    .to_string(),
            )
        }
        "/api/security/circuits" | "/api/security/circuits/stats" => {
            let circuits = state
                .controller_features
                .read()
                .await
                .values_with_prefix("security/circuit/");
            if path.ends_with("/stats") {
                routing::ok_response(
                    serde_json::json!({
                        "totalCircuits": circuits.len(),
                        "totalCircuitsBuilt": circuits.len(),
                        "activeCircuits": circuits.iter().filter(|circuit| circuit.get("active").and_then(serde_json::Value::as_bool).unwrap_or(true)).count(),
                        "averageCircuitLength": 0.0,
                        "circuitLengths": {},
                        "failedCircuits": circuits.iter().filter(|circuit| circuit.get("failed").and_then(serde_json::Value::as_bool).unwrap_or(false)).count(),
                        "totalPeers": circuits.iter().filter_map(|circuit| circuit.get("peerId").and_then(serde_json::Value::as_str)).collect::<HashSet<_>>().len(),
                    })
                    .to_string(),
                )
            } else {
                routing::ok_response(serde_json::Value::Array(circuits).to_string())
            }
        }
        "/api/security/peers/stats" => {
            let users = state.users.read().await;
            routing::ok_response(
                serde_json::json!({
                    "totalPeers": users.records.len(),
                    "activePeers": users.records.iter().filter(|user| user.status.as_deref() == Some("online")).count(),
                    "onionRoutingPeers": 0,
                    "averageQualityScore": 0.0,
                    "averageLatencyMs": 0.0,
                    "onlinePeers": users.records.iter().filter(|user| user.status.as_deref() == Some("online")).count(),
                    "watchedPeers": users.records.iter().filter(|user| user.watched).count(),
                    "bannedPeers": state.security.read().await.active_bans(),
                })
                .to_string(),
            )
        }
        _ => {
            if let Some(username) = path_segment_after(path, "/api/security/reputation/") {
                let username = decoded_path_segment(username);
                return routing::ok_response(
                    security_reputation_profile_json(state, &username)
                        .await
                        .to_string(),
                );
            }
            if let Some(username) = path_segment_after(path, "/api/security/disclosure/") {
                let username = decoded_path_segment(username);
                let configured = state
                    .controller_features
                    .read()
                    .await
                    .get(&format!("security/profile/security/disclosure/{username}"))
                    .cloned();
                let settings = configured
                    .as_ref()
                    .and_then(|value| value.get("settings"))
                    .unwrap_or(&serde_json::Value::Null);
                let peer_tier = settings
                    .get("tier")
                    .or_else(|| settings.get("peerTier"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("Unknown");
                return routing::ok_response(
                    serde_json::json!({
                        "peerTier": peer_tier,
                        "maxDisclosureTier": settings.get("maxDisclosureTier").and_then(serde_json::Value::as_str).unwrap_or("Public"),
                        "canSeeFileCounts": settings.get("canSeeFileCounts").and_then(serde_json::Value::as_bool).unwrap_or(false),
                        "canSeeFolderStructure": settings.get("canSeeFolderStructure").and_then(serde_json::Value::as_bool).unwrap_or(false),
                        "canSeeFullPaths": settings.get("canSeeFullPaths").and_then(serde_json::Value::as_bool).unwrap_or(false),
                        "canSeeAudioMetadata": settings.get("canSeeAudioMetadata").and_then(serde_json::Value::as_bool).unwrap_or(false),
                        "canBrowse": settings.get("canBrowse").and_then(serde_json::Value::as_bool).unwrap_or(false),
                        "canDownload": settings.get("canDownload").and_then(serde_json::Value::as_bool).unwrap_or(false),
                    })
                    .to_string(),
                );
            }
            routing::not_found_response()
        }
    }
}

pub(super) fn native_adversarial_mutation_response<'a>(
    body: &'a str,
    state: &'a AppState,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = HttpResponse> + Send + 'a>> {
    Box::pin(async move {
        let payload = match serde_json::from_str::<serde_json::Value>(body) {
            Ok(payload @ serde_json::Value::Object(_)) => payload,
            Ok(_) => return routing::bad_request_response("security body must be an object"),
            Err(_) => return routing::bad_request_response("invalid JSON body"),
        };
        if let Err(error) = validate_native_adversarial_settings(&payload) {
            return HttpResponse {
                status: "400 Bad Request",
                content_type: "text/plain; charset=utf-8",
                body: error.to_owned(),
            };
        }
        let current = match read_controller_compatibility_yaml(&state.config) {
            Ok(Some(current)) => current,
            Ok(None) => {
                return HttpResponse {
                    status: "404 Not Found",
                    content_type: "application/json; charset=utf-8",
                    body: serde_json::json!("Configuration file not found").to_string(),
                };
            }
            Err(error) => {
                record_daemon_log(
                    state,
                    logging::LogLevel::Error,
                    "configuration",
                    format!("failed to read adversarial configuration: {error}"),
                )
                .await;
                return routing::internal_server_error_response("Failed to persist settings");
            }
        };
        let updated = match native_adversarial_yaml_update(&current, &payload) {
            Ok(updated) => updated,
            Err(error) => return routing::bad_request_response(&error),
        };
        if let Err(error) = write_controller_compatibility_yaml(&state.config, &updated) {
            record_daemon_log(
                state,
                logging::LogLevel::Error,
                "configuration",
                format!("failed to persist adversarial settings: {error}"),
            )
            .await;
            return routing::internal_server_error_response("Failed to persist settings");
        }

        let key = "security/adversarial";
        let value = serde_json::json!({
            "resource": key,
            "settings": payload,
            "updatedAt": unix_timestamp(),
        });
        match state
            .controller_features
            .upsert(format!("security/profile/{key}"), value)
            .await
        {
            Ok(()) => HttpResponse {
                status: "200 OK",
                content_type: "application/json; charset=utf-8",
                body: serde_json::json!({
                    "message": if effective_controller_no_config_watch(state) {
                        "Adversarial settings updated. Restart required for changes to take effect."
                    } else {
                        "Adversarial settings updated successfully"
                    }
                })
                .to_string(),
            },
            Err(error) => routing::service_unavailable_response(&error),
        }
    })
}

fn security_reputation_trust_level(score: i32) -> &'static str {
    if score >= SECURITY_REPUTATION_TRUSTED_THRESHOLD {
        "Trusted"
    } else if score <= SECURITY_REPUTATION_UNTRUSTED_THRESHOLD {
        "Untrusted"
    } else {
        "Neutral"
    }
}

fn security_reputation_profile_value(
    profile: &SecurityReputationProfile,
    score: i32,
) -> serde_json::Value {
    let total_transfers = profile
        .successful_transfers
        .saturating_add(profile.failed_transfers)
        .saturating_add(profile.aborted_transfers);
    let success_rate = if total_transfers > 0 {
        profile.successful_transfers as f64 / total_transfers as f64
    } else {
        0.5
    };
    serde_json::json!({
        "username": profile.username,
        "score": score,
        "firstSeen": unix_seconds_rfc3339(profile.first_seen),
        "lastSeen": unix_seconds_rfc3339(profile.last_seen),
        "successfulTransfers": profile.successful_transfers,
        "failedTransfers": profile.failed_transfers,
        "abortedTransfers": profile.aborted_transfers,
        "totalBytesTransferred": profile.total_bytes_transferred,
        "malformedMessages": profile.malformed_messages,
        "protocolViolations": profile.protocol_violations,
        "contentMismatches": profile.content_mismatches,
        "slotsAvailableCount": profile.slots_available_count,
        "successRate": success_rate,
        "trustLevel": security_reputation_trust_level(score),
    })
}

/// Matches the oracle's real `PeerReputation`/`PeerProfile` shape for a
/// single peer, sourced from slskR's real, violation-driven score and
/// violation counters (`SecurityState.reputation`/`violations`) rather
/// than a disconnected, admin-only settings blob that never reflected
/// real behavior. Transfer counters are reconciled from the retained real
/// transfer history on each read, so the profile does not report a fabricated
/// zero after an actual transfer has completed or failed.
async fn security_reputation_profile_json(state: &AppState, username: &str) -> serde_json::Value {
    let key = username.to_ascii_lowercase();
    let transfer_metrics = {
        let transfers = state.transfers.read().await;
        let mut successful = 0_u64;
        let mut failed = 0_u64;
        let mut aborted = 0_u64;
        let mut bytes = 0_u64;
        let mut content_mismatches = 0_u64;
        let mut first_seen: Option<u64> = None;
        let mut last_seen: Option<u64> = None;
        for entry in transfers.entries.iter().filter(|entry| {
            entry
                .peer_username
                .as_deref()
                .is_some_and(|peer| peer.eq_ignore_ascii_case(username))
        }) {
            first_seen =
                Some(first_seen.map_or(entry.requested_at, |seen| seen.min(entry.requested_at)));
            last_seen = Some(last_seen.map_or(entry.updated_at, |seen| seen.max(entry.updated_at)));
            bytes = bytes.saturating_add(entry.bytes_transferred);
            match entry.status.as_str() {
                "succeeded" | "completed" => successful = successful.saturating_add(1),
                "failed" | "rejected" | "errored" => failed = failed.saturating_add(1),
                "cancelled" | "aborted" => aborted = aborted.saturating_add(1),
                _ => {}
            }
            if entry.reason.as_deref().is_some_and(|reason| {
                let reason = reason.to_ascii_lowercase();
                reason.contains("mismatch") || reason.contains("hash mismatch")
            }) {
                content_mismatches = content_mismatches.saturating_add(1);
            }
        }
        (
            successful,
            failed,
            aborted,
            bytes,
            content_mismatches,
            first_seen,
            last_seen,
        )
    };
    let mut security = state.security.write().await;
    let legacy_protocol_violations = security.violations.get(&key).copied().unwrap_or(0);
    let had_profile = security.reputation_profiles.contains_key(&key);
    security.ensure_reputation_profile(&key, username);
    if let Some(profile) = security.reputation_profiles.get_mut(&key) {
        if !had_profile {
            profile.protocol_violations = u64::from(legacy_protocol_violations);
        }
        profile.successful_transfers = transfer_metrics.0;
        profile.failed_transfers = transfer_metrics.1;
        profile.aborted_transfers = transfer_metrics.2;
        profile.total_bytes_transferred = transfer_metrics.3;
        profile.content_mismatches = transfer_metrics.4;
        if let Some(first_seen) = transfer_metrics.5 {
            profile.first_seen = profile.first_seen.min(first_seen);
        }
        if let Some(last_seen) = transfer_metrics.6 {
            profile.last_seen = profile.last_seen.max(last_seen);
        }
    }
    let score = security
        .reputation
        .get(&key)
        .copied()
        .unwrap_or(SECURITY_REPUTATION_DEFAULT_SCORE);
    let profile = security
        .reputation_profiles
        .get(&key)
        .expect("reputation profile is created above")
        .clone();
    drop(security);
    security_reputation_profile_value(&profile, score)
}

pub(super) fn normalize_security_ban_value(kind: &str, value: &str) -> Option<String> {
    match kind {
        "username" => {
            let value =
                truncate_utf8_bytes(value.trim().to_owned(), MAX_SECURITY_BAN_USERNAME_BYTES);
            (!value.is_empty()).then_some(value)
        }
        "ip" => value
            .trim()
            .parse::<std::net::IpAddr>()
            .ok()
            .map(|address| address.to_string()),
        _ => None,
    }
}

pub(super) fn security_ban_options(body: &str) -> (String, u64, bool) {
    let reason = extract_json_string_field(body, "reason")
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "Manual ban".to_owned());
    let is_permanent = extract_json_bool_field(body, "permanent").unwrap_or(false);
    let parsed_duration = extract_json_string_field(body, "duration").and_then(|duration| {
        let (days, clock) = duration
            .split_once('.')
            .and_then(|(days, clock)| days.parse::<u64>().ok().map(|days| (days, clock)))
            .unwrap_or((0, duration.as_str()));
        let parts = clock.split(':').collect::<Vec<_>>();
        if parts.len() != 3 {
            return None;
        }
        let hours = parts[0].parse::<u64>().ok()?;
        let minutes = parts[1].parse::<u64>().ok()?;
        let seconds = parts[2].split('.').next()?.parse::<u64>().ok()?;
        (minutes < 60 && seconds < 60).then(|| {
            days.saturating_mul(86_400)
                .saturating_add(hours.saturating_mul(3_600))
                .saturating_add(minutes.saturating_mul(60))
                .saturating_add(seconds)
        })
    });
    let duration_seconds = if is_permanent {
        3_650 * 86_400
    } else {
        parsed_duration
            .or_else(|| {
                extract_json_u64_field(body, "durationMinutes")
                    .map(|minutes| minutes.saturating_mul(60))
            })
            .unwrap_or(3_600)
    };
    (reason, duration_seconds, is_permanent)
}

pub(super) fn security_ban_route_tail(path: &str) -> Option<Vec<&str>> {
    let segments = path.strip_prefix("/api/")?.split('/').collect::<Vec<_>>();
    match segments.as_slice() {
        ["bans", tail @ ..] | [_, "bans", tail @ ..] => Some(tail.to_vec()),
        _ => None,
    }
}
