use super::*;

pub(crate) fn spawn_gold_star_club(state: Arc<AppState>) {
    if !gold_star_club_available(&state) {
        return;
    }
    let task_state = Arc::clone(&state);
    state.spawn_managed_task(async move {
        let state = task_state;
        loop {
            if !gold_star_club_available(&state) {
                return;
            }
            let (connected, peer_id) = {
                let session = state.session.read().await;
                (
                    session.state == "connected",
                    session
                        .username
                        .clone()
                        .or_else(|| state.config.username.clone())
                        .filter(|value| !value.trim().is_empty()),
                )
            };
            if connected {
                let Some(peer_id) = peer_id else {
                    return;
                };
                let result = {
                    let mut pods = state.pods.write().await;
                    pods.ensure_gold_star_club()
                        .and_then(|_| pods.join(pods::GOLD_STAR_CLUB_POD_ID, peer_id.clone()))
                };
                match result {
                    Ok(Some(true)) => {
                        let member_count = state
                            .pods
                            .read()
                            .await
                            .members(pods::GOLD_STAR_CLUB_POD_ID)
                            .map_or(0, |members| members.len());
                        record_daemon_log(
                            &state,
                            logging::LogLevel::Info,
                            "podcore",
                            format!(
                                "auto-joined {peer_id} to Gold Star Club ({}/{})",
                                member_count,
                                pods::GOLD_STAR_CLUB_MAX_MEMBERS
                            ),
                        )
                        .await;
                    }
                    Ok(Some(false)) => {}
                    Ok(None) => {
                        record_daemon_log(
                            &state,
                            logging::LogLevel::Warn,
                            "podcore",
                            "Gold Star Club pod disappeared before auto-join".to_owned(),
                        )
                        .await;
                    }
                    Err(error) => {
                        record_daemon_log(
                            &state,
                            logging::LogLevel::Warn,
                            "podcore",
                            format!("Gold Star Club auto-join failed: {error}"),
                        )
                        .await;
                    }
                }
                return;
            }
            time::sleep(Duration::from_secs(1)).await;
        }
    });
}

