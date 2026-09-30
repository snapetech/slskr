// Distributed parent/child transport and search runtime.
use super::*;

pub(super) async fn run_distributed_persistence_worker(
    db: Option<crate::persistence::DatabaseManager>,
    mut snapshots: watch::Receiver<DistributedPersistenceSnapshot>,
    status: watch::Sender<DistributedPersistenceStatus>,
) {
    let mut snapshot = snapshots.borrow_and_update().clone();
    loop {
        if snapshot.revision > 0 {
            let result = match db.as_ref() {
                Some(db) => snapshot.save(db).await,
                None => Ok(()),
            };
            status.send_replace(DistributedPersistenceStatus {
                revision: snapshot.revision,
                result,
            });
        }
        if snapshots.changed().await.is_err() {
            break;
        }
        snapshot = snapshots.borrow_and_update().clone();
    }
}

pub(super) async fn update_distributed_runtime<R>(
    state: &AppState,
    update: impl FnOnce(&mut DistributedRuntime) -> (R, bool),
) -> (R, Result<(), String>) {
    let (result, revision) = {
        let mut runtime = state.distributed_network.write().await;
        let (result, changed) = update(&mut runtime);
        if !changed {
            return (result, Ok(()));
        }
        runtime.persistence_revision = runtime.persistence_revision.saturating_add(1);
        let snapshot = runtime.persistence_snapshot();
        state
            .distributed_persistence_snapshots
            .send_replace(snapshot.clone());
        (result, snapshot.revision)
    };

    let mut status = state.distributed_persistence_status.clone();
    loop {
        let current = status.borrow_and_update().clone();
        if current.revision >= revision {
            return (result, current.result);
        }
        if status.changed().await.is_err() {
            return (
                result,
                Err("distributed persistence worker stopped before saving the latest state".into()),
            );
        }
    }
}

pub(super) async fn hydrate_distributed_runtime(
    runtime: &RwLock<DistributedRuntime>,
    db: &crate::persistence::DatabaseManager,
) -> Result<(), String> {
    let (tree_state, children) = DistributedRuntime::load_persisted_state(db).await?;
    let mut runtime = runtime.write().await;
    if runtime.persistence_revision == 0 {
        runtime.restore_persisted_state(tree_state, children);
    }
    Ok(())
}

pub(super) async fn connect_distributed_parent(
    state: Arc<AppState>,
    candidates: &[slskr_client::protocol::server::PossibleParent],
) -> Result<(), String> {
    if state.soulseek_distributed_settings.read().await.disabled
        || state.distributed_network.read().await.parent.is_some()
    {
        return Ok(());
    }
    let local_username = outgoing_peer_init_username(&state).await?;
    let parent = candidates
        .iter()
        .filter(|candidate| !candidate.username.eq_ignore_ascii_case(&local_username))
        .filter(|candidate| {
            candidate.port > 0
                && candidate.port <= u16::MAX as u32
                && !candidate.ip.is_unspecified()
                && !candidate.ip.is_multicast()
                && !candidate.ip.is_broadcast()
        })
        .min_by(|left, right| {
            left.username
                .cmp(&right.username)
                .then_with(|| left.ip.octets().cmp(&right.ip.octets()))
                .then_with(|| left.port.cmp(&right.port))
        })
        .ok_or_else(|| "Soulseek server returned no usable distributed parent".to_owned())?;
    let port = u16::try_from(parent.port)
        .map_err(|_| "distributed parent port is out of range".to_owned())?;
    let address = state.config.distributed_parent_override.unwrap_or_else(|| {
        SocketAddr::V4(SocketAddrV4::new(
            state.config.peer_host_override.unwrap_or(parent.ip),
            port,
        ))
    });
    let stream = connect_soulseek_tcp(&state, address, SoulseekSocketClass::Control).await?;
    let stream = time::timeout(
        state.config.soulseek_connection.timeout_inactivity,
        send_peer_init(stream, local_username, ConnectionKind::Distributed),
    )
    .await
    .map_err(|_| "distributed parent initialization timed out".to_owned())?
    .map_err(|error| format!("distributed parent initialization failed: {error}"))?;
    let (sender, receiver) = mpsc::channel(state.config.soulseek_connection.buffer_write_queue);
    let (connected, persistence_result) = update_distributed_runtime(&state, |runtime| {
        if runtime.parent.is_some() {
            return (false, false);
        }
        runtime.parent = Some(parent.username.clone());
        runtime.parent_sender = Some(sender.clone());
        runtime.branch_level = 1;
        runtime.branch_root.clone_from(&parent.username);
        (true, true)
    })
    .await;
    if !connected {
        return Ok(());
    }
    if let Err(error) = persistence_result {
        record_distributed_persistence_failure(
            &state,
            "distributed parent state save failed",
            error,
        )
        .await;
    }
    record_soulseek_diagnostic(
        &state,
        logging::LogLevel::Info,
        "distributed",
        format!(
            "Connected distributed parent {}",
            redact_username(&parent.username)
        ),
    )
    .await;
    notify_distributed_branch(&state).await;
    state.spawn_managed_task(run_distributed_link(
        Arc::clone(&state),
        parent.username.clone(),
        DistributedConnectionRole::Parent,
        stream,
        receiver,
        false,
    ));
    Ok(())
}

