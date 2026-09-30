use super::*;

pub(crate) async fn send_room_join_if_connected(state: &AppState, room_name: String) {
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

pub(crate) async fn send_room_leave_if_connected(state: &AppState, room_name: String) {
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

pub(crate) async fn record_room_dispatch_failure(
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

pub(crate) async fn persist_room_join_checked(
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

pub(crate) async fn persist_room_leave_checked(
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

pub(crate) async fn record_pod_room_mirror_failure(state: &AppState, room_name: &str, error: &str) {
    let reason = format!("pod room mirror for {room_name} dispatch failed: {error}");
    update_session(state, |snapshot| {
        snapshot.last_error = Some(reason.clone());
    })
    .await;
    record_daemon_log(state, logging::LogLevel::Error, "podcore", reason).await;
}
