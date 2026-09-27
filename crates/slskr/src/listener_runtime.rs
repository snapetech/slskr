// Listener binding, lifecycle supervision, and incoming connection handling.
use super::*;

pub(super) fn spawn_configured_listeners(
    state: Arc<AppState>,
    regular_commands: mpsc::Receiver<ListenerCommand>,
    obfuscated_commands: mpsc::Receiver<ListenerCommand>,
    shared_mesh_tcp: bool,
) {
    let shared_obfuscation = state.config.obfuscation_enabled
        && state.config.obfuscation_listen_port == 0
        && state.config.obfuscated_listener_bind.is_none();
    let regular_state = Arc::clone(&state);
    state.spawn_managed_task(run_listener_manager(
        regular_state,
        regular_commands,
        false,
        shared_obfuscation,
        shared_mesh_tcp,
    ));
    let obfuscated_state = Arc::clone(&state);
    state.spawn_managed_task(run_listener_manager(
        obfuscated_state,
        obfuscated_commands,
        true,
        false,
        false,
    ));
}

async fn record_listener_bound(
    state: &AppState,
    bind: Option<String>,
    listener: &Listener,
    obfuscated: bool,
) {
    let local_addr = listener
        .local_addr()
        .ok()
        .map(|address| address.to_string());
    update_listeners(state, |snapshot| {
        if obfuscated {
            snapshot.obfuscated_bind = bind;
            snapshot.obfuscated_local_addr = local_addr;
        } else {
            snapshot.regular_bind = bind;
            snapshot.regular_local_addr = local_addr;
        }
        snapshot.last_error = None;
    })
    .await;
}

async fn bind_managed_listener(
    state: &AppState,
    bind: Option<String>,
    obfuscated: bool,
) -> Result<Option<Listener>, String> {
    let Some(address) = bind.as_deref() else {
        update_listeners(state, |snapshot| {
            if obfuscated {
                snapshot.obfuscated_bind = None;
                snapshot.obfuscated_local_addr = None;
            } else {
                snapshot.regular_bind = None;
                snapshot.regular_local_addr = None;
            }
            snapshot.last_error = None;
        })
        .await;
        return Ok(None);
    };
    let listener = Listener::bind(address).await.map_err(|error| {
        format!(
            "{} listener bind failed: {error}",
            if obfuscated { "obfuscated" } else { "regular" }
        )
    })?;
    record_listener_bound(state, bind, &listener, obfuscated).await;
    Ok(Some(listener))
}

async fn process_listener_incoming(
    state: &Arc<AppState>,
    incoming: IncomingConnection<TcpStream>,
    remote_addr: SocketAddr,
    obfuscated: bool,
) {
    let obfuscated = obfuscated || incoming_connection_is_obfuscated(&incoming);
    let network_guard = state
        .advanced_networking
        .read()
        .await
        .security
        .network_guard
        .clone();
    if network_guard.enabled {
        let rejected = {
            let mut active = state
                .incoming_connection_ips
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let global = active.values().copied().sum::<usize>();
            let per_ip = active.get(&remote_addr.ip()).copied().unwrap_or(0);
            if global >= network_guard.max_global_connections
                || per_ip >= network_guard.max_connections_per_ip
            {
                true
            } else {
                *active.entry(remote_addr.ip()).or_default() += 1;
                false
            }
        };
        if rejected {
            update_listeners(state, |snapshot| {
                snapshot.errors += 1;
                snapshot.transfer_rejections += 1;
                snapshot.last_error =
                    Some("incoming connection rejected by the configured network guard".to_owned());
            })
            .await;
            return;
        }
    }
    let network_guard_lease = IncomingNetworkGuardLease {
        state: Arc::clone(state),
        ip: remote_addr.ip(),
        enabled: network_guard.enabled,
    };
    let incoming =
        match configure_incoming_soulseek_socket(incoming, &state.config.soulseek_connection) {
            Ok(incoming) => incoming,
            Err(error) => {
                update_listeners(state, |snapshot| {
                    snapshot.errors += 1;
                    snapshot.last_error = Some(error);
                })
                .await;
                return;
            }
        };
    if let IncomingConnection::UnknownInit { code, payload, .. } = &incoming {
        eprintln!(
            "[Warning] soulseek: Unknown initialization code {code} with {} payload bytes from {}",
            payload.len(),
            scrub_socket_addr(remote_addr)
        );
    }
    let event = format!(
        "{} from {}",
        incoming_connection_name(&incoming),
        scrub_socket_addr(remote_addr)
    );
    update_listeners(state, |snapshot| {
        if obfuscated {
            snapshot.obfuscated_accepts += 1;
        } else {
            snapshot.regular_accepts += 1;
        }
        bump_incoming_counter(snapshot, &incoming);
        snapshot.last_event = Some(event);
        snapshot.last_error = None;
    })
    .await;
    let Ok(permit) = Arc::clone(&state.incoming_connections).try_acquire_owned() else {
        update_listeners(state, |snapshot| {
            snapshot.errors += 1;
            snapshot.last_error =
                Some("incoming connection dropped because the handler pool is full".to_owned());
        })
        .await;
        return;
    };
    let task_state = Arc::clone(state);
    state.spawn_managed_task(async move {
        let _network_guard_lease = network_guard_lease;
        let _permit = permit;
        handle_owned_incoming(task_state, incoming, remote_addr).await;
    });
}

