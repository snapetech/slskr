async fn route_dispatch_group_4(context: &RouteDispatchContext<'_, '_>) -> RouteDispatchResult {
    let RouteDispatchContext {
        method,
        normalized_path,
        authorization,
        body,
        state,
        route,
        headers,
        extended_mutation,
        request_is_versioned_v0,
    } = *context;
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
            if let Err(error) = persist_collection_checked(state, &record).await {
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
                if let Err(error) = persist_collection_delete(state, id).await {
                    *collections = previous_collections;
                    *grants = previous_grants;
                    return Ok(routing::service_unavailable_response(&error));
                }
                drop(collections);
                drop(grants);
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
        ("DELETE", path) if collection_item_action_ids(path).is_some() => {
            let (item_id, requested_collection_id) =
                collection_item_action_ids(path).expect("guarded collection item path");
            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
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
        ("PUT", path) if collection_item_action_ids(path).is_some() => {
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
        ("GET", "/api/wishlist") => {
            let mut wishlist = state.wishlist.write().await;
            let json = if route.path.starts_with("/api/v0/") {
                wishlist.get_or_create();
                format!(
                    "[{}]",
                    wishlist
                        .records
                        .iter()
                        .flat_map(|record| record.items.iter())
                        .map(WishlistItem::native_json)
                        .collect::<Vec<_>>()
                        .join(",")
                )
            } else {
                wishlist.json_array()
            };
            drop(wishlist);
            Ok(routing::ok_response(json))
        }
        ("POST", "/api/wishlist") => {
            let search_text = extract_json_string_field(body, "searchText").unwrap_or_default();
            let artist =
                extract_json_string_field(body, "artist").unwrap_or_else(|| search_text.clone());
            let title = extract_json_string_field(body, "title").unwrap_or_default();
            let kind =
                extract_json_string_field(body, "kind").unwrap_or_else(|| "Audio".to_string());
            if artist.trim().is_empty() && title.trim().is_empty() {
                return Ok(routing::bad_request_response("SearchText is required"));
            }
            let filter = extract_json_string_field(body, "filter").unwrap_or_default();
            let enabled = extract_json_bool_field(body, "enabled").unwrap_or(true);
            let auto_download = extract_json_bool_field(body, "autoDownload").unwrap_or(false);
            let max_results = extract_json_u64_field(body, "maxResults").unwrap_or(100);
            if max_results == 0 || max_results > MAX_WISHLIST_RESULTS as u64 {
                return Ok(routing::bad_request_response(
                    "MaxResults must be between 1 and 10000",
                ));
            }
            let max_downloads = extract_json_optional_u64_field(body, "maxDownloads");
            if max_downloads
                .flatten()
                .is_some_and(|value| value == 0 || value > MAX_WISHLIST_DOWNLOADS)
            {
                return Ok(routing::bad_request_response(
                    "MaxDownloads must be null or between 1 and 1000000",
                ));
            }

            let mut wishlist = state.wishlist.write().await;
            let previous = wishlist.clone();
            let compatibility_contract = route.path.starts_with("/api/v0/");
            let id = compatibility_contract.then(|| uuid::Uuid::new_v4().to_string());
            match wishlist.add_item_with_contract(
                id,
                artist,
                title,
                kind,
                filter,
                enabled,
                auto_download,
                usize::try_from(max_results).unwrap_or(MAX_WISHLIST_RESULTS),
                max_downloads.flatten(),
            ) {
                Ok(item) => {
                    let mutated = wishlist.clone();
                    let json = if compatibility_contract {
                        item.native_json()
                    } else {
                        item.json()
                    };
                    drop(wishlist);
                    if let Err(error) = persist_wishlist_item_checked(state, &item).await {
                        rollback_wishlist_if_unchanged(state, previous, &mutated).await;
                        return Ok(wishlist_storage_error_response(
                            route.path.starts_with("/api/v0/"),
                            &error,
                        ));
                    }
                    Ok(routing::created_response(json))
                }
                Err(()) => {
                    drop(wishlist);
                    Ok(routing::service_unavailable_response(
                        "wishlist item capacity is full",
                    ))
                }
            }
        }
        ("GET", path) if wishlist_item_action_id(path, "/searches").is_some() => {
            let requested_item_id =
                wishlist_item_action_id(path, "/searches").expect("guarded wishlist history path");
            let native = route.path.starts_with("/api/v0/");
            let wishlist = state.wishlist.read().await;
            let Some(item_id) = wishlist.resolve_item_id(requested_item_id, native) else {
                return Ok(routing::not_found_response());
            };
            drop(wishlist);
            let searches = state.searches.read().await;
            let json = searches.wishlist_history_json(&item_id, route.query);
            drop(searches);
            Ok(routing::ok_response(json))
        }
        ("GET", path)
            if path.starts_with("/api/wishlist/") && !path.contains("/ignored-results") =>
        {
            let Some(requested_item_id) = path_segment_after(path, "/api/wishlist/") else {
                return Ok(routing::not_found_response());
            };
            let native = route.path.starts_with("/api/v0/");
            let wishlist = state.wishlist.read().await;
            let Some(item_id) = wishlist.resolve_item_id(requested_item_id, native) else {
                return Ok(routing::not_found_response());
            };
            let Some(item) = wishlist.get_item(&item_id) else {
                return Ok(routing::not_found_response());
            };
            Ok(routing::ok_response(
                if route.path.starts_with("/api/v0/") {
                    item.native_json()
                } else {
                    item.json()
                },
            ))
        }
        ("POST", "/api/wishlist/mark-all-viewed") => {
            let mut wishlist = state.wishlist.write().await;
            let previous = wishlist.clone();
            let items = wishlist.mark_all_viewed();
            let mutated = wishlist.clone();
            drop(wishlist);
            if let Err(error) = persist_wishlist_items_checked(state, &items).await {
                rollback_wishlist_if_unchanged(state, previous, &mutated).await;
                return Ok(wishlist_storage_error_response(
                    route.path.starts_with("/api/v0/"),
                    &error,
                ));
            }
            Ok(routing::no_content_response())
        }
        ("POST", path) if wishlist_item_action_id(path, "/mark-viewed").is_some() => {
            let requested_item_id = wishlist_item_action_id(path, "/mark-viewed")
                .expect("guarded wishlist viewed path");
            let native = route.path.starts_with("/api/v0/");
            let mut wishlist = state.wishlist.write().await;
            let Some(item_id) = wishlist.resolve_item_id(requested_item_id, native) else {
                return Ok(routing::not_found_response());
            };
            let previous = wishlist.clone();
            let Some(item) = wishlist.mark_viewed(&item_id) else {
                return Ok(routing::not_found_response());
            };
            let mutated = wishlist.clone();
            drop(wishlist);
            if let Err(error) = persist_wishlist_item_checked(state, &item).await {
                rollback_wishlist_if_unchanged(state, previous, &mutated).await;
                return Ok(wishlist_storage_error_response(
                    route.path.starts_with("/api/v0/"),
                    &error,
                ));
            }
            Ok(routing::no_content_response())
        }
        ("GET", path) if wishlist_ignored_results_item_id(path).is_some() => {
            let requested_item_id =
                wishlist_ignored_results_item_id(path).expect("guarded ignored path");
            let compatibility_contract = route.path.starts_with("/api/v0/");
            let wishlist = state.wishlist.read().await;
            let Some(item_id) = wishlist.resolve_item_id(requested_item_id, compatibility_contract)
            else {
                return Ok(routing::not_found_response());
            };
            let Some(rules) = wishlist.list_ignored_results(&item_id) else {
                return Ok(routing::not_found_response());
            };
            let json = serde_json::Value::Array(
                rules
                    .iter()
                    .map(|rule| {
                        if compatibility_contract {
                            rule.native_json()
                        } else {
                            rule.json()
                        }
                    })
                    .collect(),
            )
            .to_string();
            Ok(routing::ok_response(json))
        }
        ("POST", path) if wishlist_ignored_results_item_id(path).is_some() => {
            let requested_item_id =
                wishlist_ignored_results_item_id(path).expect("guarded ignored path");
            let compatibility_contract = route.path.starts_with("/api/v0/");
            let username = extract_json_string_field(body, "username").unwrap_or_default();
            let directory = extract_json_string_field(body, "directory").unwrap_or_default();
            if username.trim().is_empty() || normalize_wishlist_directory(&directory).is_empty() {
                return Ok(routing::bad_request_response(
                    if route.path.starts_with("/api/v0/") {
                        "Username and Directory are required"
                    } else {
                        "username and directory are required"
                    },
                ));
            }

            let mut wishlist = state.wishlist.write().await;
            let Some(item_id) = wishlist.resolve_item_id(requested_item_id, compatibility_contract)
            else {
                return Ok(routing::not_found_response());
            };
            let previous = wishlist.clone();
            let (rule, created) = match wishlist.ignore_result(
                &item_id,
                &username,
                &directory,
                compatibility_contract,
            ) {
                Ok(result) => result,
                Err("not_found") => return Ok(routing::not_found_response()),
                Err("capacity") => {
                    return Ok(routing::service_unavailable_response(
                        "wishlist ignored-result capacity is full",
                    ));
                }
                Err(_) => {
                    return Ok(routing::bad_request_response(
                        if route.path.starts_with("/api/v0/") {
                            "Username and Directory are required"
                        } else {
                            "username and directory are required"
                        },
                    ));
                }
            };
            let mutated = wishlist.clone();
            drop(wishlist);
            if created {
                let (previous_searches, mutated_searches, changed_searches) = {
                    let mut searches = state.searches.write().await;
                    let previous = searches.clone();
                    let changed = searches.suppress_ignored_result(&rule);
                    let mutated = searches.clone();
                    (previous, mutated, changed)
                };
                if let Err(error) = persist_wishlist_ignored_result_and_searches_checked(
                    state,
                    &rule,
                    &changed_searches,
                )
                .await
                {
                    rollback_wishlist_if_unchanged(state, previous, &mutated).await;
                    let mut searches = state.searches.write().await;
                    if *searches == mutated_searches {
                        *searches = previous_searches;
                    }
                    drop(searches);
                    return Ok(wishlist_storage_error_response(
                        route.path.starts_with("/api/v0/"),
                        &error,
                    ));
                }
            }
            let json = if compatibility_contract {
                rule.native_json()
            } else {
                rule.json()
            }
            .to_string();
            if created || compatibility_contract {
                Ok(routing::created_response(json))
            } else {
                Ok(routing::ok_response(json))
            }
        }
        ("DELETE", path) if wishlist_ignored_result_ids(path).is_some() => {
            let (requested_item_id, rule_id) =
                wishlist_ignored_result_ids(path).expect("guarded ignored rule path");
            let compatibility_contract = route.path.starts_with("/api/v0/");
            let mut wishlist = state.wishlist.write().await;
            let Some(item_id) = wishlist.resolve_item_id(requested_item_id, compatibility_contract)
            else {
                return Ok(routing::not_found_response());
            };
            let previous = wishlist.clone();
            if !wishlist.delete_ignored_result(&item_id, rule_id) {
                return Ok(routing::not_found_response());
            }
            let mutated = wishlist.clone();
            drop(wishlist);
            if let Err(error) =
                persist_wishlist_ignored_result_delete_checked(state, &item_id, rule_id).await
            {
                rollback_wishlist_if_unchanged(state, previous, &mutated).await;
                return Ok(wishlist_storage_error_response(
                    route.path.starts_with("/api/v0/"),
                    &error,
                ));
            }
            Ok(routing::no_content_response())
        }
        ("DELETE", path) if path.starts_with("/api/wishlist/") => {
            let Some(requested_item_id) = path_segment_after(path, "/api/wishlist/") else {
                return Ok(routing::not_found_response());
            };
            let compatibility_contract = route.path.starts_with("/api/v0/");
            let mut wishlist = state.wishlist.write().await;
            let Some(item_id) = wishlist.resolve_item_id(requested_item_id, compatibility_contract)
            else {
                return Ok(if compatibility_contract {
                    routing::no_content_response()
                } else {
                    routing::not_found_response()
                });
            };
            let previous = wishlist.clone();
            if let Some(record) = wishlist.remove_item(&item_id) {
                let mutated = wishlist.clone();
                let json = serde_json::json!({
                    "deleted": true,
                    "item_id": requested_item_id,
                    "remaining": record.items.len(),
                    "updated_at": record.updated_at,
                })
                .to_string();
                drop(wishlist);
                if let Err(error) = persist_wishlist_item_delete_checked(state, &item_id).await {
                    rollback_wishlist_if_unchanged(state, previous, &mutated).await;
                    return Ok(wishlist_storage_error_response(
                        route.path.starts_with("/api/v0/"),
                        &error,
                    ));
                }
                Ok(if route.path.starts_with("/api/v0/") {
                    routing::no_content_response()
                } else {
                    routing::ok_response(json)
                })
            } else {
                drop(wishlist);
                Ok(if route.path.starts_with("/api/v0/") {
                    routing::no_content_response()
                } else {
                    routing::not_found_response()
                })
            }
        }

        // CONTACTS ENDPOINTS
        ("GET", "/api/contacts/nearby") => {
            let contacts = state.contacts.read().await;
            let json = contacts.nearby_json(route.query);
            drop(contacts);
            Ok(routing::ok_response(json))
        }
        ("GET", "/api/contacts") => {
            let contacts = state.contacts.read().await;
            let json = contacts.json_array(route.query);
            drop(contacts);
            Ok(routing::ok_response(json))
        }
        ("POST", "/api/contacts") => {
            let username = extract_json_string_field(body, "username").unwrap_or_default();
            if username.is_empty() {
                return Ok(routing::conflict_response("username is required"));
            }
            let mut contacts = state.contacts.write().await;
            let previous = contacts.clone();
            let (record, created) = match contacts.create(username) {
                Ok(result) => result,
                Err(()) => {
                    return Ok(routing::service_unavailable_response(
                        "contact capacity is full",
                    ));
                }
            };
            let mutated = contacts.clone();
            let json = record.json();
            drop(contacts);
            if created {
                if let Err(error) = persist_contact_checked(state, &record).await {
                    let mut contacts = state.contacts.write().await;
                    if *contacts == mutated {
                        *contacts = previous;
                    }
                    drop(contacts);
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(routing::created_response(json))
            } else {
                Ok(routing::conflict_response("contact already exists"))
            }
        }
        ("POST", "/api/contacts/from-discovery") => {
            if route.path.starts_with("/api/v0/") {
                let peer_id = extract_json_string_field(body, "peerId")
                    .unwrap_or_default()
                    .trim()
                    .to_owned();
                let nickname = extract_json_string_field(body, "nickname")
                    .unwrap_or_default()
                    .trim()
                    .to_owned();
                if peer_id.trim().is_empty() {
                    return Ok(routing::bad_request_response("PeerId is required."));
                }
                if nickname.trim().is_empty() {
                    return Ok(routing::bad_request_response("Nickname is required."));
                }
                // The profile service always has the local signed profile
                // available.  Unknown peers still require a cached/fetched
                // profile, which this in-process compatibility layer does
                // not synthesize from a username alone.
                if !peer_id.eq_ignore_ascii_case(&local_profile_peer_id(state)) {
                    return Ok(HttpResponse {
                        status: "404 Not Found",
                        content_type: "application/json",
                        body: serde_json::json!("Profile not found.").to_string(),
                    });
                }
                let mut contacts = state.contacts.write().await;
                let previous = contacts.clone();
                let (record, added) = match contacts
                    .create_with_contract(Some(uuid::Uuid::new_v4().to_string()), nickname)
                {
                    Ok(result) => result,
                    Err(()) => {
                        return Ok(routing::service_unavailable_response(
                            "contact capacity is full",
                        ));
                    }
                };
                let mutated = contacts.clone();
                let json = record.native_json(&peer_id);
                drop(contacts);
                if added {
                    if let Err(error) = persist_contact_checked(state, &record).await {
                        let mut contacts = state.contacts.write().await;
                        if *contacts == mutated {
                            *contacts = previous;
                        }
                        drop(contacts);
                        return Ok(routing::service_unavailable_response(&error));
                    }
                }
                return Ok(if added {
                    routing::created_response(json)
                } else {
                    routing::ok_response(json)
                });
            }
            let username = extract_json_string_field(body, "username").unwrap_or_default();
            if username.is_empty() {
                return Ok(routing::bad_request_response("username is required"));
            }
            let mut contacts = state.contacts.write().await;
            let previous = contacts.clone();
            let (record, added) = match contacts.create(username.clone()) {
                Ok(result) => result,
                Err(()) => {
                    return Ok(routing::service_unavailable_response(
                        "contact capacity is full",
                    ));
                }
            };
            let mutated = contacts.clone();
            drop(contacts);
            if added {
                if let Err(error) = persist_contact_checked(state, &record).await {
                    let mut contacts = state.contacts.write().await;
                    if *contacts == mutated {
                        *contacts = previous;
                    }
                    drop(contacts);
                    return Ok(routing::service_unavailable_response(&error));
                }
            }
            let json = format!(
                "{{\"username\":\"{}\",\"discovered\":true,\"added\":{added}}}",
                json_escape(&username),
            );
            Ok(if added {
                routing::created_response(json)
            } else {
                routing::ok_response(json)
            })
        }
        ("POST", "/api/contacts/from-invite") => {
            let username = extract_json_string_field(body, "username").unwrap_or_default();
            if username.is_empty() {
                return Ok(routing::bad_request_response("username is required"));
            }
            let id = route
                .path
                .starts_with("/api/v0/")
                .then(|| uuid::Uuid::new_v4().to_string());
            let mut contacts = state.contacts.write().await;
            let previous = contacts.clone();
            let (record, added) = match contacts.create_with_contract(id, username.clone()) {
                Ok(result) => result,
                Err(()) => {
                    return Ok(routing::service_unavailable_response(
                        "contact capacity is full",
                    ));
                }
            };
            let mutated = contacts.clone();
            drop(contacts);
            if added {
                if let Err(error) = persist_contact_checked(state, &record).await {
                    let mut contacts = state.contacts.write().await;
                    if *contacts == mutated {
                        *contacts = previous;
                    }
                    drop(contacts);
                    return Ok(routing::service_unavailable_response(&error));
                }
            }
            let json = format!(
                "{{\"username\":\"{}\",\"invited\":true,\"accepted\":true,\"added\":{added}}}",
                json_escape(&username),
            );
            Ok(if added {
                routing::created_response(json)
            } else {
                routing::ok_response(json)
            })
        }
        ("GET", path) if path.starts_with("/api/contacts/") => {
            let Some(id) = path_segment_after(path, "/api/contacts/") else {
                return Ok(routing::not_found_response());
            };
            let contacts = state.contacts.read().await;
            if let Some(record) = contacts.get(id) {
                let json = record.json();
                drop(contacts);
                Ok(routing::ok_response(json))
            } else {
                drop(contacts);
                Ok(routing::not_found_response())
            }
        }
        ("PUT", path) if path.starts_with("/api/contacts/") => {
            let Some(id) = path_segment_after(path, "/api/contacts/") else {
                return Ok(routing::not_found_response());
            };
            let username = extract_json_string_field(body, "username");
            let online = extract_json_bool_field(body, "online");
            let mut contacts = state.contacts.write().await;
            let previous = contacts.clone();
            match contacts.update(id, username, online) {
                Ok(record) => {
                    let mutated = contacts.clone();
                    let json = record.json();
                    drop(contacts);
                    if let Err(error) = persist_contact_checked(state, &record).await {
                        let mut contacts = state.contacts.write().await;
                        if *contacts == mutated {
                            *contacts = previous;
                        }
                        drop(contacts);
                        return Ok(routing::service_unavailable_response(&error));
                    }
                    Ok(routing::ok_response(json))
                }
                Err(ContactUpdateError::DuplicateUsername) => {
                    drop(contacts);
                    Ok(routing::conflict_response(
                        "contact username already exists",
                    ))
                }
                Err(ContactUpdateError::NotFound) => {
                    drop(contacts);
                    Ok(routing::not_found_response())
                }
            }
        }
        ("DELETE", path) if path.starts_with("/api/contacts/") => {
            let Some(id) = path_segment_after(path, "/api/contacts/") else {
                return Ok(routing::not_found_response());
            };
            let mut contacts = state.contacts.write().await;
            let previous = contacts.clone();
            let deleted = contacts.delete(id);
            let mutated = contacts.clone();
            drop(contacts);
            if deleted {
                if let Err(error) = persist_contact_delete_checked(state, id).await {
                    let mut contacts = state.contacts.write().await;
                    if *contacts == mutated {
                        *contacts = previous;
                    }
                    drop(contacts);
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(routing::ok_response("{}".to_string()))
            } else {
                Ok(routing::not_found_response())
            }
        }

        // SHAREGROUPS ENDPOINTS
        ("GET", "/api/sharegroups") => {
            let sharegroups = state.sharegroups.read().await;
            let json = sharegroups.json_array(route.query);
            drop(sharegroups);
            Ok(routing::ok_response(json))
        }
        ("POST", "/api/sharegroups") => {
            let requested_name = extract_json_string_field(body, "name");
            let name = if route.path.starts_with("/api/v0/") {
                let Some(name) = requested_name else {
                    return Ok(routing::bad_request_response("Name is required."));
                };
                if name.trim().is_empty() {
                    return Ok(routing::bad_request_response("Name is required."));
                }
                name
            } else {
                requested_name.unwrap_or_else(|| "Untitled".to_string())
            };
            let description = extract_json_string_field(body, "description").unwrap_or_default();
            let mut sharegroups = state.sharegroups.write().await;
            let previous = sharegroups.clone();
            let Some(mut record) = sharegroups.create(name, description) else {
                return Ok(routing::service_unavailable_response(
                    "share group capacity is full",
                ));
            };
            let json = if route.path.starts_with("/api/v0/") {
                let old_id = record.id.clone();
                record.id = uuid::Uuid::new_v4().to_string();
                if let Some(stored) = sharegroups
                    .records
                    .iter_mut()
                    .find(|group| group.id == old_id)
                {
                    stored.id.clone_from(&record.id);
                }
                serde_json::json!({
                    "id": record.id,
                    "name": record.name,
                    "ownerUserId": "Anonymous",
                    "createdAt": unix_seconds_rfc3339(record.created_at),
                    "updatedAt": unix_seconds_rfc3339(record.updated_at),
                })
                .to_string()
            } else {
                record.json()
            };
            if let Err(error) = persist_share_group(state, &record).await {
                *sharegroups = previous;
                return Ok(routing::service_unavailable_response(&error));
            }
            drop(sharegroups);
            Ok(routing::created_response(json))
        }
        ("GET", path)
            if path.starts_with("/api/sharegroups/") && share_group_resource_id(path).is_some() =>
        {
            let id = share_group_resource_id(path).expect("guarded share-group resource path");
            let sharegroups = state.sharegroups.read().await;
            if let Some(record) = sharegroups.get(id) {
                let json = record.json();
                drop(sharegroups);
                Ok(routing::ok_response(json))
            } else {
                drop(sharegroups);
                Ok(routing::not_found_response())
            }
        }
        ("PUT", path)
            if path.starts_with("/api/sharegroups/") && share_group_resource_id(path).is_some() =>
        {
            let id = share_group_resource_id(path).expect("guarded share-group resource path");
            let name =
                extract_json_string_field(body, "name").unwrap_or_else(|| "Untitled".to_string());
            let description = extract_json_string_field(body, "description").unwrap_or_default();
            let mut sharegroups = state.sharegroups.write().await;
            let previous = sharegroups.clone();
            if let Some(record) = sharegroups.update(id, name, description) {
                let json = record.json();
                if let Err(error) = persist_share_group(state, &record).await {
                    *sharegroups = previous;
                    return Ok(routing::service_unavailable_response(&error));
                }
                drop(sharegroups);
                Ok(routing::ok_response(json))
            } else {
                drop(sharegroups);
                Ok(routing::not_found_response())
            }
        }
        ("DELETE", path)
            if path.starts_with("/api/sharegroups/") && share_group_resource_id(path).is_some() =>
        {
            let id = share_group_resource_id(path).expect("guarded share-group resource path");
            let mut sharegroups = state.sharegroups.write().await;
            let previous = sharegroups.clone();
            let deleted = sharegroups.delete(id);
            if deleted {
                if let Err(error) = persist_share_group_delete(state, id).await {
                    *sharegroups = previous;
                    return Ok(routing::service_unavailable_response(&error));
                }
                drop(sharegroups);
                Ok(routing::ok_response("{}".to_string()))
            } else {
                drop(sharegroups);
                Ok(routing::not_found_response())
            }
        }
        ("GET", path)
            if path.starts_with("/api/sharegroups/")
                && path.ends_with("/members")
                && share_group_members_id(path).is_some() =>
        {
            let id = share_group_members_id(path).expect("guarded share-group members path");
            let sharegroups = state.sharegroups.read().await;
            if let Some(record) = sharegroups.get(id) {
                let members = record
                    .members
                    .iter()
                    .map(|m| m.json())
                    .collect::<Vec<_>>()
                    .join(",");
                let json = format!("[{}]", members);
                drop(sharegroups);
                Ok(routing::ok_response(json))
            } else {
                drop(sharegroups);
                Ok(routing::not_found_response())
            }
        }
        ("POST", path)
            if path.starts_with("/api/sharegroups/")
                && path.ends_with("/members")
                && share_group_members_id(path).is_some() =>
        {
            let id = share_group_members_id(path).expect("guarded share-group members path");
            if route.path.starts_with("/api/v0/")
                && state.sharegroups.read().await.get(id).is_none()
            {
                return Ok(routing::not_found_response());
            }
            let username = extract_json_string_field(body, "username").unwrap_or_default();
            if username.is_empty() {
                return Ok(routing::conflict_response("username is required"));
            }
            let mut sharegroups = state.sharegroups.write().await;
            let previous = sharegroups.clone();
            match sharegroups.add_member(id, username.clone()) {
                Ok(Some((record, added))) => {
                    let member = record
                        .members
                        .iter()
                        .find(|member| member.username.eq_ignore_ascii_case(&username))
                        .cloned();
                    let json = member
                        .as_ref()
                        .map(ShareGroupMember::json)
                        .unwrap_or_else(|| {
                            format!(
                                "{{\"username\":\"{}\",\"added_at\":{}}}",
                                json_escape(&username),
                                unix_timestamp()
                            )
                        });
                    if added {
                        if let Err(error) = persist_share_group(state, &record).await {
                            *sharegroups = previous;
                            return Ok(routing::service_unavailable_response(&error));
                        }
                    }
                    drop(sharegroups);
                    Ok(if added {
                        routing::created_response(json)
                    } else {
                        routing::ok_response(json)
                    })
                }
                Ok(None) => {
                    drop(sharegroups);
                    Ok(routing::not_found_response())
                }
                Err(()) => {
                    drop(sharegroups);
                    Ok(routing::service_unavailable_response(
                        "share group member capacity is full",
                    ))
                }
            }
        }
        ("DELETE", path)
            if path.starts_with("/api/sharegroups/")
                && path.contains("/members/")
                && share_group_member_path(path).is_some() =>
        {
            let (id, username) =
                share_group_member_path(path).expect("guarded share-group member path");
            let mut sharegroups = state.sharegroups.write().await;
            let previous = sharegroups.clone();
            if let Some(record) = sharegroups.remove_member(id, &username) {
                if let Err(error) = persist_share_group(state, &record).await {
                    *sharegroups = previous;
                    return Ok(routing::service_unavailable_response(&error));
                }
                drop(sharegroups);
                Ok(routing::ok_response("{}".to_string()))
            } else {
                drop(sharegroups);
                Ok(routing::not_found_response())
            }
        }

        // USER NOTES ENDPOINTS
        ("GET", "/api/users/notes")
            if route.path.starts_with("/api/v0/") || route.path.starts_with("/api/v1/") =>
        {
            let notes = state.user_notes.read().await;
            let value = notes
                .records
                .iter()
                .filter_map(|record| {
                    serde_json::from_str::<serde_json::Value>(&record.native_json()).ok()
                })
                .collect::<Vec<_>>();
            Ok(routing::ok_response(serde_json::json!(value).to_string()))
        }
        ("GET", "/api/users/notes") => {
            let notes = state.user_notes.read().await;
            let json = notes.json(None);
            drop(notes);
            Ok(routing::ok_response(json))
        }
        ("POST", "/api/users/notes") => {
            let username = extract_json_string_field(body, "username").unwrap_or_default();
            let note = extract_json_string_field(body, "note").unwrap_or_default();
            if username.is_empty() {
                return Ok(routing::bad_request_response("Username is required."));
            }
            let mut notes = state.user_notes.write().await;
            let previous = notes.clone();
            let versioned =
                route.path.starts_with("/api/v0/") || route.path.starts_with("/api/v1/");
            let record = if versioned {
                notes.set_versioned(
                    username,
                    note,
                    extract_json_string_field(body, "color").unwrap_or_default(),
                    extract_json_string_field(body, "icon").unwrap_or_default(),
                    extract_json_bool_field(body, "isHighPriority").unwrap_or(false),
                )
            } else {
                notes.create(username, note)
            };
            let Some(record) = record else {
                return Ok(routing::service_unavailable_response(
                    "user note capacity is full",
                ));
            };
            let mutated = notes.clone();
            let json = if versioned {
                record.native_json()
            } else {
                record.json()
            };
            drop(notes);
            if let Err(error) = persist_user_note_checked(state, &record).await {
                let mut notes = state.user_notes.write().await;
                if *notes == mutated {
                    *notes = previous;
                }
                drop(notes);
                return Ok(routing::service_unavailable_response(&error));
            }
            Ok(if versioned {
                routing::ok_response(json)
            } else {
                routing::created_response(json)
            })
        }
        ("GET", path) if path.starts_with("/api/users/notes/") => {
            let Some(id) = path_segment_after(path, "/api/users/notes/") else {
                return Ok(routing::not_found_response());
            };
            let notes = state.user_notes.read().await;
            let versioned =
                route.path.starts_with("/api/v0/") || route.path.starts_with("/api/v1/");
            let record = if versioned {
                notes.get_by_username(id)
            } else {
                notes.get(id)
            };
            if let Some(record) = record {
                let json = if versioned {
                    record.native_json()
                } else {
                    record.json()
                };
                drop(notes);
                Ok(routing::ok_response(json))
            } else {
                drop(notes);
                Ok(routing::not_found_response())
            }
        }
        ("PUT", path) if path.starts_with("/api/users/notes/") => {
            let Some(id) = path_segment_after(path, "/api/users/notes/") else {
                return Ok(routing::not_found_response());
            };
            let note = extract_json_string_field(body, "note").unwrap_or_default();
            let mut notes = state.user_notes.write().await;
            let previous = notes.clone();
            if let Some(record) = notes.update(id, note) {
                let mutated = notes.clone();
                let json = record.json();
                drop(notes);
                if let Err(error) = persist_user_note_checked(state, &record).await {
                    let mut notes = state.user_notes.write().await;
                    if *notes == mutated {
                        *notes = previous;
                    }
                    drop(notes);
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(routing::ok_response(json))
            } else {
                drop(notes);
                Ok(routing::not_found_response())
            }
        }
        ("DELETE", path) if path.starts_with("/api/users/notes/") => {
            let Some(id) = path_segment_after(path, "/api/users/notes/") else {
                return Ok(routing::not_found_response());
            };
            let mut notes = state.user_notes.write().await;
            let previous = notes.clone();
            let versioned =
                route.path.starts_with("/api/v0/") || route.path.starts_with("/api/v1/");
            let removed_id = if versioned {
                notes.delete_by_username(id).map(|record| record.id)
            } else {
                notes.delete(id).then(|| id.to_owned())
            };
            let mutated = notes.clone();
            drop(notes);
            if let Some(removed_id) = removed_id {
                if let Err(error) = persist_user_note_delete_checked(state, &removed_id).await {
                    let mut notes = state.user_notes.write().await;
                    if *notes == mutated {
                        *notes = previous;
                    }
                    drop(notes);
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(if versioned {
                    routing::no_content_response()
                } else {
                    routing::ok_response("{}".to_string())
                })
            } else if versioned {
                Ok(routing::no_content_response())
            } else {
                Ok(routing::not_found_response())
            }
        }

        // INTERESTS ENDPOINTS (Liked)
        ("GET", "/api/soulseek/interests") => {
            let interests = state.interests.read().await;
            let json = interests.json_liked();
            drop(interests);
            Ok(routing::ok_response(json))
        }
        ("POST", "/api/soulseek/interests") => {
            let versioned = route.path.starts_with("/api/v0/");
            let name = extract_json_string_field(body, "item")
                .or_else(|| extract_json_string_field(body, "name"))
                .map(|name| name.trim().to_owned())
                .unwrap_or_default();
            if name.is_empty() {
                return Ok(routing::bad_request_response("item is required"));
            }
            if versioned && state.session.read().await.state != "connected" {
                return Ok(routing::service_unavailable_response(
                    "Soulseek session is disconnected",
                ));
            }
            let mut interests = state.interests.write().await;
            let previous = interests.clone();
            let Some((record, created)) = interests.add_liked(name) else {
                return Ok(routing::service_unavailable_response(
                    "liked interest capacity is full",
                ));
            };
            let mutated = interests.clone();
            let json = record.json();
            drop(interests);
            if created {
                if let Err(error) = persist_interest_checked(state, &record).await {
                    let mut interests = state.interests.write().await;
                    if *interests == mutated {
                        *interests = previous;
                    }
                    drop(interests);
                    return Ok(routing::service_unavailable_response(&error));
                }
                if versioned {
                    if let Err(error) = send_active_interest_command(
                        state,
                        ServerMessage::AddThingILike {
                            item: record.name.clone(),
                        },
                    )
                    .await
                    {
                        return Ok(routing::service_unavailable_response(&error));
                    }
                    Ok(routing::no_content_response())
                } else {
                    Ok(routing::created_response(json))
                }
            } else if versioned {
                if let Err(error) = send_active_interest_command(
                    state,
                    ServerMessage::AddThingILike {
                        item: record.name.clone(),
                    },
                )
                .await
                {
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(routing::no_content_response())
            } else {
                Ok(routing::ok_response(json))
            }
        }
        ("DELETE", path) if path.starts_with("/api/soulseek/interests/") => {
            let versioned = route.path.starts_with("/api/v0/");
            let Some(raw_item) = path_segment_after(path, "/api/soulseek/interests/") else {
                return Ok(routing::not_found_response());
            };
            let item = decoded_path_segment(raw_item).trim().to_owned();
            if item.is_empty() {
                return Ok(routing::bad_request_response("item is required"));
            }
            if versioned && state.session.read().await.state != "connected" {
                return Ok(routing::service_unavailable_response(
                    "Soulseek session is disconnected",
                ));
            }
            let mut interests = state.interests.write().await;
            let previous = interests.clone();
            let id = interests
                .liked
                .iter()
                .find(|record| {
                    record.id.eq_ignore_ascii_case(&item) || record.name.eq_ignore_ascii_case(&item)
                })
                .map(|record| record.id.clone());
            let deleted = id.as_deref().is_some_and(|id| interests.remove_liked(id));
            let mutated = interests.clone();
            drop(interests);
            if deleted {
                let existing_id = id.as_deref().expect("deleted liked interest id");
                let published_item = id
                    .as_deref()
                    .and_then(|id| {
                        previous
                            .liked
                            .iter()
                            .find(|record| record.id == id)
                            .map(|record| record.name.clone())
                    })
                    .unwrap_or_else(|| item.clone());
                if let Err(error) = persist_interest_delete_checked(state, existing_id).await {
                    let mut interests = state.interests.write().await;
                    if *interests == mutated {
                        *interests = previous;
                    }
                    drop(interests);
                    return Ok(routing::service_unavailable_response(&error));
                }
                if versioned {
                    if let Err(error) = send_active_interest_command(
                        state,
                        ServerMessage::RemoveThingILike {
                            item: published_item,
                        },
                    )
                    .await
                    {
                        return Ok(routing::service_unavailable_response(&error));
                    }
                    Ok(routing::no_content_response())
                } else {
                    Ok(routing::ok_response("{}".to_string()))
                }
            } else if versioned {
                if let Err(error) =
                    send_active_interest_command(state, ServerMessage::RemoveThingILike { item })
                        .await
                {
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(routing::no_content_response())
            } else {
                Ok(routing::not_found_response())
            }
        }

        // INTERESTS ENDPOINTS (Hated)
        ("GET", "/api/soulseek/hated-interests") => {
            let interests = state.interests.read().await;
            let json = interests.json_hated();
            drop(interests);
            Ok(routing::ok_response(json))
        }
        ("POST", "/api/soulseek/hated-interests") => {
            let versioned = route.path.starts_with("/api/v0/");
            let name = extract_json_string_field(body, "item")
                .or_else(|| extract_json_string_field(body, "name"))
                .map(|name| name.trim().to_owned())
                .unwrap_or_default();
            if name.is_empty() {
                return Ok(routing::bad_request_response("item is required"));
            }
            if versioned && state.session.read().await.state != "connected" {
                return Ok(routing::service_unavailable_response(
                    "Soulseek session is disconnected",
                ));
            }
            let mut interests = state.interests.write().await;
            let previous = interests.clone();
            let Some((record, created)) = interests.add_hated(name) else {
                return Ok(routing::service_unavailable_response(
                    "hated interest capacity is full",
                ));
            };
            let mutated = interests.clone();
            let json = record.json();
            drop(interests);
            if created {
                if let Err(error) = persist_interest_checked(state, &record).await {
                    let mut interests = state.interests.write().await;
                    if *interests == mutated {
                        *interests = previous;
                    }
                    drop(interests);
                    return Ok(routing::service_unavailable_response(&error));
                }
                if versioned {
                    if let Err(error) = send_active_interest_command(
                        state,
                        ServerMessage::AddThingIHate {
                            item: record.name.clone(),
                        },
                    )
                    .await
                    {
                        return Ok(routing::service_unavailable_response(&error));
                    }
                    Ok(routing::no_content_response())
                } else {
                    Ok(routing::created_response(json))
                }
            } else if versioned {
                if let Err(error) = send_active_interest_command(
                    state,
                    ServerMessage::AddThingIHate {
                        item: record.name.clone(),
                    },
                )
                .await
                {
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(routing::no_content_response())
            } else {
                Ok(routing::ok_response(json))
            }
        }
        ("DELETE", path) if path.starts_with("/api/soulseek/hated-interests/") => {
            let versioned = route.path.starts_with("/api/v0/");
            let Some(raw_item) = path_segment_after(path, "/api/soulseek/hated-interests/") else {
                return Ok(routing::not_found_response());
            };
            let item = decoded_path_segment(raw_item).trim().to_owned();
            if item.is_empty() {
                return Ok(routing::bad_request_response("item is required"));
            }
            if versioned && state.session.read().await.state != "connected" {
                return Ok(routing::service_unavailable_response(
                    "Soulseek session is disconnected",
                ));
            }
            let mut interests = state.interests.write().await;
            let previous = interests.clone();
            let id = interests
                .hated
                .iter()
                .find(|record| {
                    record.id.eq_ignore_ascii_case(&item) || record.name.eq_ignore_ascii_case(&item)
                })
                .map(|record| record.id.clone());
            let deleted = id.as_deref().is_some_and(|id| interests.remove_hated(id));
            let mutated = interests.clone();
            drop(interests);
            if deleted {
                let existing_id = id.as_deref().expect("deleted hated interest id");
                let published_item = id
                    .as_deref()
                    .and_then(|id| {
                        previous
                            .hated
                            .iter()
                            .find(|record| record.id == id)
                            .map(|record| record.name.clone())
                    })
                    .unwrap_or_else(|| item.clone());
                if let Err(error) = persist_interest_delete_checked(state, existing_id).await {
                    let mut interests = state.interests.write().await;
                    if *interests == mutated {
                        *interests = previous;
                    }
                    drop(interests);
                    return Ok(routing::service_unavailable_response(&error));
                }
                if versioned {
                    if let Err(error) = send_active_interest_command(
                        state,
                        ServerMessage::RemoveThingIHate {
                            item: published_item,
                        },
                    )
                    .await
                    {
                        return Ok(routing::service_unavailable_response(&error));
                    }
                    Ok(routing::no_content_response())
                } else {
                    Ok(routing::ok_response("{}".to_string()))
                }
            } else if versioned {
                if let Err(error) =
                    send_active_interest_command(state, ServerMessage::RemoveThingIHate { item })
                        .await
                {
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(routing::no_content_response())
            } else {
                Ok(routing::not_found_response())
            }
        }

        // SHARE GRANTS ENDPOINTS
        ("GET", "/api/share-grants") => {
            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            let grants = state.share_grants.read().await;
            let records = grants.records.clone();
            drop(grants);
            let mut visible = Vec::with_capacity(records.len());
            for record in &records {
                if !share_grant_collection_forbids(
                    state,
                    &record.collection_id,
                    caller_id.as_deref(),
                )
                .await
                {
                    visible.push(record.json());
                }
            }
            Ok(routing::ok_response(format!("[{}]", visible.join(","))))
        }
        ("POST", "/api/share-grants") => {
            let collection_id = extract_json_string_field(body, "collection_id")
                .or_else(|| extract_json_string_field(body, "collectionId"))
                .unwrap_or_default();
            let caller_id = utils::authenticated_caller_id(
                &state.config,
                authorization,
                headers.cookie.as_deref(),
                headers.remote_addr,
            );
            if !collection_id.is_empty() {
                let collections = state.collections.read().await;
                let collection_exists = collections.get(&collection_id).is_some();
                drop(collections);
                if !collection_exists {
                    return Ok(routing::not_found_response());
                }
                // Matches the oracle's real Create: a grant may only be
                // created against a collection the caller actually owns.
                if share_grant_collection_forbids(state, &collection_id, caller_id.as_deref()).await
                {
                    return Ok(routing::not_found_response());
                }
            }
            let username = extract_json_string_field(body, "username").unwrap_or_default();
            let Some(username) = normalize_share_grant_username(&username) else {
                return Ok(routing::conflict_response(
                    "collection_id and username are required",
                ));
            };
            if collection_id.is_empty() {
                return Ok(routing::conflict_response(
                    "collection_id and username are required",
                ));
            }
            let compatibility_contract = route.path.starts_with("/api/v0/");
            let id = compatibility_contract.then(|| uuid::Uuid::new_v4().to_string());
            let permissions = share_grant_permissions_from_request(body, compatibility_contract);
            let mut grants = state.share_grants.write().await;
            let previous = grants.clone();
            let Some((record, created)) = grants.create_with_contract_and_permissions(
                id,
                collection_id,
                username,
                &permissions,
            ) else {
                return Ok(routing::service_unavailable_response(
                    "share grant capacity is full",
                ));
            };
            let json = record.json();
            if created {
                if let Err(error) = persist_share_grant(state, &record).await {
                    *grants = previous;
                    return Ok(routing::service_unavailable_response(&error));
                }
                drop(grants);
                Ok(routing::created_response(json))
            } else {
                drop(grants);
                Ok(routing::ok_response(json))
            }
        }
        ("GET", path)
            if path.starts_with("/api/share-grants/")
                && path.ends_with("/manifest")
                && share_grant_manifest_id(path).is_some() =>
        {
            let grant_id =
                share_grant_manifest_id(path).expect("guarded share-grant manifest path");
            if query_parameter(route.query, "token").is_some() {
                return Ok(routing::bad_request_response(
                    "share tokens must be sent in X-Share-Token",
                ));
            }
            let api_authorized = !state.config.auth_required
                || is_authorized(&state.config, authorization, headers.cookie.as_deref());
            let share_token = request_share_token(authorization, &headers);
            if let Some(token) = share_token {
                let mut tokens = state.share_access_tokens.write().await;
                let token_record = tokens.validate(&token);
                drop(tokens);
                if token_record.as_ref().map(|record| record.grant_id.as_str()) != Some(grant_id) {
                    return Ok(routing::unauthorized_response());
                }
            } else if !api_authorized {
                return Ok(routing::unauthorized_response());
            }

            let grants = state.share_grants.read().await;
            let Some(grant) = grants.get(grant_id) else {
                drop(grants);
                return Ok(routing::not_found_response());
            };
            drop(grants);
            let collections = state.collections.read().await;
            let Some(collection) = collections.get(&grant.collection_id) else {
                drop(collections);
                return Ok(routing::not_found_response());
            };
            let items = collection
                .items
                .iter()
                .map(|item| {
                    serde_json::from_str::<serde_json::Value>(&item.json())
                        .unwrap_or_else(|_| serde_json::json!({ "id": item.id }))
                })
                .collect::<Vec<_>>();
            let item_count = items.len();
            let collection_value = serde_json::from_str::<serde_json::Value>(&collection.json())
                .unwrap_or_else(|_| serde_json::json!({ "id": collection.id }));
            drop(collections);
            Ok(routing::ok_response(
                serde_json::json!({
                    "share": serde_json::from_str::<serde_json::Value>(&grant.json())
                        .unwrap_or_else(|_| serde_json::json!({ "id": grant.id })),
                    "collection": collection_value,
                    "items": items,
                    "itemCount": item_count,
                    "permissions": grant.permissions,
                })
                .to_string(),
            ))
        }
        _ => Err(ROUTE_NOT_HANDLED.to_owned()),
    }
}
