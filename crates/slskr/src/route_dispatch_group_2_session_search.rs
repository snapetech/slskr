async fn route_dispatch_group_2_session_search(
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
        ("GET", "/api/session") => {
            let snapshot = state.session.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: snapshot.json(),
            })
        }
        ("POST", "/api/session/connect") => {
            if let Err(error) = send_session_command(state, SessionCommand::Connect).await {
                return Ok(routing::service_unavailable_response(&error));
            }
            Ok(routing::accepted_response("{\"accepted\":true}".to_owned()))
        }
        ("POST", "/api/session/ping") => {
            if let Err(error) = send_session_command(state, SessionCommand::Ping).await {
                return Ok(routing::service_unavailable_response(&error));
            }
            Ok(routing::accepted_response("{\"accepted\":true}".to_owned()))
        }
        ("POST", "/api/session/disconnect") => {
            if let Err(error) = send_session_command(state, SessionCommand::Disconnect).await {
                return Ok(routing::service_unavailable_response(&error));
            }
            Ok(routing::accepted_response("{\"accepted\":true}".to_owned()))
        }
        ("POST", "/api/session/privileges/check") => {
            if let Err(error) = send_session_command(state, SessionCommand::CheckPrivileges).await {
                return Ok(routing::service_unavailable_response(&error));
            }
            Ok(routing::accepted_response("{\"accepted\":true}".to_owned()))
        }
        ("GET", "/api/listeners") => {
            let snapshot = state.listeners.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: snapshot.json(),
            })
        }
        ("GET", "/api/users") => {
            let users = state.users.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: users.json(),
            })
        }
        ("GET", "/api/searches/records") => {
            let mut searches = state.searches.write().await;
            let previous_searches = searches.clone();
            let expired = searches.expire_due();
            let body = searches.json(route.query);
            let mutated_searches = searches.clone();
            drop(searches);
            persist_expired_searches_with_rollback(
                state,
                previous_searches,
                &mutated_searches,
                &expired,
            )
            .await?;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body,
            })
        }
        ("GET", "/api/searches") => {
            let mut searches = state.searches.write().await;
            let previous_searches = searches.clone();
            let expired = searches.expire_due();
            let body = searches.controller_list_json(route.query);
            let mutated_searches = searches.clone();
            drop(searches);
            persist_expired_searches_with_rollback(
                state,
                previous_searches,
                &mutated_searches,
                &expired,
            )
            .await?;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body,
            })
        }
        ("GET", _path) if search_token_path(normalized_path, "").is_some() => {
            let Some(token) = search_token_path(normalized_path, "") else {
                return Ok(routing::not_found_response());
            };
            let mut searches = state.searches.write().await;
            let previous_searches = searches.clone();
            let expired = searches.expire_due();
            if let Some(record) = searches.get(token) {
                let body = record.json_with_query(route.query);
                let mutated_searches = searches.clone();
                drop(searches);
                persist_expired_searches_with_rollback(
                    state,
                    previous_searches,
                    &mutated_searches,
                    &expired,
                )
                .await?;
                Ok(HttpResponse {
                    status: "200 OK",
                    content_type: "application/json",
                    body,
                })
            } else {
                let mutated_searches = searches.clone();
                drop(searches);
                persist_expired_searches_with_rollback(
                    state,
                    previous_searches,
                    &mutated_searches,
                    &expired,
                )
                .await?;
                Ok(routing::not_found_response())
            }
        }
        ("GET", "/api/rooms") => {
            let rooms = state.rooms.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: rooms.json(route.query),
            })
        }
        ("GET", "/api/messages") => {
            let messages = state.messages.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: messages.json(route.query),
            })
        }
        ("GET", "/api/transfers") if route.path.starts_with("/api/v0/") => {
            let transfers = state.transfers.read().await;
            let downloads = serde_json::from_str::<Vec<serde_json::Value>>(
                &transfers.controller_transfers_json(0, None),
            )
            .unwrap_or_default();
            let uploads = serde_json::from_str::<Vec<serde_json::Value>>(
                &transfers.controller_transfers_json(1, None),
            )
            .unwrap_or_default();
            Ok(routing::ok_response(
                serde_json::Value::Array(downloads.into_iter().chain(uploads).collect())
                    .to_string(),
            ))
        }
        ("GET", "/api/transfers") => {
            let transfers = state.transfers.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: transfers.json(route.query),
            })
        }
        ("GET", "/api/transfers/stats") => {
            let transfers = state.transfers.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: transfers.stats_json(),
            })
        }
        ("POST", "/api/searches") => {
            if route.path.starts_with("/api/v0/")
                && extract_json_string_field(body, "acquisitionProfile")
                    .is_some_and(|profile| !is_known_acquisition_profile(&profile))
            {
                return Ok(HttpResponse {
                    status: "400 Bad Request",
                    content_type: "application/json",
                    body: serde_json::json!(
                        "The field AcquisitionProfile must be a known acquisition profile"
                    )
                    .to_string(),
                });
            }
            let query = match extract_json_string_field(body, "query")
                .or_else(|| extract_json_string_field(body, "searchText"))
            {
                Some(q) if !q.trim().is_empty() => q.trim().to_owned(),
                Some(_) => {
                    return Ok(routing::bad_request_response(
                        "search query must not be blank",
                    ))
                }
                None => {
                    return Ok(routing::bad_request_response(
                        "query/searchText is required",
                    ))
                }
            };

            let target_str =
                extract_json_string_field(body, "target").unwrap_or_else(|| "global".to_string());
            let external_id = extract_json_string_field(body, "id");
            let username_opt = extract_json_string_field(body, "username");
            let room_opt = extract_json_string_field(body, "room");
            let wishlist_item_id = extract_json_string_field(body, "wishlistItemId")
                .or_else(|| extract_json_string_field(body, "wishlist_item_id"));
            let ttl_seconds = match search_ttl_seconds_from_body(body) {
                Ok(ttl_seconds) => ttl_seconds,
                Err(error) => return Ok(routing::bad_request_response(error)),
            };

            if !matches!(target_str.as_str(), "global" | "user" | "room" | "wishlist") {
                return Ok(routing::bad_request_response("invalid search target"));
            }
            if target_str == "user" && username_opt.is_none() {
                return Ok(routing::bad_request_response(
                    "username is required for user search",
                ));
            }
            if target_str == "room" && room_opt.is_none() {
                return Ok(routing::bad_request_response(
                    "room is required for room target",
                ));
            }

            if route.path.starts_with("/api/v0/") {
                let session_state = state.session.read().await.state;
                if session_state != "connected" {
                    let display_state = match session_state {
                        "connecting" => "Connecting",
                        "disconnecting" => "Disconnecting",
                        _ => "Disconnected",
                    };
                    let failed_target = search_target_static(&target_str);
                    let failed_target_name = if target_str == "user" {
                        username_opt.clone()
                    } else if target_str == "room" {
                        room_opt.clone()
                    } else if target_str == "wishlist" {
                        wishlist_item_id.clone()
                    } else {
                        None
                    };
                    let mut searches = state.searches.write().await;
                    let previous_searches = searches.clone();
                    let outcome = searches.create(
                        external_id,
                        query.clone(),
                        failed_target,
                        failed_target_name,
                        Vec::new(),
                        ttl_seconds,
                    );
                    let Ok(outcome) = outcome else {
                        return Ok(disconnected_search_conflict_response(state, display_state));
                    };
                    let token = outcome.record.token;
                    let evicted = outcome.evicted;
                    let expired = outcome.expired;
                    let Some((failed_record, _)) = searches.set_status_by_token(token, "failed")
                    else {
                        return Ok(disconnected_search_conflict_response(state, display_state));
                    };
                    let mutated_searches = searches.clone();
                    drop(searches);
                    let mut upserts = expired.clone();
                    upserts.push(failed_record.clone());
                    if let Err(error) = persist_search_transition(state, &upserts, &evicted).await {
                        rollback_searches_if_unchanged(state, previous_searches, &mutated_searches)
                            .await;
                        return Ok(routing::service_unavailable_response(&error));
                    }
                    return Ok(disconnected_search_conflict_response(state, display_state));
                }
            }

            let shares = state.shares.read().await;
            let matching_results = search_shares(&shares.entries, &query);
            drop(shares);

            let session_command_permit = match state.session_commands.reserve().await {
                Ok(permit) => permit,
                Err(_) => {
                    return Ok(routing::service_unavailable_response(
                        "session manager is not running",
                    ));
                }
            };

            let mut searches = state.searches.write().await;
            let previous_searches = searches.clone();
            let target_name = if target_str == "user" {
                username_opt.clone()
            } else if target_str == "room" {
                room_opt.clone()
            } else if target_str == "wishlist" {
                wishlist_item_id.clone()
            } else {
                None
            };
            let result_count = matching_results.len();

            let target = search_target_static(&target_str);
            let outcome = match searches.create(
                external_id,
                query.clone(),
                target,
                target_name.clone(),
                matching_results,
                ttl_seconds,
            ) {
                Ok(outcome) => outcome,
                Err(error) => return Ok(search_create_error_response(error)),
            };
            let record = outcome.record;
            let evicted = outcome.evicted;
            let expired = outcome.expired;
            let token = record.token;
            let mutated_searches = searches.clone();

            let dispatch_target = match target_str.as_str() {
                "user" => SearchDispatchTarget::User(username_opt.clone().unwrap_or_default()),
                "room" => SearchDispatchTarget::Room(room_opt.clone().unwrap_or_default()),
                "wishlist" => SearchDispatchTarget::Wishlist,
                _ => SearchDispatchTarget::Global,
            };
            drop(searches);

            let mut upserts = expired.clone();
            upserts.push(record.clone());
            if let Err(error) = persist_search_transition(state, &upserts, &evicted).await {
                rollback_searches_if_unchanged(state, previous_searches, &mutated_searches).await;
                return Ok(routing::service_unavailable_response(&error));
            }
            for expired_record in &expired {
                publish_search_hub_event(state, "update", expired_record);
            }
            session_command_permit.send(SessionCommand::Search {
                token,
                query: query.clone(),
                target: dispatch_target,
            });

            record_event(state, "search.started", format!("{}", token), None).await;

            // Dispatch webhook for search.created event
            let webhook_data = serde_json::json!({
                "token": token,
                "query": query,
                "target": target_str,
                "target_name": target_name,
                "result_count": result_count,
            });
            let correlation_id = format!("search_{}", token);
            dispatch_webhook_event(
                state,
                correlation_id,
                webhooks::WebhookEvent::SearchCreated,
                webhook_data,
            )
            .await;

            // Return simplified response matching C# compatibility contract
            let search_id = record.id.replace("-", "");
            let results_json = record
                .results
                .iter()
                .map(|r| r.json())
                .collect::<Vec<_>>()
                .join(",");
            let response_body = format!(
                r#"{{"searchId":"{}","query":"{}","results":[{}]}}"#,
                json_escape(&search_id),
                json_escape(&record.query),
                results_json
            );
            Ok(routing::ok_response(response_body))
        }

        ("POST", _path) if search_token_path(normalized_path, "/complete").is_some() => {
            let Some(token) = search_token_path(normalized_path, "/complete") else {
                return Ok(routing::not_found_response());
            };

            // A wishlist search that completes with too few usable results gets
            // one of the current smart-fallback queries immediately.  The
            // session loop also handles this at expiry time, but waiting for the
            // five-second TTL here makes manual/API completion observably slower
            // and loses the current upstream behavior.
            let fallback_query = {
                let record = state.searches.read().await.get(token);
                if record.as_ref().is_some_and(|record| {
                    record.status == "active"
                        && search_fallback::is_enabled_for_source(record.target)
                }) {
                    let record = record.expect("active wishlist search record");
                    let wishlist_policy = if let Some(item_id) = record.wishlist_item_id() {
                        state.wishlist.read().await.result_policy_for(item_id)
                    } else {
                        None
                    };
                    let response_limit = wishlist_policy
                        .as_ref()
                        .map(|policy| policy.max_results)
                        .unwrap_or(MAX_SEARCH_RESULTS_PER_SEARCH);
                    let file_count = record
                        .results
                        .len()
                        .saturating_add(record.hidden_locked_count);
                    (search_fallback::needs_fallback(
                        record.raw_response_count,
                        file_count,
                        response_limit,
                        response_limit,
                    ))
                    .then(|| search_fallback::create_queries(&record.query))
                    .and_then(|queries| queries.get(record.fallback_attempts).cloned())
                } else {
                    None
                }
            };

            if let Some(fallback_query) = fallback_query {
                let (previous_record, fallback_record) = {
                    let mut searches = state.searches.write().await;
                    let previous_record = searches.get(token);
                    let fallback_record = searches.reset_for_fallback(token, fallback_query, 5);
                    (previous_record, fallback_record)
                };
                if let Some(fallback_record) = fallback_record {
                    let body_json = fallback_record.json();
                    if let Err(error) = persist_search_record(state, &fallback_record).await {
                        if let Some(previous_record) = previous_record.as_ref() {
                            rollback_search_record_if_unchanged(
                                state,
                                previous_record,
                                &fallback_record,
                            )
                            .await;
                        }
                        return Err(error);
                    }
                    record_event(
                        state,
                        "wishlist.search.fallback_started",
                        fallback_record.token.to_string(),
                        Some(
                            serde_json::json!({
                                "query": fallback_record.query,
                                "attempt": fallback_record.fallback_attempts,
                                "source": "completion",
                            })
                            .to_string(),
                        ),
                    )
                    .await;
                    if let Err(error) = send_session_command(
                        state,
                        SessionCommand::Search {
                            token: fallback_record.token,
                            query: fallback_record.query,
                            target: SearchDispatchTarget::Wishlist,
                        },
                    )
                    .await
                    {
                        update_session(state, |snapshot| {
                            snapshot.last_error = Some(error.clone());
                        })
                        .await;
                    }
                    return Ok(routing::ok_response(body_json));
                }
            }

            let mut searches = state.searches.write().await;
            if let Some((record, transitioned)) = searches.complete(token) {
                let body_json = record.json();

                // Dispatch webhook for search.completed event
                let result_count = record.results.len();
                let webhook_data = serde_json::json!({
                    "token": token,
                    "query": record.query,
                    "result_count": result_count,
                    "target": record.target,
                });
                let correlation_id = format!("search_{}", token);

                drop(searches);
                if transitioned {
                    persist_search_record(state, &record).await?;
                    publish_search_hub_event(state, "update", &record);
                }
                if transitioned && record.wishlist_item_id().is_some() {
                    {
                        let _wishlist_search_persistence =
                            state.wishlist_search_persistence_lock.lock().await;
                        let mut wishlist = state.wishlist.write().await;
                        let previous = wishlist.clone();
                        let item = wishlist.record_completed_search(&record);
                        let mutated = wishlist.clone();
                        drop(wishlist);
                        if let Some(item) = item {
                            if let Err(error) = persist_wishlist_item_checked(state, &item).await {
                                rollback_wishlist_if_unchanged(state, previous, &mutated).await;
                                return Ok(routing::service_unavailable_response(&error));
                            }
                        }
                    }
                    if let Err(error) = auto_download_completed_wishlist(state, &record).await {
                        update_session(state, |snapshot| {
                            snapshot.last_error = Some(error.clone());
                        })
                        .await;
                        record_daemon_log(state, logging::LogLevel::Warn, "wishlist", error).await;
                    }
                }

                if transitioned {
                    dispatch_webhook_event(
                        state,
                        correlation_id,
                        webhooks::WebhookEvent::SearchCompleted,
                        webhook_data,
                    )
                    .await;
                }

                Ok(routing::ok_response(body_json))
            } else {
                drop(searches);
                Ok(routing::not_found_response())
            }
        }

        ("POST", _path)
            if search_token_path(normalized_path, "/cancel").is_some()
                || search_token_path(normalized_path, "/fail").is_some()
                || search_token_path(normalized_path, "/expire").is_some() =>
        {
            let (token, status, event_kind) =
                if let Some(token) = search_token_path(normalized_path, "/cancel") {
                    (token, "cancelled", "search.cancelled")
                } else if let Some(token) = search_token_path(normalized_path, "/fail") {
                    (token, "failed", "search.failed")
                } else if let Some(token) = search_token_path(normalized_path, "/expire") {
                    (token, "expired", "search.expired")
                } else {
                    return Ok(routing::not_found_response());
                };
            let mut searches = state.searches.write().await;
            if let Some((record, transitioned)) = searches.set_status_by_token(token, status) {
                let body_json = record.json();
                drop(searches);
                if transitioned {
                    persist_search_record(state, &record).await?;
                    record_event(state, event_kind, token.to_string(), None).await;
                }
                Ok(routing::ok_response(body_json))
            } else {
                drop(searches);
                Ok(routing::not_found_response())
            }
        }

        ("POST", "/api/searches/prune") => {
            let mut searches = state.searches.write().await;
            let previous_searches = searches.clone();
            let pruned_records = searches.prune_expired();
            let pruned = pruned_records.len();
            let remaining = searches.records.len();
            let mutated_searches = searches.clone();
            drop(searches);
            for record in &pruned_records {
                if let Err(error) = delete_persisted_search(state, record).await {
                    rollback_searches_if_unchanged(state, previous_searches, &mutated_searches)
                        .await;
                    return Err(error);
                }
                publish_search_hub_event(state, "delete", record);
            }
            Ok(routing::ok_response(format!(
                "{{\"pruned\":{},\"remaining\":{}}}",
                pruned, remaining
            )))
        }

        ("POST", "/api/search-responses") => {
            let payload = match serde_json::from_str::<serde_json::Value>(body) {
                Ok(payload) => payload,
                Err(_) => return Ok(routing::bad_request_response("invalid JSON body")),
            };
            if search_response_exceeds_wire_limits(&payload) {
                return Ok(routing::bad_request_response(
                    "search response exceeds result limits",
                ));
            }

            let token = match payload.get("token").and_then(serde_json::Value::as_u64) {
                Some(t) => match u32::try_from(t) {
                    Ok(token) => token,
                    Err(_) => {
                        return Ok(routing::bad_request_response("token exceeds u32 range"));
                    }
                },
                None => return Ok(routing::bad_request_response("token is required")),
            };

            let peer_username = payload
                .get("peer_username")
                .or_else(|| payload.get("username"))
                .and_then(serde_json::Value::as_str);
            let slot_free = payload
                .get("slot_free")
                .or_else(|| payload.get("hasFreeUploadSlot"))
                .and_then(serde_json::Value::as_bool);
            let average_speed = payload
                .get("average_speed")
                .or_else(|| payload.get("uploadSpeed"))
                .and_then(serde_json::Value::as_u64)
                .and_then(|value| u32::try_from(value).ok());
            let queue_length = payload
                .get("queue_length")
                .or_else(|| payload.get("queueLength"))
                .and_then(serde_json::Value::as_u64)
                .and_then(|value| u32::try_from(value).ok());
            let locked = payload
                .get("locked")
                .or_else(|| payload.get("isLocked"))
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            let mut entries = Vec::new();
            if let Some(files) = payload.get("files").and_then(serde_json::Value::as_array) {
                entries.extend(files.iter().filter_map(|file| {
                    SearchResultEntry::from_json_file(
                        file,
                        peer_username,
                        false,
                        slot_free,
                        average_speed,
                        queue_length,
                    )
                }));
            }
            if let Some(files) = payload
                .get("lockedFiles")
                .and_then(serde_json::Value::as_array)
            {
                entries.extend(files.iter().filter_map(|file| {
                    SearchResultEntry::from_json_file(
                        file,
                        peer_username,
                        true,
                        slot_free,
                        average_speed,
                        queue_length,
                    )
                }));
            }

            if entries.is_empty() {
                let entry = SearchResultEntry::from_json_file(
                    &payload,
                    peer_username,
                    locked,
                    slot_free,
                    average_speed,
                    queue_length,
                )
                .unwrap_or_else(|| {
                    bounded_search_result_entry(SearchResultEntry {
                        peer_username: peer_username.map(str::to_owned),
                        filename: String::new(),
                        size: payload
                            .get("size")
                            .and_then(serde_json::Value::as_u64)
                            .unwrap_or(0),
                        bit_rate: payload
                            .get("bitRate")
                            .or_else(|| payload.get("bit_rate"))
                            .and_then(serde_json::Value::as_u64)
                            .and_then(|value| u32::try_from(value).ok()),
                        sample_rate: payload
                            .get("sampleRate")
                            .or_else(|| payload.get("sample_rate"))
                            .and_then(serde_json::Value::as_u64)
                            .and_then(|value| u32::try_from(value).ok()),
                        bit_depth: payload
                            .get("bitDepth")
                            .or_else(|| payload.get("bit_depth"))
                            .and_then(serde_json::Value::as_u64)
                            .and_then(|value| u32::try_from(value).ok()),
                        length_seconds: payload
                            .get("length")
                            .or_else(|| payload.get("lengthSeconds"))
                            .or_else(|| payload.get("length_seconds"))
                            .and_then(serde_json::Value::as_u64)
                            .and_then(|value| u32::try_from(value).ok()),
                        locked,
                        slot_free,
                        average_speed,
                        queue_length,
                        extension: String::new(),
                    })
                });
                entries.push(entry);
            }

            let wishlist_item_id = {
                let searches = state.searches.read().await;
                searches
                    .get(token)
                    .and_then(|record| record.wishlist_item_id().map(str::to_owned))
            };
            let _wishlist_search_persistence = if wishlist_item_id.is_some() {
                Some(state.wishlist_search_persistence_lock.lock().await)
            } else {
                None
            };
            let (wishlist_policy, wishlist_counts) =
                if let Some(item_id) = wishlist_item_id.as_deref() {
                    let wishlist = state.wishlist.read().await;
                    let ignored_results = wishlist.ignored_results_for(item_id);
                    let policy = wishlist.result_policy_for(item_id);
                    let mut filtered_out = 0_usize;
                    let mut ignored = 0_usize;
                    let mut hidden_locked = 0_usize;
                    for entry in &entries {
                        if policy
                            .as_ref()
                            .is_some_and(|policy| !policy.filter.matches_entry(entry))
                        {
                            filtered_out = filtered_out.saturating_add(1);
                        } else if entry.peer_username.as_deref().is_some_and(|username| {
                            ignored_results
                                .iter()
                                .any(|rule| rule.matches(username, &entry.filename))
                        }) {
                            ignored = ignored.saturating_add(1);
                        } else if entry.locked {
                            hidden_locked = hidden_locked.saturating_add(1);
                        }
                    }
                    entries.retain(|entry| {
                        if entry.locked
                            || policy
                                .as_ref()
                                .is_some_and(|policy| !policy.filter.matches_entry(entry))
                        {
                            return false;
                        }
                        let Some(username) = entry.peer_username.as_deref() else {
                            return true;
                        };
                        !ignored_results
                            .iter()
                            .any(|rule| rule.matches(username, &entry.filename))
                    });
                    (policy, Some((filtered_out, ignored, hidden_locked)))
                } else {
                    (None, None)
                };

            let mut searches = state.searches.write().await;
            let aggregate_remaining =
                MAX_TOTAL_SEARCH_RESULTS.saturating_sub(searches.total_results());
            if let Some(record) = searches.records.iter_mut().find(|r| r.token == token) {
                if let Some((filtered_out, ignored, hidden_locked)) = wishlist_counts {
                    record.raw_response_count = record.raw_response_count.saturating_add(1);
                    record.filtered_out_count =
                        record.filtered_out_count.saturating_add(filtered_out);
                    record.ignored_result_count =
                        record.ignored_result_count.saturating_add(ignored);
                    record.hidden_locked_count =
                        record.hidden_locked_count.saturating_add(hidden_locked);
                }
                let appended = record.extend_results_with_limit_bounded(
                    entries,
                    aggregate_remaining,
                    wishlist_policy
                        .as_ref()
                        .map(|policy| policy.max_results)
                        .unwrap_or(MAX_SEARCH_RESULTS_PER_SEARCH),
                );
                record.updated_at = unix_timestamp();
                let response_json = record.json();
                let record = record.clone();
                drop(searches);
                persist_search_result_delta(state, &record, &appended).await?;
                publish_search_hub_event(state, "update", &record);
                Ok(routing::ok_response(response_json))
            } else {
                drop(searches);
                Ok(routing::not_found_response())
            }
        }

        // TRANSFER ENDPOINTS
        ("GET", "/api/downloads") => {
            let requested_state = query_parameter(route.query, "status")
                .or_else(|| query_parameter(route.query, "state"))
                .map(|value| value.to_ascii_lowercase())
                .filter(|value| {
                    matches!(
                        value.as_str(),
                        "active" | "completed" | "failed" | "cancelled"
                    )
                });
            let transfers = state.transfers.read().await;
            let downloads = transfers
                .entries
                .iter()
                .filter(|entry| entry.direction == 0)
                .filter(|entry| {
                    requested_state.as_deref().is_none_or(|requested| {
                        native_download_status(entry.status.as_str()) == requested
                    })
                })
                .map(native_compatibility_download_json)
                .collect::<Vec<_>>();
            Ok(routing::ok_response(
                serde_json::json!({"downloads": downloads}).to_string(),
            ))
        }
        ("GET", "/api/v0/downloads/requests") | ("GET", "/api/downloads/requests") => {
            let requested_state = query_parameter(route.query, "state")
                .map(|value| value.to_ascii_lowercase())
                .filter(|value| {
                    matches!(
                        value.as_str(),
                        "active" | "completed" | "failed" | "cancelled"
                    )
                });
            let transfers = state.transfers.read().await;
            let mut grouped = std::collections::BTreeMap::<&str, Vec<&TransferEntry>>::new();
            for entry in transfers
                .entries
                .iter()
                .filter(|entry| entry.direction == 0)
            {
                if let Some(request_id) = entry.request_id.as_deref() {
                    grouped.entry(request_id).or_default().push(entry);
                }
            }
            let mut requests = grouped
                .into_values()
                .filter(|entries| {
                    requested_state.as_deref().is_none_or(|requested| {
                        download_request_state(entries).eq_ignore_ascii_case(requested)
                    })
                })
                .map(|entries| download_request_projection(&entries, false))
                .collect::<Vec<_>>();
            requests.sort_by_key(|entry| {
                std::cmp::Reverse(entry["request"]["createdAt"].as_u64().unwrap_or(0))
            });
            Ok(routing::ok_response(
                serde_json::Value::Array(requests).to_string(),
            ))
        }

        ("GET", path) if download_request_path(path).is_some() => {
            let Some((request_id, None)) = download_request_path(path) else {
                return Ok(routing::not_found_response());
            };
            let transfers = state.transfers.read().await;
            let attempts = transfers
                .entries
                .iter()
                .filter(|entry| {
                    entry.direction == 0 && entry.request_id.as_deref() == Some(request_id)
                })
                .collect::<Vec<_>>();
            if attempts.is_empty() {
                return Ok(routing::not_found_response());
            }
            Ok(routing::ok_response(
                download_request_projection(&attempts, true).to_string(),
            ))
        }

        ("PATCH", path) if download_request_path(path).is_some() => {
            let Some((request_id, Some("name"))) = download_request_path(path) else {
                return Ok(routing::not_found_response());
            };
            let Some(name) = extract_json_string_field(body, "name")
                .map(|name| name.trim().to_owned())
                .filter(|name| !name.is_empty())
            else {
                return Ok(routing::bad_request_response("name is required"));
            };
            if name.len() > MAX_TRANSFER_REQUEST_NAME_BYTES {
                return Ok(routing::bad_request_response(
                    "name must be 512 bytes or fewer",
                ));
            }
            let mut transfers = state.transfers.write().await;
            let previous = transfers.mutation_snapshot();
            let mut updated = Vec::new();
            for entry in transfers.entries.iter_mut().filter(|entry| {
                entry.direction == 0 && entry.request_id.as_deref() == Some(request_id)
            }) {
                entry.request_name = Some(name.clone());
                entry.updated_at = unix_timestamp();
                entry.updated_at_ms = next_transfer_updated_at_ms(entry.updated_at_ms);
                updated.push(entry.clone());
            }
            if updated.is_empty() {
                return Ok(routing::not_found_response());
            }
            let mutated = transfers.mutation_snapshot();
            transfers.persist_state();
            drop(transfers);
            if let Err(error) = persist_transfer_records(state, &updated).await {
                rollback_transfer_mutation_if_unchanged(state, previous, mutated).await;
                return Ok(routing::service_unavailable_response(&error));
            }
            let attempts = updated.iter().collect::<Vec<_>>();
            Ok(routing::ok_response(
                download_request_projection(&attempts, true).to_string(),
            ))
        }

        _ => Err(ROUTE_NOT_HANDLED.to_owned()),
    }
}
