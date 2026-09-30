use super::*;

pub(crate) async fn project_server_message(
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
