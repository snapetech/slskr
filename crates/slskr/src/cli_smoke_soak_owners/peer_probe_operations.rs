use super::*;

pub(super) async fn run_direct_peer_message_smoke(local_username: &str) -> Result<(), String> {
    let listener = Listener::bind("127.0.0.1:0")
        .await
        .map_err(|error| format!("peer listener bind failed: {error}"))?;
    let address = listener
        .local_addr()
        .map_err(|error| format!("peer listener address failed: {error}"))?;
    let accept_task = tokio::spawn(async move { listener.accept().await });

    let stream = TcpStream::connect(address)
        .await
        .map_err(|error| format!("peer direct connect failed: {error}"))?;
    let stream = send_peer_init(stream, local_username, ConnectionKind::PeerMessages)
        .await
        .map_err(|error| format!("peer init send failed: {error}"))?;
    let mut outbound = PeerMessageConnection::new(stream);

    let (incoming, _) = accept_task
        .await
        .map_err(|error| format!("peer accept task failed: {error}"))?
        .map_err(|error| format!("peer accept failed: {error}"))?;
    let mut inbound = match incoming {
        IncomingConnection::PeerInit {
            kind: ConnectionKind::PeerMessages,
            stream,
            ..
        } => PeerMessageConnection::new(stream),
        other => {
            return Err(format!(
                "unexpected peer message inbound: {}",
                incoming_connection_name(&other)
            ))
        }
    };

    outbound
        .send(&PeerMessage::UserInfoRequest)
        .await
        .map_err(|error| format!("peer user-info request send failed: {error}"))?;
    let request = inbound
        .receive()
        .await
        .map_err(|error| format!("peer user-info request receive failed: {error}"))?;
    if request != PeerMessage::UserInfoRequest {
        return Err(format!("unexpected peer message: {request:?}"));
    }

    inbound
        .send(&PeerMessage::UserInfoResponse(UserInfo {
            description: "slskr local peer smoke".to_owned(),
            picture: None,
            total_uploads: 0,
            queue_size: 0,
            slots_free: true,
            upload_permissions: None,
        }))
        .await
        .map_err(|error| format!("peer user-info response send failed: {error}"))?;
    let response = outbound
        .receive()
        .await
        .map_err(|error| format!("peer user-info response receive failed: {error}"))?;
    if !matches!(response, PeerMessage::UserInfoResponse(_)) {
        return Err(format!("unexpected peer response: {response:?}"));
    }

    println!("direct peer-message smoke completed");
    Ok(())
}

pub(super) async fn run_obfuscated_peer_message_smoke(local_username: &str) -> Result<(), String> {
    let listener = Listener::bind("127.0.0.1:0")
        .await
        .map_err(|error| format!("obfuscated peer listener bind failed: {error}"))?;
    let address = listener
        .local_addr()
        .map_err(|error| format!("obfuscated peer listener address failed: {error}"))?;
    let accept_task = tokio::spawn(async move { listener.accept_obfuscated().await });

    let stream = TcpStream::connect(address)
        .await
        .map_err(|error| format!("obfuscated peer direct connect failed: {error}"))?;
    let stream = send_obfuscated_peer_init(stream, local_username, ConnectionKind::PeerMessages)
        .await
        .map_err(|error| format!("obfuscated peer init send failed: {error}"))?;
    let mut outbound = ObfuscatedPeerMessageConnection::new(stream);

    let (incoming, _) = accept_task
        .await
        .map_err(|error| format!("obfuscated peer accept task failed: {error}"))?
        .map_err(|error| format!("obfuscated peer accept failed: {error}"))?;
    let mut inbound = match incoming {
        IncomingConnection::ObfuscatedPeerMessages(connection) => connection,
        other => {
            return Err(format!(
                "unexpected obfuscated peer inbound: {}",
                incoming_connection_name(&other)
            ))
        }
    };

    outbound
        .send(&PeerMessage::UserInfoRequest)
        .await
        .map_err(|error| format!("obfuscated user-info request send failed: {error}"))?;
    let request = inbound
        .receive()
        .await
        .map_err(|error| format!("obfuscated user-info request receive failed: {error}"))?;
    if request != PeerMessage::UserInfoRequest {
        return Err(format!("unexpected obfuscated peer message: {request:?}"));
    }

    inbound
        .send(&PeerMessage::UserInfoResponse(UserInfo {
            description: "slskr obfuscated peer smoke".to_owned(),
            picture: None,
            total_uploads: 0,
            queue_size: 0,
            slots_free: true,
            upload_permissions: None,
        }))
        .await
        .map_err(|error| format!("obfuscated user-info response send failed: {error}"))?;
    let response = outbound
        .receive()
        .await
        .map_err(|error| format!("obfuscated user-info response receive failed: {error}"))?;
    if !matches!(response, PeerMessage::UserInfoResponse(_)) {
        return Err(format!("unexpected obfuscated peer response: {response:?}"));
    }

    println!("obfuscated peer-message smoke completed");
    Ok(())
}

