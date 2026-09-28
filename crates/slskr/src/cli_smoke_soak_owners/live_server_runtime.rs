use super::*;

pub(super) async fn run_server_soak<S>(
    session: &mut ServerSession<S>,
    config: &LiveSoakConfig,
    progress: Arc<AtomicU64>,
) -> Result<(), String>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let deadline = Instant::now() + config.duration;
    let mut next_ping = Instant::now() + config.ping_interval;
    let mut next_search = Instant::now() + config.search_interval;
    let send_timeout = env_duration_secs("SLSK_SOAK_SERVER_SEND_TIMEOUT_SECONDS", 20, false)?;
    let mut search_token = config.search_token;
    let mut events = 0usize;

    while Instant::now() < deadline && events < config.max_events {
        let now = Instant::now();
        if now >= next_ping {
            time::timeout(send_timeout, session.send_ping())
                .await
                .map_err(|_| "periodic ping send timed out".to_owned())?
                .map_err(|error| format!("periodic ping failed: {error}"))?;
            next_ping = Instant::now() + config.ping_interval;
            progress.store(unix_seconds(), Ordering::Relaxed);
            println!("server ping sent");
        }
        if config.active_probes && now >= next_search {
            if let Some(query) = &config.search_query {
                search_token = search_token.wrapping_add(1).max(1);
                time::timeout(
                    send_timeout,
                    dispatch_live_soak_search(session, query, search_token),
                )
                .await
                .map_err(|_| "search dispatch timed out".to_owned())??;
                progress.store(unix_seconds(), Ordering::Relaxed);
            }
            next_search = Instant::now() + config.search_interval;
        }

        let next_action = if config.active_probes && config.search_query.is_some() {
            next_ping.min(next_search)
        } else {
            next_ping
        };
        let next_wait = next_action
            .min(deadline)
            .saturating_duration_since(Instant::now());

        match time::timeout(next_wait, session.receive()).await {
            Ok(Ok(message)) => {
                events += 1;
                progress.store(unix_seconds(), Ordering::Relaxed);
                handle_server_message(session, message).await?;
            }
            Ok(Err(error)) => return Err(format!("server receive failed: {error}")),
            Err(_) => {}
        }
    }

    println!("server soak observed {events} event(s)");
    Ok(())
}

pub(super) async fn run_live_soak_server_watchdog(
    progress: Arc<AtomicU64>,
    duration: Duration,
    interval: Duration,
    stale_seconds: u64,
) {
    let deadline = Instant::now() + duration;
    let mut last_reported = 0_u64;

    while Instant::now() < deadline {
        time::sleep(interval).await;
        let now = unix_seconds();
        let last = progress.load(Ordering::Relaxed);
        let idle = now.saturating_sub(last);
        if idle >= stale_seconds && last != last_reported {
            println!(
                "live soak server watchdog stale idle_seconds={} stale_threshold_seconds={}",
                idle, stale_seconds
            );
            last_reported = last;
        }
    }
}

pub(super) async fn dispatch_live_soak_search<S>(
    session: &mut ServerSession<S>,
    query: &str,
    token: u32,
) -> Result<(), String>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    session
        .send_server_message(ServerMessage::FileSearchRequest(SearchRequest {
            token,
            query: query.to_owned(),
        }))
        .await
        .map_err(|error| format!("search dispatch failed: {error}"))?;
    println!(
        "active probe: file search dispatched token={token} query={}",
        redact_query(query)
    );
    Ok(())
}

pub(super) async fn handle_server_message<S>(
    session: &mut ServerSession<S>,
    message: ServerMessage,
) -> Result<(), String>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    match message {
        ServerMessage::MessageUserResponse(private_message) => {
            let id = private_message.id;
            session
                .send_server_message(ServerMessage::MessageAcked { id })
                .await
                .map_err(|error| format!("message ack failed: {error}"))?;
            println!("server event: private_message acked id={id}");
        }
        ServerMessage::CheckPrivilegesResponse { seconds } => {
            println!("server event: privileges seconds={seconds}");
        }
        ServerMessage::WatchUserResponse(user) => {
            println!("server event: watched user exists={}", user.exists);
        }
        ServerMessage::JoinedRoom(room) => {
            println!(
                "server event: joined room users={} private={}",
                room.users.len(),
                room.owner.is_some()
            );
        }
        ServerMessage::GetUserStatusResponse(status) => {
            println!(
                "server event: user status status={} privileged={}",
                status.status, status.privileged
            );
        }
        ServerMessage::GetUserStats { stats, .. } => {
            println!(
                "server event: user stats files={} dirs={}",
                stats.file_count, stats.directory_count
            );
        }
        ServerMessage::GetPeerAddressResponse(address) => {
            println!(
                "server event: peer address port={} obfuscation_type={} obfuscated_port={}",
                address.port, address.obfuscation_type, address.obfuscated_port
            );
        }
        ServerMessage::ConnectToPeerResponse(response) => {
            println!(
                "server event: connect_to_peer received requester={} kind={} endpoint={}:{} token={} host_override={}",
                redact_username(&response.username),
                response.connection_type,
                response.ip,
                response.port,
                response.token,
                optional_env("SLSK_SOAK_INDIRECT_HOST_OVERRIDE").is_some()
            );
            let timeout_seconds = env_u64("SLSK_SOAK_INDIRECT_TIMEOUT_SECONDS", 20)?
                .checked_add(5)
                .ok_or_else(|| "SLSK_SOAK_INDIRECT_TIMEOUT_SECONDS is too large".to_owned())?;
            let timeout = validated_duration_secs(
                "SLSK_SOAK_INDIRECT_TIMEOUT_SECONDS",
                timeout_seconds,
                false,
            )?;
            // An unsolicited public ConnectToPeer response can require a
            // full TCP timeout when the remote endpoint is stale or filtered.
            // Never hold the server receive loop on that socket attempt: a
            // later valid indirect request must still be dispatched while an
            // unrelated peer is being retried.
            tokio::spawn(async move {
                match time::timeout(timeout, handle_live_soak_connect_to_peer_response(response))
                    .await
                {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => {
                        println!("server event: connect_to_peer failed: {error}");
                    }
                    Err(_) => {
                        println!("server event: connect_to_peer probe timed out");
                    }
                }
            });
        }
        ServerMessage::RoomList(rooms) => {
            println!(
                "server event: room list public={} owned_private={} private={} operated_private={}",
                rooms.public_rooms.len(),
                rooms.owned_private_rooms.len(),
                rooms.private_rooms.len(),
                rooms.operated_private_rooms.len()
            );
        }
        ServerMessage::ExcludedSearchPhrases(phrases) => {
            println!("server event: excluded phrases count={}", phrases.len());
        }
        ServerMessage::FileSearchIncoming { .. } => {
            println!("server event: incoming search");
        }
        ServerMessage::PossibleParents(parents) => {
            println!("server event: possible parents count={}", parents.len());
        }
        ServerMessage::WishlistInterval { seconds } => {
            println!("server event: wishlist interval seconds={seconds}");
        }
        ServerMessage::ParentMinSpeed { speed } => {
            println!("server event: parent min speed={speed}");
        }
        ServerMessage::ParentSpeedRatio { ratio } => {
            println!("server event: parent speed ratio={ratio}");
        }
        ServerMessage::ResetDistributed => {
            println!("server event: reset distributed");
        }
        ServerMessage::Relogged => {
            return Err("account was logged in elsewhere".to_owned());
        }
        ServerMessage::Unknown { code, payload } => {
            println!(
                "server event: unknown code={code} payload_len={}",
                payload.len()
            );
        }
        other => {
            println!("server event: {}", server_message_name(&other));
        }
    }

    Ok(())
}

