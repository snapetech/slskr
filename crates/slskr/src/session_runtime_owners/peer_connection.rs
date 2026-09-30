use super::*;

pub(super) async fn handle_connect_to_peer_request(
    state: &AppState,
    session: &mut ServerSession<TcpStream>,
    request: &ConnectToPeerRequest,
) -> Result<(), String> {
    let kind = ConnectionKind::try_from_connection_type(&request.connection_type)
        .map_err(|error| format!("unsupported connect-to-peer kind: {error}"))?;
    let Some(address) = test_user_endpoint_peer_address(state, &request.username) else {
        session
            .send_server_message(ServerMessage::CantConnectToPeerRequest {
                token: request.token,
                username: request.username.clone(),
            })
            .await
            .map_err(|error| format!("cant-connect response failed: {error}"))?;
        return Err("no endpoint available for incoming connect-to-peer request".to_owned());
    };

    let stream = connect_pierce_firewall(state, &address, request.token).await?;
    match kind {
        ConnectionKind::PeerMessages => {
            handle_plain_peer_messages(
                state,
                PeerMessageConnection::new(stream),
                Some(request.username.clone()),
            )
            .await
        }
        ConnectionKind::FileTransfer => {
            handle_inbound_file_transfer(
                state,
                slskr_client::file_transfer::FileTransferConnection::new(stream),
                Some(request.token),
            )
            .await
        }
        ConnectionKind::Distributed => Ok(()),
    }
}

pub(super) async fn connect_pierce_firewall(
    state: &AppState,
    address: &PeerAddress,
    token: u32,
) -> Result<TcpStream, String> {
    let peer_ip = peer_connect_ip(state, address);
    let port = u16::try_from(address.port).map_err(|_| "peer port is out of range".to_owned())?;
    if port == 0 {
        return Err("peer did not advertise a pierce-firewall port".to_owned());
    }
    let stream = connect_soulseek_tcp(
        state,
        SocketAddr::V4(SocketAddrV4::new(peer_ip, port)),
        SoulseekSocketClass::Control,
    )
    .await
    .map_err(|error| format!("pierce-firewall connect failed: {error}"))?;
    time::timeout(
        state.config.soulseek_connection.timeout_inactivity,
        send_pierce_firewall(stream, token),
    )
    .await
    .map_err(|_| "pierce-firewall init timed out".to_owned())?
    .map_err(|error| format!("pierce-firewall init failed: {error}"))
}
