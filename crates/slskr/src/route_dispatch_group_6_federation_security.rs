async fn route_dispatch_group_6_federation_security(
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
        ("GET", "/api/solid/status") => {
            let media = state.media_services.read().await;
            Ok(routing::ok_response(
                serde_json::json!({
                    "enabled": true,
                    "clientId": media
                        .solid
                        .client_id_url
                        .clone()
                        .unwrap_or_else(|| "/solid/clientid.jsonld".to_owned()),
                    "redirectPath": media.solid.redirect_path,
                })
                .to_string(),
            ))
        }

        ("GET", "/api/federation/diagnostics")
            if route.path == "/api/v0/federation/diagnostics" =>
        {
            Ok(federation_diagnostics_response(&state.config))
        }

        ("GET", "/api/federation/diagnostics") => {
            let users = state.users.read().await;
            let mesh = state.mesh.read().await;
            let user_count = users.records.len();
            let watched_users = users.records.iter().filter(|user| user.watched).count();
            let mesh_capabilities = mesh.capability_records.len();
            let items = mesh
                .capability_records
                .iter()
                .map(|record| {
                    serde_json::json!({
                        "username": record.username,
                        "issuedAt": record.issued_at_unix,
                        "expiresAt": record.expires_at_unix,
                        "features": record.features.clone(),
                        "endpoints": record.endpoints.clone(),
                        "source": "peer-capability",
                    })
                })
                .chain(
                    users
                        .records
                        .iter()
                        .filter(|user| user.watched)
                        .map(|user| {
                            serde_json::json!({
                                "username": user.username,
                                "status": user.status,
                                "source": "watched-user",
                            })
                        }),
                )
                .collect::<Vec<_>>();
            let item_count = items.len();
            let checks = vec![
                serde_json::json!({
                    "id": "watched-users",
                    "status": if users.records.is_empty() { "empty" } else { "ready" },
                    "count": users.records.len(),
                }),
                serde_json::json!({
                    "id": "mesh-capabilities",
                    "status": if mesh.capability_records.is_empty() { "empty" } else { "ready" },
                    "count": mesh.capability_records.len(),
                }),
            ];
            let ready = checks.iter().any(|check| {
                check.get("status").and_then(serde_json::Value::as_str) == Some("ready")
            });
            drop(mesh);
            drop(users);
            Ok(routing::ok_response(
                serde_json::json!({
                    "federation": {"enabled": ready, "watchedUsers": watched_users},
                    "publishing": {"enabled": false},
                    "pods": {"enabled": true},
                    "mesh": {"capabilityRecords": mesh_capabilities},
                    "status": if ready { "ready" } else { "empty" },
                    "checks": checks,
                    "items": items,
                    "itemCount": item_count,
                    "counts": {
                        "users": user_count,
                        "watchedUsers": watched_users,
                        "meshCapabilities": mesh_capabilities,
                    },
                    "warnings": [],
                    "errors": [],
                })
                .to_string(),
            ))
        }

        ("GET", "/api/security/dashboard") => {
            let users = state.users.read().await;
            let webhooks = state.webhooks.read().await;
            let events = state.events.read().await;
            let security = state.security.read().await;
            let watched = users.records.iter().filter(|user| user.watched).count();
            let webhook_count = webhooks.get_all().len();
            let event_count = events.records.len();
            let ban_count = security.active_bans();
            let bans = security
                .json_value()
                .get("bans")
                .cloned()
                .unwrap_or_else(|| serde_json::json!([]));
            // Matches the oracle's real PeerReputation.GetStats(): real
            // per-peer scores and violation counts, not watch/online
            // status (which has nothing to do with reputation).
            let mut peer_keys = security
                .reputation
                .keys()
                .chain(security.reputation_profiles.keys())
                .cloned()
                .collect::<Vec<_>>();
            peer_keys.sort_unstable();
            peer_keys.dedup();
            let total_peers = peer_keys.len();
            let mut total_score = 0_i64;
            let mut trusted_peers = 0;
            let mut untrusted_peers = 0;
            let mut total_successful_transfers = 0_u64;
            let mut total_failed_transfers = 0_u64;
            let mut total_protocol_violations = 0_u64;
            for username in &peer_keys {
                let score = security
                    .reputation
                    .get(username)
                    .copied()
                    .unwrap_or(SECURITY_REPUTATION_DEFAULT_SCORE);
                total_score += i64::from(score);
                if score >= SECURITY_REPUTATION_TRUSTED_THRESHOLD {
                    trusted_peers += 1;
                }
                if score <= SECURITY_REPUTATION_UNTRUSTED_THRESHOLD {
                    untrusted_peers += 1;
                }
                if let Some(profile) = security.reputation_profiles.get(username) {
                    total_successful_transfers =
                        total_successful_transfers.saturating_add(profile.successful_transfers);
                    total_failed_transfers =
                        total_failed_transfers.saturating_add(profile.failed_transfers);
                    total_protocol_violations =
                        total_protocol_violations.saturating_add(profile.protocol_violations);
                } else {
                    total_protocol_violations = total_protocol_violations.saturating_add(
                        u64::from(security.violations.get(username).copied().unwrap_or(0)),
                    );
                }
            }
            let average_score = if total_peers > 0 {
                total_score as f64 / total_peers as f64
            } else {
                f64::from(SECURITY_REPUTATION_DEFAULT_SCORE)
            };
            let reputation_stats = serde_json::json!({
                "totalPeers": total_peers,
                "trustedPeers": trusted_peers,
                "untrustedPeers": untrusted_peers,
                "averageScore": average_score,
                "totalSuccessfulTransfers": total_successful_transfers,
                "totalFailedTransfers": total_failed_transfers,
                "totalProtocolViolations": total_protocol_violations,
            });
            drop(security);
            drop(events);
            drop(webhooks);
            drop(users);
            Ok(routing::ok_response(
                serde_json::json!({
                    "eventStats": {"totalEvents": event_count},
                    "networkGuardStats": {"globalConnections": watched},
                    "violationStats": {"activeBans": ban_count},
                    "reputationStats": reputation_stats.clone(),
                    "paranoidStats": {"enabled": false},
                    "fingerprintStats": {"knownFingerprints": 0},
                    "entropyStats": {"checks": 0},
                    "consensusStats": {"decisions": 0},
                    "verificationStats": {"verified": 0},
                    "disclosureStats": {"disclosures": 0},
                    "temporalStats": {"events": event_count},
                    "enabled": true,
                    "status": "local",
                    "stats": {
                        "networkGuardStats": { "globalConnections": watched },
                        "reputationStats": reputation_stats,
                        "threatStats": { "activeThreats": untrusted_peers },
                        "banStats": { "activeBans": ban_count }
                    },
                    "events": event_count,
                    "webhooks": webhook_count,
                    "bans": bans
                })
                .to_string(),
            ))
        }

        ("GET", "/api/security/status") => {
            let users = state.users.read().await;
            let events = state.events.read().await;
            let security = state.security.read().await;
            let watched = users.records.iter().filter(|user| user.watched).count();
            let offline_watched = users
                .records
                .iter()
                .filter(|user| user.watched && user.status.as_deref() == Some("offline"))
                .count();
            let event_count = events.records.len();
            let ban_count = security.active_bans();
            drop(security);
            drop(events);
            drop(users);
            Ok(routing::ok_response(
                serde_json::json!({
                    "enabled": true,
                    "status": "local",
                    "watchedPeers": watched,
                    "suspiciousPeers": offline_watched,
                    "activeBans": ban_count,
                    "events": event_count,
                })
                .to_string(),
            ))
        }

        ("GET", path) if path.starts_with("/api/security/") => {
            Ok(security_extended_response(path, route.query, state).await)
        }

        ("GET", "/api/soulseek/mesh-rendezvous/status") => {
            let users = state.users.read().await;
            let mesh = state.mesh.read().await;
            let body = mesh.status_json(&users);
            drop(mesh);
            drop(users);
            Ok(routing::ok_response(body))
        }

        ("GET", "/api/soulseek/mesh-rendezvous/users") => {
            let users = state.users.read().await;
            let mesh = state.mesh.read().await;
            let body = mesh.users_json(&users);
            drop(mesh);
            drop(users);
            Ok(routing::ok_response(body))
        }

        ("GET", "/api/soulseek/mesh-rendezvous/discover") => {
            let users = state.users.read().await;
            let mesh = state.mesh.read().await;
            let candidates = if mesh.rendezvous.active_probe_enabled() {
                mesh.candidate_usernames(&users)
                    .into_iter()
                    .take(MAX_ACTIVE_MESH_DISCOVERY_PROBES)
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            let body = mesh.discover_json(&users);
            drop(mesh);
            drop(users);
            for username in candidates {
                if let Err(error) =
                    send_session_command(state, SessionCommand::ProbePeerCapability(username)).await
                {
                    return Ok(routing::service_unavailable_response(&error));
                }
            }
            Ok(routing::ok_response(body))
        }

        ("GET", "/api/soulseek/peer-capabilities") => {
            let mesh = state.mesh.read().await;
            let body = serde_json::Value::Array(mesh.capability_records_json()).to_string();
            drop(mesh);
            Ok(routing::ok_response(body))
        }

        ("GET", "/api/mesh/transport") if route.path.starts_with("/api/v0/") => {
            let dht_sessions = match state.dht.as_ref() {
                Some(dht) => dht.peers().await.len(),
                None => 0,
            };
            let overlay_sessions = match state.private_gateway.as_ref() {
                Some(gateway) => gateway.active_connection_count().await,
                None => 0,
            };
            Ok(routing::ok_response(
                serde_json::json!({
                    "dht": dht_sessions,
                    "overlay": overlay_sessions,
                    "natType": "Unknown",
                })
                .to_string(),
            ))
        }
        ("GET", "/api/mesh/transport") => {
            let gateway = state.private_gateway.as_ref();
            let enabled = gateway.is_some();
            let connected_peers = match gateway {
                Some(gateway) => gateway.active_connection_count().await,
                None => 0,
            };
            Ok(routing::ok_response(serde_json::json!({
                  "dht": {
                      "enabled": state.dht.is_some(),
                      "running": state.dht.is_some(),
                  },
                  "overlay": {
                      "enabled": enabled,
                      "endpoint": gateway.map(|gateway| gateway.bind().to_string()),
                  },
                  "status": if enabled { "Healthy" } else { "Disabled" },
                  "health": if enabled { "Healthy" } else { "Disabled" },
                  "description": if enabled {
                      "TLS mesh service transport is listening"
                  } else {
                      "Mesh service transport is disabled"
                  },
                  "transportPreference": "Auto",
                  "overlayBind": gateway.map(|gateway| gateway.bind()),
                  "overlayPort": gateway.map(|gateway| gateway.bind().port()),
                  "certificateSha256": gateway.map(|gateway| hex::encode(gateway.certificate_sha256())),
                  "dhtEnabled": state.dht.is_some(),
                  "connectedPeers": connected_peers,
                  "totalPeers": connected_peers,
                  "activeCircuits": 0,
                  "activeStreams": 0,
                  "bootstrapPeers": [],
                  "isolatedPeers": 0,
                  "quorumPeers": 0,
                  "relayedPeers": 0,
                  "natType": "Unknown",
                  "publicEndpoint": null,
                  "lastDhtError": null,
                  "lastDhtPublishUtc": null
              }).to_string()))
        }

        _ => Err(ROUTE_NOT_HANDLED.to_owned()),
    }
}
