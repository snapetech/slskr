use super::*;

pub(super) fn spawn_vpn_polling(state: Arc<AppState>) {
    if !state.config.integrations.vpn.enabled {
        return;
    }
    let interval = Duration::from_millis(state.config.integrations.vpn.polling_interval);
    let task_state = Arc::clone(&state);
    state.spawn_managed_task(async move {
        let state = task_state;
        let mut timer = time::interval_at(Instant::now() + interval, interval);
        loop {
            timer.tick().await;
            let options = state.integration_settings.read().await.vpn.clone();
            let previous_ready = state.runtime.read().await.vpn.is_ready;
            let status = match vpn::poll_once(&options, state.config.controller_profile).await {
                Ok(status) => status,
                Err(error) => {
                    record_daemon_log(
                        &state,
                        logging::LogLevel::Warn,
                        "vpn",
                        format!("Failed to fetch status from VPN client Gluetun: {error}"),
                    )
                    .await;
                    vpn::Status::default()
                }
            };
            synchronize_soulseek_advertised_port(&state, &status).await;
            if let Some(dht) = state.dht.as_ref() {
                let dht_settings = state.advanced_networking.read().await.dht.clone();
                let synchronized = match dht_settings.vpn_port_sync.as_str() {
                    "primary" => status.forwarded_port,
                    "target_port" => status
                        .port_forwards
                        .iter()
                        .find(|forward| {
                            u16::try_from(forward.local_port).ok()
                                == Some(dht_settings.overlay_port)
                        })
                        .and_then(|forward| u16::try_from(forward.public_port).ok()),
                    _ => None,
                };
                dht.set_advertised_overlay_port(
                    synchronized.unwrap_or_else(|| dht_settings.effective_overlay_port()),
                );
            }
            let reconnect = {
                let mut runtime = state.runtime.write().await;
                runtime.vpn = status.clone();
                if !status.is_ready && previous_ready && state.config.reconnect {
                    runtime.vpn_reconnect_requested = true;
                }
                let reconnect = status.is_ready && runtime.vpn_reconnect_requested;
                if reconnect {
                    runtime.vpn_reconnect_requested = false;
                }
                reconnect
            };
            if !status.is_ready {
                if let Err(error) =
                    send_session_command(&state, SessionCommand::VpnDisconnect).await
                {
                    record_daemon_log(
                        &state,
                        logging::LogLevel::Warn,
                        "vpn",
                        format!("failed to publish VPN disconnect command: {error}"),
                    )
                    .await;
                }
            } else if reconnect {
                if let Err(error) = send_session_command(&state, SessionCommand::Connect).await {
                    record_daemon_log(
                        &state,
                        logging::LogLevel::Warn,
                        "vpn",
                        format!("failed to publish VPN reconnect command: {error}"),
                    )
                    .await;
                }
            }
        }
    });
}

/// Keep the server's public Soulseek endpoint advertisement aligned with a
/// VPN's forwarded port. The local listener remains bound to the configured
/// port; only the metadata sent to the server changes, matching the current
/// upstream VPN integration behavior.
pub(super) async fn synchronize_soulseek_advertised_port(
    state: &Arc<AppState>,
    status: &vpn::Status,
) {
    let configured_port = state.config.advertised_port;
    let desired_port = if status.is_ready {
        status
            .forwarded_port
            .map(u32::from)
            .unwrap_or(configured_port)
    } else {
        configured_port
    };

    let (regular_changed, obfuscated_changed) = {
        let current_regular = *state
            .advertised_port
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let current_obfuscated = *state
            .obfuscated_advertised_port
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let obfuscated_shares_regular = state.config.obfuscation_enabled
            && state.config.obfuscation_listen_port == 0
            && state.config.obfuscated_listener_bind.is_none()
            && current_obfuscated == Some(current_regular);

        let regular_changed = current_regular != desired_port;
        if regular_changed {
            *state
                .advertised_port
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = desired_port;
        }

        let obfuscated_changed =
            if obfuscated_shares_regular && current_obfuscated != Some(desired_port) {
                *state
                    .obfuscated_advertised_port
                    .write()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(desired_port);
                true
            } else {
                false
            };
        (regular_changed, obfuscated_changed)
    };

    if !(regular_changed || obfuscated_changed)
        || !status.is_ready
        || state.session.read().await.state != "connected"
    {
        return;
    }

    if let Err(error) = send_session_command(
        state,
        SessionCommand::SetWaitPort {
            port: desired_port,
            obfuscated_port: effective_obfuscated_advertised_port(state),
        },
    )
    .await
    {
        record_daemon_log(
            state,
            logging::LogLevel::Warn,
            "vpn",
            format!("failed to update the Soulseek VPN public port advertisement: {error}"),
        )
        .await;
    }
}

pub(super) enum ReconnectWake {
    DelayElapsed,
    Command(SessionCommand),
    ChannelClosed,
}

pub(super) async fn wait_for_reconnect_or_command(
    receiver: &mut mpsc::Receiver<SessionCommand>,
    delay: Duration,
) -> ReconnectWake {
    tokio::select! {
        () = time::sleep(delay) => ReconnectWake::DelayElapsed,
        command = receiver.recv() => command
            .map(ReconnectWake::Command)
            .unwrap_or(ReconnectWake::ChannelClosed),
    }
}
