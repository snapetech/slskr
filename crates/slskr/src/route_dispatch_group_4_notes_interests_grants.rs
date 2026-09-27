async fn route_dispatch_group_4_notes_interests_grants(
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
            let _user_note_persistence = state.user_note_persistence_lock.lock().await;
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
            let _user_note_persistence = state.user_note_persistence_lock.lock().await;
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
            let _user_note_persistence = state.user_note_persistence_lock.lock().await;
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
            let _interest_persistence = state.interest_persistence_lock.lock().await;
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
                drop(_interest_persistence);
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
                drop(_interest_persistence);
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
            let _interest_persistence = state.interest_persistence_lock.lock().await;
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
                drop(_interest_persistence);
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
                drop(_interest_persistence);
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
            let _interest_persistence = state.interest_persistence_lock.lock().await;
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
                drop(_interest_persistence);
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
                drop(_interest_persistence);
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
            let _interest_persistence = state.interest_persistence_lock.lock().await;
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
                drop(_interest_persistence);
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
                drop(_interest_persistence);
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
            let _collection_grant_persistence =
                state.collection_grant_persistence_lock.lock().await;
            let mut grants = state.share_grants.write().await;
            // The initial owner check above keeps unauthorized requests away
            // from the mutation path. Recheck under the canonical
            // grant-then-collection lock order so a collection deleted while
            // this request waited for the grant store cannot acquire a new
            // orphan grant.
            let collection_allows_grant = {
                let collections = state.collections.read().await;
                collections.get(&collection_id).is_some_and(|collection| {
                    !collection_owner_forbids(caller_id.as_deref(), &collection.owner_user_id)
                })
            };
            if !collection_allows_grant {
                drop(grants);
                return Ok(routing::not_found_response());
            }
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
            let mutated = grants.clone();
            drop(grants);
            if created {
                if let Err(error) = persist_share_grant(state, &record).await {
                    let mut grants = state.share_grants.write().await;
                    if share_grant_store_matches(&grants, &mutated) {
                        *grants = previous;
                    }
                    drop(grants);
                    return Ok(routing::service_unavailable_response(&error));
                }
                Ok(routing::created_response(json))
            } else {
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
