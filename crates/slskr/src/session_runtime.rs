use super::*;

async fn publish_configured_interests(state: &AppState, session: &mut ServerSession<TcpStream>) {
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

async fn replay_watched_users(state: &AppState, session: &mut ServerSession<TcpStream>) {
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

async fn sync_contact_statuses(state: &AppState, session: &mut ServerSession<TcpStream>) {
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

async fn sync_room_tickers(state: &AppState, session: &mut ServerSession<TcpStream>) {
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

async fn check_privileges_after_login(state: &AppState) {
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

pub(super) async fn dispatch_queued_downloads_after_login(state: &AppState) {
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

pub(super) async fn send_room_join_if_connected(state: &AppState, room_name: String) {
    let connected = {
        let session = state.session.read().await;
        session.state == "connected"
    };

    if connected {
        if let Err(error) =
            send_session_command(state, SessionCommand::JoinRoom(room_name.clone())).await
        {
            record_room_dispatch_failure(state, "join", &room_name, &error).await;
        }
    } else {
        record_daemon_log(
            state,
            logging::LogLevel::Info,
            "rooms",
            format!("recorded room join for {room_name}; Soulseek join will run after connect"),
        )
        .await;
    }
}

pub(super) async fn send_room_leave_if_connected(state: &AppState, room_name: String) {
    let connected = {
        let session = state.session.read().await;
        session.state == "connected"
    };

    if connected {
        if let Err(error) =
            send_session_command(state, SessionCommand::LeaveRoom(room_name.clone())).await
        {
            record_room_dispatch_failure(state, "leave", &room_name, &error).await;
        }
    } else {
        record_daemon_log(
            state,
            logging::LogLevel::Info,
            "rooms",
            format!("recorded room leave for {room_name}; no Soulseek session is connected"),
        )
        .await;
    }
}

pub(super) async fn record_room_dispatch_failure(
    state: &AppState,
    action: &str,
    room_name: &str,
    error: &str,
) {
    let reason = format!("room {action} for {room_name} dispatch failed: {error}");
    update_session(state, |snapshot| {
        snapshot.last_error = Some(reason.clone());
    })
    .await;
    record_daemon_log(state, logging::LogLevel::Error, "rooms", reason).await;
}

pub(super) async fn persist_room_join_checked(
    state: &AppState,
    room: &str,
) -> Result<bool, String> {
    let Some(db) = state.db.as_ref() else {
        return Ok(false);
    };
    db.subscribe_room(room, None)
        .await
        .map_err(|error| format!("room subscription persistence failed: {error}"))?;
    Ok(true)
}

pub(super) async fn persist_room_leave_checked(
    state: &AppState,
    room: &str,
) -> Result<bool, String> {
    let Some(db) = state.db.as_ref() else {
        return Ok(false);
    };
    db.unsubscribe_room(room)
        .await
        .map_err(|error| format!("room unsubscription persistence failed: {error}"))?;
    Ok(true)
}

pub(super) async fn record_pod_room_mirror_failure(state: &AppState, room_name: &str, error: &str) {
    let reason = format!("pod room mirror for {room_name} dispatch failed: {error}");
    update_session(state, |snapshot| {
        snapshot.last_error = Some(reason.clone());
    })
    .await;
    record_daemon_log(state, logging::LogLevel::Error, "podcore", reason).await;
}

async fn replay_joined_rooms(state: &AppState, session: &mut ServerSession<TcpStream>) {
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

async fn send_session_ping(
    state: &AppState,
    session: &mut Option<ServerSession<TcpStream>>,
    next_ping: &mut Instant,
) {
    let Some(active_session) = session.as_mut() else {
        return;
    };

    match active_session.send_ping().await {
        Ok(()) => {
            *next_ping = Instant::now() + state.config.ping_interval;
            update_session(state, |snapshot| {
                snapshot.state = "connected";
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
                snapshot.last_error = Some(format!("ping failed: {error}"));
                snapshot.supporter = None;
                snapshot.connected_at = None;
            })
            .await;
        }
    }
}

async fn send_active_server_message(
    state: &AppState,
    session: &mut Option<ServerSession<TcpStream>>,
    message: ServerMessage,
    action: &str,
) {
    let Some(active_session) = session.as_mut() else {
        eprintln!("cannot {action}: server session is disconnected");
        update_session(state, |snapshot| {
            snapshot.state = "disconnected";
            snapshot.last_error = Some(format!("cannot {action} while disconnected"));
        })
        .await;
        return;
    };

    match active_session.send_server_message(message).await {
        Ok(()) => {
            eprintln!("sent server message for {action}");
            update_session(state, |snapshot| {
                snapshot.last_error = None;
            })
            .await;
        }
        Err(error) => {
            eprintln!("failed to send server message for {action}: {error}");
            *session = None;
            clear_connected_server_address(state);
            reset_distributed_network(state, None).await;
            update_session(state, |snapshot| {
                snapshot.state = "error";
                snapshot.last_error = Some(format!("{action} failed: {error}"));
                snapshot.supporter = None;
                snapshot.connected_at = None;
            })
            .await;
            if state.config.reconnect {
                state.runtime.write().await.set_reconnect_pending(true);
                update_session(state, |snapshot| {
                    snapshot.last_error = Some(format!("{action} failed; reconnect pending"));
                })
                .await;
            }
        }
    }
}

pub(super) fn is_remote_queue_response(reason: &str) -> bool {
    reason
        .trim()
        .trim_end_matches('.')
        .eq_ignore_ascii_case("queued")
}

async fn handle_connect_to_peer_request(
    state: &AppState,
    session: &mut ServerSession<TcpStream>,
    request: &ConnectToPeerRequest,
) -> Result<(), String> {
    let kind = ConnectionKind::try_from_connection_type(&request.connection_type)
        .map_err(|error| format!("unsupported connect-to-peer kind: {error}"))?;
    let Some(address) = test_user_endpoint_peer_address(state, &request.username) else {
        session
            .send_server_message(ServerMessage::CantConnectToPeerRequest {
                token: request.token,
                username: request.username.clone(),
            })
            .await
            .map_err(|error| format!("cant-connect response failed: {error}"))?;
        return Err("no endpoint available for incoming connect-to-peer request".to_owned());
    };

    let stream = connect_pierce_firewall(state, &address, request.token).await?;
    match kind {
        ConnectionKind::PeerMessages => {
            handle_plain_peer_messages(
                state,
                PeerMessageConnection::new(stream),
                Some(request.username.clone()),
            )
            .await
        }
        ConnectionKind::FileTransfer => {
            handle_inbound_file_transfer(
                state,
                slskr_client::file_transfer::FileTransferConnection::new(stream),
                Some(request.token),
            )
            .await
        }
        ConnectionKind::Distributed => Ok(()),
    }
}

async fn connect_pierce_firewall(
    state: &AppState,
    address: &PeerAddress,
    token: u32,
) -> Result<TcpStream, String> {
    let peer_ip = peer_connect_ip(state, address);
    let port = u16::try_from(address.port).map_err(|_| "peer port is out of range".to_owned())?;
    if port == 0 {
        return Err("peer did not advertise a pierce-firewall port".to_owned());
    }
    let stream = connect_soulseek_tcp(
        state,
        SocketAddr::V4(SocketAddrV4::new(peer_ip, port)),
        SoulseekSocketClass::Control,
    )
    .await
    .map_err(|error| format!("pierce-firewall connect failed: {error}"))?;
    time::timeout(
        state.config.soulseek_connection.timeout_inactivity,
        send_pierce_firewall(stream, token),
    )
    .await
    .map_err(|_| "pierce-firewall init timed out".to_owned())?
    .map_err(|error| format!("pierce-firewall init failed: {error}"))
}

pub(super) async fn send_session_command(
    state: &AppState,
    command: SessionCommand,
) -> Result<(), String> {
    state
        .session_commands
        .send(command)
        .await
        .map_err(|_| "session manager is not running".to_owned())
}

pub(super) async fn send_active_interest_command(
    state: &AppState,
    message: ServerMessage,
) -> Result<(), String> {
    if state.session.read().await.state != "connected" {
        return Err("Soulseek session is disconnected".to_owned());
    }
    send_session_command(state, SessionCommand::SendServerMessage(message)).await
}

pub(super) fn try_send_session_command(
    state: &AppState,
    command: SessionCommand,
) -> Result<(), String> {
    state
        .session_commands
        .try_send(command)
        .map_err(|error| format!("session command queue rejected request: {error}"))
}

pub(super) async fn update_session<F>(state: &AppState, update: F)
where
    F: FnOnce(&mut SessionSnapshot),
{
    let mut snapshot = state.session.write().await;
    let previous_error = snapshot.last_error.clone();
    update(&mut snapshot);
    if snapshot.last_error != previous_error {
        if let Some(error) = snapshot.last_error.as_deref() {
            eprintln!("[Error] session state: {error}");
        }
    }
    snapshot.updated_at = unix_timestamp();
}

pub(super) async fn update_listeners<F>(state: &AppState, update: F)
where
    F: FnOnce(&mut ListenerSnapshot),
{
    let mut snapshot = state.listeners.write().await;
    update(&mut snapshot);
    snapshot.updated_at = unix_timestamp();
}

pub(super) async fn handle_session_command(
    state: &Arc<AppState>,
    command: SessionCommand,
    session: &mut Option<ServerSession<TcpStream>>,
    next_ping: &mut Instant,
    reconnect_requested: &mut bool,
) {
    match command {
        SessionCommand::Connect => {
            *reconnect_requested = false;
            if session.is_none() {
                let _connected = connect_session(state, session, next_ping).await;
            } else {
                update_session(state, |snapshot| {
                    snapshot.state = "connected";
                    snapshot.last_error = None;
                })
                .await;
            }
        }
        SessionCommand::Disconnect => {
            *session = None;
            *reconnect_requested = false;
            clear_connected_server_address(state);
            reset_distributed_network(state, None).await;
            let mut runtime = state.runtime.write().await;
            runtime.set_reconnect_pending(false);
            runtime.vpn_reconnect_requested = false;
            drop(runtime);
            update_session(state, |snapshot| {
                snapshot.state = "disconnected";
                snapshot.supporter = None;
                snapshot.last_error = None;
                snapshot.connected_at = None;
            })
            .await;
        }
        SessionCommand::VpnDisconnect => {
            *session = None;
            *reconnect_requested = false;
            clear_connected_server_address(state);
            reset_distributed_network(state, None).await;
            state.runtime.write().await.set_reconnect_pending(false);
            update_session(state, |snapshot| {
                snapshot.state = "disconnected";
                snapshot.supporter = None;
                snapshot.last_error = None;
                snapshot.connected_at = None;
            })
            .await;
        }
        SessionCommand::Ping => {
            if session.is_some() {
                send_session_ping(state, session, next_ping).await;
            } else {
                update_session(state, |snapshot| {
                    snapshot.state = "disconnected";
                    snapshot.last_error = Some("cannot ping while disconnected".to_owned());
                })
                .await;
            }
        }
        SessionCommand::CheckPrivileges => {
            send_active_server_message(
                state,
                session,
                ServerMessage::CheckPrivilegesRequest,
                "check privileges",
            )
            .await;
        }
        SessionCommand::SetWaitPort {
            port,
            obfuscated_port,
        } => {
            let Some(active_session) = session.as_mut() else {
                return;
            };
            let result = if let Some(obfuscated_port) = obfuscated_port {
                active_session
                    .set_wait_port_obfuscated(port, ROTATED_OBFUSCATION_TYPE, obfuscated_port)
                    .await
            } else {
                active_session.set_wait_port(port).await
            };
            if let Err(error) = result {
                let reason = format!("set wait port failed: {error}");
                update_session(state, |snapshot| {
                    snapshot.state = "error";
                    snapshot.last_error = Some(reason.clone());
                })
                .await;
                record_daemon_log(state, logging::LogLevel::Error, "session", reason).await;
            }
        }
        SessionCommand::DistributedBranch {
            has_parent,
            accept_children,
            level,
            root,
        } => {
            let Some(active_session) = session.as_mut() else {
                return;
            };
            for message in [
                ServerMessage::HaveNoParent {
                    no_parent: !has_parent,
                },
                ServerMessage::AcceptChildren {
                    accept: accept_children,
                },
                ServerMessage::BranchLevel { level },
                ServerMessage::BranchRoot { username: root },
            ] {
                if let Err(error) = active_session.send_server_message(message).await {
                    update_session(state, |snapshot| {
                        snapshot.last_error =
                            Some(format!("distributed branch update failed: {error}"));
                    })
                    .await;
                    break;
                }
            }
        }
        SessionCommand::Search {
            token,
            query,
            target,
        } => {
            let is_active = state
                .searches
                .read()
                .await
                .get(token)
                .is_some_and(|record| record.status == "active");
            if !is_active {
                eprintln!("skipping search {token}: search is no longer active");
                return;
            }
            send_active_server_message(
                state,
                session,
                search_dispatch_message(token, query, target),
                "dispatch search",
            )
            .await;
        }
        SessionCommand::WatchUser(username) => {
            send_active_server_message(
                state,
                session,
                ServerMessage::WatchUserRequest { username },
                "watch user",
            )
            .await;
        }
        SessionCommand::UnwatchUser(username) => {
            send_active_server_message(
                state,
                session,
                ServerMessage::UnwatchUser { username },
                "unwatch user",
            )
            .await;
        }
        SessionCommand::BrowseUser(username) => {
            send_active_server_message(
                state,
                session,
                ServerMessage::GetPeerAddressRequest { username },
                "request peer address for browse",
            )
            .await;
        }
        SessionCommand::BrowseFolder { username, .. } => {
            send_active_server_message(
                state,
                session,
                ServerMessage::GetPeerAddressRequest { username },
                "request peer address for folder browse",
            )
            .await;
        }
        SessionCommand::IndirectBrowse { username, token } => {
            send_active_server_message(
                state,
                session,
                ServerMessage::ConnectToPeerRequest(ConnectToPeerRequest {
                    token,
                    username,
                    connection_type: ConnectionKind::PeerMessages.as_str().to_owned(),
                }),
                "request indirect browse",
            )
            .await;
        }
        SessionCommand::RequestUserStats(username) => {
            send_active_server_message(
                state,
                session,
                ServerMessage::GetUserStatsRequest { username },
                "request user stats",
            )
            .await;
        }
        SessionCommand::RequestUserInterests(username) => {
            let result = match session.as_mut() {
                Some(active_session) => active_session
                    .send_server_message(ServerMessage::GetUserInterestsRequest {
                        username: username.clone(),
                    })
                    .await
                    .map(|_| ())
                    .map_err(|error| format!("request user interests failed: {error}")),
                None => Err("server session is disconnected".to_owned()),
            };
            if let Err(error) = result {
                if let Some(waiters) = state
                    .pending_user_interests
                    .write()
                    .await
                    .remove(&username.to_ascii_lowercase())
                {
                    for waiter in waiters {
                        let _ = waiter.send(Err(error.clone()));
                    }
                }
            }
        }
        SessionCommand::SendServerMessage(message) => {
            send_active_server_message(state, session, message, "send server interest").await;
        }
        SessionCommand::TransferPeer { id, username } => {
            send_active_server_message(
                state,
                session,
                ServerMessage::GetPeerAddressRequest { username },
                &format!("request peer address for transfer {id}"),
            )
            .await;
        }
        SessionCommand::RequestPeerEndpoint(username) => {
            send_active_server_message(
                state,
                session,
                ServerMessage::GetPeerAddressRequest { username },
                "request peer endpoint for preview stream",
            )
            .await;
        }
        SessionCommand::ProbePeerCapability(username) => {
            let task_state = Arc::clone(state);
            tokio::spawn(async move {
                if let Err(error) = probe_peer_capability(&task_state, &username).await {
                    eprintln!(
                        "peer capability probe failed for {}: {error}",
                        redact_username(&username)
                    );
                    update_listeners(&task_state, |snapshot| {
                        snapshot.last_error = Some(format!(
                            "peer capability probe failed for {}: {error}",
                            redact_username(&username)
                        ));
                    })
                    .await;
                }
            });
        }
        SessionCommand::IndirectTransfer {
            id,
            username,
            token,
        } => {
            send_active_server_message(
                state,
                session,
                ServerMessage::ConnectToPeerRequest(ConnectToPeerRequest {
                    token,
                    username,
                    connection_type: ConnectionKind::FileTransfer.as_str().to_owned(),
                }),
                &format!("request indirect file transfer {id}"),
            )
            .await;
        }
        SessionCommand::MessageUser { username, body } => {
            send_active_server_message(
                state,
                session,
                ServerMessage::MessageUserRequest {
                    username,
                    message: body,
                },
                "message user",
            )
            .await;
        }
        SessionCommand::MessageUsers { usernames, body } => {
            send_active_server_message(
                state,
                session,
                ServerMessage::MessageUsers {
                    usernames,
                    message: body,
                },
                "message users",
            )
            .await;
        }
        SessionCommand::MessageAcked { id } => {
            send_active_server_message(
                state,
                session,
                ServerMessage::MessageAcked { id },
                "ack message",
            )
            .await;
        }
        SessionCommand::RefreshRooms => {
            send_active_server_message(
                state,
                session,
                ServerMessage::RoomListRequest,
                "refresh rooms",
            )
            .await;
        }
        SessionCommand::JoinRoom(room) => {
            send_active_server_message(
                state,
                session,
                ServerMessage::JoinRoom {
                    room,
                    private: false,
                },
                "join room",
            )
            .await;
        }
        SessionCommand::LeaveRoom(room) => {
            send_active_server_message(
                state,
                session,
                ServerMessage::LeaveRoom { room },
                "leave room",
            )
            .await;
        }
        SessionCommand::SayRoom { room, body } => {
            send_active_server_message(
                state,
                session,
                ServerMessage::SayChatroomRequest {
                    room,
                    message: body,
                },
                "say room",
            )
            .await;
        }
        SessionCommand::SetRoomTicker { room, ticker } => {
            send_active_server_message(
                state,
                session,
                ServerMessage::SetRoomTicker { room, ticker },
                "set room ticker",
            )
            .await;
        }
        SessionCommand::AddRoomMember { room, username } => {
            send_active_server_message(
                state,
                session,
                ServerMessage::PrivateRoomAddUser { room, username },
                "add private room member",
            )
            .await;
        }
    }
}
pub(super) async fn handle_peer_message<F, Fut>(
    state: &AppState,
    message: PeerMessage,
    peer_username: Option<&str>,
    send_response: F,
) -> Result<(), String>
where
    F: FnOnce(PeerMessage) -> Fut,
    Fut: std::future::Future<Output = Result<(), String>>,
{
    let capability = match decode_peer_capability_message(&message) {
        Ok(capability) => capability,
        Err(error) => {
            if let Some(username) = peer_username {
                record_peer_security_violation(state, username).await;
            }
            return Err(format!("peer capability message rejected: {error}"));
        }
    };
    if let Some(mut envelope) = capability {
        if !state
            .advanced_networking
            .read()
            .await
            .mesh
            .enable_soulseek_capability_handshake
        {
            return Err("peer capability handshake is disabled by configuration".to_owned());
        }
        let peer_username = peer_username
            .map(str::trim)
            .filter(|username| !username.is_empty())
            .ok_or_else(|| {
                "peer capability message rejected: authenticated peer username is unavailable"
                    .to_owned()
            })?;
        envelope.descriptor.username = peer_username.to_owned();
        let message_type = envelope.message_type;
        let nonce = envelope.nonce;
        let username = envelope.descriptor.username.clone();
        let projection = {
            let mut mesh = state.mesh.write().await;
            if let Err(error) = mesh.update_capability(envelope.descriptor) {
                Err(error)
            } else {
                Ok(mesh.persisted_capability_projection())
            }
        };
        let projection = match projection {
            Ok(projection) => projection,
            Err(error) => {
                record_peer_security_violation(state, peer_username).await;
                return Err(error);
            }
        };
        let feature_result = state
            .controller_features
            .upsert(
                "hashdb/peers".to_owned(),
                serde_json::json!({"peers": projection}),
            )
            .await;
        if let Err(error) = feature_result {
            record_daemon_log(
                state,
                logging::LogLevel::Error,
                "distributed",
                format!("peer capability projection persistence failed: {error}"),
            )
            .await;
        }
        if message_type == PeerCapabilityMessageType::Hello {
            let acknowledgement = PeerCapabilityEnvelope::new(
                PeerCapabilityMessageType::Acknowledge,
                nonce,
                local_capability_descriptor(state).await?,
            );
            let response = peer_capability_message(&acknowledgement)
                .map_err(|error| format!("peer capability acknowledgement failed: {error}"))?;
            send_response(response).await?;
        }
        update_listeners(state, |snapshot| {
            snapshot.last_event = Some(format!(
                "peer_capability_{}:{}",
                if message_type == PeerCapabilityMessageType::Hello {
                    "hello"
                } else {
                    "acknowledge"
                },
                redact_username(&username)
            ));
            snapshot.last_error = None;
        })
        .await;
        return Ok(());
    }
    match message {
        PeerMessage::UserInfoRequest => {
            update_listeners(state, |snapshot| {
                snapshot.user_info_requests += 1;
                snapshot.last_event = Some("user_info_request".to_owned());
            })
            .await;
            let (upload_slots, queue_position) =
                upload_queue_forecast(state, peer_username.unwrap_or_default()).await;
            send_response(PeerMessage::UserInfoResponse(UserInfo {
                description: effective_user_info_description(state).await,
                picture: effective_user_info_picture(state).await,
                total_uploads: upload_slots,
                queue_size: queue_position,
                slots_free: queue_position == 0,
                upload_permissions: None,
            }))
            .await?;
            update_listeners(state, |snapshot| {
                snapshot.user_info_responses += 1;
                snapshot.last_event = Some("user_info_response".to_owned());
                snapshot.last_error = None;
            })
            .await;
        }
        PeerMessage::GetShareFileList => {
            update_listeners(state, |snapshot| {
                snapshot.share_list_requests += 1;
                snapshot.last_event = Some("share_list_request".to_owned());
            })
            .await;
            let entries = {
                let shares = state.shares.read().await;
                shares.entries.clone()
            };
            send_response(PeerMessage::SharedFileListResponse(
                build_shared_file_list_payload(&entries)?,
            ))
            .await?;
            update_listeners(state, |snapshot| {
                snapshot.share_list_responses += 1;
                snapshot.last_event = Some("share_list_response".to_owned());
                snapshot.last_error = None;
            })
            .await;
        }
        PeerMessage::FileSearchRequest { token, query } => {
            update_listeners(state, |snapshot| {
                snapshot.file_search_requests += 1;
                snapshot.last_event = Some("file_search_request".to_owned());
            })
            .await;
            if let Some(mut response) = build_file_search_response(state, token, &query).await {
                if let Some(username) = peer_username
                    .map(str::trim)
                    .filter(|username| !username.is_empty())
                {
                    if state.config.controller_profile == ControllerProfile::Native
                        && state
                            .managed_blacklist
                            .read()
                            .await
                            .username_is_blacklisted(username)
                    {
                        return Ok(());
                    }
                    if matches!(
                        state.config.controller_profile,
                        ControllerProfile::Legacy | ControllerProfile::Native
                    ) && state.session.read().await.state == "connected"
                        && request_peer_endpoint(state, username).await.is_err()
                    {
                        // Frozen controllers resolve the requester's endpoint
                        // before accepting an incoming search response. If the
                        // server cannot resolve that peer, they drop it.
                        return Ok(());
                    }
                    let (_, queue_position) = upload_queue_forecast(state, username).await;
                    response.slot_free = queue_position == 0;
                    response.queue_length = queue_position;
                }
                send_response(PeerMessage::FileSearchResponse(response)).await?;
                update_listeners(state, |snapshot| {
                    snapshot.file_search_responses += 1;
                    snapshot.last_event = Some("file_search_response".to_owned());
                    snapshot.last_error = None;
                })
                .await;
            }
        }
        PeerMessage::FileSearchResponse(response) => {
            state
                .remote_path_encodings
                .write()
                .await
                .remember_search_response(&response);
            let wishlist_item_id = {
                let searches = state.searches.read().await;
                searches
                    .get(response.token)
                    .and_then(|record| record.wishlist_item_id().map(str::to_owned))
            };
            let _wishlist_search_persistence = if wishlist_item_id.is_some() {
                Some(state.wishlist_search_persistence_lock.lock().await)
            } else {
                None
            };
            let (ignored_results, wishlist_policy) =
                if let Some(item_id) = wishlist_item_id.as_deref() {
                    let wishlist = state.wishlist.read().await;
                    (
                        wishlist.ignored_results_for(item_id),
                        wishlist.result_policy_for(item_id),
                    )
                } else {
                    (Vec::new(), None)
                };
            let updated_record = {
                let mut searches = state.searches.write().await;
                searches.add_peer_response_filtered(
                    &response,
                    &ignored_results,
                    wishlist_policy.as_ref(),
                )
            };
            let accepted = updated_record.is_some();
            if let Some((record, appended)) = updated_record.as_ref() {
                persist_search_result_delta(state, record, appended).await?;
                publish_search_hub_event(state, "update", record);
            };
            update_listeners(state, |snapshot| {
                snapshot.file_search_responses += 1;
                snapshot.last_event = Some(if accepted {
                    "file_search_response".to_owned()
                } else {
                    "file_search_response_unmatched".to_owned()
                });
                snapshot.last_error = None;
            })
            .await;
        }
        PeerMessage::FolderContentsRequest(request) => {
            let entries = {
                let shares = state.shares.read().await;
                shares.entries.clone()
            };
            send_response(PeerMessage::FolderContentsResponse(
                build_folder_contents_payload(
                    &entries,
                    request.token,
                    &request.folder,
                    request.folder_encoding,
                )?,
            ))
            .await?;
        }
        PeerMessage::TransferRequest(request) => {
            if !state.config.transfer_allow_inbound {
                let reason = "inbound transfers are disabled".to_owned();
                record_transfer_rejection(
                    state,
                    request.direction,
                    request.token,
                    request.filename.clone(),
                    request.size,
                    reason.clone(),
                )
                .await;
                send_response(PeerMessage::TransferResponse(TransferResponse::Rejected {
                    token: request.token,
                    reason,
                }))
                .await?;
                update_listeners(state, |snapshot| {
                    snapshot.transfer_rejections += 1;
                    snapshot.last_event = Some("transfer_rejected_policy".to_owned());
                    snapshot.last_error = None;
                })
                .await;
            } else if !transfer_capacity_available(state, None).await {
                let reason = "transfer limit reached".to_owned();
                record_transfer_rejection(
                    state,
                    request.direction,
                    request.token,
                    request.filename.clone(),
                    request.size,
                    reason.clone(),
                )
                .await;
                send_response(PeerMessage::TransferResponse(TransferResponse::Rejected {
                    token: request.token,
                    reason,
                }))
                .await?;
                update_listeners(state, |snapshot| {
                    snapshot.transfer_rejections += 1;
                    snapshot.last_event = Some("transfer_rejected_limit".to_owned());
                    snapshot.last_error = None;
                })
                .await;
            } else if let Some(shared_file) = find_shared_local_file(state, &request.filename).await
            {
                if request.direction == 0 {
                    if let Some(username) = peer_username {
                        if let Err(reason) = inbound_upload_policy(
                            state,
                            username,
                            &request.filename,
                            shared_file.size,
                        )
                        .await
                        {
                            record_transfer_rejection(
                                state,
                                request.direction,
                                request.token,
                                request.filename.clone(),
                                request.size,
                                reason.clone(),
                            )
                            .await;
                            send_response(PeerMessage::TransferResponse(
                                TransferResponse::Rejected {
                                    token: request.token,
                                    reason,
                                },
                            ))
                            .await?;
                            update_listeners(state, |snapshot| {
                                snapshot.transfer_rejections += 1;
                                snapshot.last_event =
                                    Some("transfer_rejected_group_policy".to_owned());
                                snapshot.last_error = None;
                            })
                            .await;
                            return Ok(());
                        }
                    }
                }
                {
                    let mut transfers = state.transfers.write().await;
                    transfers.record_accepted_inbound_request(
                        1,
                        request.token,
                        peer_username.map(str::to_owned),
                        request.filename.clone(),
                        shared_file.local_path.display().to_string(),
                        shared_file.size,
                    );
                }
                persist_transfer_durability(state).await;
                send_response(PeerMessage::TransferResponse(TransferResponse::Allowed {
                    token: request.token,
                    size: Some(shared_file.size),
                }))
                .await?;
                update_listeners(state, |snapshot| {
                    snapshot.last_event = Some("transfer_accepted".to_owned());
                    snapshot.last_error = None;
                })
                .await;
            } else {
                let reason = "requested file is not available from local shares".to_owned();
                record_transfer_rejection(
                    state,
                    request.direction,
                    request.token,
                    request.filename.clone(),
                    request.size,
                    reason.clone(),
                )
                .await;
                send_response(PeerMessage::TransferResponse(TransferResponse::Rejected {
                    token: request.token,
                    reason,
                }))
                .await?;
                update_listeners(state, |snapshot| {
                    snapshot.transfer_rejections += 1;
                    snapshot.last_event = Some("transfer_rejected".to_owned());
                    snapshot.last_error = None;
                })
                .await;
            }
        }
        other => {
            update_listeners(state, |snapshot| {
                snapshot.unsupported_peer_messages += 1;
                snapshot.last_event = Some(format!(
                    "unsupported_peer_message:{}",
                    peer_message_name(&other)
                ));
            })
            .await;
        }
    }
    Ok(())
}
pub(super) async fn connect_session(
    state: &AppState,
    session: &mut Option<ServerSession<TcpStream>>,
    next_ping: &mut Instant,
) -> bool {
    if state.config.integrations.vpn.enabled && !state.runtime.read().await.vpn.is_ready {
        let mut runtime = state.runtime.write().await;
        runtime.vpn_reconnect_requested = true;
        drop(runtime);
        update_session(state, |snapshot| {
            snapshot.state = "disconnected";
            snapshot.last_error = Some("Waiting for VPN client".to_owned());
        })
        .await;
        record_daemon_log(
            state,
            logging::LogLevel::Info,
            "vpn",
            "Waiting for VPN client",
        )
        .await;
        return false;
    }
    let server_address = effective_server_address(state);
    let credentials = {
        let runtime_credentials = state.runtime_credentials.read().await;
        runtime_credentials.clone()
    };
    let credentials = match credentials {
        Some(credentials) => Some((credentials, "runtime")),
        None => state
            .configured_credentials
            .read()
            .await
            .clone()
            .map(|credentials| (credentials, "config")),
    };
    let Some((credentials, credential_source)) = credentials else {
        let reason =
            "Soulseek credentials are required; enter them in the web UI or configure a credential store";
        update_session(state, |snapshot| {
            snapshot.state = "error";
            snapshot.last_error = Some(reason.to_owned());
        })
        .await;
        record_daemon_log(state, logging::LogLevel::Error, "session", reason).await;
        return false;
    };
    let connected_username = credentials.username.clone();

    record_daemon_log(
        state,
        logging::LogLevel::Info,
        "session",
        format!(
            "connecting to Soulseek server {} as {}",
            server_address,
            redact_username(&credentials.username)
        ),
    )
    .await;
    record_daemon_log(
        state,
        logging::LogLevel::Info,
        "session",
        format!("Soulseek credential source: {credential_source}"),
    )
    .await;
    update_session(state, |snapshot| {
        snapshot.state = "connecting";
        snapshot.last_error = None;
        snapshot.supporter = None;
        snapshot.connected_at = None;
    })
    .await;

    let connection =
        match connect_soulseek_tcp(state, server_address.as_str(), SoulseekSocketClass::Control)
            .await
        {
            Ok(stream) => ServerConnection::new(stream),
            Err(error) => {
                let reason = format!("connect failed: {error}");
                update_session(state, |snapshot| {
                    snapshot.state = "error";
                    snapshot.last_error = Some(reason.clone());
                    snapshot.supporter = None;
                })
                .await;
                record_daemon_log(state, logging::LogLevel::Error, "session", reason).await;
                return false;
            }
        };
    let mut new_session = ServerSession::new(connection);
    let initial_wait_port = WaitPort {
        port: effective_advertised_port(state),
        obfuscation: effective_obfuscated_advertised_port(state).map(|port| ObfuscatedPort {
            kind: ROTATED_OBFUSCATION_TYPE,
            port,
        }),
    };
    let info = match new_session
        .login_with_wait_port(credentials, initial_wait_port)
        .await
    {
        Ok(info) => info,
        Err(error) => {
            let reason = format!("login failed: {error}");
            update_session(state, |snapshot| {
                snapshot.state = "error";
                snapshot.last_error = Some(reason.clone());
                snapshot.supporter = None;
            })
            .await;
            record_daemon_log(state, logging::LogLevel::Error, "session", reason).await;
            return false;
        }
    };
    reset_distributed_network(state, Some(&connected_username)).await;
    let wait_port_result = if state.config.obfuscation_enabled {
        if let Some(obfuscated_port) = effective_obfuscated_advertised_port(state) {
            new_session
                .set_wait_port_obfuscated(
                    effective_advertised_port(state),
                    ROTATED_OBFUSCATION_TYPE,
                    obfuscated_port,
                )
                .await
        } else {
            new_session
                .set_wait_port(effective_advertised_port(state))
                .await
        }
    } else {
        new_session
            .set_wait_port(effective_advertised_port(state))
            .await
    };
    if let Err(error) = wait_port_result {
        let reason = format!("set wait port failed: {error}");
        update_session(state, |snapshot| {
            snapshot.state = "error";
            snapshot.last_error = Some(reason.clone());
            snapshot.supporter = None;
        })
        .await;
        record_daemon_log(state, logging::LogLevel::Error, "session", reason).await;
        return false;
    }
    if let Err(error) = new_session
        .send_server_message(ServerMessage::SetStatus { status: 2 })
        .await
    {
        let reason = format!("set status failed: {error}");
        update_session(state, |snapshot| {
            snapshot.state = "error";
            snapshot.last_error = Some(reason.clone());
            snapshot.supporter = None;
        })
        .await;
        record_daemon_log(state, logging::LogLevel::Error, "session", reason).await;
        return false;
    }
    let (folders, files) = {
        let shares = state.shares.read().await;
        let files = u32::try_from(shares.entries.len()).unwrap_or(u32::MAX);
        let folders = u32::try_from(
            shares
                .entries
                .iter()
                .map(|entry| virtual_folder(&entry.filename))
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
        )
        .unwrap_or(u32::MAX);
        (folders, files)
    };
    if let Err(error) = new_session
        .send_server_message(ServerMessage::SharedFoldersFiles { folders, files })
        .await
    {
        let reason = format!("share count update failed: {error}");
        update_session(state, |snapshot| {
            snapshot.state = "error";
            snapshot.last_error = Some(reason.clone());
            snapshot.supporter = None;
        })
        .await;
        record_daemon_log(state, logging::LogLevel::Error, "session", reason).await;
        return false;
    }
    let distributed_settings = *state.soulseek_distributed_settings.read().await;
    if !distributed_settings.disabled {
        // Use persisted distributed tree state if available, otherwise start fresh
        let (branch_level, branch_root) = {
            let runtime = state.distributed_network.read().await;
            (runtime.branch_level, runtime.branch_root.clone())
        };

        for message in [
            ServerMessage::HaveNoParent { no_parent: true },
            ServerMessage::AcceptChildren {
                accept: !distributed_settings.disable_children,
            },
            ServerMessage::BranchLevel {
                level: branch_level,
            },
            ServerMessage::BranchRoot {
                username: branch_root,
            },
        ] {
            if let Err(error) = new_session.send_server_message(message).await {
                let reason = format!("distributed network initialization failed: {error}");
                update_session(state, |snapshot| {
                    snapshot.state = "error";
                    snapshot.last_error = Some(reason.clone());
                })
                .await;
                record_daemon_log(state, logging::LogLevel::Error, "distributed", reason).await;
                return false;
            }
        }
    }
    if let Err(error) = new_session.send_ping().await {
        let reason = format!("initial ping failed: {error}");
        update_session(state, |snapshot| {
            snapshot.state = "error";
            snapshot.last_error = Some(reason.clone());
            snapshot.supporter = None;
        })
        .await;
        record_daemon_log(state, logging::LogLevel::Error, "session", reason).await;
        return false;
    }

    update_session(state, |snapshot| {
        snapshot.state = "connected";
        snapshot.username = Some(connected_username.clone());
        snapshot.supporter = Some(info.is_supporter);
        snapshot.last_error = None;
        snapshot.connected_at = Some(unix_timestamp());
        if snapshot.server_messages_seen > 0 {
            snapshot.reconnects += 1;
        }
    })
    .await;
    *state
        .connected_server_address
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(server_address);
    state.runtime.write().await.set_reconnect_pending(false);
    *next_ping = Instant::now() + state.config.ping_interval;
    replay_joined_rooms(state, &mut new_session).await;
    replay_watched_users(state, &mut new_session).await;
    sync_contact_statuses(state, &mut new_session).await;
    sync_room_tickers(state, &mut new_session).await;
    publish_configured_interests(state, &mut new_session).await;
    check_privileges_after_login(state).await;
    dispatch_queued_downloads_after_login(state).await;
    *session = Some(new_session);
    record_daemon_log(
        state,
        logging::LogLevel::Info,
        "session",
        format!(
            "Soulseek login succeeded; supporter={}",
            if info.is_supporter { "true" } else { "false" }
        ),
    )
    .await;
    true
}
pub(super) async fn send_due_wishlist_search(
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
pub(super) async fn project_server_message(
    state: &Arc<AppState>,
    session: &mut ServerSession<TcpStream>,
    message: &ServerMessage,
) {
    let message_username = match message {
        ServerMessage::MessageUserResponse(message) => Some(message.username.as_str()),
        ServerMessage::SayChatroomResponse { username, .. }
        | ServerMessage::GlobalRoomMessage { username, .. } => Some(username.as_str()),
        _ => None,
    };
    if let Some(username) = message_username {
        if state.managed_blacklist.write().await.is_blacklisted(
            Some(username),
            None,
            unix_timestamp(),
        ) {
            return;
        }
    }
    match message {
        ServerMessage::UserInterests(interests) => {
            let key = interests.username.to_ascii_lowercase();
            let _ = state
                .controller_features
                .import_soulseek_interests(&interests.username, &interests.liked, &interests.hated)
                .await;
            if let Some(waiters) = state.pending_user_interests.write().await.remove(&key) {
                for waiter in waiters {
                    let _ = waiter.send(Ok(interests.clone()));
                }
            }
        }
        ServerMessage::WatchUserResponse(user) => {
            let _user_persistence = state.user_persistence_lock.lock().await;
            let record = {
                let mut users = state.users.write().await;
                users.apply_watched_user(user)
            };
            let persistence_failure = if let Some(record) = record.as_ref() {
                persist_user_projection(state, record).await.err()
            } else {
                None
            };
            drop(_user_persistence);
            if let Some(error) = persistence_failure {
                update_session(state, |snapshot| snapshot.last_error = Some(error)).await;
            }
        }
        ServerMessage::GetUserStatusResponse(status) => {
            let _user_persistence = state.user_persistence_lock.lock().await;
            let record = {
                let mut users = state.users.write().await;
                users.apply_status(status)
            };
            let persistence_failure = if let Some(record) = record.as_ref() {
                persist_user_projection(state, record).await.err()
            } else {
                None
            };
            drop(_user_persistence);
            if let Some(error) = persistence_failure {
                update_session(state, |snapshot| snapshot.last_error = Some(error)).await;
            }
        }
        ServerMessage::GetUserStats { username, stats } => {
            let _user_persistence = state.user_persistence_lock.lock().await;
            let record = {
                let mut users = state.users.write().await;
                users.apply_stats(username.clone(), stats)
            };
            let persistence_failure = if let Some(record) = record.as_ref() {
                persist_user_projection(state, record).await.err()
            } else {
                None
            };
            drop(_user_persistence);
            if let Some(error) = persistence_failure {
                update_session(state, |snapshot| snapshot.last_error = Some(error)).await;
            }
        }
        ServerMessage::CheckPrivilegesResponse { seconds } => {
            update_session(state, |snapshot| {
                snapshot.privileges_seconds = Some(*seconds);
            })
            .await;
        }
        ServerMessage::NotifyPrivileges { seconds } => {
            update_session(state, |snapshot| {
                snapshot.privileges_seconds = Some(*seconds);
            })
            .await;
            // Auto-acknowledge privilege notifications if configured
            if state
                .config
                .soulseek_connection
                .auto_acknowledge_privilege_notifications
            {
                if let Err(error) = session
                    .send_server_message(ServerMessage::AckNotifyPrivileges { token: *seconds })
                    .await
                {
                    update_session(state, |snapshot| {
                        snapshot.last_error = Some(format!(
                            "privilege notification auto-acknowledgment failed: {error}"
                        ));
                    })
                    .await;
                } else {
                    record_event(
                        state,
                        "privilege.auto_acked",
                        "privileges",
                        Some(format!("seconds={seconds}")),
                    )
                    .await;
                }
            }
        }
        ServerMessage::MessageUserResponse(message) => {
            if mesh_sync::handle_private_message(
                state,
                session,
                &message.username,
                &message.message,
            )
            .await
            {
                return;
            }
            if handle_incoming_soulseek_pod_message(state, &message.username, &message.message)
                .await
            {
                return;
            }
            eprintln!(
                "received private message id={} from {}",
                message.id,
                redact_username(&message.username)
            );
            let _message_persistence = state.message_persistence_lock.lock().await;
            let mut messages = state.messages.write().await;
            let record = if let Some(existing) = messages.records.iter_mut().find(|record| {
                record.direction == "inbound"
                    && record.username.eq_ignore_ascii_case(&message.username)
                    && record.source_id == Some(message.id)
                    && record.source_timestamp == Some(message.timestamp)
            }) {
                existing.was_replayed = message.was_replayed;
                existing.acknowledged = false;
                existing.clone()
            } else {
                let mut record =
                    messages.add(message.username.clone(), "inbound", message.message.clone());
                record.source_id = Some(message.id);
                record.source_timestamp = Some(message.timestamp);
                record.was_replayed = message.was_replayed;
                if let Some(stored) = messages
                    .records
                    .iter_mut()
                    .find(|stored| stored.id == record.id)
                {
                    *stored = record.clone();
                }
                record
            };
            let message_id = record.id;
            drop(messages);
            let persistence_result = persist_message_record_checked(state, &record).await;
            drop(_message_persistence);
            if let Err(error) = persistence_result {
                update_session(state, |snapshot| snapshot.last_error = Some(error)).await;
                return;
            }
            record_event(
                state,
                "message.received",
                message.username.clone(),
                Some(format!("id={message_id}")),
            )
            .await;
            if message.is_new {
                send_private_message_notifications(state, &message.username, &message.message)
                    .await;
            }
            let auto_response = state
                .private_message_auto_response_settings
                .read()
                .await
                .clone();
            if message.is_new
                && auto_response.enabled
                && !auto_response.message.trim().is_empty()
                && is_private_message_auto_response_candidate(&message.message)
            {
                let cooldown_seconds = auto_response.cooldown_minutes.saturating_mul(60);
                let should_respond = state
                    .private_message_auto_responses
                    .write()
                    .await
                    .should_respond(&message.username, unix_timestamp(), cooldown_seconds);
                if should_respond {
                    let response_body = auto_response.message.trim().to_owned();
                    match session
                        .send_server_message(ServerMessage::MessageUserRequest {
                            username: message.username.clone(),
                            message: response_body.clone(),
                        })
                        .await
                    {
                        Ok(()) => {
                            let _message_persistence = state.message_persistence_lock.lock().await;
                            let mut messages = state.messages.write().await;
                            let response =
                                messages.add(message.username.clone(), "outbound", response_body);
                            drop(messages);
                            let persistence_result =
                                persist_message_record_checked(state, &response).await;
                            drop(_message_persistence);
                            if let Err(error) = persistence_result {
                                update_session(state, |snapshot| snapshot.last_error = Some(error))
                                    .await;
                            }
                            record_event(
                                state,
                                "message.auto_responded",
                                message.username.clone(),
                                Some(format!("id={}", response.id)),
                            )
                            .await;
                        }
                        Err(error) => {
                            state
                                .private_message_auto_responses
                                .write()
                                .await
                                .release(&message.username);
                            update_session(state, |snapshot| {
                                snapshot.last_error =
                                    Some(format!("private-message auto response failed: {error}"));
                            })
                            .await;
                        }
                    }
                }
            }
            // Auto-acknowledge private messages if configured
            if state
                .config
                .soulseek_connection
                .auto_acknowledge_private_messages
            {
                if let Err(error) = session
                    .send_server_message(ServerMessage::MessageAcked { id: message.id })
                    .await
                {
                    update_session(state, |snapshot| {
                        snapshot.last_error = Some(format!(
                            "private-message auto-acknowledgment failed: {error}"
                        ));
                    })
                    .await;
                } else {
                    let _message_persistence = state.message_persistence_lock.lock().await;
                    let mut messages = state.messages.write().await;
                    messages.ack(u64::from(message.id));
                    drop(messages);
                    let persistence_result =
                        persist_message_ack_checked(state, u64::from(message.id)).await;
                    drop(_message_persistence);
                    if let Err(error) = persistence_result {
                        update_session(state, |snapshot| snapshot.last_error = Some(error)).await;
                    }
                    record_event(
                        state,
                        "message.auto_acked",
                        "messages",
                        Some(format!("id={}", message.id)),
                    )
                    .await;
                }
            }
        }
        ServerMessage::MessageAcked { id } => {
            let _message_persistence = state.message_persistence_lock.lock().await;
            let mut messages = state.messages.write().await;
            messages.ack(u64::from(*id));
            drop(messages);
            let persistence_result = persist_message_ack_checked(state, u64::from(*id)).await;
            drop(_message_persistence);
            if let Err(error) = persistence_result {
                update_session(state, |snapshot| snapshot.last_error = Some(error)).await;
            }
            record_event(state, "message.acked", "messages", Some(format!("id={id}"))).await;
        }
        ServerMessage::RoomList(room_list) => {
            let mut rooms = state.rooms.write().await;
            rooms.apply_room_list(room_list);
            drop(rooms);
            record_event(state, "room.list.updated", "rooms", None).await;
            record_event(state, "room.users.updated", "rooms", None).await;
        }
        ServerMessage::FileSearchIncoming {
            username,
            token,
            query,
        } => {
            record_soulseek_diagnostic(
                state,
                logging::LogLevel::Debug,
                "search",
                format!(
                    "Received incoming public search from {} for {:?}",
                    redact_username(username),
                    query
                ),
            )
            .await;
            schedule_incoming_search_response(
                Arc::clone(state),
                username.clone(),
                *token,
                query.clone(),
            )
            .await;
        }
        ServerMessage::EmbeddedMessage {
            distributed_code,
            payload,
        } => {
            handle_embedded_distributed_search(state, None, None, *distributed_code, payload).await;
        }
        ServerMessage::GetPeerAddressResponse(address) => {
            remember_peer_endpoint(state, address.clone()).await;
            project_peer_browse_response(state, address).await;
            project_peer_transfer_response(state, address).await;
        }
        ServerMessage::PossibleParents(parents) => {
            if let Err(error) = connect_distributed_parent(Arc::clone(state), parents).await {
                // Distributed parent selection is an optional overlay path;
                // a timeout here must not mark the authenticated Soulseek
                // server session unhealthy.
                record_daemon_log(state, logging::LogLevel::Warn, "distributed", error).await;
            }
        }
        ServerMessage::ConnectToPeerRequest(request) => {
            if let Err(error) = handle_connect_to_peer_request(state, session, request).await {
                update_session(state, |snapshot| {
                    snapshot.last_error = Some(format!(
                        "connect-to-peer request from {} failed: {error}",
                        redact_username(&request.username)
                    ));
                })
                .await;
            }
        }
        ServerMessage::ConnectToPeerResponse(response) => {
            project_indirect_browse_response(state, response).await;
            project_indirect_transfer_response(state, response).await;
        }
        ServerMessage::CantConnectToPeerResponse { token } => {
            fail_indirect_browse(
                state,
                *token,
                "server reported cant-connect-to-peer".to_owned(),
            )
            .await;
            fail_indirect_transfer(
                state,
                *token,
                "server reported cant-connect-to-peer".to_owned(),
            )
            .await;
            record_event(
                state,
                "user.cannot.connect",
                format!("token:{}", token),
                Some("server reported cant-connect-to-peer".to_owned()),
            )
            .await;
        }
        ServerMessage::CantCreateRoom { room } => {
            let mut rooms = state.rooms.write().await;
            rooms.fail_join(room, "server reported cant-create-room".to_owned());
            drop(rooms);
            record_event(
                state,
                "room.updated",
                room.clone(),
                Some("cant-create-room".to_owned()),
            )
            .await;
        }
        ServerMessage::CantJoinRoom { room } => {
            let mut rooms = state.rooms.write().await;
            rooms.fail_join(room, "server reported cant-join-room".to_owned());
            drop(rooms);
            record_event(
                state,
                "room.updated",
                room.clone(),
                Some("cant-join-room".to_owned()),
            )
            .await;
        }
        ServerMessage::JoinedRoom(joined) => {
            let room = joined.room.clone();
            let roster = joined
                .users
                .iter()
                .map(|user| RoomRosterEntry {
                    username: user.username.clone(),
                    status: user.status,
                    average_speed: user.average_speed,
                    upload_count: user.upload_count,
                    file_count: user.file_count,
                    directory_count: user.directory_count,
                    slots_free: user.slots_free,
                    country_code: user.country_code.clone(),
                })
                .collect();
            let mut rooms = state.rooms.write().await;
            rooms.join(room.clone());
            rooms.apply_roster(&room, roster);
            drop(rooms);
            record_event(state, "room.joined", room.clone(), None).await;
        }
        ServerMessage::SayChatroomResponse {
            room,
            username,
            message,
        }
        | ServerMessage::GlobalRoomMessage {
            room,
            username,
            message,
        } => {
            let mut rooms = state.rooms.write().await;
            if rooms
                .add_message(room, username.clone(), message.clone())
                .is_none()
            {
                rooms.join(room.clone());
                rooms.add_message(room, username.clone(), message.clone());
            }
            drop(rooms);
            bridge_soulseek_room_message_to_pods(state, room, username, message).await;
            if state
                .config
                .username
                .as_deref()
                .is_some_and(|local_username| message.contains(local_username))
            {
                send_room_mention_notifications(state, room, username, message).await;
            }
            record_event(
                state,
                "room.message",
                room.clone(),
                Some(format!("username={username}")),
            )
            .await;
        }
        ServerMessage::LeaveRoom { room } => {
            let mut rooms = state.rooms.write().await;
            rooms.leave(room);
            drop(rooms);
            record_event(state, "room.left", room.clone(), None).await;
        }
        _ => {}
    }
}

pub(super) fn spawn_gold_star_club(state: Arc<AppState>) {
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

pub(super) fn spawn_session_manager(
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

fn spawn_wishlist_smart_fallback(state: Arc<AppState>, token: u32) {
    tokio::spawn(async move {
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

pub(super) fn search_dispatch_message(
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

pub(super) async fn handle_incoming_soulseek_pod_message(
    state: &Arc<AppState>,
    username: &str,
    message: &str,
) -> bool {
    const POD_MESSAGE_PREFIX: &str = "PODMSG:";
    const MAX_MESSAGE_BYTES: usize = 16 * 1024;
    const MAX_JSON_BYTES: usize = 12 * 1024;
    if !message.starts_with(POD_MESSAGE_PREFIX) {
        return false;
    }
    if message.len() > MAX_MESSAGE_BYTES {
        return true;
    }
    let json_payload = &message[POD_MESSAGE_PREFIX.len()..];
    if json_payload.is_empty() || json_payload.len() > MAX_JSON_BYTES {
        return true;
    }
    let Ok(payload) = serde_json::from_str::<serde_json::Value>(json_payload) else {
        return true;
    };
    let string_field = |camel: &str, pascal: &str| {
        payload
            .get(camel)
            .or_else(|| payload.get(pascal))
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .unwrap_or_default()
    };
    let pod_id = string_field("podId", "PodId");
    let channel_id = string_field("channelId", "ChannelId");
    let sender_peer_id = string_field("senderPeerId", "SenderPeerId");
    let message_id = string_field("messageId", "MessageId");
    let body = string_field("body", "Body");
    let signature = string_field("signature", "Signature");
    let timestamp_unix_ms = payload
        .get("timestampUnixMs")
        .or_else(|| payload.get("TimestampUnixMs"))
        .and_then(serde_json::Value::as_i64)
        .unwrap_or_default();
    let sig_version = payload
        .get("sigVersion")
        .or_else(|| payload.get("SigVersion"))
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(1);
    if pod_id.is_empty()
        || channel_id.is_empty()
        || sender_peer_id.is_empty()
        || message_id.is_empty()
        || body.is_empty()
        || timestamp_unix_ms <= 0
        || sender_peer_id != format!("bridge:{username}")
        || body.len() > 10_000
    {
        return true;
    }
    let normalized = serde_json::json!({
        "messageId": message_id,
        "podId": pod_id,
        "channelId": channel_id,
        "senderPeerId": sender_peer_id,
        "body": body,
        "timestampUnixMs": timestamp_unix_ms,
        "sigVersion": sig_version,
    });
    let (canonical, canonical_pod_id) = pod_message_canonical_payload(&normalized);
    let signature_mode = state
        .advanced_networking
        .read()
        .await
        .pod_security_signature_mode;
    if !pod_verify_signature(
        &normalized,
        &canonical,
        &canonical_pod_id,
        signature,
        signature_mode,
        state,
    )
    .await
    {
        return true;
    }
    let (binding, stored) = {
        let mut channels = state.pod_channels.write().await;
        let pods = state.pods.read().await;
        if pods.get(pod_id).is_none()
            || !pods.channel_exists(pod_id, channel_id)
            || !pods.is_member(pod_id, sender_peer_id)
        {
            return true;
        }
        let binding = pods.soulseek_binding(pod_id, channel_id);
        let stored = channels.append_with_id(
            message_id.to_owned(),
            pod_id.to_owned(),
            channel_id.to_owned(),
            sender_peer_id.to_owned(),
            body.to_owned(),
            signature.to_owned(),
            u64::try_from(timestamp_unix_ms).unwrap_or_default(),
            u8::try_from(sig_version).unwrap_or(1),
        );
        (binding, stored)
    };
    let Ok(stored) = stored else {
        return true;
    };
    if let Some(binding) =
        binding.filter(|binding| binding.kind == "room" && binding.mode == "mirror")
    {
        let room = binding.identifier;
        if let Err(error) = try_send_session_command(
            state,
            SessionCommand::SayRoom {
                room: room.clone(),
                body: format!("[Pod:{}] {}", stored.sender_peer_id, stored.body),
            },
        ) {
            record_pod_room_mirror_failure(state, &room, &error).await;
        }
    }
    record_event(
        state,
        "pod.message.received",
        pod_id.to_owned(),
        Some(format!("channel={channel_id}")),
    )
    .await;
    true
}

pub(super) async fn schedule_incoming_search_response(
    state: Arc<AppState>,
    username: String,
    token: u32,
    query: String,
) {
    let circuit_breaker = state
        .core_workflow_settings
        .read()
        .await
        .incoming_search
        .circuit_breaker;
    let depth = state
        .incoming_search_queue_depth
        .fetch_add(1, Ordering::AcqRel);
    if depth >= circuit_breaker {
        state
            .incoming_search_queue_depth
            .fetch_sub(1, Ordering::AcqRel);
        return;
    }
    tokio::spawn(async move {
        let gate = Arc::clone(&state.incoming_searches);
        let result = async {
            let _permit = gate
                .acquire_owned()
                .await
                .map_err(|_| "incoming search gate closed".to_owned())?;
            let Some(response) = build_file_search_response(&state, token, &query).await else {
                record_soulseek_diagnostic(
                    &state,
                    logging::LogLevel::Debug,
                    "search",
                    format!(
                        "Incoming public search for {:?} matched no shared files",
                        query
                    ),
                )
                .await;
                return Ok::<(), String>(());
            };
            let address = request_peer_endpoint(&state, &username).await?;
            send_peer_message_oneway(&state, &address, PeerMessage::FileSearchResponse(response))
                .await?;
            record_soulseek_diagnostic(
                &state,
                logging::LogLevel::Debug,
                "search",
                format!(
                    "Sent incoming public search response to {} for {:?}",
                    redact_username(&username),
                    query
                ),
            )
            .await;
            Ok(())
        }
        .await;
        state
            .incoming_search_queue_depth
            .fetch_sub(1, Ordering::AcqRel);
        if let Err(error) = result {
            record_daemon_log(
                &state,
                logging::LogLevel::Warn,
                "search",
                format!(
                    "incoming search response for {} failed: {error}",
                    redact_username(&username)
                ),
            )
            .await;
        }
    });
}

pub(super) async fn bridge_soulseek_room_message_to_pods(
    state: &AppState,
    room: &str,
    username: &str,
    message: &str,
) {
    let bindings = state.pods.read().await.room_bindings(room);
    for binding in bindings {
        let append_result = {
            let mut channels = state.pod_channels.write().await;
            let pods = state.pods.read().await;
            if pods.channel_exists(&binding.pod_id, &binding.channel_id)
                && pods
                    .soulseek_binding(&binding.pod_id, &binding.channel_id)
                    .as_ref()
                    == Some(&binding)
            {
                Some(channels.append(
                    binding.pod_id.clone(),
                    binding.channel_id.clone(),
                    format!("bridge:{username}"),
                    format!("[Soulseek:{username}] {message}"),
                    String::new(),
                    unix_timestamp_millis(),
                ))
            } else {
                None
            }
        };
        match append_result {
            None => continue,
            Some(Err(error)) => {
                update_session(state, |snapshot| {
                    snapshot.last_error =
                        Some(format!("pod room bridge persistence failed: {error}"));
                })
                .await;
            }
            Some(Ok(_)) => {
                record_event(
                    state,
                    "pod.message.bridged",
                    binding.pod_id,
                    Some(format!("channel={}", binding.channel_id)),
                )
                .await;
            }
        }
    }
}
