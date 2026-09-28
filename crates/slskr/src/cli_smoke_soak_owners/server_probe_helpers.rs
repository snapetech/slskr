use super::*;

pub(super) async fn wait_for_connect_to_peer_response(
    session: &mut ServerSession<TcpStream>,
    token: u32,
    timeout: Duration,
) -> Result<ConnectToPeerResponse, String> {
    let deadline = Instant::now() + timeout;

    while Instant::now() < deadline {
        match time::timeout(
            deadline.saturating_duration_since(Instant::now()),
            session.receive(),
        )
        .await
        {
            Ok(Ok(ServerMessage::ConnectToPeerResponse(response))) if response.token == token => {
                return Ok(response);
            }
            Ok(Ok(ServerMessage::MessageUserResponse(private_message))) => {
                session
                    .send_server_message(ServerMessage::MessageAcked {
                        id: private_message.id,
                    })
                    .await
                    .map_err(|error| format!("indirect message ack failed: {error}"))?;
            }
            Ok(Ok(ServerMessage::CantConnectToPeerResponse { token: failed }))
                if failed == token =>
            {
                return Err("server reported indirect connect failure".to_owned());
            }
            Ok(Ok(ServerMessage::Relogged)) => {
                return Err("account was logged in elsewhere".to_owned());
            }
            Ok(Ok(_)) => {}
            Ok(Err(error)) => return Err(format!("indirect server receive failed: {error}")),
            Err(_) => break,
        }
    }

    Err("timed out waiting for indirect connect response".to_owned())
}

pub(in crate::cli) async fn wait_for_cant_connect_response(
    session: &mut ServerSession<TcpStream>,
    token: u32,
    timeout: Duration,
) -> Result<(), String> {
    let deadline = Instant::now() + timeout;

    while Instant::now() < deadline {
        match time::timeout(
            deadline.saturating_duration_since(Instant::now()),
            session.receive(),
        )
        .await
        {
            Ok(Ok(ServerMessage::CantConnectToPeerResponse { token: failed }))
                if failed == token =>
            {
                return Ok(());
            }
            Ok(Ok(ServerMessage::MessageUserResponse(private_message))) => {
                session
                    .send_server_message(ServerMessage::MessageAcked {
                        id: private_message.id,
                    })
                    .await
                    .map_err(|error| format!("negative indirect message ack failed: {error}"))?;
            }
            Ok(Ok(ServerMessage::Relogged)) => {
                return Err("account was logged in elsewhere".to_owned());
            }
            Ok(Ok(_)) => {}
            Ok(Err(error)) => {
                return Err(format!("negative indirect server receive failed: {error}"))
            }
            Err(_) => break,
        }
    }

    Err("timed out waiting for cant-connect response".to_owned())
}

pub(in crate::cli) async fn resolve_peer_address(
    username: &str,
    password: &str,
    peer_username: &str,
    server_address: &str,
    timeout: Duration,
) -> Result<slskr_client::protocol::server::PeerAddress, String> {
    let connection = ServerConnection::connect(server_address)
        .await
        .map_err(|error| format!("connect failed: {error}"))?;
    let mut session = ServerSession::new(connection);
    session
        .login(LoginCredentials::default_client(
            username.to_owned(),
            password.to_owned(),
        ))
        .await
        .map_err(|error| format!("login failed for configured user: {error}"))?;
    if let Some(wait_port) = optional_env("SLSK_WAIT_PORT")
        .or_else(|| optional_env("SLSK_SEARCH_WAIT_PORT"))
        .map(|value| {
            value
                .parse::<u32>()
                .map_err(|error| format!("invalid SLSK_WAIT_PORT/SLSK_SEARCH_WAIT_PORT: {error}"))
        })
        .transpose()?
    {
        session
            .send_server_message(ServerMessage::SetWaitPort(WaitPort {
                port: wait_port,
                obfuscation: None,
            }))
            .await
            .map_err(|error| format!("set wait port failed: {error}"))?;
    }
    session
        .send_server_message(ServerMessage::GetPeerAddressRequest {
            username: peer_username.to_owned(),
        })
        .await
        .map_err(|error| format!("peer-address request failed: {error}"))?;
    wait_for_peer_address_response(&mut session, timeout).await
}

pub(in crate::cli) async fn wait_for_advertised_port_metadata(
    session: &mut ServerSession<TcpStream>,
    username: &str,
    expected_port: u16,
    timeout: Duration,
) -> Result<(), String> {
    let deadline = Instant::now() + timeout;
    let mut last_port = None;

    while Instant::now() < deadline {
        session
            .send_server_message(ServerMessage::GetPeerAddressRequest {
                username: username.to_owned(),
            })
            .await
            .map_err(|error| format!("indirect metadata request failed: {error}"))?;
        let remaining = deadline.saturating_duration_since(Instant::now());
        let attempt_timeout = remaining.min(Duration::from_secs(3));
        match time::timeout(
            attempt_timeout,
            wait_for_peer_address_response(session, attempt_timeout),
        )
        .await
        {
            Ok(Ok(address)) if address.port == u32::from(expected_port) => {
                println!(
                    "indirect wait-port metadata propagated; port={} obfuscation_type={} obfuscated_port={}",
                    address.port, address.obfuscation_type, address.obfuscated_port
                );
                return Ok(());
            }
            Ok(Ok(address)) => {
                last_port = Some(address.port);
            }
            Ok(Err(error)) => {
                last_port = Some(0);
                if remaining <= Duration::from_secs(1) {
                    return Err(format!(
                        "indirect wait-port metadata did not propagate: {error}"
                    ));
                }
            }
            Err(_) => {
                last_port = Some(0);
            }
        }
        time::sleep(Duration::from_millis(500)).await;
    }

    Err(format!(
        "indirect wait-port metadata did not propagate: expected port={expected_port}, last_port={}",
        last_port.unwrap_or(0)
    ))
}