pub(super) async fn run_direct_file_transfer_smoke(local_username: &str) -> Result<(), String> {
    let listener = Listener::bind("127.0.0.1:0")
        .await
        .map_err(|error| format!("file listener bind failed: {error}"))?;
    let address = listener
        .local_addr()
        .map_err(|error| format!("file listener address failed: {error}"))?;
    let accept_task = tokio::spawn(async move { listener.accept().await });

    let stream = TcpStream::connect(address)
        .await
        .map_err(|error| format!("file direct connect failed: {error}"))?;
    let stream = send_peer_init(stream, local_username, ConnectionKind::FileTransfer)
        .await
        .map_err(|error| format!("file peer init send failed: {error}"))?;
    let mut outbound = FileTransferConnection::new(stream);

    let (incoming, _) = accept_task
        .await
        .map_err(|error| format!("file accept task failed: {error}"))?
        .map_err(|error| format!("file accept failed: {error}"))?;
    let mut inbound = match incoming {
        IncomingConnection::PeerInit {
            kind: ConnectionKind::FileTransfer,
            stream,
            ..
        } => FileTransferConnection::new(stream),
        other => {
            return Err(format!(
                "unexpected file inbound: {}",
                incoming_connection_name(&other)
            ))
        }
    };

    outbound
        .send_token(0x51ab_0001)
        .await
        .map_err(|error| format!("file token send failed: {error}"))?;
    outbound
        .send_offset(2)
        .await
        .map_err(|error| format!("file offset send failed: {error}"))?;
    outbound
        .write_chunk(b"slskr")
        .await
        .map_err(|error| format!("file chunk send failed: {error}"))?;

    let token = inbound
        .receive_token()
        .await
        .map_err(|error| format!("file token receive failed: {error}"))?;
    let offset = inbound
        .receive_offset()
        .await
        .map_err(|error| format!("file offset receive failed: {error}"))?;
    let chunk = inbound
        .read_chunk(5)
        .await
        .map_err(|error| format!("file chunk receive failed: {error}"))?;
    if token != 0x51ab_0001 || offset != 2 || chunk != b"slskr" {
        return Err("file transfer smoke payload mismatch".to_owned());
    }

    println!("direct file-transfer smoke completed");
    Ok(())
}

pub(super) async fn run_indirect_peer_message_smoke(
    requester_session: &mut ServerSession<TcpStream>,
    target_session: &mut ServerSession<TcpStream>,
    listener: Listener,
    requester_username: &str,
    target_username: &str,
    host_override: Option<&str>,
    timeout: Duration,
) -> Result<(), String> {
    let token = 0x51ab_1001;
    let request = IndirectPeerRequest::new(
        token,
        target_username.to_owned(),
        ConnectionKind::PeerMessages,
    );
    requester_session
        .send_server_message(request.server_message())
        .await
        .map_err(|error| format!("indirect connect request failed: {error}"))?;
    requester_session
        .send_server_message(ServerMessage::GetPeerAddressRequest {
            username: target_username.to_owned(),
        })
        .await
        .map_err(|error| format!("indirect peer-address request failed: {error}"))?;

    let response = wait_for_connect_to_peer_response(target_session, token, timeout).await?;
    if response.username != requester_username {
        return Err("indirect connect response requester mismatch".to_owned());
    }
    if ConnectionKind::try_from_connection_type(&response.connection_type)
        .map_err(|error| format!("indirect response connection type failed: {error}"))?
        != ConnectionKind::PeerMessages
    {
        return Err("indirect connect response kind mismatch".to_owned());
    }

    let connect_host = host_override
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| response.ip.to_string());
    let connect_address = format!("{connect_host}:{}", response.port);
    let accept_task = tokio::spawn(async move { listener.accept().await });
    let stream = time::timeout(timeout, TcpStream::connect(connect_address.as_str()))
        .await
        .map_err(|_| "indirect peer connect timed out".to_owned())?
        .map_err(|error| format!("indirect peer connect failed: {error}"))?;
    let stream = send_pierce_firewall(stream, response.token)
        .await
        .map_err(|error| format!("indirect pierce-firewall send failed: {error}"))?;
    let mut outbound = PeerMessageConnection::new(stream);

    let (incoming, _) = time::timeout(timeout, accept_task)
        .await
        .map_err(|_| "indirect listener accept timed out".to_owned())?
        .map_err(|error| format!("indirect accept task failed: {error}"))?
        .map_err(|error| format!("indirect accept failed: {error}"))?;
    let stream = request
        .complete(incoming)
        .map_err(|error| format!("indirect completion failed: {error}"))?;
    let mut inbound = PeerMessageConnection::new(stream);

    outbound
        .send(&PeerMessage::UserInfoRequest)
        .await
        .map_err(|error| format!("indirect user-info request send failed: {error}"))?;
    let peer_message = inbound
        .receive()
        .await
        .map_err(|error| format!("indirect user-info request receive failed: {error}"))?;
    if peer_message != PeerMessage::UserInfoRequest {
        return Err(format!(
            "unexpected indirect peer message: {peer_message:?}"
        ));
    }
    inbound
        .send(&PeerMessage::UserInfoResponse(UserInfo {
            description: "slskr indirect peer smoke".to_owned(),
            picture: None,
            total_uploads: 0,
            queue_size: 0,
            slots_free: true,
            upload_permissions: None,
        }))
        .await
        .map_err(|error| format!("indirect user-info response send failed: {error}"))?;
    let response = outbound
        .receive()
        .await
        .map_err(|error| format!("indirect user-info response receive failed: {error}"))?;
    if !matches!(response, PeerMessage::UserInfoResponse(_)) {
        return Err(format!("unexpected indirect peer response: {response:?}"));
    }

    println!(
        "indirect peer-message smoke completed; host_override={}; requester={}",
        host_override.is_some(),
        redact_username(requester_username)
    );
    Ok(())
}
