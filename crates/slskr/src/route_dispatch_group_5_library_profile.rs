async fn route_dispatch_group_5_library_profile(
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
        ("GET", path)
            if path.starts_with("/api/share-grants/")
                && share_grant_resource_id(path).is_some() =>
        {
            let id = share_grant_resource_id(path).expect("guarded share-grant resource path");
            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            let grants = state.share_grants.read().await;
            let record = grants.get(id);
            drop(grants);
            let Some(record) = record else {
                return Ok(routing::not_found_response());
            };
            if share_grant_collection_forbids(state, &record.collection_id, caller_id.as_deref())
                .await
            {
                return Ok(routing::not_found_response());
            }
            Ok(routing::ok_response(record.json()))
        }
        ("GET", path)
            if path.starts_with("/api/share-grants/by-collection/")
                && share_grant_collection_id(path).is_some() =>
        {
            let collection_id =
                share_grant_collection_id(path).expect("guarded share-grant collection path");
            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            // SharesController.GetByCollection first resolves the collection
            // and returns NotFound when the collection no longer exists.
            let collection_exists = state.collections.read().await.get(collection_id).is_some();
            if !collection_exists {
                return Ok(routing::not_found_response());
            }
            // Matches the oracle's real GetByCollection: this is the
            // "outgoing shares" owner-perspective view, so it 404s unless
            // the caller actually owns this collection.
            if share_grant_collection_forbids(state, collection_id, caller_id.as_deref()).await {
                return Ok(routing::not_found_response());
            }
            let grants = state.share_grants.read().await;
            let records = grants.get_by_collection(collection_id);
            let json = records
                .iter()
                .map(|r| r.json())
                .collect::<Vec<_>>()
                .join(",");
            let response = format!("[{}]", json);
            drop(grants);
            Ok(routing::ok_response(response))
        }
        ("PUT", path)
            if path.starts_with("/api/share-grants/")
                && share_grant_resource_id(path).is_some() =>
        {
            let id = share_grant_resource_id(path).expect("guarded share-grant resource path");
            let permissions = extract_json_string_field(body, "permissions")
                .unwrap_or_else(|| "read".to_string());
            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            let owning_collection_id = state
                .share_grants
                .read()
                .await
                .get(id)
                .map(|record| record.collection_id.clone());
            if let Some(collection_id) = owning_collection_id.as_deref() {
                if share_grant_collection_forbids(state, collection_id, caller_id.as_deref()).await
                {
                    return Ok(routing::not_found_response());
                }
            }
            let _collection_grant_persistence =
                state.collection_grant_persistence_lock.lock().await;
            let mut grants = state.share_grants.write().await;
            if grants.get(id).map(|record| record.collection_id.clone()) != owning_collection_id {
                drop(grants);
                return Ok(routing::not_found_response());
            }
            let previous = grants.clone();
            let requested_limit = match crate::share_stream_limits::request_limit(body) {
                Ok(limit) => limit,
                Err(error) => return Ok(routing::bad_request_response(error)),
            };
            let permissions = if requested_limit.is_some()
                && extract_json_string_field(body, "permissions").is_none()
            {
                grants
                    .get(id)
                    .map(|record| record.permissions)
                    .unwrap_or(permissions)
            } else {
                permissions
            };
            if let Some(record) = grants.update(id, permissions) {
                let record = match requested_limit {
                    Some(limit) => grants
                        .set_stream_limit(id, limit)
                        .expect("updated grant remains owned"),
                    None => record,
                };
                let json = record.json();
                let mutated = grants.clone();
                drop(grants);
                if let Err(error) = persist_share_grant(state, &record).await {
                    let mut grants = state.share_grants.write().await;
                    if share_grant_store_matches(&grants, &mutated) {
                        *grants = previous;
                    }
                    drop(grants);
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(routing::ok_response(json))
            } else {
                drop(grants);
                Ok(routing::not_found_response())
            }
        }
        ("DELETE", path)
            if path.starts_with("/api/share-grants/")
                && share_grant_resource_id(path).is_some() =>
        {
            let id = share_grant_resource_id(path).expect("guarded share-grant resource path");
            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            let owning_collection_id = state
                .share_grants
                .read()
                .await
                .get(id)
                .map(|record| record.collection_id.clone());
            if let Some(collection_id) = owning_collection_id.as_deref() {
                if share_grant_collection_forbids(state, collection_id, caller_id.as_deref()).await
                {
                    return Ok(routing::not_found_response());
                }
            }
            let _collection_grant_persistence =
                state.collection_grant_persistence_lock.lock().await;
            let mut grants = state.share_grants.write().await;
            if grants.get(id).map(|record| record.collection_id.clone()) != owning_collection_id {
                drop(grants);
                return Ok(routing::not_found_response());
            }
            let previous = grants.clone();
            let deleted = grants.delete(id);
            if deleted {
                let mutated = grants.clone();
                drop(grants);
                if let Err(error) = persist_share_grant_delete_checked(state, id).await {
                    let mut grants = state.share_grants.write().await;
                    if share_grant_store_matches(&grants, &mutated) {
                        *grants = previous;
                    }
                    drop(grants);
                    return Ok(routing::service_unavailable_response(&error));
                }
                state.share_access_tokens.write().await.revoke_grant(id);
                state
                    .stream_tickets
                    .write()
                    .await
                    .revoke_source(&format!("share:{id}"));
                Ok(routing::ok_response("{}".to_string()))
            } else {
                drop(grants);
                Ok(routing::not_found_response())
            }
        }

        // LIBRARY ITEMS ENDPOINTS
        ("GET", "/api/library/items") => {
            if state.config.controller_profile == ControllerProfile::Native {
                return Ok(routing::ok_response(
                    native_library_items_search_json(state, route.query).await,
                ));
            }
            let library = state.library.read().await;
            let json = library.json();
            drop(library);
            Ok(routing::ok_response(json))
        }
        ("GET", "/api/library/items/browser") => {
            Ok(library_browser_response(state, route.query).await)
        }
        ("POST", "/api/library/items") => {
            let artist = extract_json_string_field(body, "artist").unwrap_or_default();
            let title = extract_json_string_field(body, "title").unwrap_or_default();
            let kind =
                extract_json_string_field(body, "kind").unwrap_or_else(|| "Audio".to_string());
            let _library_persistence = state.library_persistence_lock.lock().await;
            let mut library = state.library.write().await;
            let previous = library.clone();
            let Some(record) = library.create(artist, title, kind) else {
                return Ok(routing::service_unavailable_response(
                    "library item capacity is full",
                ));
            };
            let mutated = library.clone();
            let json = record.json();
            drop(library);
            if let Err(error) = persist_library_item_checked(state, &record).await {
                rollback_library_if_unchanged(state, previous, &mutated).await;
                return Ok(routing::service_unavailable_response(&error));
            }
            Ok(routing::created_response(json))
        }
        ("GET", path) if path.starts_with("/api/library/items/") => {
            let Some(id) = path_segment_after(path, "/api/library/items/") else {
                return Ok(routing::not_found_response());
            };
            let library = state.library.read().await;
            if let Some(record) = library.get(id) {
                let json = record.json();
                drop(library);
                Ok(routing::ok_response(json))
            } else {
                drop(library);
                Ok(routing::not_found_response())
            }
        }
        ("DELETE", path) if path.starts_with("/api/library/items/") => {
            let Some(id) = path_segment_after(path, "/api/library/items/") else {
                return Ok(routing::not_found_response());
            };
            let _library_persistence = state.library_persistence_lock.lock().await;
            let mut library = state.library.write().await;
            let previous = library.clone();
            let deleted = library.delete(id);
            let mutated = library.clone();
            drop(library);
            if deleted {
                if let Err(error) = persist_library_item_delete_checked(state, id).await {
                    rollback_library_if_unchanged(state, previous, &mutated).await;
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(routing::ok_response("{}".to_string()))
            } else {
                Ok(routing::not_found_response())
            }
        }

        // DESTINATIONS ENDPOINTS
        ("GET", "/api/destinations") => {
            let destinations = state.destinations.read().await;
            let json = if destinations.records.is_empty() {
                let configured = DestinationStore::from_config(
                    &state.config.downloads_dir,
                    &state.config.core_workflow.destinations,
                );
                if route.path.starts_with("/api/v0/") {
                    configured.versioned_list()
                } else {
                    configured.list()
                }
            } else if route.path.starts_with("/api/v0/") {
                destinations.versioned_list()
            } else {
                destinations.list()
            };
            drop(destinations);
            Ok(routing::ok_response(json))
        }
        ("GET", "/api/destinations/default") => {
            let destinations = state.destinations.read().await;
            let json = if destinations.records.is_empty() {
                let configured = DestinationStore::from_config(
                    &state.config.downloads_dir,
                    &state.config.core_workflow.destinations,
                );
                if route.path.starts_with("/api/v0/") {
                    configured.versioned_default()
                } else {
                    configured.default()
                }
            } else if route.path.starts_with("/api/v0/") {
                destinations.versioned_default()
            } else {
                destinations.default()
            };
            drop(destinations);
            Ok(routing::ok_response(json))
        }
        ("GET", "/api/config/download-filter") => {
            let exclusions = effective_download_exclusions(state).await;
            Ok(routing::ok_response(
                serde_json::json!({
                    "exclude": exclusions,
                    "maxTerms": 100,
                    "maxTermLength": 256,
                })
                .to_string(),
            ))
        }
        ("PUT", "/api/config/download-filter") => Ok(update_download_filter(state, body).await),

        // BROWSE ENDPOINTS
        ("GET", path)
            if path.starts_with("/api/users/")
                && path.ends_with("/browse/status")
                && user_route_username(path, "/browse/status").is_some() =>
        {
            let username = user_route_username(path, "/browse/status")
                .expect("guarded user browse status path");
            if state.runtime.read().await.relay_agent_enabled {
                return Ok(controller_forbidden_response());
            }
            let browse = state.browse.read().await;
            let tracked = browse
                .records
                .iter()
                .find(|record| record.username == username)
                .map(BrowseRecord::controller_status_json);
            drop(browse);
            match tracked {
                Some(body) => Ok(routing::ok_response(body)),
                None => Ok(routing::not_found_response()),
            }
        }
        // ADDITIONAL MISSING USER ENDPOINTS (Phase 5)
        ("GET", "/api/profile/me") => {
            if route.path.starts_with("/api/v0/") {
                let display_name = pod_request_peer_id(state)
                    .await
                    .unwrap_or_else(|| "Unknown".to_owned());
                let descriptor = match local_capability_descriptor(state).await {
                    Ok(descriptor) => descriptor,
                    Err(error) => return Ok(routing::service_unavailable_response(&error)),
                };
                let peer_id = local_profile_peer_id(state);
                return Ok(routing::ok_response(
                    serde_json::json!({
                        "peerId": peer_id,
                        "publicKey": STANDARD.encode(descriptor.public_key),
                        "displayName": display_name,
                        "avatar": null,
                        "capabilities": 0,
                        "endpoints": [],
                        "createdAt": unix_seconds_rfc3339(descriptor.issued_at_unix),
                        "expiresAt": (chrono::Utc::now() + chrono::Duration::days(7)).to_rfc3339(),
                        "signature": descriptor.signature.map(|signature| STANDARD.encode(signature)).unwrap_or_default(),
                    })
                    .to_string(),
                ));
            }
            let session = state.session.read().await;
            let username = session
                .username
                .clone()
                .or_else(|| state.config.username.clone())
                .unwrap_or_else(|| "local".to_owned());
            let privileges_seconds = session.privileges_seconds.unwrap_or(0);
            let connected = session.state == "connected";
            drop(session);
            let users = state.users.read().await;
            let watched = users
                .records
                .iter()
                .any(|user| user.username.eq_ignore_ascii_case(&username) && user.watched);
            drop(users);
            let descriptor = match local_capability_descriptor(state).await {
                Ok(descriptor) => descriptor,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            let json = serde_json::json!({
                "peerId": descriptor.peer_id,
                "publicKey": STANDARD.encode(descriptor.public_key),
                "displayName": username.clone(),
                "capabilities": descriptor.features,
                "endpoints": descriptor.endpoints,
                "createdAt": descriptor.issued_at_unix,
                "expiresAt": descriptor.expires_at_unix,
                "signature": descriptor.signature.map(|signature| STANDARD.encode(signature)).unwrap_or_default(),
                "username": username,
                "description": "",
                "picture": "",
                "user_type": if privileges_seconds > 0 { "privileged" } else { "normal" },
                "privilegesSeconds": privileges_seconds,
                "connected": connected,
                "watched": watched,
            }).to_string();
            Ok(routing::ok_response(json))
        }

        ("GET", path)
            if route.path.starts_with("/api/v0/") && path.starts_with("/api/profile/") =>
        {
            let Some(raw_peer_id) = path.strip_prefix("/api/profile/") else {
                return Ok(routing::not_found_response());
            };
            if raw_peer_id.is_empty() {
                return Ok(routing::bad_request_response("PeerId is required."));
            }
            if raw_peer_id.contains('/') {
                return Ok(routing::not_found_response());
            }
            let peer_id = decoded_path_segment(raw_peer_id).trim().to_owned();
            if peer_id.is_empty() {
                return Ok(routing::bad_request_response("PeerId is required."));
            }
            let local_peer_id = local_profile_peer_id(state);
            if !peer_id.eq_ignore_ascii_case(&local_peer_id) {
                return Ok(routing::not_found_response());
            }
            let display_name = pod_request_peer_id(state)
                .await
                .unwrap_or_else(|| "Unknown".to_owned());
            return Ok(routing::ok_response(
                serde_json::json!({
                    "peerId": local_peer_id,
                    "displayName": display_name,
                    "avatar": null,
                    "capabilities": 0,
                    "endpoints": [],
                })
                .to_string(),
            ));
        }

        ("GET", path) if path.starts_with("/api/profile/") => {
            let Some(username) = path_segment_after(path, "/api/profile/") else {
                return Ok(routing::not_found_response());
            };
            let username = decoded_path_segment(username);
            let users = state.users.read().await;
            let user = users
                .records
                .iter()
                .find(|user| user.username.eq_ignore_ascii_case(&username))
                .cloned();
            drop(users);
            let json = serde_json::json!({
                "username": username,
                "description": "",
                "picture": "",
                "user_type": "normal",
                "watched": user.as_ref().is_some_and(|user| user.watched),
                "status": user.as_ref().and_then(|user| user.status.clone()).unwrap_or_else(|| "Unknown".to_owned()),
                "averageSpeed": user.as_ref().and_then(|user| user.average_speed).unwrap_or(0),
                "uploadCount": user.as_ref().and_then(|user| user.upload_count).unwrap_or(0),
                "fileCount": user.as_ref().and_then(|user| user.file_count).unwrap_or(0),
                "directoryCount": user.as_ref().and_then(|user| user.directory_count).unwrap_or(0),
            }).to_string();
            Ok(routing::ok_response(json))
        }

        // CONVERSATIONS ENDPOINT
        _ => Err(ROUTE_NOT_HANDLED.to_owned()),
    }
}
