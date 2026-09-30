use super::*;

pub(super) async fn send_session_ping(
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

pub(super) async fn send_active_server_message(
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

pub(crate) fn is_remote_queue_response(reason: &str) -> bool {
    reason
        .trim()
        .trim_end_matches('.')
        .eq_ignore_ascii_case("queued")
}

pub(crate) async fn send_session_command(
    state: &AppState,
    command: SessionCommand,
) -> Result<(), String> {
    state
        .session_commands
        .send(command)
        .await
        .map_err(|_| "session manager is not running".to_owned())
}

pub(crate) async fn send_active_interest_command(
    state: &AppState,
    message: ServerMessage,
) -> Result<(), String> {
    if state.session.read().await.state != "connected" {
        return Err("Soulseek session is disconnected".to_owned());
    }
    send_session_command(state, SessionCommand::SendServerMessage(message)).await
}

pub(crate) fn try_send_session_command(
    state: &AppState,
    command: SessionCommand,
) -> Result<(), String> {
    state
        .session_commands
        .try_send(command)
        .map_err(|error| format!("session command queue rejected request: {error}"))
}

pub(crate) async fn update_session<F>(state: &AppState, update: F)
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

pub(crate) async fn update_listeners<F>(state: &AppState, update: F)
where
    F: FnOnce(&mut ListenerSnapshot),
{
    let mut snapshot = state.listeners.write().await;
    update(&mut snapshot);
    snapshot.updated_at = unix_timestamp();
}
