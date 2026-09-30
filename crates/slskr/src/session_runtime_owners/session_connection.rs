use super::*;

pub(crate) async fn connect_session(
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