pub(super) async fn register_distributed_child(
    state: Arc<AppState>,
    username: String,
    stream: TcpStream,
    obfuscated: bool,
) -> Result<(), String> {
    let username = username.trim().to_owned();
    let settings = *state.soulseek_distributed_settings.read().await;
    if settings.disabled {
        return Err("distributed network is disabled".to_owned());
    }
    if settings.disable_children {
        return Err("distributed child connections are disabled".to_owned());
    }
    if username.is_empty() || username.chars().any(char::is_control) {
        return Err("distributed child username is invalid".to_owned());
    }
    let key = username.to_ascii_lowercase();
    let (sender, receiver) = mpsc::channel(state.config.soulseek_connection.buffer_write_queue);
    let (registration, persistence_result) = update_distributed_runtime(&state, |runtime| {
        if username.eq_ignore_ascii_case(&runtime.local_username)
            || runtime
                .parent
                .as_deref()
                .is_some_and(|parent| parent.eq_ignore_ascii_case(&username))
        {
            return (
                Err("distributed child would create an identity loop".to_owned()),
                false,
            );
        }
        if runtime.children.len() >= settings.child_limit && !runtime.children.contains_key(&key) {
            return (Err("distributed child capacity is full".to_owned()), false);
        }
        runtime.children.insert(key.clone(), sender.clone());
        runtime.child_depths.insert(key.clone(), 0);
        (
            Ok((runtime.branch_level, runtime.branch_root.clone())),
            true,
        )
    })
    .await;
    let (level, root) = registration?;
    if let Err(error) = persistence_result {
        record_distributed_persistence_failure(
            &state,
            "distributed child state save failed",
            error,
        )
        .await;
    }
    state.spawn_managed_task(run_distributed_link(
        Arc::clone(&state),
        username.clone(),
        DistributedConnectionRole::Child,
        stream,
        receiver,
        obfuscated,
    ));
    for message in [
        DistributedMessage::BranchLevel { level },
        DistributedMessage::BranchRoot { username: root },
    ] {
        sender
            .send(message)
            .await
            .map_err(|_| "distributed child closed during initialization".to_owned())?;
    }
    notify_distributed_child_depth(&state).await;
    record_soulseek_diagnostic(
        &state,
        logging::LogLevel::Info,
        "distributed",
        format!("Accepted distributed child {}", redact_username(&username)),
    )
    .await;
    Ok(())
}