pub(in crate::cli) async fn login_probe_session(
    server_address: &str,
    username: String,
    password: String,
) -> Result<ServerSession<TcpStream>, String> {
    let connection = ServerConnection::connect(server_address)
        .await
        .map_err(|error| format!("connect failed: {error}"))?;
    let mut session = ServerSession::new(connection);
    session
        .login(LoginCredentials::default_client(username, password))
        .await
        .map_err(|error| format!("login failed for configured user: {error}"))?;
    Ok(session)
}

pub(in crate::cli) async fn wait_for_private_message(
    session: &mut ServerSession<TcpStream>,
    sender: &str,
    body: &str,
    timeout: Duration,
    label: &str,
) -> Result<u32, String> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        match time::timeout(
            deadline.saturating_duration_since(Instant::now()),
            session.receive(),
        )
        .await
        {
            Ok(Ok(ServerMessage::MessageUserResponse(private_message)))
                if private_message.username == sender && private_message.message == body =>
            {
                return Ok(private_message.id);
            }
            Ok(Ok(ServerMessage::MessageUserResponse(private_message))) => {
                session
                    .send_server_message(ServerMessage::MessageAcked {
                        id: private_message.id,
                    })
                    .await
                    .map_err(|error| format!("{label} unrelated message ack failed: {error}"))?;
            }
            Ok(Ok(ServerMessage::Relogged)) => {
                return Err("account was logged in elsewhere".to_owned());
            }
            Ok(Ok(_)) => {}
            Ok(Err(error)) => return Err(format!("{label} receive failed: {error}")),
            Err(_) => break,
        }
    }
    Err(format!("{label} timed out"))
}

pub(in crate::cli) async fn wait_for_room_message(
    session: &mut ServerSession<TcpStream>,
    room: &str,
    username: &str,
    body: &str,
    timeout: Duration,
) -> Result<(), String> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        match time::timeout(
            deadline.saturating_duration_since(Instant::now()),
            session.receive(),
        )
        .await
        {
            Ok(Ok(ServerMessage::SayChatroomResponse {
                room: got_room,
                username: got_username,
                message,
            }))
            | Ok(Ok(ServerMessage::GlobalRoomMessage {
                room: got_room,
                username: got_username,
                message,
            })) if got_room == room && got_username == username && message == body => {
                return Ok(());
            }
            Ok(Ok(ServerMessage::MessageUserResponse(private_message))) => {
                session
                    .send_server_message(ServerMessage::MessageAcked {
                        id: private_message.id,
                    })
                    .await
                    .map_err(|error| format!("room probe unrelated message ack failed: {error}"))?;
            }
            Ok(Ok(ServerMessage::Relogged)) => {
                return Err("account was logged in elsewhere".to_owned());
            }
            Ok(Ok(_)) => {}
            Ok(Err(error)) => return Err(format!("room probe receive failed: {error}")),
            Err(_) => break,
        }
    }
    Err("room message timed out".to_owned())
}

pub(in crate::cli) async fn wait_for_room_join(
    session: &mut ServerSession<TcpStream>,
    room: &str,
    timeout: Duration,
) -> Result<(), String> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        match time::timeout(
            deadline.saturating_duration_since(Instant::now()),
            session.receive(),
        )
        .await
        {
            Ok(Ok(ServerMessage::JoinedRoom(joined))) if joined.room == room => return Ok(()),
            Ok(Ok(ServerMessage::CantCreateRoom { room: rejected })) if rejected == room => {
                return Err("server rejected room creation".to_owned());
            }
            Ok(Ok(ServerMessage::CantJoinRoom { room: rejected })) if rejected == room => {
                return Err("server rejected room join".to_owned());
            }
            Ok(Ok(ServerMessage::MessageUserResponse(private_message))) => {
                session
                    .send_server_message(ServerMessage::MessageAcked {
                        id: private_message.id,
                    })
                    .await
                    .map_err(|error| format!("room join message ack failed: {error}"))?;
            }
            Ok(Ok(ServerMessage::Relogged)) => {
                return Err("account was logged in elsewhere".to_owned());
            }
            Ok(Ok(_)) => {}
            Ok(Err(error)) => return Err(format!("room join receive failed: {error}")),
            Err(_) => break,
        }
    }
    Err("room join timed out".to_owned())
}

pub(in crate::cli) async fn wait_for_peer_address_response(
    session: &mut ServerSession<TcpStream>,
    timeout: Duration,
) -> Result<slskr_client::protocol::server::PeerAddress, String> {
    let deadline = Instant::now() + timeout;

    while Instant::now() < deadline {
        match time::timeout(
            deadline.saturating_duration_since(Instant::now()),
            session.receive(),
        )
        .await
        {
            Ok(Ok(ServerMessage::GetPeerAddressResponse(address))) => return Ok(address),
            Ok(Ok(ServerMessage::MessageUserResponse(private_message))) => {
                session
                    .send_server_message(ServerMessage::MessageAcked {
                        id: private_message.id,
                    })
                    .await
                    .map_err(|error| format!("probe message ack failed: {error}"))?;
            }
            Ok(Ok(ServerMessage::Relogged)) => {
                return Err("account was logged in elsewhere".to_owned());
            }
            Ok(Ok(_)) => {}
            Ok(Err(error)) => return Err(format!("probe server receive failed: {error}")),
            Err(_) => break,
        }
    }

    Err("timed out waiting for peer-address response".to_owned())
}
