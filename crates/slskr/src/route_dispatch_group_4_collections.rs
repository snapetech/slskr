async fn route_dispatch_group_4_collections(
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
        ("GET", "/api/database/stats") => Ok(routing::ok_response(
            database_stats_value(state).await.to_string(),
        )),
        ("POST", "/api/database/cleanup") => Ok(routing::ok_response(
            database_cleanup_value(state, body).await.to_string(),
        )),
        ("POST", "/api/database/vacuum") => Ok(routing::ok_response(
            database_vacuum_value(state).await.to_string(),
        )),

        // COLLECTIONS ENDPOINTS
        ("GET", "/api/collections") => {
            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            let collections = state.collections.read().await;
            let json = if route.path.starts_with("/api/v0/") {
                format!(
                    "[{}]",
                    collections
                        .records
                        .iter()
                        .filter(|record| {
                            !collection_owner_forbids(caller_id.as_deref(), &record.owner_user_id)
                        })
                        .map(CollectionRecord::native_json)
                        .collect::<Vec<_>>()
                        .join(",")
                )
            } else {
                collections.json_array(route.query, caller_id.as_deref())
            };
            drop(collections);
            Ok(routing::ok_response(json))
        }
        ("GET", "/api/shared") => {
            let shares = state.shares.read().await;
            let entries = shares
                .roots
                .iter()
                .map(|root| {
                    let mut value = controller_share_value(root);
                    value["name"] = serde_json::json!(root.label);
                    value
                })
                .collect::<Vec<_>>();
            drop(shares);
            Ok(routing::ok_response(
                serde_json::Value::Array(entries).to_string(),
            ))
        }
        ("POST", "/api/collections") => {
            let Some(name) = extract_json_string_field(body, "title")
                .or_else(|| extract_json_string_field(body, "name"))
                .filter(|value| !value.trim().is_empty())
            else {
                return Ok(routing::bad_request_response("title is required"));
            };
            let description = extract_json_string_field(body, "description").unwrap_or_default();
            // Matches the oracle's real AuthenticatedWebUserId.Resolve:
            // the collection's real owner is the caller's own resolved
            // identity, never a hardcoded placeholder -- empty when no
            // per-caller identity is resolvable (single-operator mode).
            let owner_user_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            )
            .unwrap_or_default();
            let _collection_persistence = state.collection_grant_persistence_lock.lock().await;
            let mut collections = state.collections.write().await;
            let previous = collections.clone();
            let compatibility_contract = route.path.starts_with("/api/v0/");
            let record = if compatibility_contract {
                let collection_type = extract_json_string_field(body, "type")
                    .filter(|value| value.trim() == "Playlist")
                    .map(|_| "Playlist".to_owned())
                    .unwrap_or_else(|| "ShareList".to_owned());
                collections.create_with_contract(
                    uuid::Uuid::new_v4().to_string(),
                    owner_user_id,
                    name,
                    description,
                    collection_type,
                )
            } else {
                collections.create(owner_user_id, name, description)
            };
            let Some(record) = record else {
                return Ok(routing::service_unavailable_response(
                    "collection capacity is full",
                ));
            };
            let mutated = collections.clone();
            let json = if compatibility_contract {
                record.native_json()
            } else {
                record.json()
            };
            drop(collections);
            if let Err(error) = persist_collection_created(state, &record).await {
                rollback_collections_if_unchanged(state, previous, &mutated).await;
                return Ok(routing::service_unavailable_response(&error));
            }
            Ok(routing::created_response(json))
        }
        ("GET", path)
            if path.starts_with("/api/collections/")
                && !path.ends_with("/items")
                && path.matches('/').count() == 3 =>
        {
            let id = path.strip_prefix("/api/collections/").unwrap_or("");
            if id.is_empty() {
                return Ok(routing::not_found_response());
            }
            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            let collections = state.collections.read().await;
            if let Some(record) = collections.get(id).filter(|record| {
                !collection_owner_forbids(caller_id.as_deref(), &record.owner_user_id)
            }) {
                let json = if route.path.starts_with("/api/v0/") {
                    record.native_json()
                } else {
                    record.json()
                };
                drop(collections);
                Ok(routing::ok_response(json))
            } else {
                drop(collections);
                Ok(routing::not_found_response())
            }
        }
        ("PUT", path)
            if path.starts_with("/api/collections/")
                && !path.contains("/items")
                && path.matches('/').count() == 3 =>
        {
            let id = path.strip_prefix("/api/collections/").unwrap_or("");
            if id.is_empty() {
                return Ok(routing::not_found_response());
            }
            let compatibility_contract = route.path.starts_with("/api/v0/");
            let (name, description, collection_type) = if compatibility_contract {
                let request = match serde_json::from_str::<serde_json::Value>(body) {
                    Ok(request @ serde_json::Value::Object(_)) => request,
                    _ => return Ok(routing::bad_request_response("Request is required.")),
                };
                let name = request
                    .get("title")
                    .and_then(serde_json::Value::as_str)
                    .map(str::trim)
                    .map(str::to_owned);
                if name.as_ref().is_some_and(|name| name.is_empty()) {
                    return Ok(routing::bad_request_response("Title cannot be blank."));
                }
                let description = request
                    .get("description")
                    .and_then(serde_json::Value::as_str)
                    .map(str::trim)
                    .map(str::to_owned);
                let collection_type =
                    request
                        .get("type")
                        .and_then(serde_json::Value::as_str)
                        .map(|value| {
                            if value.trim() == "Playlist" {
                                "Playlist".to_owned()
                            } else {
                                "ShareList".to_owned()
                            }
                        });
                (name, description, collection_type)
            } else {
                let Some(name) = extract_json_string_field(body, "title")
                    .or_else(|| extract_json_string_field(body, "name"))
                    .filter(|value| !value.trim().is_empty())
                else {
                    return Ok(routing::bad_request_response("title is required"));
                };
                (
                    Some(name),
                    Some(extract_json_string_field(body, "description").unwrap_or_default()),
                    None,
                )
            };
            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            let _collection_persistence = state.collection_grant_persistence_lock.lock().await;
            let mut collections = state.collections.write().await;
            if collections.get(id).is_some_and(|record| {
                collection_owner_forbids(caller_id.as_deref(), &record.owner_user_id)
            }) {
                drop(collections);
                return Ok(routing::not_found_response());
            }
            let previous = collections.clone();
            let updated = if compatibility_contract {
                collections.update_contract(id, name, description, collection_type)
            } else {
                collections.update(
                    id,
                    name.unwrap_or_default(),
                    description.unwrap_or_default(),
                )
            };
            if let Some(record) = updated {
                let mutated = collections.clone();
                let json = if compatibility_contract {
                    record.native_json()
                } else {
                    record.json()
                };
                drop(collections);
                if let Err(error) = persist_collection_checked(state, &record).await {
                    rollback_collections_if_unchanged(state, previous, &mutated).await;
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(routing::ok_response(json))
            } else {
                drop(collections);
                Ok(routing::not_found_response())
            }
        }
        ("DELETE", path)
            if path.starts_with("/api/collections/")
                && !path.contains("/items")
                && path.matches('/').count() == 3 =>
        {
            let id = path.strip_prefix("/api/collections/").unwrap_or("");
            if id.is_empty() {
                return Ok(routing::not_found_response());
            }
            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            let _collection_persistence = state.collection_grant_persistence_lock.lock().await;
            let mut grants = state.share_grants.write().await;
            let mut collections = state.collections.write().await;
            if collections.get(id).is_some_and(|record| {
                collection_owner_forbids(caller_id.as_deref(), &record.owner_user_id)
            }) {
                drop(collections);
                drop(grants);
                return Ok(routing::not_found_response());
            }
            let previous_collections = collections.clone();
            let previous_grants = grants.clone();
            let deleted = collections.delete(id);
            if deleted {
                let revoked_grants = grants.delete_by_collection(id);
                let mutated_collections = collections.clone();
                let mutated_grants = grants.clone();
                drop(collections);
                drop(grants);
                if let Err(error) = persist_collection_delete(state, id).await {
                    let mut grants = state.share_grants.write().await;
                    let mut collections = state.collections.write().await;
                    if share_grant_store_matches(&grants, &mutated_grants)
                        && *collections == mutated_collections
                    {
                        *collections = previous_collections;
                        *grants = previous_grants;
                    }
                    drop(collections);
                    drop(grants);
                    return Ok(routing::service_unavailable_response(&error));
                }
                let mut tokens = state.share_access_tokens.write().await;
                for grant in &revoked_grants {
                    tokens.revoke_grant(&grant.id);
                }
                drop(tokens);
                let mut tickets = state.stream_tickets.write().await;
                for grant in revoked_grants {
                    tickets.revoke_source(&format!("share:{}", grant.id));
                }
                drop(tickets);
                Ok(if route.path.starts_with("/api/v0/") {
                    routing::no_content_response()
                } else {
                    routing::ok_response("{}".to_string())
                })
            } else {
                drop(collections);
                drop(grants);
                Ok(routing::not_found_response())
            }
        }
        ("GET", path)
            if path.starts_with("/api/collections/")
                && path.ends_with("/items")
                && collection_items_id(path).is_some() =>
        {
            let id = collection_items_id(path).expect("guarded collection items path");
            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            let collections = state.collections.read().await;
            if let Some(record) = collections.get(id).filter(|record| {
                !collection_owner_forbids(caller_id.as_deref(), &record.owner_user_id)
            }) {
                let compatibility_contract = route.path.starts_with("/api/v0/");
                let items = record
                    .items
                    .iter()
                    .enumerate()
                    .map(|(ordinal, item)| {
                        if compatibility_contract {
                            item.native_json(&record.id, ordinal)
                        } else {
                            item.json()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(",");
                let json = format!("[{}]", items);
                drop(collections);
                Ok(routing::ok_response(json))
            } else {
                drop(collections);
                Ok(routing::not_found_response())
            }
        }
        ("POST", path)
            if path.starts_with("/api/collections/")
                && path.ends_with("/items")
                && collection_items_id(path).is_some() =>
        {
            let id = collection_items_id(path).expect("guarded collection items path");
            let compatibility_contract = route.path.starts_with("/api/v0/");
            let content_id = extract_json_string_field(body, "contentId")
                .or_else(|| extract_json_string_field(body, "content_id"))
                .unwrap_or_default();
            if compatibility_contract && content_id.trim().is_empty() {
                return Ok(routing::bad_request_response("ContentId is required."));
            }
            let artist = extract_json_string_field(body, "artist").unwrap_or_default();
            let title = extract_json_string_field(body, "title").unwrap_or_default();
            let kind = extract_json_string_field(body, "mediaKind")
                .or_else(|| extract_json_string_field(body, "kind"))
                .unwrap_or_else(|| "Audio".to_string());
            let file_name = extract_json_string_field(body, "fileName").unwrap_or_default();
            let album = extract_json_string_field(body, "album").unwrap_or_default();
            let content_hash = extract_json_string_field(body, "contentHash")
                .or_else(|| extract_json_string_field(body, "sha256"))
                .unwrap_or_default();

            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            let _collection_persistence = state.collection_grant_persistence_lock.lock().await;
            let mut collections = state.collections.write().await;
            if collections.get(id).is_some_and(|record| {
                collection_owner_forbids(caller_id.as_deref(), &record.owner_user_id)
            }) {
                drop(collections);
                return Ok(routing::not_found_response());
            }
            let previous = collections.clone();
            match collections.add_item_with_contract(
                id,
                compatibility_contract.then(|| uuid::Uuid::new_v4().to_string()),
                content_id,
                artist,
                title,
                kind,
                file_name,
                album,
                content_hash,
            ) {
                Ok(Some(item)) => {
                    let record = collections
                        .get(id)
                        .expect("item was added to an existing collection");
                    let mutated = collections.clone();
                    let json = if compatibility_contract {
                        item.native_json(id, record.items.len().saturating_sub(1))
                    } else {
                        item.json()
                    };
                    drop(collections);
                    if let Err(error) = persist_collection_checked(state, &record).await {
                        rollback_collections_if_unchanged(state, previous, &mutated).await;
                        return Ok(routing::service_unavailable_response(&error));
                    }
                    Ok(routing::created_response(json))
                }
                Ok(None) => {
                    drop(collections);
                    Ok(routing::not_found_response())
                }
                Err(()) => {
                    drop(collections);
                    Ok(routing::service_unavailable_response(
                        "collection item capacity is full",
                    ))
                }
            }
        }
        ("DELETE", path)
            if !path.ends_with("/items/reorder") && collection_item_action_ids(path).is_some() =>
        {
            let (item_id, requested_collection_id) =
                collection_item_action_ids(path).expect("guarded collection item path");
            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            let _collection_persistence = state.collection_grant_persistence_lock.lock().await;
            let mut collections = state.collections.write().await;
            let collection_id = collections.collection_id_for_item(item_id);
            if requested_collection_id
                .is_some_and(|expected| collection_id.as_deref() != Some(expected))
            {
                drop(collections);
                return Ok(routing::not_found_response());
            }
            if collection_id
                .as_deref()
                .and_then(|id| collections.get(id))
                .is_some_and(|record| {
                    collection_owner_forbids(caller_id.as_deref(), &record.owner_user_id)
                })
            {
                drop(collections);
                return Ok(routing::not_found_response());
            }
            let previous = collections.clone();
            if let Some(item) = collections.remove_item(item_id) {
                let record = collection_id
                    .as_deref()
                    .and_then(|id| collections.get(id))
                    .expect("removed item belonged to an existing collection");
                let mutated = collections.clone();
                let json = serde_json::json!({
                    "deleted": true,
                    "item": serde_json::from_str::<serde_json::Value>(&item.json())
                        .unwrap_or_else(|_| serde_json::json!({ "id": item_id })),
                })
                .to_string();
                drop(collections);
                if let Err(error) = persist_collection_checked(state, &record).await {
                    rollback_collections_if_unchanged(state, previous, &mutated).await;
                    return Ok(routing::service_unavailable_response(&error));
                }
                if route.path.starts_with("/api/v0/") {
                    Ok(routing::no_content_response())
                } else {
                    Ok(routing::ok_response(json))
                }
            } else {
                drop(collections);
                Ok(routing::not_found_response())
            }
        }
        ("PUT", path)
            if !path.ends_with("/items/reorder") && collection_item_action_ids(path).is_some() =>
        {
            let (item_id, requested_collection_id) =
                collection_item_action_ids(path).expect("guarded collection item path");
            let compatibility_contract = route.path.starts_with("/api/v0/");
            let content_id = extract_json_string_field(body, "contentId")
                .or_else(|| extract_json_string_field(body, "content_id"));
            let artist = extract_json_string_field(body, "artist");
            let title = extract_json_string_field(body, "title");
            let kind = extract_json_string_field(body, "kind")
                .or_else(|| extract_json_string_field(body, "mediaKind"));
            let file_name = extract_json_string_field(body, "fileName");
            let album = extract_json_string_field(body, "album");
            let content_hash = extract_json_string_field(body, "contentHash")
                .or_else(|| extract_json_string_field(body, "sha256"));

            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            let _collection_persistence = state.collection_grant_persistence_lock.lock().await;
            let mut collections = state.collections.write().await;
            let collection_id = collections.collection_id_for_item(item_id);
            if requested_collection_id
                .is_some_and(|expected| collection_id.as_deref() != Some(expected))
            {
                drop(collections);
                return Ok(routing::not_found_response());
            }
            if collection_id
                .as_deref()
                .and_then(|id| collections.get(id))
                .is_some_and(|record| {
                    collection_owner_forbids(caller_id.as_deref(), &record.owner_user_id)
                })
            {
                drop(collections);
                return Ok(routing::not_found_response());
            }
            let previous = collections.clone();
            let updated = if compatibility_contract {
                collections.update_item_contract(
                    item_id,
                    content_id,
                    artist,
                    title,
                    kind,
                    file_name,
                    album,
                    content_hash,
                )
            } else {
                collections.update_item(item_id, artist, title, kind)
            };
            if let Some(item) = updated {
                let record = collection_id
                    .as_deref()
                    .and_then(|id| collections.get(id))
                    .expect("updated item belonged to an existing collection");
                let mutated = collections.clone();
                let json = if compatibility_contract {
                    let ordinal = record
                        .items
                        .iter()
                        .position(|candidate| candidate.id == item.id)
                        .unwrap_or_default();
                    item.native_json(&record.id, ordinal)
                } else {
                    item.json()
                };
                drop(collections);
                if let Err(error) = persist_collection_checked(state, &record).await {
                    rollback_collections_if_unchanged(state, previous, &mutated).await;
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(routing::ok_response(json))
            } else {
                drop(collections);
                Ok(routing::not_found_response())
            }
        }

        // WISHLIST ENDPOINTS
        _ => Err(ROUTE_NOT_HANDLED.to_owned()),
    }
}