async fn run_distributed_link(
    state: Arc<AppState>,
    username: String,
    role: DistributedConnectionRole,
    stream: TcpStream,
    mut receiver: mpsc::Receiver<DistributedMessage>,
    obfuscated: bool,
) {
    let (read_half, write_half) = stream.into_split();
    let mut reader = if obfuscated {
        DistributedConnection::new_obfuscated(read_half)
    } else {
        DistributedConnection::new(read_half)
    };
    let mut writer = if obfuscated {
        DistributedConnection::new_obfuscated(write_half)
    } else {
        DistributedConnection::new(write_half)
    };
    loop {
        tokio::select! {
            received = time::timeout(
                state.config.soulseek_connection.timeout_inactivity,
                reader.receive(),
            ) => match received {
                Ok(Ok(message)) => handle_distributed_message(&state, &username, role, message).await,
                Ok(Err(error)) => {
                    record_soulseek_diagnostic(
                        &state,
                        logging::LogLevel::Debug,
                        "distributed",
                        format!(
                            "Distributed {} {} receive failed: {error}",
                            if role == DistributedConnectionRole::Parent {
                                "parent"
                            } else {
                                "child"
                            },
                            redact_username(&username)
                        ),
                    )
                    .await;
                    break;
                }
                Err(_) => break,
            },
            outbound = receiver.recv() => match outbound {
                Some(message) => {
                    if time::timeout(
                        state.config.soulseek_connection.timeout_inactivity,
                        writer.send(&message),
                    ).await.is_err() {
                        break;
                    }
                }
                None => break,
            }
        }
    }
    let (_, persistence_result) = update_distributed_runtime(&state, |runtime| {
        let changed = match role {
            DistributedConnectionRole::Parent => {
                if runtime
                    .parent
                    .as_deref()
                    .is_some_and(|parent| parent.eq_ignore_ascii_case(&username))
                {
                    runtime.reset_branch();
                    true
                } else {
                    false
                }
            }
            DistributedConnectionRole::Child => {
                let key = username.to_ascii_lowercase();
                runtime.children.remove(&key);
                runtime.child_depths.remove(&key);
                true
            }
        };
        ((), changed)
    })
    .await;
    if let Err(error) = persistence_result {
        record_distributed_persistence_failure(
            &state,
            "distributed disconnect state save failed",
            error,
        )
        .await;
    }
    if role == DistributedConnectionRole::Parent {
        notify_distributed_branch(&state).await;
    } else {
        notify_distributed_child_depth(&state).await;
    }
    record_soulseek_diagnostic(
        &state,
        logging::LogLevel::Debug,
        "distributed",
        format!(
            "Distributed {} {} disconnected",
            if role == DistributedConnectionRole::Parent {
                "parent"
            } else {
                "child"
            },
            redact_username(&username)
        ),
    )
    .await;
}

pub(super) async fn handle_distributed_message(
    state: &Arc<AppState>,
    username: &str,
    role: DistributedConnectionRole,
    message: DistributedMessage,
) {
    match message {
        DistributedMessage::Ping => {
            let response = DistributedMessage::PingResponse {
                token: next_distributed_ping_token(),
            };
            if let Err(error) = send_distributed_message(state, username, role, response).await {
                record_soulseek_diagnostic(
                    state,
                    logging::LogLevel::Debug,
                    "distributed",
                    format!(
                        "Distributed ping response to {} failed: {error}",
                        redact_username(username)
                    ),
                )
                .await;
            }
        }
        DistributedMessage::Search(search) => {
            record_soulseek_diagnostic(
                state,
                logging::LogLevel::Debug,
                "search",
                format!(
                    "Received distributed public search from {} for {:?}",
                    redact_username(&search.username),
                    search.query
                ),
            )
            .await;
            forward_distributed_search(state, role, username, &search).await;
            respond_to_distributed_search(Arc::clone(state), search).await;
        }
        DistributedMessage::EmbeddedMessage { code, payload } => {
            handle_embedded_distributed_search(state, Some(role), Some(username), code, &payload)
                .await;
        }
        DistributedMessage::BranchLevel { level } if role == DistributedConnectionRole::Parent => {
            let (_, persistence_result) = update_distributed_runtime(state, |runtime| {
                runtime.branch_level = level.saturating_add(1);
                ((), true)
            })
            .await;
            if let Err(error) = persistence_result {
                record_distributed_persistence_failure(
                    state,
                    "distributed branch-level state save failed",
                    error,
                )
                .await;
            }
            notify_distributed_branch(state).await;
        }
        DistributedMessage::BranchRoot { username: root }
            if role == DistributedConnectionRole::Parent && !root.trim().is_empty() =>
        {
            let (updated, persistence_result) = update_distributed_runtime(state, |runtime| {
                if root.eq_ignore_ascii_case(&runtime.local_username) {
                    return (false, false);
                }
                runtime.branch_root = root;
                (true, true)
            })
            .await;
            if !updated {
                return;
            }
            if let Err(error) = persistence_result {
                record_distributed_persistence_failure(
                    state,
                    "distributed branch-root state save failed",
                    error,
                )
                .await;
            }

            notify_distributed_branch(state).await;
        }
        DistributedMessage::ChildDepth { depth } if role == DistributedConnectionRole::Child => {
            let (_, persistence_result) = update_distributed_runtime(state, |runtime| {
                runtime
                    .child_depths
                    .insert(username.to_ascii_lowercase(), depth);
                ((), true)
            })
            .await;
            if let Err(error) = persistence_result {
                record_distributed_persistence_failure(
                    state,
                    "distributed child-depth state save failed",
                    error,
                )
                .await;
            }
            notify_distributed_child_depth(state).await;
        }
        DistributedMessage::PingResponse { .. }
        | DistributedMessage::EmbeddedServerMessage(_)
        | DistributedMessage::Unknown { .. }
        | DistributedMessage::BranchLevel { .. }
        | DistributedMessage::BranchRoot { .. }
        | DistributedMessage::ChildDepth { .. } => {}
    }
}

