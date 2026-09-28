use super::*;

pub(in crate::cli) async fn live_soak() -> Result<(), String> {
    let config = LiveSoakConfig::from_env()?;
    let listener = Listener::bind(config.listener_bind.as_str())
        .await
        .map_err(|error| format!("listener bind failed: {error}"))?;
    let listener_address = listener
        .local_addr()
        .map_err(|error| format!("listener address failed: {error}"))?;
    let obfuscated_listener = if let Some(bind) = &config.obfuscated_listener_bind {
        let listener = Listener::bind(bind.as_str())
            .await
            .map_err(|error| format!("obfuscated listener bind failed: {error}"))?;
        let address = listener
            .local_addr()
            .map_err(|error| format!("obfuscated listener address failed: {error}"))?;
        Some((listener, address))
    } else {
        None
    };

    let connection = ServerConnection::connect(config.server_address.as_str())
        .await
        .map_err(|error| format!("connect failed: {error}"))?;
    let mut session = ServerSession::new(connection);
    let info = session
        .login(LoginCredentials::default_client(
            config.username.clone(),
            config.password.clone(),
        ))
        .await
        .map_err(|error| format!("login failed for configured user: {error}"))?;

    let advertised_port = if std::env::var("SLSK_SOAK_ADVERTISED_PORT").is_err()
        && config.listener_bind.rsplit_once(':').map(|(_, port)| port) == Some("0")
    {
        listener_address.port()
    } else {
        config.advertised_port
    };

    if let Some((_, obfuscated_address)) = &obfuscated_listener {
        let obfuscated_advertised_port = config
            .obfuscated_advertised_port
            .unwrap_or_else(|| obfuscated_address.port());
        session
            .set_wait_port_obfuscated(
                u32::from(advertised_port),
                ROTATED_OBFUSCATION_TYPE,
                u32::from(obfuscated_advertised_port),
            )
            .await
            .map_err(|error| format!("set obfuscated wait port failed: {error}"))?;
    } else {
        session
            .set_wait_port(u32::from(advertised_port))
            .await
            .map_err(|error| format!("set wait port failed: {error}"))?;
    }
    session
        .send_server_message(ServerMessage::SetStatus { status: 2 })
        .await
        .map_err(|error| format!("set status failed: {error}"))?;
    session
        .send_server_message(ServerMessage::SharedFoldersFiles {
            folders: config.shared_folders,
            files: config.shared_files,
        })
        .await
        .map_err(|error| format!("share count update failed: {error}"))?;
    session
        .send_server_message(ServerMessage::CheckPrivilegesRequest)
        .await
        .map_err(|error| format!("check privileges failed: {error}"))?;
    session
        .send_ping()
        .await
        .map_err(|error| format!("initial ping failed: {error}"))?;

    if let Some(peer) = &config.peer_username {
        for message in peer_probe_messages(peer) {
            session
                .send_server_message(message)
                .await
                .map_err(|error| format!("peer probe failed: {error}"))?;
        }
    }

    if config.active_probes {
        session
            .send_server_message(ServerMessage::RoomListRequest)
            .await
            .map_err(|error| format!("room list refresh failed: {error}"))?;
        println!("active probe: room list requested");
        if let Some(query) = &config.search_query {
            dispatch_live_soak_search(&mut session, query, config.search_token).await?;
        }
    }

    println!(
        "live soak started; supporter={}; listener={}; advertised_port={}; obfuscated_port={}; duration_seconds={}; search_enabled={}",
        info.is_supporter,
        listener_address,
        advertised_port,
        obfuscated_listener
            .as_ref()
            .map(|(_, address)| {
                config
                    .obfuscated_advertised_port
                    .unwrap_or_else(|| address.port())
                    .to_string()
            })
            .unwrap_or_else(|| "disabled".to_owned()),
        config.duration.as_secs(),
        config.search_query.is_some()
    );

    let listener_duration = config.duration;
    let listener_task =
        tokio::spawn(async move { run_listener(listener, listener_duration).await });
    let obfuscated_listener_task = obfuscated_listener.map(|(listener, _)| {
        let duration = config.duration;
        tokio::spawn(async move { run_obfuscated_listener(listener, duration).await })
    });
    let server_progress = Arc::new(AtomicU64::new(unix_seconds()));
    let watchdog_task = tokio::spawn(run_live_soak_server_watchdog(
        server_progress.clone(),
        config.duration,
        config.watchdog_interval,
        config.watchdog_stale_seconds,
    ));
    let server_result = run_server_soak(&mut session, &config, server_progress).await;
    watchdog_task.abort();
    let listener_result = listener_task
        .await
        .map_err(|error| format!("listener task failed: {error}"))?;
    let obfuscated_listener_result = if let Some(task) = obfuscated_listener_task {
        Some(
            task.await
                .map_err(|error| format!("obfuscated listener task failed: {error}"))?,
        )
    } else {
        None
    };

    server_result?;
    listener_result?;
    if let Some(result) = obfuscated_listener_result {
        result?;
    }
    println!("live soak completed");
    Ok(())
}
