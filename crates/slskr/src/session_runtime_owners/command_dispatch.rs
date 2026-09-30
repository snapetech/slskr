use super::*;

pub(crate) async fn handle_session_command(
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
            state.spawn_managed_task(async move {
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