pub(super) async fn handle_embedded_distributed_search(
    state: &Arc<AppState>,
    role: Option<DistributedConnectionRole>,
    source_username: Option<&str>,
    code: u8,
    payload: &[u8],
) {
    let message = match DistributedMessage::decode(InitFrame::new(code, payload.to_vec())) {
        Ok(message) => message,
        Err(error) => {
            record_soulseek_diagnostic(
                state,
                logging::LogLevel::Debug,
                "distributed",
                format!("Failed to decode embedded distributed message {code}: {error}"),
            )
            .await;
            return;
        }
    };
    let DistributedMessage::Search(search) = message else {
        record_soulseek_diagnostic(
            state,
            logging::LogLevel::Trace,
            "distributed",
            format!("Ignored embedded distributed message {code}"),
        )
        .await;
        return;
    };

    record_soulseek_diagnostic(
        state,
        logging::LogLevel::Debug,
        "search",
        format!(
            "Received embedded distributed public search from {} for {:?}",
            source_username
                .map(redact_username)
                .unwrap_or_else(|| "server".to_owned()),
            search.query
        ),
    )
    .await;

    if role.is_none_or(|role| role == DistributedConnectionRole::Parent) {
        broadcast_distributed_search(state, &search, source_username).await;
    }

    let local_username = state
        .distributed_network
        .read()
        .await
        .local_username
        .clone();
    if search.username.eq_ignore_ascii_case(&local_username) {
        return;
    }
    respond_to_distributed_search(Arc::clone(state), search).await;
}

async fn send_distributed_message(
    state: &Arc<AppState>,
    username: &str,
    role: DistributedConnectionRole,
    message: DistributedMessage,
) -> Result<(), String> {
    let sender = {
        let runtime = state.distributed_network.read().await;
        match role {
            DistributedConnectionRole::Parent => runtime.parent_sender.clone(),
            DistributedConnectionRole::Child => runtime
                .children
                .get(&username.to_ascii_lowercase())
                .cloned(),
        }
    };
    let Some(sender) = sender else {
        return Err("distributed connection sender is unavailable".to_owned());
    };
    sender
        .send(message)
        .await
        .map_err(|_| "distributed connection closed before ping response".to_owned())
}

async fn forward_distributed_search(
    state: &AppState,
    role: DistributedConnectionRole,
    source_username: &str,
    search: &DistributedSearch,
) {
    if role != DistributedConnectionRole::Parent {
        return;
    }
    broadcast_distributed_search(state, search, Some(source_username)).await;
}

async fn broadcast_distributed_search(
    state: &AppState,
    search: &DistributedSearch,
    source_username: Option<&str>,
) {
    let senders = state
        .distributed_network
        .read()
        .await
        .children
        .iter()
        .filter(|(username, _)| {
            source_username.is_none_or(|source| !username.eq_ignore_ascii_case(source))
        })
        .map(|(_, sender)| sender.clone())
        .collect::<Vec<_>>();
    for sender in senders {
        let _ = sender.try_send(DistributedMessage::Search(search.clone()));
    }
}