pub(crate) fn spawn_session_manager(
    state: Arc<AppState>,
    mut receiver: mpsc::Receiver<SessionCommand>,
) {
    let task_state = Arc::clone(&state);
    state.spawn_managed_task(async move {
        let state = task_state;
        let mut session = None;
        let mut next_ping = Instant::now() + state.config.ping_interval;
        let wishlist_interval = if state.config.controller_profile
            == ControllerProfile::Native
        {
            state.config.core_workflow.wishlist.interval
        } else {
            Duration::from_secs(30)
        };
        let wishlist_override = (state.config.controller_profile
            == ControllerProfile::Native)
            .then_some(wishlist_interval);
        let mut wishlist_scheduler = WishlistSearchScheduler::new(
            Vec::<String>::new(),
            WishlistSearchSchedulerOptions::new(wishlist_interval, wishlist_override)
                .expect("valid wishlist scheduler options"),
        )
        .expect("valid empty wishlist scheduler");

        // Load persisted wishlist scheduler state
        if let Some(db) = state.db.as_ref() {
            match db
                .load_wishlist_scheduler_state()
                .await
                .map_err(|error| error.to_string())
            {
                Ok(Some((next_index, server_interval))) => {
                    wishlist_scheduler.set_next_index(next_index);
                    wishlist_scheduler.set_server_interval(server_interval);
                }
                Ok(None) => {}
                Err(error) => {
                    record_daemon_log(
                        &state,
                        logging::LogLevel::Warn,
                        "wishlist",
                        format!("failed to load persisted wishlist scheduler state: {error}"),
                    )
                    .await;
                }
            }
        }

        // Load persisted distributed tree state
        if let Some(db) = state.db.as_ref() {
            if let Err(error) = hydrate_distributed_runtime(&state.distributed_network, db).await {
                record_distributed_persistence_failure(
                    &state,
                    "distributed state load failed",
                    error,
                )
                .await;
            }
        }

        let mut next_wishlist_search = Instant::now() + wishlist_scheduler.interval();
        let mut reconnect_requested = false;

        loop {
            while let Ok(command) = receiver.try_recv() {
                handle_session_command(
                    &state,
                    command,
                    &mut session,
                    &mut next_ping,
                    &mut reconnect_requested,
                )
                .await;
            }

            // Commands drained above may have failed while writing to the
            // server.  That path records the reconnect requirement in shared
            // runtime state; copy it before deciding whether this loop may
            // wait forever for another command.
            if state.runtime.read().await.application_reconnect_pending {
                reconnect_requested = true;
            }

            if reconnect_requested && session.is_none() {
                match wait_for_reconnect_or_command(&mut receiver, state.config.reconnect_delay)
                    .await
                {
                    ReconnectWake::DelayElapsed => {
                        let connected = connect_session(&state, &mut session, &mut next_ping).await;
                        reconnect_requested = !connected && state.config.reconnect;
                    }
                    ReconnectWake::Command(command) => {
                        handle_session_command(
                            &state,
                            command,
                            &mut session,
                            &mut next_ping,
                            &mut reconnect_requested,
                        )
                        .await;
                        if state.runtime.read().await.application_reconnect_pending {
                            reconnect_requested = true;
                        }
                    }
                    ReconnectWake::ChannelClosed => break,
                }
                continue;
            }

            if let Some(active_session) = session.as_mut() {
                if matches!(
                    time::timeout(Duration::from_millis(250), active_session.readable()).await,
                    Ok(Ok(()))
                ) {
                    match time::timeout(Duration::from_secs(1), active_session.receive()).await {
                        Ok(Ok(message)) => {
                            if wishlist_scheduler.apply_server_message(&message) {
                                next_wishlist_search =
                                    Instant::now() + wishlist_scheduler.interval();
                                // Persist updated scheduler state
                                if let Some(db) = state.db.as_ref() {
                                    if let Err(error) = db
                                        .save_wishlist_scheduler_state(
                                            wishlist_scheduler.next_index(),
                                            wishlist_scheduler.server_interval_seconds(),
                                        )
                                        .await
                                        .map_err(|error| error.to_string())
                                    {
                                        record_daemon_log(
                                            &state,
                                            logging::LogLevel::Warn,
                                            "wishlist",
                                            format!(
                                                "failed to persist server wishlist scheduler state: {error}"
                                            ),
                                        )
                                        .await;
                                    }
                                }
                            }
                            let relogged = matches!(message, ServerMessage::Relogged);
                            project_server_message(&state, active_session, &message).await;
                            update_session(&state, |snapshot| {
                                snapshot.state = "connected";
                                snapshot.server_messages_seen += 1;
                                snapshot.last_server_message =
                                    Some(server_message_name(&message).to_string());
                            })
                            .await;
                            if relogged {
                                session = None;
                                clear_connected_server_address(&state);
                                reset_distributed_network(&state, None).await;
                                update_session(&state, |snapshot| {
                                    snapshot.state = "disconnected";
                                    snapshot.last_error =
                                        Some("server reported relogged/kicked".to_owned());
                                    snapshot.supporter = None;
                                    snapshot.connected_at = None;
                                })
                                .await;
                                reconnect_requested = false;
                            }
                        }
                        Ok(Err(ClientError::ConnectionClosed)) => {
                            session = None;
                            clear_connected_server_address(&state);
                            reset_distributed_network(&state, None).await;
                            let reconnect = state.config.reconnect;
                            update_session(&state, |snapshot| {
                                snapshot.state = "disconnected";
                                snapshot.last_error = None;
                                snapshot.supporter = None;
                                snapshot.connected_at = None;
                            })
                            .await;
                            record_daemon_log(
                                &state,
                                logging::LogLevel::Info,
                                "session",
                                if reconnect {
                                    "Soulseek server closed the connection; reconnect pending"
                                } else {
                                    "Soulseek server closed the connection"
                                },
                            )
                            .await;
                            reconnect_requested = reconnect;
                        }
                        Ok(Err(error)) => {
                            eprintln!("server receive failed: {error}");
                            session = None;
                            clear_connected_server_address(&state);
                            reset_distributed_network(&state, None).await;
                            update_session(&state, |snapshot| {
                                snapshot.state = "error";
                                snapshot.last_error =
                                    Some(format!("server receive failed: {error}"));
                                snapshot.supporter = None;
                                snapshot.connected_at = None;
                            })
                            .await;
                            reconnect_requested = state.config.reconnect;
                        }
                        Err(_) => {}
                    }
                }

                if session.is_some() && Instant::now() >= next_ping {
                    send_session_ping(&state, &mut session, &mut next_ping).await;
                    reconnect_requested = session.is_none() && state.config.reconnect;
                }

                let wishlist_enabled = state.core_workflow_settings.read().await.wishlist.enabled;
                if wishlist_enabled && session.is_some() && Instant::now() >= next_wishlist_search {
                    send_due_wishlist_search(
                        &state,
                        &mut session,
                        &mut wishlist_scheduler,
                        &mut next_wishlist_search,
                    )
                    .await;
                    reconnect_requested = session.is_none() && state.config.reconnect;
                }
            } else if let Some(command) = receiver.recv().await {
                handle_session_command(
                    &state,
                    command,
                    &mut session,
                    &mut next_ping,
                    &mut reconnect_requested,
                )
                .await;
                if state.runtime.read().await.application_reconnect_pending {
                    reconnect_requested = true;
                }
            } else {
                break;
            }
        }
    });
}