fn incoming_connection_is_obfuscated(incoming: &IncomingConnection<TcpStream>) -> bool {
    matches!(
        incoming,
        IncomingConnection::ObfuscatedPeerMessages(_)
            | IncomingConnection::PeerInit {
                obfuscated: true,
                ..
            }
    )
}

struct IncomingNetworkGuardLease {
    state: Arc<AppState>,
    ip: IpAddr,
    enabled: bool,
}

impl Drop for IncomingNetworkGuardLease {
    fn drop(&mut self) {
        release_incoming_network_guard(&self.state, self.ip, self.enabled);
    }
}

fn release_incoming_network_guard(state: &AppState, ip: IpAddr, enabled: bool) {
    if !enabled {
        return;
    }
    let mut active = state
        .incoming_connection_ips
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(count) = active.get_mut(&ip) {
        *count = count.saturating_sub(1);
        if *count == 0 {
            active.remove(&ip);
        }
    }
}

async fn run_listener_manager(
    state: Arc<AppState>,
    mut commands: mpsc::Receiver<ListenerCommand>,
    obfuscated: bool,
    shared_obfuscation: bool,
    shared_mesh_tcp: bool,
) {
    let handshake_slots = Arc::new(Semaphore::new(MAX_PENDING_LISTENER_HANDSHAKES));
    let mut active_bind = if obfuscated && state.config.obfuscation_enabled {
        state.config.obfuscated_listener_bind.clone()
    } else if obfuscated {
        None
    } else {
        state.config.listener_bind.clone()
    };
    let mut listener = match bind_managed_listener(&state, active_bind.clone(), obfuscated).await {
        Ok(listener) => listener,
        Err(error) => {
            update_listeners(&state, |snapshot| {
                snapshot.errors += 1;
                snapshot.last_error = Some(error);
            })
            .await;
            None
        }
    };

    loop {
        if let Some(active_listener) = listener.as_ref() {
            tokio::select! {
                command = commands.recv() => {
                    let Some(ListenerCommand::Reconfigure { bind, response }) = command else {
                        break;
                    };
                    if bind == active_bind {
                        let _ = response.send(Ok(false));
                        continue;
                    }
                    let previous_bind = active_bind.clone();
                    if let Some(address) = bind.as_deref() {
                        match Listener::bind(address).await {
                            Ok(probe) => drop(probe),
                            Err(error) => {
                                let error = format!(
                                    "{} listener bind failed: {error}",
                                    if obfuscated { "obfuscated" } else { "regular" }
                                );
                                update_listeners(&state, |snapshot| {
                                    snapshot.errors += 1;
                                    snapshot.last_error = Some(error.clone());
                                })
                                .await;
                                let _ = response.send(Err(error));
                                continue;
                            }
                        }
                    }
                    drop(listener.take());
                    match bind_managed_listener(&state, bind.clone(), obfuscated).await {
                        Ok(reconfigured) => {
                            active_bind = bind;
                            listener = reconfigured;
                            let _ = response.send(Ok(true));
                        }
                        Err(error) => {
                            listener = bind_managed_listener(
                                &state,
                                previous_bind.clone(),
                                obfuscated,
                            )
                                .await
                                .unwrap_or(None);
                            active_bind = previous_bind;
                            update_listeners(&state, |snapshot| {
                                snapshot.errors += 1;
                                snapshot.last_error = Some(error.clone());
                            })
                            .await;
                            let _ = response.send(Err(error));
                        }
                    }
                }
                accepted = active_listener.accept_raw() => {
                    match accepted {
                        Ok((stream, remote_addr)) => {
                            let Some(handshake_permit) = handshake_slots.clone().try_acquire_owned().ok() else {
                                drop(stream);
                                update_listeners(&state, |snapshot| {
                                    snapshot.errors += 1;
                                    snapshot.last_error = Some(format!(
                                        "{} listener handshake pool is full",
                                        if obfuscated { "obfuscated" } else { "regular" }
                                    ));
                                })
                                .await;
                                continue;
                            };
                            let task_state = Arc::clone(&state);
                            state.spawn_managed_task(async move {
                                let _handshake_permit = handshake_permit;
                                if shared_mesh_tcp {
                                    process_shared_mesh_connection(task_state, stream, remote_addr)
                                        .await;
                                } else {
                                    process_listener_connection(
                                        task_state,
                                        stream,
                                        remote_addr,
                                        obfuscated,
                                        shared_obfuscation,
                                    )
                                    .await;
                                }
                            });
                        }
                        Err(error) => {
                            update_listeners(&state, |snapshot| {
                                snapshot.errors += 1;
                                snapshot.last_error = Some(format!(
                                    "{} listener accept failed: {error}",
                                    if obfuscated { "obfuscated" } else { "regular" }
                                ));
                            })
                            .await;
                        }
                    }
                }
            }
        } else {
            let Some(ListenerCommand::Reconfigure { bind, response }) = commands.recv().await
            else {
                break;
            };
            if bind == active_bind {
                let _ = response.send(Ok(false));
                continue;
            }
            match bind_managed_listener(&state, bind.clone(), obfuscated).await {
                Ok(reconfigured) => {
                    active_bind = bind;
                    listener = reconfigured;
                    let _ = response.send(Ok(true));
                }
                Err(error) => {
                    update_listeners(&state, |snapshot| {
                        snapshot.errors += 1;
                        snapshot.last_error = Some(error.clone());
                    })
                    .await;
                    let _ = response.send(Err(error));
                }
            }
        }
    }
}

