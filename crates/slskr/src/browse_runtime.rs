use super::*;

#[cfg(feature = "full-controller-tests")]
pub(super) async fn persist_browse_projection(state: &AppState, record: &BrowseRecord) {
    if let Err(error) = persist_browse_record_checked(state, record).await {
        report_browse_projection_failure(state, record, error).await;
    }
}

pub(super) async fn report_browse_projection_failure(
    state: &AppState,
    record: &BrowseRecord,
    error: String,
) {
    update_session(state, |snapshot| {
        snapshot.last_error = Some(format!(
            "browse persistence for {} failed: {error}",
            redact_username(&record.username)
        ));
    })
    .await;
}

pub(super) async fn project_peer_browse_response(state: &AppState, address: &PeerAddress) {
    let requested_folder = {
        let browse = state.browse.read().await;
        browse.requested_folder(&address.username)
    };
    let Some(requested_folder) = requested_folder else {
        return;
    };

    let result = if let Some(folder) = requested_folder {
        fetch_peer_folder(state, address, folder).await
    } else {
        fetch_peer_browse(state, address).await
    };

    match result {
        Ok(entries) => {
            state
                .remote_path_encodings
                .write()
                .await
                .remember_browse_entries(&address.username, &entries);
            let _browse_persistence = state.browse_persistence_lock.lock().await;
            let mut browse = state.browse.write().await;
            let record = browse.add_entries(address.username.clone(), entries, true);
            drop(browse);
            let persistence_failure = if let Some(record) = record.as_ref() {
                persist_browse_record_checked(state, record)
                    .await
                    .err()
                    .map(|error| (record.clone(), error))
            } else {
                None
            };
            drop(_browse_persistence);
            if let Some((record, error)) = persistence_failure {
                report_browse_projection_failure(state, &record, error).await;
            }
        }
        Err(error) => {
            let _browse_persistence = state.browse_persistence_lock.lock().await;
            let mut browse = state.browse.write().await;
            let token = browse
                .mark_indirect_pending(&address.username, format!("direct browse failed: {error}"));
            let pending_record = browse.get(&address.username);
            drop(browse);
            let persistence_failure = if let Some(record) = pending_record.as_ref() {
                persist_browse_record_checked(state, record)
                    .await
                    .err()
                    .map(|error| (record.clone(), error))
            } else {
                None
            };
            drop(_browse_persistence);
            if let Some((record, error)) = persistence_failure {
                report_browse_projection_failure(state, &record, error).await;
            }
            if let Some(token) = token {
                if let Err(error) = try_send_session_command(
                    state,
                    SessionCommand::IndirectBrowse {
                        username: address.username.clone(),
                        token,
                    },
                ) {
                    let _browse_persistence = state.browse_persistence_lock.lock().await;
                    let failed = {
                        let mut browse = state.browse.write().await;
                        browse.fail(address.username.clone(), error.clone())
                    };
                    let persistence_failure = if let Some(record) = failed.as_ref() {
                        persist_browse_record_checked(state, record)
                            .await
                            .err()
                            .map(|error| (record.clone(), error))
                    } else {
                        None
                    };
                    drop(_browse_persistence);
                    if let Some((record, error)) = persistence_failure {
                        report_browse_projection_failure(state, &record, error).await;
                    }
                    record_event(
                        state,
                        "browse.failed",
                        address.username.clone(),
                        Some(error.clone()),
                    )
                    .await;
                    update_session(state, |snapshot| {
                        snapshot.last_error = Some(format!(
                            "indirect browse {} dispatch failed: {error}",
                            redact_username(&address.username)
                        ));
                    })
                    .await;
                    return;
                }
                record_event(
                    state,
                    "browse.indirect.requested",
                    address.username.clone(),
                    Some(format!("token {token}")),
                )
                .await;
            } else {
                let _browse_persistence = state.browse_persistence_lock.lock().await;
                let mut browse = state.browse.write().await;
                let record = browse.fail(address.username.clone(), error.clone());
                drop(browse);
                let persistence_failure = if let Some(record) = record.as_ref() {
                    persist_browse_record_checked(state, record)
                        .await
                        .err()
                        .map(|error| (record.clone(), error))
                } else {
                    None
                };
                drop(_browse_persistence);
                if let Some((record, error)) = persistence_failure {
                    report_browse_projection_failure(state, &record, error).await;
                }
                record_event(
                    state,
                    "browse.failed",
                    address.username.clone(),
                    Some(error.clone()),
                )
                .await;
                update_session(state, |snapshot| {
                    snapshot.last_error = Some(format!(
                        "browse {} failed: {error}",
                        redact_username(&address.username)
                    ));
                })
                .await;
            }
        }
    }
}

pub(super) async fn project_indirect_browse_response(
    state: &AppState,
    response: &ConnectToPeerResponse,
) {
    let Ok(kind) = ConnectionKind::try_from_connection_type(&response.connection_type) else {
        return;
    };
    if kind != ConnectionKind::PeerMessages {
        return;
    }
    let requested_folder = {
        let browse = state.browse.read().await;
        browse.pending_indirect(&response.username, response.token)
    };
    let Some(requested_folder) = requested_folder else {
        return;
    };

    let result = if let Some(folder) = requested_folder {
        fetch_indirect_peer_folder(state, response, folder).await
    } else {
        fetch_indirect_peer_browse(state, response).await
    };
    match result {
        Ok(entries) => {
            state
                .remote_path_encodings
                .write()
                .await
                .remember_browse_entries(&response.username, &entries);
            let _browse_persistence = state.browse_persistence_lock.lock().await;
            let mut browse = state.browse.write().await;
            let record = browse.add_entries(response.username.clone(), entries, true);
            drop(browse);
            let persistence_failure = if let Some(record) = record.as_ref() {
                persist_browse_record_checked(state, record)
                    .await
                    .err()
                    .map(|error| (record.clone(), error))
            } else {
                None
            };
            drop(_browse_persistence);
            if let Some((record, error)) = persistence_failure {
                report_browse_projection_failure(state, &record, error).await;
            }
        }
        Err(error) => {
            let _browse_persistence = state.browse_persistence_lock.lock().await;
            let mut browse = state.browse.write().await;
            let record = browse.fail(response.username.clone(), error.clone());
            drop(browse);
            let persistence_failure = if let Some(record) = record.as_ref() {
                persist_browse_record_checked(state, record)
                    .await
                    .err()
                    .map(|error| (record.clone(), error))
            } else {
                None
            };
            drop(_browse_persistence);
            if let Some((record, error)) = persistence_failure {
                report_browse_projection_failure(state, &record, error).await;
            }
            record_event(
                state,
                "browse.failed",
                response.username.clone(),
                Some(error.clone()),
            )
            .await;
            update_session(state, |snapshot| {
                snapshot.last_error = Some(format!(
                    "indirect browse {} failed: {error}",
                    redact_username(&response.username)
                ));
            })
            .await;
        }
    }
}
