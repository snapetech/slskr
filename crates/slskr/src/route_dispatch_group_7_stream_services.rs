async fn route_dispatch_group_7_stream_services(
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
        ("GET", "/solid/clientid.jsonld") => Ok(solid_client_id_document_response(state).await),

        ("GET", "/api/slskdn") => {
            let session = state.session.read().await;
            let shares = state.shares.read().await;
            let searches = state.searches.read().await;
            let transfers = state.transfers.read().await;
            let users = state.users.read().await;
            let rooms = state.rooms.read().await;
            let library = state.library.read().await;
            let body = serde_json::json!({
                "status": "local",
                "enabled": true,
                "connected": session.state == "connected",
                "shares": shares.entries.len(),
                "searches": searches.records.len(),
                "transfers": transfers.entries.len(),
                "users": users.records.len(),
                "rooms": rooms.records.len(),
                "libraryItems": library.records.len(),
            })
            .to_string();
            drop(library);
            drop(rooms);
            drop(users);
            drop(transfers);
            drop(searches);
            drop(shares);
            drop(session);
            Ok(routing::ok_response(body))
        }

        ("GET", "/api/slskdn/library/health") => {
            let limit = match query_parameter(route.query, "limit") {
                None => 100,
                Some(value) => match value.parse::<i64>() {
                    Ok(value) if (1..=250).contains(&value) => value as usize,
                    _ => {
                        return Ok(routing::bad_request_response(
                            "limit must be between 1 and 250",
                        ))
                    }
                },
            };
            let path_filter = query_parameter(route.query, "path")
                .filter(|value| !value.trim().is_empty())
                .map(|value| value.trim().to_owned());
            let library = state.library.read().await;
            let all_issues = library.health_issues();
            let total_issues = all_issues.len();
            let issues = all_issues
                .into_iter()
                .take(limit)
                .map(|issue| {
                    serde_json::json!({
                        "type": "MissingMetadata",
                        "file": "",
                        "mb_recording_id": "",
                        "reason": issue
                            .get("message")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("Library metadata is incomplete"),
                        "severity": "Medium",
                    })
                })
                .collect::<Vec<_>>();
            let body = serde_json::json!({
                "path": path_filter.unwrap_or_else(|| "(all)".to_owned()),
                "summary": {
                    "total_issues": total_issues,
                    "issues_open": total_issues,
                    "issues_resolved": 0,
                },
                "issues": issues,
            })
            .to_string();
            drop(library);
            Ok(routing::ok_response(body))
        }

        ("POST", "/api/slskdn/warm-cache") => {
            let shares = state.shares.read().await;
            let searches = state.searches.read().await;
            let library = state.library.read().await;
            let warmed = shares.entries.len() + searches.records.len() + library.records.len();
            drop(library);
            drop(searches);
            drop(shares);
            let body = match mutate_runtime_compat_state(state, |runtime, _| {
                runtime.record_cache_warm(warmed).to_string()
            })
            .await
            {
                Ok(body) => body,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            Ok(routing::accepted_response(body))
        }

        ("POST", path)
            if path.starts_with("/api/streams/")
                && path.ends_with("/share-ticket")
                && share_stream_content_id(path).is_some() =>
        {
            let content_id =
                share_stream_content_id(path).expect("guarded share stream ticket path");
            let Some(token) = request_share_token(authorization, &headers) else {
                return Ok(routing::unauthorized_response());
            };
            let mut tokens = state.share_access_tokens.write().await;
            let token_record = tokens.validate(&token);
            drop(tokens);
            let Some(token_record) = token_record else {
                return Ok(routing::unauthorized_response());
            };
            let grants = state.share_grants.read().await;
            let Some(grant) = grants.get(&token_record.grant_id) else {
                drop(grants);
                return Ok(routing::unauthorized_response());
            };
            drop(grants);
            if !share_grant_allows_stream(&grant.permissions) {
                return Ok(routing::forbidden_response(
                    "Streaming not allowed for this share",
                ));
            }
            let collections = state.collections.read().await;
            let Some(collection) = collections.get(&grant.collection_id) else {
                drop(collections);
                return Ok(routing::not_found_response());
            };
            let Some(item) = collection
                .items
                .iter()
                .find(|item| item.content_id == content_id)
                .cloned()
            else {
                drop(collections);
                return Ok(routing::not_found_response());
            };
            drop(collections);
            let filename = if item.title.trim().is_empty() {
                item.content_id.clone()
            } else {
                item.title.clone()
            };
            let mut tickets = state.stream_tickets.write().await;
            let Some((ticket, _)) = tickets.issue(
                "share",
                &format!("share:{}", grant.id),
                item.content_id,
                filename.clone(),
                Some(grant.username),
                0,
                preview_stream_content_type(&filename).to_owned(),
                SHARE_STREAM_TICKET_TTL_SECONDS,
            ) else {
                return Ok(routing::service_unavailable_response(
                    "share stream ticket capacity is full",
                ));
            };
            drop(tickets);
            Ok(routing::ok_response(
                serde_json::json!({
                    "ticket": ticket,
                    "expiresInSeconds": SHARE_STREAM_TICKET_TTL_SECONDS,
                })
                .to_string(),
            ))
        }

        ("GET" | "HEAD", path) if path.starts_with("/api/streams/") && path.len() > 13 => {
            let stream_id = decoded_path_segment(&path[13..]);
            if query_parameter(route.query, "token").is_some() {
                return Ok(routing::bad_request_response(
                    "share tokens must be exchanged for stream tickets",
                ));
            }
            let api_authorized = !state.config.auth_required
                || is_authorized(&state.config, authorization, headers.cookie.as_deref());
            let ticket = query_parameter(route.query, "ticket");
            let ticket_record = if let Some(ticket) = ticket.as_deref() {
                let mut tickets = state.stream_tickets.write().await;
                let record = tickets.get(ticket);
                drop(tickets);
                record.filter(|record| record.family == "share" && record.content_id == stream_id)
            } else {
                None
            };
            if ticket.is_some() && ticket_record.is_none() {
                return Ok(routing::unauthorized_response());
            }
            let share = find_shared_entry_for_content(state, None, Some(&stream_id)).await;
            let transfers = state.transfers.read().await;
            let transfer = stream_id
                .strip_prefix("transfer-")
                .and_then(|id| id.parse::<u64>().ok())
                .and_then(|id| transfers.entries.iter().find(|entry| entry.id == id));
            if !api_authorized && ticket_record.is_none() {
                drop(transfers);
                return Ok(routing::unauthorized_response());
            }
            let body = serde_json::json!({
                "id": stream_id,
                "status": if transfer.is_some() || share.is_some() || ticket_record.is_some() { "available" } else { "not_found" },
                "ticket": ticket.as_ref().map(|_| "accepted"),
                "transfer": transfer.map(|entry| serde_json::json!({
                    "id": entry.id,
                    "filename": entry.filename,
                    "bytesTransferred": entry.bytes_transferred,
                    "size": entry.size,
                    "state": entry.status,
                })),
                "share": share.map(|entry| serde_json::json!({
                    "filename": entry.filename,
                    "size": entry.size,
                    "extension": entry.extension,
                })),
            }).to_string();
            drop(transfers);
            Ok(routing::ok_response(body))
        }

        ("POST", "/api/peer-streams/tickets") | ("POST", "/api/mesh-streams/tickets") => {
            let family = if normalized_path.starts_with("/api/mesh-streams") {
                "mesh"
            } else {
                "peer"
            };
            match create_preview_stream_ticket(state, family, body).await {
                Ok(ticket) => Ok(routing::ok_response(ticket)),
                Err(error) if error == "preview stream ticket capacity is full" => {
                    Ok(HttpResponse {
                        status: "429 Too Many Requests",
                        content_type: "text/plain; charset=utf-8",
                        body: if family == "mesh" {
                            "Mesh stream limit reached.".to_owned()
                        } else {
                            "Peer stream limit reached.".to_owned()
                        },
                    })
                }
                Err(error) => Ok(routing::bad_request_response(&error)),
            }
        }

        ("GET", path)
            if path.starts_with("/api/peer-streams/") || path.starts_with("/api/mesh-streams/") =>
        {
            let (family, raw_ticket) = if let Some(ticket) = path.strip_prefix("/api/mesh-streams/")
            {
                ("mesh", ticket)
            } else if let Some(ticket) = path.strip_prefix("/api/peer-streams/") {
                ("peer", ticket)
            } else {
                unreachable!()
            };
            let ticket = decoded_path_segment(raw_ticket);
            match open_preview_stream_ticket(state, family, &ticket).await {
                Some(body) => Ok(routing::ok_response(body)),
                None => Ok(routing::not_found_response()),
            }
        }

        ("POST", "/api/listening-party/radio/party/content") => {
            let room =
                extract_json_string_field(body, "room").unwrap_or_else(|| "radio".to_owned());
            let title = extract_json_string_field(body, "title")
                .unwrap_or_else(|| "party content".to_owned());
            let artist = extract_json_string_field(body, "artist").unwrap_or_default();
            let mut rooms = state.rooms.write().await;
            let Some(room_record) = rooms.join(room.clone()) else {
                return Ok(routing::service_unavailable_response(
                    "room capacity is full",
                ));
            };
            let room_record = rooms
                .add_message(
                    &room,
                    "local".to_owned(),
                    if artist.trim().is_empty() {
                        title.clone()
                    } else {
                        format!("{artist} - {title}")
                    },
                )
                .unwrap_or(room_record);
            let active_count = rooms.records.iter().filter(|room| room.joined).count();
            drop(rooms);
            let _now_playing_persistence = state.now_playing_persistence_lock.lock().await;
            let mut now_playing = state.now_playing.write().await;
            let playing = now_playing.upsert(room.clone(), artist, title);
            drop(now_playing);
            if let Err(error) = persist_now_playing_checked(state, &playing).await {
                update_session(state, |snapshot| snapshot.last_error = Some(error)).await;
            }
            Ok(routing::accepted_response(serde_json::json!({
                "status": "queued",
                "room": room,
                "activePartyCount": active_count,
                "party": serde_json::from_str::<serde_json::Value>(&room_record.json()).unwrap_or_else(|_| serde_json::json!({})),
                "nowPlaying": serde_json::from_str::<serde_json::Value>(&playing.json()).unwrap_or_else(|_| serde_json::json!({})),
            }).to_string()))
        }

        ("GET", "/api/mesh/health")
            if route.path.starts_with("/api/v0/")
                && state.config.controller_profile == ControllerProfile::Native =>
        {
            let routing_nodes = if let Some(dht) = state.dht.as_ref() {
                serde_json::from_str::<serde_json::Value>(&dht.status_json().await)
                    .ok()
                    .and_then(|value| value["dhtNodeCount"].as_u64())
                    .unwrap_or(0)
            } else {
                0
            };
            let discovery = state.content_discovery.read().await;
            let stored_keys = discovery.hash_entries().len();
            let content_peer_hints = discovery
                .shadow_records()
                .iter()
                .map(|record| record.peer_ids.len())
                .sum::<usize>();
            drop(discovery);
            Ok(routing::ok_response(
                serde_json::json!({
                    "routingNodes": routing_nodes,
                    "storedKeys": stored_keys,
                    "contentPeerHints": content_peer_hints,
                    "generatedAt": chrono::Utc::now().to_rfc3339(),
                })
                .to_string(),
            ))
        }

        ("GET", "/api/mesh/health") => {
            let users = state.users.read().await;
            let mesh = state.mesh.read().await;
            let candidate_count = mesh.candidate_usernames(&users).len();
            let capability_count = mesh.capability_records.len();
            drop(mesh);
            drop(users);
            Ok(routing::ok_response(serde_json::json!({
                "status": if candidate_count > 0 || capability_count > 0 { "ready" } else { "empty" },
                "healthy": true,
                "candidates": candidate_count,
                "capabilities": capability_count,
                "interestTag": MESH_RENDEZVOUS_INTEREST_TAG,
            }).to_string()))
        }

        ("POST", "/api/multisource/swarm")
        | ("POST", "/api/multisource/swarm/async")
        | ("POST", "/api/multisource/download")
            if normalized_path != "/api/multisource/download"
                || serde_json::from_str::<serde_json::Value>(body)
                    .ok()
                    .is_some_and(|value| {
                        value
                            .get("sources")
                            .and_then(serde_json::Value::as_array)
                            .is_some_and(|sources| !sources.is_empty())
                    }) =>
        {
            if route.path.starts_with("/api/v0/")
                && state.config.controller_profile == ControllerProfile::Native
            {
                if normalized_path == "/api/multisource/download" {
                    return Ok(multisource_versioned_download_response(body, state).await);
                }
                return Ok(
                    multisource_versioned_swarm_response(normalized_path, body, state).await,
                );
            }
            let mut request = match serde_json::from_str::<multisource::SwarmRequest>(body) {
                Ok(request) => request,
                Err(_) => return Ok(routing::bad_request_response("invalid swarm request")),
            };
            if request.sources.is_empty() {
                let expected_hash = request.expected_hash.clone().unwrap_or_default();
                request.sources =
                    discover_mesh_range_sources(state, &expected_hash, request.file_size).await;
            }
            if let Err(error) = multisource::validate_request(&mut request) {
                return Ok(routing::bad_request_response(&error));
            }
            let id = uuid::Uuid::new_v4().to_string();
            let relative_path = request.output_path.clone().unwrap_or_else(|| {
                format!("multisource/{id}-{}", virtual_basename(&request.filename))
            });
            let downloads_dir = effective_downloads_dir(state);
            let output_path =
                match safe_download_path(&downloads_dir, &relative_path).and_then(|path| {
                    ensure_scoped_download_path(&downloads_dir, path.to_string_lossy().as_ref())
                }) {
                    Ok(path) => path,
                    Err(error) => return Ok(routing::bad_request_response(&error)),
                };
            let public_output_path = output_path
                .strip_prefix(&downloads_dir)
                .map(|path| path.to_string_lossy().replace('\\', "/"))
                .map_err(|_| "multisource output path escaped the download root".to_owned())?;
            let job = multisource::new_job(
                id.clone(),
                &request,
                public_output_path.clone(),
                unix_timestamp(),
            );
            state.multisource.write().await.insert(job.clone());
            let store = Arc::clone(&state.multisource);
            if normalized_path == "/api/multisource/swarm/async" {
                tokio::spawn(multisource::execute(
                    id.clone(),
                    request,
                    output_path,
                    public_output_path,
                    store,
                ));
                return Ok(routing::accepted_response(
                    serde_json::json!({
                        "id": id,
                        "status": "queued",
                        "job": job,
                    })
                    .to_string(),
                ));
            }
            let result =
                multisource::execute(id, request, output_path, public_output_path, store).await;
            Ok(routing::ok_response(
                serde_json::to_string(&result)
                    .map_err(|error| format!("multisource result serialization failed: {error}"))?,
            ))
        }

        ("POST", "/api/multisource/download") => {
            if route.path.starts_with("/api/v0/")
                && state.config.controller_profile == ControllerProfile::Native
            {
                return Ok(multisource_versioned_download_response(body, state).await);
            }
            if route.path.starts_with("/api/v0/")
                && serde_json::from_str::<serde_json::Value>(body)
                    .ok()
                    .and_then(|value| {
                        value
                            .get("sources")
                            .and_then(serde_json::Value::as_array)
                            .map(Vec::len)
                    })
                    .is_some_and(|count| count < 2)
            {
                return Ok(HttpResponse {
                    status: "400 Bad Request",
                    content_type: "application/json",
                    body: serde_json::json!("At least 2 verified sources are required").to_string(),
                });
            }
            let filename = extract_json_string_field(body, "filename")
                .or_else(|| extract_json_string_field(body, "path"))
                .unwrap_or_else(|| "multisource-download".to_owned());
            let size = extract_json_u64_field(body, "size");
            let peer = extract_json_string_field(body, "username")
                .or_else(|| extract_json_string_field(body, "peer"));
            let mut transfers = state.transfers.write().await;
            let entry = transfers.create(0, peer, filename, None, size);
            let body = serde_json::json!({
                "id": format!("transfer-{}", entry.id),
                "transfer_id": entry.id,
                "status": "queued",
                "job": serde_json::from_str::<serde_json::Value>(&entry.json()).unwrap_or_else(|_| serde_json::json!({})),
            }).to_string();
            drop(transfers);
            Ok(routing::accepted_response(body))
        }

        _ => Err(ROUTE_NOT_HANDLED.to_owned()),
    }
}
