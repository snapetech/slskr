use super::*;

pub(crate) async fn send_due_wishlist_search(
    state: &Arc<AppState>,
    session: &mut Option<ServerSession<TcpStream>>,
    scheduler: &mut WishlistSearchScheduler,
    next_wishlist_search: &mut Instant,
) {
    let (terms, wishlist_item_ids) = {
        let wishlist = state.wishlist.read().await;
        let terms = wishlist.search_terms();
        let item_ids = terms
            .iter()
            .filter_map(|term| {
                wishlist
                    .item_id_for_search_text(term)
                    .map(|item_id| (term.clone(), item_id))
            })
            .collect::<BTreeMap<_, _>>();
        (terms, item_ids)
    };
    scheduler.replace_terms(terms);
    *next_wishlist_search = Instant::now() + scheduler.interval();

    // Persist updated scheduler state after term replacement
    if let Some(db) = state.db.as_ref() {
        if let Err(error) = db
            .save_wishlist_scheduler_state(
                scheduler.next_index(),
                scheduler.server_interval_seconds(),
            )
            .await
            .map_err(|error| error.to_string())
        {
            record_daemon_log(
                state,
                logging::LogLevel::Warn,
                "wishlist",
                format!("failed to persist wishlist scheduler term state: {error}"),
            )
            .await;
        }
    }

    let (previous_searches, expired_searches) = {
        let mut searches = state.searches.write().await;
        let previous_searches = searches.clone();
        let expired_searches = searches.expire_due();
        (previous_searches, expired_searches)
    };
    let mut fallback_started = false;
    for record in expired_searches {
        let fallback_query = if search_fallback::is_enabled_for_source(record.target) {
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
        };

        if let Some(fallback_query) = fallback_query {
            let (previous_record, fallback_record) = {
                let mut searches = state.searches.write().await;
                let previous_record = previous_searches
                    .records
                    .iter()
                    .find(|previous| previous.token == record.token)
                    .cloned();
                let fallback_record =
                    searches.reset_for_fallback(record.token, fallback_query.clone(), 5);
                (previous_record, fallback_record)
            };
            if let Some(fallback_record) = fallback_record {
                if let Err(error) = persist_search_record(state, &fallback_record).await {
                    if let Some(previous_record) = previous_record.as_ref() {
                        rollback_search_record_if_unchanged(
                            state,
                            previous_record,
                            &fallback_record,
                        )
                        .await;
                    }
                    update_session(state, |snapshot| {
                        snapshot.last_error = Some(error.clone());
                    })
                    .await;
                    continue;
                }
                publish_search_hub_event(state, "update", &fallback_record);
                record_event(
                    state,
                    "wishlist.search.fallback_started",
                    fallback_record.token.to_string(),
                    Some(
                        serde_json::json!({
                            "query": fallback_record.query,
                            "attempt": fallback_record.fallback_attempts,
                        })
                        .to_string(),
                    ),
                )
                .await;
                send_active_server_message(
                    state,
                    session,
                    ServerMessage::WishlistSearch(SearchRequest {
                        token: fallback_record.token,
                        query: fallback_record.query.clone(),
                    }),
                    "wishlist smart fallback search",
                )
                .await;
                fallback_started = true;
                continue;
            }
        }

        if let Err(error) = persist_search_record(state, &record).await {
            if let Some(previous_record) = previous_searches
                .records
                .iter()
                .find(|previous| previous.token == record.token)
            {
                rollback_search_record_if_unchanged(state, previous_record, &record).await;
            }
            update_session(state, |snapshot| {
                snapshot.last_error = Some(error.clone());
            })
            .await;
            continue;
        }
        publish_search_hub_event(state, "update", &record);
        if record.wishlist_item_id().is_none() {
            continue;
        }
        {
            let _wishlist_search_persistence = state.wishlist_search_persistence_lock.lock().await;
            let mut wishlist = state.wishlist.write().await;
            let previous = wishlist.clone();
            let item = wishlist.record_completed_search(&record);
            let mutated = wishlist.clone();
            drop(wishlist);
            if let Some(item) = item {
                if let Err(error) = persist_wishlist_item_checked(state, &item).await {
                    rollback_wishlist_if_unchanged(state, previous, &mutated).await;
                    if let Some(previous_record) = previous_searches
                        .records
                        .iter()
                        .find(|previous| previous.token == record.token)
                    {
                        rollback_search_record_if_unchanged(state, previous_record, &record).await;
                    }
                    drop(_wishlist_search_persistence);
                    update_session(state, |snapshot| {
                        snapshot.last_error = Some(error.clone());
                    })
                    .await;
                    continue;
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

    if fallback_started {
        return;
    }

    let (record, evicted, expired, message, previous_searches, mutated_searches) = {
        let mut searches = state.searches.write().await;
        let previous_searches = searches.clone();
        let token = match searches.allocate_token() {
            Ok(token) => token,
            Err(error) => {
                drop(searches);
                update_session(state, |snapshot| {
                    snapshot.last_error =
                        Some(format!("wishlist search allocation failed: {error:?}"));
                })
                .await;
                return;
            }
        };
        let Some(ServerMessage::WishlistSearch(SearchRequest { token, query })) =
            scheduler.next_search_message(token)
        else {
            return;
        };
        let wishlist_item_id = wishlist_item_ids.get(&query).cloned();
        let outcome = match searches.create_scheduled_wishlist_for_item(
            query.clone(),
            wishlist_item_id.clone(),
            DEFAULT_WISHLIST_SEARCH_TTL_SECONDS,
        ) {
            Ok(outcome) => outcome,
            Err(error) => {
                drop(searches);
                update_session(state, |snapshot| {
                    snapshot.last_error =
                        Some(format!("wishlist search capacity failed: {error:?}"));
                })
                .await;
                return;
            }
        };
        debug_assert_eq!(outcome.record.token, token);
        let mutated_searches = searches.clone();
        (
            outcome.record,
            outcome.evicted,
            outcome.expired,
            ServerMessage::WishlistSearch(SearchRequest { token, query }),
            previous_searches,
            mutated_searches,
        )
    };

    let mut upserts = expired.clone();
    upserts.push(record.clone());
    if let Err(error) = persist_search_transition(state, &upserts, &evicted).await {
        rollback_searches_if_unchanged(state, previous_searches, &mutated_searches).await;
        update_session(state, |snapshot| {
            snapshot.last_error = Some(error.clone());
        })
        .await;
        return;
    }
    for expired_record in &expired {
        publish_search_hub_event(state, "update", expired_record);
    }

    record_event(
        state,
        "wishlist.search.started",
        record.token.to_string(),
        None,
    )
    .await;

    let Some(active_session) = session.as_mut() else {
        return;
    };
    match active_session.send_server_message(message).await {
        Ok(()) => {
            spawn_wishlist_smart_fallback(Arc::clone(state), record.token);
            update_session(state, |snapshot| {
                snapshot.last_error = None;
            })
            .await;
        }
        Err(error) => {
            *session = None;
            clear_connected_server_address(state);
            reset_distributed_network(state, None).await;
            update_session(state, |snapshot| {
                snapshot.state = "error";
                snapshot.last_error = Some(format!("wishlist search failed: {error}"));
                snapshot.supporter = None;
                snapshot.connected_at = None;
            })
            .await;
        }
    }
}

pub(super) fn spawn_wishlist_smart_fallback(state: Arc<AppState>, token: u32) {
    let task_state = Arc::clone(&state);
    state.spawn_managed_task(async move {
        let state = task_state;
        // Current upstream gives the initial Soulseek query a short response
        // window before trying one bounded query with suppressed terms removed.
        // Keep this independent of the server-advertised wishlist interval so
        // a quiet search cannot wait several minutes for its first fallback.
        time::sleep(Duration::from_secs(5)).await;
        let Some(record) = state.searches.read().await.get(token) else {
            return;
        };
        if record.status != "active" || !search_fallback::is_enabled_for_source(record.target) {
            return;
        }
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
        let Some(fallback_query) = search_fallback::needs_fallback(
            record.raw_response_count,
            file_count,
            response_limit,
            response_limit,
        )
        .then(|| search_fallback::create_queries(&record.query))
        .and_then(|queries| queries.get(record.fallback_attempts).cloned()) else {
            return;
        };
        drop(record);

        let (previous_record, fallback_record) = {
            let mut searches = state.searches.write().await;
            let previous_record = searches.get(token);
            let fallback_record = searches.reset_for_fallback(token, fallback_query.clone(), 5);
            (previous_record, fallback_record)
        };
        let Some(fallback_record) = fallback_record else {
            return;
        };
        if let Err(error) = persist_search_record(&state, &fallback_record).await {
            if let Some(previous_record) = previous_record.as_ref() {
                rollback_search_record_if_unchanged(&state, previous_record, &fallback_record)
                    .await;
            }
            update_session(&state, |snapshot| {
                snapshot.last_error = Some(error);
            })
            .await;
            return;
        }
        record_event(
            &state,
            "wishlist.search.fallback_started",
            fallback_record.token.to_string(),
            Some(
                serde_json::json!({
                    "query": fallback_record.query,
                    "attempt": fallback_record.fallback_attempts,
                    "source": "initial_search_timeout",
                })
                .to_string(),
            ),
        )
        .await;
        let Ok(permit) = state.session_commands.reserve().await else {
            return;
        };
        permit.send(SessionCommand::Search {
            token: fallback_record.token,
            query: fallback_record.query,
            target: SearchDispatchTarget::Wishlist,
        });
    });
}

pub(crate) fn search_dispatch_message(
    token: u32,
    query: String,
    target: SearchDispatchTarget,
) -> ServerMessage {
    match target {
        SearchDispatchTarget::Global => {
            ServerMessage::FileSearchRequest(SearchRequest { token, query })
        }
        SearchDispatchTarget::Wishlist => {
            ServerMessage::WishlistSearch(SearchRequest { token, query })
        }
        SearchDispatchTarget::User(username) => ServerMessage::UserSearch(TargetedSearchRequest {
            target: username,
            token,
            query,
        }),
        SearchDispatchTarget::Room(room) => ServerMessage::RoomSearch(TargetedSearchRequest {
            target: room,
            token,
            query,
        }),
    }
}