async fn process_listener_connection(
    state: Arc<AppState>,
    stream: TcpStream,
    remote_addr: SocketAddr,
    obfuscated: bool,
    shared_obfuscation: bool,
) {
    let incoming = match time::timeout(DEFAULT_INIT_HANDSHAKE_TIMEOUT, async move {
        if obfuscated {
            demux_obfuscated_incoming(stream).await
        } else if shared_obfuscation {
            demux_shared_incoming(stream).await
        } else {
            demux_incoming(stream).await
        }
    })
    .await
    {
        Ok(Ok(incoming)) => incoming,
        Ok(Err(error)) => {
            update_listeners(&state, |snapshot| {
                snapshot.errors += 1;
                snapshot.last_error = Some(format!(
                    "{} listener initialization failed: {error}",
                    if obfuscated {
                        "obfuscated"
                    } else if shared_obfuscation {
                        "shared"
                    } else {
                        "regular"
                    }
                ));
            })
            .await;
            return;
        }
        Err(_) => {
            update_listeners(&state, |snapshot| {
                snapshot.errors += 1;
                snapshot.last_error = Some(format!(
                    "{} listener initialization timed out",
                    if obfuscated {
                        "obfuscated"
                    } else if shared_obfuscation {
                        "shared"
                    } else {
                        "regular"
                    }
                ));
            })
            .await;
            return;
        }
    };

    process_listener_incoming(&state, incoming, remote_addr, obfuscated).await;
}

async fn process_shared_mesh_connection(
    state: Arc<AppState>,
    stream: TcpStream,
    remote_addr: SocketAddr,
) {
    let incoming = match time::timeout(
        DEFAULT_INIT_HANDSHAKE_TIMEOUT,
        demux_shared_mesh_incoming(stream),
    )
    .await
    {
        Ok(Ok(incoming)) => incoming,
        Ok(Err(error)) => {
            update_listeners(&state, |snapshot| {
                snapshot.errors += 1;
                snapshot.last_error = Some(format!(
                    "shared Soulseek/mesh TCP initialization failed: {error}"
                ));
            })
            .await;
            return;
        }
        Err(_) => {
            update_listeners(&state, |snapshot| {
                snapshot.errors += 1;
                snapshot.last_error =
                    Some("shared Soulseek/mesh TCP initialization timed out".to_owned());
            })
            .await;
            return;
        }
    };

    match incoming {
        SharedIncomingConnection::Soulseek(incoming) => {
            process_listener_incoming(&state, incoming, remote_addr, false).await;
        }
        SharedIncomingConnection::MeshOverlay(stream) => {
            process_shared_mesh_overlay_connection(state, stream, remote_addr).await;
        }
    }
}

