async fn route_dispatch_group_7_network_admin(
    context: &RouteDispatchContext<'_, '_>,
) -> RouteDispatchResult {
    let RouteDispatchContext {
        method,
        normalized_path,
        authorization,
        body,
        state,
        route,
        headers,
        state_arc,
        extended_mutation,
        request_is_versioned_v0,
    } = context.clone();
    match (method, normalized_path) {
        ("GET", "/api/playback/status") => {
            let now_playing = state.now_playing.read().await;
            let body = serde_json::json!({
                "status": if now_playing.records.is_empty() { "stopped" } else { "playing" },
                "nowPlaying": now_playing.records.iter().map(|record| {
                    serde_json::from_str::<serde_json::Value>(&record.json()).unwrap_or_else(|_| serde_json::json!({}))
                }).collect::<Vec<_>>(),
                "count": now_playing.records.len(),
                "updated_at": now_playing.updated_at,
            }).to_string();
            drop(now_playing);
            Ok(routing::ok_response(body))
        }

        ("GET", "/api/traces") => {
            let events = state.events.read().await;
            let traces = events
                .records
                .iter()
                .rev()
                .take(100)
                .map(|event| {
                    serde_json::json!({
                        "id": event.id,
                        "kind": event.kind,
                        "resource": event.resource,
                        "detail": event.detail,
                        "created_at": event.created_at,
                    })
                })
                .collect::<Vec<_>>();
            let count = traces.len();
            drop(events);
            Ok(routing::ok_response(
                serde_json::json!({
                    "traces": traces,
                    "count": count,
                })
                .to_string(),
            ))
        }

        // FairnessController.GetSummary is a versioned native profile DTO, while the
        // unversioned /api/fairness route remains slskR's legacy ranking
        // projection.  With no recorded traffic, the frozen guard returns a
        // neutral upload/download ratio, a zero overlay/Soulseek ratio, and
        // an explicit within-constraints reason.
        ("GET", "/api/fairness")
            if state.config.controller_profile == ControllerProfile::Native
                && route.path == "/api/v0/fairness/summary" =>
        {
            let totals = if let Some(db) = state.db.as_ref() {
                match db.get_traffic_totals().await {
                    Ok(totals) => totals,
                    Err(error) => {
                        return Ok(routing::internal_server_error_response(&format!(
                            "fairness storage unavailable: {error}"
                        )))
                    }
                }
            } else {
                persistence::TrafficTotalsRecord::default()
            };
            let overlay_upload = totals.overlay_upload_bytes.max(0) as f64;
            let overlay_download = totals.overlay_download_bytes.max(0) as f64;
            let soulseek_upload = totals.soulseek_upload_bytes.max(0) as f64;
            let upload_download_ratio = if overlay_download > 0.0 {
                overlay_upload / overlay_download
            } else {
                1.0
            };
            let overlay_to_soulseek_ratio = if soulseek_upload > 0.0 {
                overlay_upload / soulseek_upload
            } else if overlay_upload > 0.0 {
                f64::INFINITY
            } else {
                0.0
            };
            let mut throttle = upload_download_ratio < 0.5;
            let mut reasons = Vec::new();
            if throttle {
                reasons.push(format!(
                    "overlay upload/download ratio {upload_download_ratio:.2} below minimum 0.50"
                ));
            }
            if overlay_to_soulseek_ratio > 3.0 {
                throttle = true;
                reasons.push(format!(
                    "overlay/Soulseek upload ratio {overlay_to_soulseek_ratio:.2} above maximum 3.00"
                ));
            }
            let reason = if reasons.is_empty() {
                "within fairness constraints".to_owned()
            } else {
                reasons.join("; ")
            };
            Ok(routing::ok_response(
                serde_json::json!({
                    "throttleOverlayDownloads": throttle,
                    "reason": reason,
                    "overlayUploadDownloadRatio": upload_download_ratio,
                    "overlayToSoulseekUploadRatio": if overlay_to_soulseek_ratio.is_finite() {
                        serde_json::json!(overlay_to_soulseek_ratio)
                    } else {
                        serde_json::json!("Infinity")
                    },
                    "totals": {
                        "overlayUploadBytes": totals.overlay_upload_bytes.max(0),
                        "overlayDownloadBytes": totals.overlay_download_bytes.max(0),
                        "soulseekUploadBytes": totals.soulseek_upload_bytes.max(0),
                        "soulseekDownloadBytes": totals.soulseek_download_bytes.max(0),
                    },
                })
                .to_string(),
            ))
        }
        ("GET", "/api/fairness") | ("GET", "/api/ranking") => {
            let mut response = native_compat_response(method, normalized_path, state).await;
            if normalized_path == "/api/fairness" {
                let transfers = state.transfers.read().await;
                let downloaded = transfers
                    .entries
                    .iter()
                    .filter(|entry| entry.direction == 0)
                    .map(|entry| entry.bytes_transferred)
                    .sum::<u64>();
                let uploaded = transfers
                    .entries
                    .iter()
                    .filter(|entry| entry.direction != 0)
                    .map(|entry| entry.bytes_transferred)
                    .sum::<u64>();
                let mut value = serde_json::from_str::<serde_json::Value>(&response.body)
                    .unwrap_or_else(|_| serde_json::json!({}));
                value["throttleOverlayDownloads"] = serde_json::json!(false);
                value["reason"] = serde_json::Value::Null;
                value["overlayUploadDownloadRatio"] = serde_json::json!(0.0);
                value["overlayToSoulseekUploadRatio"] = serde_json::json!(0.0);
                value["totals"] = serde_json::json!({
                    "downloadedBytes": downloaded,
                    "uploadedBytes": uploaded,
                });
                response.body = value.to_string();
            }
            Ok(response)
        }

        ("GET", "/api/port-forwarding/status") => Ok(routing::ok_response(
            serde_json::to_string(&state.port_forwarding.statuses().await)
                .unwrap_or_else(|_| "[]".to_owned()),
        )),

        ("GET", path) if path.starts_with("/api/port-forwarding/status/") => {
            let Some(local_port) = path_segment_after(path, "/api/port-forwarding/status/") else {
                return Ok(routing::not_found_response());
            };
            if local_port.parse::<u16>().is_err() || local_port == "0" {
                return Ok(routing::not_found_response());
            }
            let local_port = local_port.parse::<u16>().unwrap_or_default();
            match state.port_forwarding.status(local_port).await {
                Some(status) => Ok(routing::ok_response(
                    serde_json::to_string(&status).unwrap_or_else(|_| "{}".to_owned()),
                )),
                None => Ok(routing::not_found_response()),
            }
        }

        ("GET", "/api/port-forwarding/available-ports") => {
            let start_port = match query_bounded_usize(route.query, "startPort", 1, 65_535) {
                Ok(value) => value.unwrap_or(1_024),
                Err(()) => {
                    return Ok(routing::bad_request_response("Invalid port range"));
                }
            };
            let end_port = match query_bounded_usize(route.query, "endPort", 1, 65_535) {
                Ok(value) => value.unwrap_or(65_535),
                Err(()) => {
                    return Ok(routing::bad_request_response("Invalid port range"));
                }
            };
            if start_port > end_port {
                return Ok(routing::bad_request_response("Invalid port range"));
            }
            let limit = match query_bounded_usize(route.query, "limit", 1, 1_000) {
                Ok(value) => value,
                Err(()) => {
                    return Ok(routing::bad_request_response(
                        "Limit must be between 1 and 1000",
                    ));
                }
            };
            let used_ports = state
                .port_forwarding
                .used_ports()
                .await
                .into_iter()
                .map(usize::from)
                .filter(|port| (start_port..=end_port).contains(port))
                .collect::<HashSet<_>>();
            let available_port_count = end_port - start_port + 1 - used_ports.len();
            let returned_port_count = limit.unwrap_or(1_000).min(available_port_count);
            let available_ports = (start_port..=end_port)
                .filter(|port| !used_ports.contains(port))
                .take(returned_port_count)
                .collect::<Vec<_>>();
            Ok(routing::ok_response(
                serde_json::json!({
                    "availablePortCount": available_port_count,
                    "availablePorts": available_ports,
                    "usedPortCount": used_ports.len(),
                })
                .to_string(),
            ))
        }

        ("GET", "/api/port-forwarding/stream-stats") => {
            let rules = state.port_forwarding.statuses().await;
            Ok(routing::ok_response(
                serde_json::json!({
                    "totalForwardingRules": rules.len(),
                    "activeRules": rules.iter().filter(|rule| rule.is_active).count(),
                    "totalConnections": rules.iter().map(|rule| rule.active_connections).sum::<usize>(),
                    "totalBytesForwarded": rules.iter().map(|rule| rule.bytes_forwarded).sum::<u64>(),
                    "rules": rules,
                })
                .to_string(),
            ))
        }

        ("POST", "/api/port-forwarding/start") => {
            let request = match serde_json::from_str::<serde_json::Value>(body) {
                Ok(value) if value.is_object() => value,
                _ => return Ok(routing::bad_request_response("Request is required")),
            };
            let local_port = request.get("localPort").and_then(serde_json::Value::as_u64);
            let destination_port = request
                .get("destinationPort")
                .and_then(serde_json::Value::as_u64);
            let pod_id = request
                .get("podId")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .unwrap_or_default();
            let destination_host = request
                .get("destinationHost")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .unwrap_or_default();
            let service_name = request
                .get("serviceName")
                .and_then(serde_json::Value::as_str)
                .map(str::trim);
            if !matches!(local_port, Some(1_024..=65_535)) {
                return Ok(routing::bad_request_response(
                    "Local port must be between 1024 and 65535",
                ));
            }
            if pod_id.is_empty() || pod_id.len() > 100 {
                return Ok(routing::bad_request_response(
                    "PodId must be between 1 and 100 characters",
                ));
            }
            if destination_host.is_empty() || destination_host.len() > 253 {
                return Ok(routing::bad_request_response(
                    "Destination host must be between 1 and 253 characters",
                ));
            }
            if !matches!(destination_port, Some(1..=65_535)) {
                return Ok(routing::bad_request_response(
                    "Destination port must be between 1 and 65535",
                ));
            }
            if service_name.is_some_and(|value| value.len() > 100) {
                return Ok(routing::bad_request_response(
                    "Service name must be at most 100 characters",
                ));
            }
            let local_port = local_port.unwrap_or_default() as u16;
            let destination_port = destination_port.unwrap_or_default() as u16;
            let Some(local_username) = pod_request_peer_id(state).await else {
                return Ok(routing::forbidden_response(
                    "Authenticated peer identity is required",
                ));
            };
            let (pod, pod_gateway_certificate_sha256) = {
                let pods = state.pods.read().await;
                if !pods.is_member(pod_id, &local_username) {
                    return Ok(routing::forbidden_response(
                        "Only pod members can start port forwarding",
                    ));
                }
                if !pods.destination_allowed(pod_id, destination_host, destination_port) {
                    return Ok(routing::forbidden_response(
                        "Destination is not allowed by the Pod private-gateway policy",
                    ));
                }
                (pods.get(pod_id), pods.gateway_certificate_sha256(pod_id))
            };
            let Some(pod) = pod else {
                return Ok(routing::not_found_response());
            };
            let gateway_peer_id = pod
                .private_service_policy
                .as_ref()
                .and_then(serde_json::Value::as_object)
                .and_then(|policy| policy.get("gatewayPeerId"))
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty());
            let Some(gateway_peer_id) = gateway_peer_id else {
                return Ok(routing::service_unavailable_response(
                    "Pod private-gateway policy has no designated gateway",
                ));
            };
            let trusted_gateway = state
                .config
                .trusted_mesh_peers
                .iter()
                .find(|peer| peer.matches(gateway_peer_id));
            if let (Some(pod_pin), Some(trusted)) =
                (pod_gateway_certificate_sha256, trusted_gateway)
            {
                if pod_pin != trusted.certificate_sha256 {
                    return Ok(routing::service_unavailable_response(
                        "Pod and operator gateway certificate pins conflict",
                    ));
                }
            }
            let gateway_certificate_sha256 = pod_gateway_certificate_sha256
                .or_else(|| trusted_gateway.map(|peer| peer.certificate_sha256));
            let Some(gateway_certificate_sha256) = gateway_certificate_sha256 else {
                return Ok(routing::service_unavailable_response(
                    "Gateway has no authenticated TLS certificate pin",
                ));
            };
            let descriptor = state
                .mesh
                .read()
                .await
                .capability_records
                .iter()
                .find(|descriptor| {
                    descriptor.peer_id.eq_ignore_ascii_case(gateway_peer_id)
                        || descriptor.username.eq_ignore_ascii_case(gateway_peer_id)
                })
                .cloned();
            if descriptor.is_none() && trusted_gateway.is_none() {
                return Ok(routing::service_unavailable_response(
                    "Gateway peer capability record and operator trust entry are unavailable",
                ));
            }
            let gateway_username = descriptor
                .as_ref()
                .map(|descriptor| descriptor.username.clone())
                .or_else(|| trusted_gateway.map(|peer| peer.username.clone()))
                .expect("a descriptor or trusted gateway exists");
            let mut gateway_endpoints = Vec::new();
            if let Some(trusted) = trusted_gateway {
                gateway_endpoints.push(trusted.overlay_endpoint);
            }
            if let Some(descriptor) = descriptor.as_ref() {
                if let Some(overlay_port) = descriptor.overlay_port {
                    if let Some(peer_address) =
                        cached_peer_endpoint(state, &descriptor.username).await
                    {
                        let endpoint = SocketAddr::V4(SocketAddrV4::new(
                            peer_connect_ip(state, &peer_address),
                            overlay_port,
                        ));
                        if !gateway_endpoints.contains(&endpoint) {
                            gateway_endpoints.push(endpoint);
                        }
                    }
                }
            }
            if let Some(dht) = state.dht.as_ref() {
                for endpoint in dht.peers().await {
                    if gateway_endpoints.len() >= port_forwarding::MAX_GATEWAY_ENDPOINTS {
                        break;
                    }
                    if !gateway_endpoints.contains(&endpoint) {
                        gateway_endpoints.push(endpoint);
                    }
                }
            }
            if gateway_endpoints.is_empty() {
                if let Some(descriptor) = descriptor.as_ref() {
                    if let Some(overlay_port) = descriptor.overlay_port {
                        let peer_address =
                            match request_peer_endpoint(state, &descriptor.username).await {
                                Ok(address) => address,
                                Err(error) => {
                                    return Ok(routing::service_unavailable_response(&error));
                                }
                            };
                        gateway_endpoints.push(SocketAddr::V4(SocketAddrV4::new(
                            peer_connect_ip(state, &peer_address),
                            overlay_port,
                        )));
                    }
                }
            }
            if gateway_endpoints.is_empty() {
                return Ok(routing::service_unavailable_response(
                    "Gateway has no reachable overlay endpoint",
                ));
            }
            match state
                .port_forwarding
                .start(port_forwarding::StartRequest {
                    local_port,
                    pod_id: pod_id.to_owned(),
                    destination_host: destination_host.to_owned(),
                    destination_port,
                    service_name: service_name.map(str::to_owned),
                    gateway_username,
                    gateway_endpoints,
                    gateway_certificate_sha256,
                    local_username,
                    authentication_key: Arc::new(state.capability_signing_key.clone()),
                })
                .await
            {
                Ok(_) => Ok(routing::ok_response(
                    r#"{"message":"Port forwarding started"}"#.to_owned(),
                )),
                Err(error) if error.contains("already being forwarded") => {
                    Ok(routing::conflict_response(&error))
                }
                Err(error) => {
                    eprintln!("port forwarding start failed: {error}");
                    Ok(
                        if state.config.controller_profile == ControllerProfile::Native
                            && route.path.starts_with("/api/v0/")
                        {
                            routing::internal_server_error_response(
                                "Failed to start port forwarding",
                            )
                        } else {
                            routing::service_unavailable_response("port forwarding is unavailable")
                        },
                    )
                }
            }
        }

        ("POST", path) if path.starts_with("/api/port-forwarding/stop/") => {
            let Some(local_port) = path_segment_after(path, "/api/port-forwarding/stop/") else {
                return Ok(routing::not_found_response());
            };
            if local_port.parse::<u16>().is_err() || local_port == "0" {
                return Ok(routing::not_found_response());
            }
            state
                .port_forwarding
                .stop(local_port.parse().unwrap_or_default())
                .await;
            Ok(routing::ok_response(
                r#"{"message":"Port forwarding stopped"}"#.to_owned(),
            ))
        }

        ("GET", "/api/portforwarding/status") => Ok(routing::ok_response(
            serde_json::to_string(&state.port_forwarding.statuses().await)
                .unwrap_or_else(|_| "[]".to_owned()),
        )),

        ("GET", "/api/signals") => {
            let session = state.session.read().await;
            let events = state.events.read().await;
            let body = serde_json::json!({
                "signals": events.records.iter().rev().take(25).map(|event| {
                    serde_json::json!({
                        "id": event.id,
                        "kind": event.kind,
                        "resource": event.resource,
                        "created_at": event.created_at,
                    })
                }).collect::<Vec<_>>(),
                "connected": session.state == "connected",
                "count": events.records.len().min(25),
            })
            .to_string();
            drop(events);
            drop(session);
            Ok(routing::ok_response(body))
        }

        ("POST", "/api/backfill") => {
            let searches = state.searches.read().await;
            let shares = state.shares.read().await;
            let queued = searches.records.len() + shares.entries.len();
            drop(shares);
            drop(searches);
            let body = match mutate_runtime_compat_state(state, |runtime, _| {
                runtime.record_backfill(queued).to_string()
            })
            .await
            {
                Ok(body) => body,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            Ok(routing::accepted_response(body))
        }
        ("GET", path) if path.starts_with("/api/mediacore/") => {
            Ok(mediacore_extended_response(path, route.query, state).await)
        }
        ("GET", path) if extended_controller_get_route(path) => {
            Ok(extended_controller_get_response(
                path,
                route.query,
                state,
                route.path.starts_with("/api/v0/"),
            )
            .await)
        }
        ("GET", path) if extended_controller_dynamic_get_route(path) => {
            Ok(extended_controller_dynamic_get_response(
                path,
                route.query,
                state,
                route.path.starts_with("/api/v0/"),
            )
            .await)
        }
        (method, path) if extended_mutation => Ok(extended_controller_mutation_response(
            method,
            path,
            route.query,
            body,
            state,
            route.path.starts_with("/api/v0/"),
            &headers,
        )
        .await),
        (method, path) if native_compat_route(method, path) => {
            Ok(native_compat_response(method, path, state).await)
        }
        ("GET", path) if is_spa_navigation_path(path) => Ok(index_html_response()),
        ("HEAD", path) if is_spa_navigation_path(path) => Ok(head_response(index_html_response())),
        _ => Err(ROUTE_NOT_HANDLED.to_owned()),
    }
}