pub(super) async fn handle_live_soak_connect_to_peer_response(
    response: ConnectToPeerResponse,
) -> Result<(), String> {
    let kind = ConnectionKind::try_from_connection_type(&response.connection_type)
        .map_err(|error| format!("connect-to-peer response kind failed: {error}"))?;

    let timeout = env_duration_secs("SLSK_SOAK_INDIRECT_TIMEOUT_SECONDS", 20, false)?;
    let host =
        optional_env("SLSK_SOAK_INDIRECT_HOST_OVERRIDE").unwrap_or_else(|| response.ip.to_string());
    let port = u16::try_from(response.port).map_err(|_| {
        format!(
            "connect-to-peer response advertised invalid port: {}",
            response.port
        )
    })?;
    let stream = time::timeout(timeout, TcpStream::connect((host.as_str(), port)))
        .await
        .map_err(|_| "live soak indirect connect timed out".to_owned())?
        .map_err(|error| format!("live soak indirect connect failed: {error}"))?;
    let stream = send_pierce_firewall(stream, response.token)
        .await
        .map_err(|error| format!("live soak pierce-firewall send failed: {error}"))?;

    if kind == ConnectionKind::PeerMessages {
        let mut peer = PeerMessageConnection::new(stream);
        peer.send(&PeerMessage::UserInfoRequest)
            .await
            .map_err(|error| format!("live soak indirect user-info request failed: {error}"))?;
        let deadline = Instant::now() + timeout;
        for _ in 0..16 {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                log_live_soak_indirect_close("response timed out");
                return Ok(());
            }
            let peer_response = match time::timeout(remaining, peer.receive()).await {
                Ok(Ok(message)) => message,
                Ok(Err(error)) => {
                    log_live_soak_indirect_close(peer_close_reason(&error.to_string()));
                    return Ok(());
                }
                Err(_) => {
                    log_live_soak_indirect_close("response timed out");
                    return Ok(());
                }
            };
            match peer_response {
                PeerMessage::UserInfoResponse(_) => break,
                PeerMessage::UserInfoRequest => {
                    peer.send(&PeerMessage::UserInfoResponse(UserInfo {
                        description: "slskr live soak indirect".to_owned(),
                        picture: None,
                        total_uploads: 0,
                        queue_size: 0,
                        slots_free: true,
                        upload_permissions: None,
                    }))
                    .await
                    .map_err(|error| {
                        format!("live soak indirect user-info response send failed: {error}")
                    })?;
                    break;
                }
                other => println!(
                    "live soak indirect interleaved peer message: {}",
                    peer_message_name(&other)
                ),
            }
        }
    } else if kind == ConnectionKind::Distributed {
        let mut distributed = DistributedConnection::new(stream);
        distributed
            .send(&DistributedMessage::Ping)
            .await
            .map_err(|error| format!("live soak indirect distributed ping failed: {error}"))?;
    } else if kind == ConnectionKind::FileTransfer {
        let mut transfer = FileTransferConnection::new(stream);
        let token = time::timeout(timeout, transfer.receive_token())
            .await
            .map_err(|_| "live soak indirect file token timed out".to_owned())?
            .map_err(|error| format!("live soak indirect file token failed: {error}"))?;
        transfer
            .send_token(token)
            .await
            .map_err(|error| format!("live soak indirect file token echo failed: {error}"))?;
    }

    println!(
        "server event: connect_to_peer answered requester={} kind={} token={} host_override={}",
        redact_username(&response.username),
        response.connection_type,
        response.token,
        optional_env("SLSK_SOAK_INDIRECT_HOST_OVERRIDE").is_some()
    );
    Ok(())
}