async fn process_shared_mesh_overlay_connection(
    state: Arc<AppState>,
    stream: TcpStream,
    remote_addr: SocketAddr,
) {
    update_listeners(&state, |snapshot| {
        snapshot.regular_accepts += 1;
        snapshot.last_event = Some(format!(
            "mesh_overlay from {}",
            scrub_socket_addr(remote_addr)
        ));
        snapshot.last_error = None;
    })
    .await;
    if let Some(gateway) = state.private_gateway.clone() {
        gateway
            .handle_accepted_tcp(stream, Arc::clone(&state))
            .await;
    } else {
        update_listeners(&state, |snapshot| {
            snapshot.errors += 1;
            snapshot.last_error =
                Some("shared mesh connection received without an overlay gateway".to_owned());
        })
        .await;
    }
}

async fn handle_owned_incoming(
    state: Arc<AppState>,
    incoming: IncomingConnection<TcpStream>,
    remote_addr: SocketAddr,
) {
    let incoming_name = incoming_connection_name(&incoming);
    let result = match incoming {
        IncomingConnection::PeerMessages(peer) => {
            handle_plain_peer_messages_with_address(&state, peer, None, Some(remote_addr.ip()))
                .await
        }
        IncomingConnection::ObfuscatedPeerMessages(peer) => {
            let peer_username = peer.peer_username().map(str::to_owned);
            handle_obfuscated_peer_messages_with_address(
                &state,
                peer,
                peer_username,
                Some(remote_addr.ip()),
            )
            .await
        }
        IncomingConnection::PeerInit {
            kind: ConnectionKind::PeerMessages,
            username,
            stream,
            ..
        } => {
            handle_plain_peer_messages_with_address(
                &state,
                PeerMessageConnection::new(stream),
                Some(username),
                Some(remote_addr.ip()),
            )
            .await
        }
        IncomingConnection::FileTransfer(file) => {
            handle_inbound_file_transfer(&state, file, None).await
        }
        IncomingConnection::PeerInit {
            kind: ConnectionKind::FileTransfer,
            token,
            stream,
            obfuscated,
            ..
        } => {
            handle_inbound_file_transfer(
                &state,
                if obfuscated {
                    slskr_client::file_transfer::FileTransferConnection::new_obfuscated(stream)
                } else {
                    slskr_client::file_transfer::FileTransferConnection::new(stream)
                },
                (token != 0).then_some(token),
            )
            .await
        }
        IncomingConnection::PeerInit {
            kind: ConnectionKind::Distributed,
            username,
            stream,
            obfuscated,
            ..
        } => register_distributed_child(Arc::clone(&state), username, stream, obfuscated).await,
        IncomingConnection::Distributed(connection) => {
            let username = format!("direct-{}", remote_addr.ip());
            let (stream, obfuscated) = connection.into_parts();
            register_distributed_child(Arc::clone(&state), username, stream, obfuscated).await
        }
        IncomingConnection::PierceFirewall { token, stream } => {
            let has_file_transfer = {
                let transfers = state.transfers.read().await;
                transfers
                    .pending_inbound_file_transfer(Some(token))
                    .is_some()
            };
            if has_file_transfer {
                handle_inbound_file_transfer(
                    &state,
                    slskr_client::file_transfer::FileTransferConnection::new(stream),
                    Some(token),
                )
                .await
            } else {
                handle_plain_peer_messages_with_address(
                    &state,
                    PeerMessageConnection::new(stream),
                    None,
                    Some(remote_addr.ip()),
                )
                .await
            }
        }
        _ => Ok(()),
    };

    if let Err(error) = result {
        let level = incoming_peer_error_level(&error);
        let message = format!(
            "Incoming {incoming_name} from {} failed: {error}",
            scrub_socket_addr(remote_addr)
        );
        record_soulseek_diagnostic(&state, level, "soulseek", message).await;
        update_listeners(&state, |snapshot| {
            snapshot.errors += 1;
            if level >= logging::LogLevel::Warn {
                snapshot.last_error = Some(error);
            }
        })
        .await;
    }
}

fn incoming_peer_error_level(error: &str) -> logging::LogLevel {
    let error = error.to_ascii_lowercase();
    if [
        "unexpected end of input",
        "unexpected eof",
        "connection reset",
        "connection closed",
        "broken pipe",
        "timed out",
    ]
    .iter()
    .any(|marker| error.contains(marker))
    {
        logging::LogLevel::Debug
    } else {
        logging::LogLevel::Warn
    }
}

#[cfg(test)]
mod incoming_peer_error_tests {
    #[test]
    fn expected_peer_disconnects_are_debug_diagnostics() {
        assert_eq!(
            super::incoming_peer_error_level(
                "peer message receive failed: decode error: unexpected end of input"
            ),
            super::logging::LogLevel::Debug
        );
        assert_eq!(
            super::incoming_peer_error_level("peer authentication failed"),
            super::logging::LogLevel::Warn
        );
    }
}
