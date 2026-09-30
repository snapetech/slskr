use super::*;

pub(super) async fn publish_configured_interests(
    state: &AppState,
    session: &mut ServerSession<TcpStream>,
) {
    // Publish all interests from the interest_store (includes both config and API-added)
    let (liked, hated) = {
        let store = state.interests.read().await;
        (
            store
                .liked
                .iter()
                .map(|r| r.name.clone())
                .collect::<Vec<_>>(),
            store
                .hated
                .iter()
                .map(|r| r.name.clone())
                .collect::<Vec<_>>(),
        )
    };

    for (kind, item) in liked
        .into_iter()
        .map(|item| ("liked", item))
        .chain(hated.into_iter().map(|item| ("hated", item)))
    {
        let message = if kind == "liked" {
            ServerMessage::AddThingILike { item: item.clone() }
        } else {
            ServerMessage::AddThingIHate { item: item.clone() }
        };
        if let Err(error) = session.send_server_message(message).await {
            record_daemon_log(
                state,
                logging::LogLevel::Warn,
                "interests",
                format!("failed to publish {kind} interest {item}: {error}"),
            )
            .await;
        }
    }
}

pub(super) async fn replay_watched_users(state: &AppState, session: &mut ServerSession<TcpStream>) {
    let watched: Vec<String> = {
        let users = state.users.read().await;
        users
            .records
            .iter()
            .filter(|record| record.watched)
            .map(|record| record.username.clone())
            .collect()
    };

    if watched.is_empty() {
        return;
    }

    record_daemon_log(
        state,
        logging::LogLevel::Info,
        "users",
        format!("replaying {} watched users after login", watched.len()),
    )
    .await;

    for username in watched {
        let message = ServerMessage::WatchUserRequest {
            username: username.clone(),
        };
        if let Err(error) = session.send_server_message(message).await {
            record_daemon_log(
                state,
                logging::LogLevel::Warn,
                "users",
                format!("failed to replay watched user {}: {}", username, error),
            )
            .await;
        }
    }
}

pub(super) async fn sync_contact_statuses(
    state: &AppState,
    session: &mut ServerSession<TcpStream>,
) {
    let contacts: Vec<String> = {
        let store = state.contacts.read().await;
        store
            .records
            .iter()
            .map(|record| record.username.clone())
            .collect()
    };

    if contacts.is_empty() {
        return;
    }

    record_daemon_log(
        state,
        logging::LogLevel::Info,
        "contacts",
        format!("syncing {} contact statuses after login", contacts.len()),
    )
    .await;

    for username in contacts {
        let message = ServerMessage::GetUserStatusRequest {
            username: username.clone(),
        };
        if let Err(error) = session.send_server_message(message).await {
            record_daemon_log(
                state,
                logging::LogLevel::Warn,
                "contacts",
                format!("failed to sync contact status for {}: {}", username, error),
            )
            .await;
        }
    }
}

pub(super) async fn sync_room_tickers(state: &AppState, session: &mut ServerSession<TcpStream>) {
    let tickers: Vec<(String, String)> = {
        let rooms = state.rooms.read().await;
        rooms
            .records
            .iter()
            .filter_map(|record| {
                record
                    .ticker
                    .as_ref()
                    .map(|ticker| (record.name.clone(), ticker.clone()))
            })
            .collect()
    };

    if tickers.is_empty() {
        return;
    }

    record_daemon_log(
        state,
        logging::LogLevel::Info,
        "rooms",
        format!("syncing {} room tickers after login", tickers.len()),
    )
    .await;

    for (room, ticker) in tickers {
        let message = ServerMessage::SetRoomTicker {
            room: room.clone(),
            ticker: ticker.clone(),
        };
        if let Err(error) = session.send_server_message(message).await {
            record_daemon_log(
                state,
                logging::LogLevel::Warn,
                "rooms",
                format!("failed to sync ticker for room {}: {}", room, error),
            )
            .await;
        }
    }
}

pub(super) async fn check_privileges_after_login(state: &AppState) {
    record_daemon_log(
        state,
        logging::LogLevel::Info,
        "session",
        "checking privileges after login",
    )
    .await;

    if let Err(error) = send_session_command(state, SessionCommand::CheckPrivileges).await {
        record_daemon_log(
            state,
            logging::LogLevel::Warn,
            "session",
            format!("failed to check privileges after login: {}", error),
        )
        .await;
    }
}

pub(crate) async fn dispatch_queued_downloads_after_login(state: &AppState) {
    let exclusions = effective_download_exclusions(state).await;
    cancel_downloads_blocked_by_policy(state, &exclusions).await;
    let queued: Vec<TransferEntry> = {
        let queue = state.transfers.read().await;
        queue
            .entries
            .iter()
            .filter(|entry| entry.direction == 0 && entry.status == "queued")
            .cloned()
            .collect()
    };

    if queued.is_empty() {
        return;
    }

    record_daemon_log(
        state,
        logging::LogLevel::Info,
        "transfers",
        format!("dispatching {} queued downloads after login", queued.len()),
    )
    .await;

    for entry in queued {
        let Some(peer_username) = entry.peer_username.as_deref() else {
            continue;
        };
        let command = SessionCommand::TransferPeer {
            id: entry.id,
            username: peer_username.to_owned(),
        };
        if let Err(error) = send_session_command(state, command).await {
            record_daemon_log(
                state,
                logging::LogLevel::Warn,
                "transfers",
                format!(
                    "failed to dispatch queued download {} for {}: {}",
                    entry.filename, peer_username, error
                ),
            )
            .await;
        }
    }
}

pub(super) async fn replay_joined_rooms(state: &AppState, session: &mut ServerSession<TcpStream>) {
    let joined_rooms = {
        let rooms = state.rooms.read().await;
        rooms
            .records
            .iter()
            .filter(|room| room.joined)
            .map(|room| room.name.clone())
            .collect::<Vec<_>>()
    };

    for room in joined_rooms {
        match session
            .send_server_message(ServerMessage::JoinRoom {
                room: room.clone(),
                private: false,
            })
            .await
        {
            Ok(()) => {
                record_daemon_log(
                    state,
                    logging::LogLevel::Info,
                    "rooms",
                    format!("replayed persisted room join for {room}"),
                )
                .await;
            }
            Err(error) => {
                let reason = format!("replay room join for {room} failed: {error}");
                update_session(state, |snapshot| {
                    snapshot.last_error = Some(reason.clone());
                })
                .await;
                record_daemon_log(state, logging::LogLevel::Warn, "rooms", reason).await;
            }
        }
    }
}