async fn respond_to_distributed_search(state: Arc<AppState>, search: DistributedSearch) {
    let Some(response) = build_file_search_response(&state, search.token, &search.query).await
    else {
        record_soulseek_diagnostic(
            &state,
            logging::LogLevel::Debug,
            "search",
            format!(
                "Distributed public search for {:?} matched no shared files",
                search.query
            ),
        )
        .await;
        return;
    };
    let address = match request_peer_endpoint(&state, &search.username).await {
        Ok(address) => address,
        Err(error) => {
            record_soulseek_diagnostic(
                &state,
                logging::LogLevel::Warn,
                "search",
                format!(
                    "Distributed public search response endpoint lookup for {} failed: {error}",
                    redact_username(&search.username)
                ),
            )
            .await;
            return;
        }
    };
    match send_peer_message_oneway(&state, &address, PeerMessage::FileSearchResponse(response))
        .await
    {
        Ok(()) => {
            record_soulseek_diagnostic(
                &state,
                logging::LogLevel::Debug,
                "search",
                format!(
                    "Sent distributed public search response to {} for {:?}",
                    redact_username(&search.username),
                    search.query
                ),
            )
            .await;
        }
        Err(error) => {
            record_soulseek_diagnostic(
                &state,
                logging::LogLevel::Warn,
                "search",
                format!(
                    "Distributed public search response to {} failed: {error}",
                    redact_username(&search.username)
                ),
            )
            .await;
        }
    }
}

pub(super) async fn notify_distributed_branch(state: &AppState) {
    let settings = *state.soulseek_distributed_settings.read().await;
    let (has_parent, level, root, children, child_count) = {
        let runtime = state.distributed_network.read().await;
        (
            runtime.parent.is_some(),
            runtime.branch_level,
            runtime.branch_root.clone(),
            runtime.children.values().cloned().collect::<Vec<_>>(),
            runtime.children.len(),
        )
    };
    for child in children {
        let _ = child.try_send(DistributedMessage::BranchLevel { level });
        let _ = child.try_send(DistributedMessage::BranchRoot {
            username: root.clone(),
        });
    }
    let _ = state
        .session_commands
        .try_send(SessionCommand::DistributedBranch {
            has_parent,
            accept_children: !settings.disabled
                && !settings.disable_children
                && child_count < settings.child_limit,
            level,
            root,
        });
}

pub(super) async fn reset_distributed_network(state: &AppState, username: Option<&str>) {
    let (_, persistence_result) = update_distributed_runtime(state, |runtime| {
        if let Some(username) = username {
            runtime.local_username = username.trim().to_owned();
        }
        runtime.children.clear();
        runtime.child_depths.clear();
        runtime.reset_branch();
        ((), true)
    })
    .await;
    if let Err(error) = persistence_result {
        record_distributed_persistence_failure(state, "distributed reset state save failed", error)
            .await;
    }
}

pub(super) async fn apply_distributed_settings(
    state: &AppState,
    settings: crate::config::SoulseekDistributedSettings,
) {
    let previous = {
        let mut current = state.soulseek_distributed_settings.write().await;
        let previous = *current;
        *current = settings;
        previous
    };
    if previous == settings {
        return;
    }
    let (_, persistence_result) = update_distributed_runtime(state, |runtime| {
        if settings.disabled {
            runtime.children.clear();
            runtime.child_depths.clear();
            runtime.reset_branch();
        } else {
            if settings.disable_children {
                runtime.children.clear();
                runtime.child_depths.clear();
            } else if runtime.children.len() > settings.child_limit {
                let excess = runtime
                    .children
                    .keys()
                    .skip(settings.child_limit)
                    .cloned()
                    .collect::<Vec<_>>();
                for username in excess {
                    runtime.children.remove(&username);
                    runtime.child_depths.remove(&username);
                }
            }
        }
        ((), true)
    })
    .await;
    if let Err(error) = persistence_result {
        record_distributed_persistence_failure(
            state,
            "distributed settings state save failed",
            error,
        )
        .await;
    }
    notify_distributed_branch(state).await;
    notify_distributed_child_depth(state).await;
}

pub(super) async fn notify_distributed_child_depth(state: &AppState) {
    let (parent, depth) = {
        let runtime = state.distributed_network.read().await;
        (runtime.parent_sender.clone(), runtime.child_depth())
    };
    if let Some(parent) = parent {
        let _ = parent.try_send(DistributedMessage::ChildDepth { depth });
    }
}
