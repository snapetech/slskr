use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum OutboundPeerTransport {
    Regular,
    Obfuscated,
}

pub(super) fn outbound_peer_dial_order(
    config: &AppConfig,
    regular_available: bool,
    obfuscated_available: bool,
) -> Vec<OutboundPeerTransport> {
    let obfuscated_available = config.obfuscation_enabled && obfuscated_available;
    let mut order = Vec::with_capacity(2);
    if config.prefer_obfuscated_outbound() {
        if obfuscated_available {
            order.push(OutboundPeerTransport::Obfuscated);
        }
        if regular_available {
            order.push(OutboundPeerTransport::Regular);
        }
    } else if regular_available {
        // Compatibility mode is regular-only when the peer advertises a
        // regular endpoint. The upstream client does not silently switch to
        // obfuscation after a failed regular dial; obfuscation is only the
        // usable path when no regular endpoint was advertised.
        order.push(OutboundPeerTransport::Regular);
    } else if obfuscated_available {
        order.push(OutboundPeerTransport::Obfuscated);
    }
    order
}

fn peer_supports_obfuscated_dial(address: &PeerAddress) -> bool {
    address.obfuscation_type == ROTATED_OBFUSCATION_TYPE && address.obfuscated_port != 0
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SoulseekSocketClass {
    Control,
    Transfer,
}

fn configure_soulseek_socket(
    stream: &TcpStream,
    settings: &crate::config::SoulseekConnectionSettings,
    class: SoulseekSocketClass,
) -> Result<(), String> {
    let (read_buffer, write_buffer) = match class {
        SoulseekSocketClass::Control => (settings.buffer_read, settings.buffer_write),
        SoulseekSocketClass::Transfer => (settings.buffer_transfer, settings.buffer_transfer),
    };
    let socket = SockRef::from(stream);
    socket
        .set_recv_buffer_size(read_buffer)
        .map_err(|error| format!("failed to set Soulseek receive buffer: {error}"))?;
    socket
        .set_send_buffer_size(write_buffer)
        .map_err(|error| format!("failed to set Soulseek send buffer: {error}"))?;
    Ok(())
}

pub(super) fn configure_incoming_soulseek_socket(
    incoming: IncomingConnection<TcpStream>,
    settings: &crate::config::SoulseekConnectionSettings,
) -> Result<IncomingConnection<TcpStream>, String> {
    let configured = match incoming {
        IncomingConnection::PeerMessages(connection) => {
            let stream = connection.into_inner();
            configure_soulseek_socket(&stream, settings, SoulseekSocketClass::Control)?;
            IncomingConnection::PeerMessages(PeerMessageConnection::new(stream))
        }
        IncomingConnection::ObfuscatedPeerMessages(connection) => {
            let (stream, peer_username) = connection.into_parts();
            configure_soulseek_socket(&stream, settings, SoulseekSocketClass::Control)?;
            IncomingConnection::ObfuscatedPeerMessages(
                ObfuscatedPeerMessageConnection::with_peer_username(stream, peer_username),
            )
        }
        IncomingConnection::FileTransfer(connection) => {
            let stream = connection.into_inner();
            configure_soulseek_socket(&stream, settings, SoulseekSocketClass::Transfer)?;
            IncomingConnection::FileTransfer(
                slskr_client::file_transfer::FileTransferConnection::new(stream),
            )
        }
        IncomingConnection::Distributed(connection) => {
            let (stream, obfuscated) = connection.into_parts();
            configure_soulseek_socket(&stream, settings, SoulseekSocketClass::Control)?;
            let connection = if obfuscated {
                slskr_client::stream::DistributedConnection::new_obfuscated(stream)
            } else {
                slskr_client::stream::DistributedConnection::new(stream)
            };
            IncomingConnection::Distributed(connection)
        }
        IncomingConnection::PeerInit {
            username,
            kind,
            token,
            stream,
            obfuscated,
        } => {
            let class = if kind == ConnectionKind::FileTransfer {
                SoulseekSocketClass::Transfer
            } else {
                SoulseekSocketClass::Control
            };
            configure_soulseek_socket(&stream, settings, class)?;
            IncomingConnection::PeerInit {
                username,
                kind,
                token,
                stream,
                obfuscated,
            }
        }
        IncomingConnection::PierceFirewall { token, stream } => {
            configure_soulseek_socket(&stream, settings, SoulseekSocketClass::Control)?;
            IncomingConnection::PierceFirewall { token, stream }
        }
        IncomingConnection::UnknownInit {
            code,
            payload,
            stream,
        } => {
            configure_soulseek_socket(&stream, settings, SoulseekSocketClass::Control)?;
            IncomingConnection::UnknownInit {
                code,
                payload,
                stream,
            }
        }
    };
    Ok(configured)
}

pub(super) async fn connect_soulseek_tcp(
    state: &AppState,
    target: impl ToString,
    class: SoulseekSocketClass,
) -> Result<TcpStream, String> {
    let target = target.to_string();
    let settings = &state.config.soulseek_connection;
    let stream = time::timeout(settings.timeout_connect, async {
        if settings.proxy.enabled {
            let port = settings.proxy.port.ok_or_else(|| {
                "Soulseek proxy port is required when the proxy is enabled".to_owned()
            })?;
            let proxy = format_host_port(&settings.proxy.address, port);
            let mut stream = TcpStream::connect(&proxy)
                .await
                .map_err(|error| format!("SOCKS5 proxy connect to {proxy} failed: {error}"))?;
            socks5_connect(
                &mut stream,
                &target,
                &settings.proxy.username,
                &settings.proxy.password,
            )
            .await?;
            Ok(stream)
        } else {
            TcpStream::connect(&target)
                .await
                .map_err(|error| format!("Soulseek connect to {target} failed: {error}"))
        }
    })
    .await
    .map_err(|_| format!("Soulseek connect to {target} timed out"))??;

    configure_soulseek_socket(&stream, settings, class)?;
    Ok(stream)
}

async fn socks5_connect(
    stream: &mut TcpStream,
    target: &str,
    username: &str,
    password: &str,
) -> Result<(), String> {
    let use_auth = !username.is_empty() || !password.is_empty();
    let greeting: &[u8] = if use_auth {
        &[0x05, 0x02, 0x00, 0x02]
    } else {
        &[0x05, 0x01, 0x00]
    };
    stream
        .write_all(greeting)
        .await
        .map_err(|error| format!("SOCKS5 greeting failed: {error}"))?;
    let mut selection = [0_u8; 2];
    stream
        .read_exact(&mut selection)
        .await
        .map_err(|error| format!("SOCKS5 method selection failed: {error}"))?;
    if selection[0] != 0x05 {
        return Err(format!("SOCKS5 proxy returned version {}", selection[0]));
    }
    match selection[1] {
        0x00 => {}
        0x02 if use_auth => {
            let username = username.as_bytes();
            let password = password.as_bytes();
            let username_len = u8::try_from(username.len())
                .map_err(|_| "SOCKS5 proxy username exceeds 255 bytes".to_owned())?;
            let password_len = u8::try_from(password.len())
                .map_err(|_| "SOCKS5 proxy password exceeds 255 bytes".to_owned())?;
            let mut auth = Vec::with_capacity(3 + username.len() + password.len());
            auth.extend_from_slice(&[0x01, username_len]);
            auth.extend_from_slice(username);
            auth.push(password_len);
            auth.extend_from_slice(password);
            stream
                .write_all(&auth)
                .await
                .map_err(|error| format!("SOCKS5 authentication request failed: {error}"))?;
            let mut response = [0_u8; 2];
            stream
                .read_exact(&mut response)
                .await
                .map_err(|error| format!("SOCKS5 authentication response failed: {error}"))?;
            if response != [0x01, 0x00] {
                return Err("SOCKS5 proxy authentication failed".to_owned());
            }
        }
        0xff => return Err("SOCKS5 proxy rejected all authentication methods".to_owned()),
        method => {
            return Err(format!(
                "SOCKS5 proxy selected unsupported authentication method {method}"
            ));
        }
    }

    let (host, port) = split_soulseek_target(target)?;
    let mut request = vec![0x05, 0x01, 0x00];
    if let Ok(ip) = host.parse::<IpAddr>() {
        match ip {
            IpAddr::V4(ip) => {
                request.push(0x01);
                request.extend_from_slice(&ip.octets());
            }
            IpAddr::V6(ip) => {
                request.push(0x04);
                request.extend_from_slice(&ip.octets());
            }
        }
    } else {
        let bytes = host.as_bytes();
        let length = u8::try_from(bytes.len())
            .map_err(|_| "SOCKS5 target hostname exceeds 255 bytes".to_owned())?;
        request.extend_from_slice(&[0x03, length]);
        request.extend_from_slice(bytes);
    }
    request.extend_from_slice(&port.to_be_bytes());
    stream
        .write_all(&request)
        .await
        .map_err(|error| format!("SOCKS5 connect request failed: {error}"))?;

    let mut response = [0_u8; 4];
    stream
        .read_exact(&mut response)
        .await
        .map_err(|error| format!("SOCKS5 connect response failed: {error}"))?;
    if response[0] != 0x05 {
        return Err(format!("SOCKS5 proxy returned version {}", response[0]));
    }
    if response[1] != 0x00 {
        return Err(format!(
            "SOCKS5 proxy connect failed with status {}",
            response[1]
        ));
    }
    let address_len = match response[3] {
        0x01 => 4,
        0x04 => 16,
        0x03 => {
            let mut length = [0_u8; 1];
            stream
                .read_exact(&mut length)
                .await
                .map_err(|error| format!("SOCKS5 bound hostname length failed: {error}"))?;
            usize::from(length[0])
        }
        kind => return Err(format!("SOCKS5 proxy returned address type {kind}")),
    };
    let mut bound_address_and_port = vec![0_u8; address_len + 2];
    stream
        .read_exact(&mut bound_address_and_port)
        .await
        .map_err(|error| format!("SOCKS5 bound address failed: {error}"))?;
    Ok(())
}

fn split_soulseek_target(target: &str) -> Result<(&str, u16), String> {
    let (host, port) = target
        .rsplit_once(':')
        .ok_or_else(|| format!("Soulseek target has no port: {target}"))?;
    let host = host
        .strip_prefix('[')
        .and_then(|host| host.strip_suffix(']'))
        .unwrap_or(host);
    if host.is_empty() {
        return Err(format!("Soulseek target has no host: {target}"));
    }
    let port = port
        .parse::<u16>()
        .map_err(|_| format!("Soulseek target has an invalid port: {target}"))?;
    Ok((host, port))
}

fn format_host_port(host: &str, port: u16) -> String {
    let host = host.trim_matches(['[', ']']);
    if host.contains(':') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}

pub(super) async fn connect_file_transfer_preferred(
    state: &AppState,
    address: &PeerAddress,
) -> Result<slskr_client::file_transfer::FileTransferConnection<TcpStream>, String> {
    let peer_ip = peer_connect_ip(state, address);
    let regular_port = u16::try_from(address.port).ok().filter(|port| *port != 0);
    let mut last_error = (address.port != 0 && regular_port.is_none())
        .then(|| "peer port is out of range".to_owned());
    let username = outgoing_peer_init_username(state).await?;
    for transport in outbound_peer_dial_order(
        &state.config,
        regular_port.is_some(),
        peer_supports_obfuscated_dial(address),
    ) {
        let result = match transport {
            OutboundPeerTransport::Regular => {
                let stream = connect_soulseek_tcp(
                    state,
                    SocketAddr::V4(SocketAddrV4::new(
                        peer_ip,
                        regular_port.expect("regular transport requires a port"),
                    )),
                    SoulseekSocketClass::Transfer,
                )
                .await;
                match stream {
                    Ok(stream) => time::timeout(
                        state.config.soulseek_connection.timeout_inactivity,
                        send_peer_init(stream, username.clone(), ConnectionKind::FileTransfer),
                    )
                    .await
                    .map_err(|_| "file-transfer init timed out".to_owned())
                    .and_then(|result| {
                        result
                            .map(slskr_client::file_transfer::FileTransferConnection::new)
                            .map_err(|error| format!("file-transfer init failed: {error}"))
                    }),
                    Err(error) => Err(format!("file-transfer connect failed: {error}")),
                }
            }
            OutboundPeerTransport::Obfuscated => {
                connect_obfuscated_file_transfer(state, address).await
            }
        };
        match result {
            Ok(connection) => return Ok(connection),
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error.unwrap_or_else(|| "peer did not advertise a file-transfer port".to_owned()))
}

async fn connect_obfuscated_file_transfer(
    state: &AppState,
    address: &PeerAddress,
) -> Result<slskr_client::file_transfer::FileTransferConnection<TcpStream>, String> {
    let peer_ip = peer_connect_ip(state, address);
    let stream = connect_soulseek_tcp(
        state,
        SocketAddr::V4(SocketAddrV4::new(peer_ip, address.obfuscated_port)),
        SoulseekSocketClass::Transfer,
    )
    .await
    .map_err(|error| format!("obfuscated file-transfer connect failed: {error}"))?;
    let stream = time::timeout(
        state.config.soulseek_connection.timeout_inactivity,
        send_obfuscated_peer_init(
            stream,
            outgoing_peer_init_username(state).await?,
            ConnectionKind::FileTransfer,
        ),
    )
    .await
    .map_err(|_| "obfuscated file-transfer init timed out".to_owned())?
    .map_err(|error| format!("obfuscated file-transfer init failed: {error}"))?;
    Ok(slskr_client::file_transfer::FileTransferConnection::new_obfuscated(stream))
}

pub(super) async fn connect_indirect_file_transfer(
    state: &AppState,
    response: &ConnectToPeerResponse,
) -> Result<slskr_client::file_transfer::FileTransferConnection<TcpStream>, String> {
    let port = u16::try_from(response.port)
        .map_err(|_| format!("connect-to-peer port is out of range: {}", response.port))?;
    let peer_ip = state.config.peer_host_override.unwrap_or(response.ip);
    let stream = connect_soulseek_tcp(
        state,
        SocketAddr::V4(SocketAddrV4::new(peer_ip, port)),
        SoulseekSocketClass::Transfer,
    )
    .await
    .map_err(|error| format!("indirect file-transfer connect failed: {error}"))?;
    let stream = time::timeout(
        state.config.soulseek_connection.timeout_inactivity,
        send_pierce_firewall(stream, response.token),
    )
    .await
    .map_err(|_| "indirect pierce-firewall timed out".to_owned())?
    .map_err(|error| format!("indirect pierce-firewall failed: {error}"))?;
    Ok(slskr_client::file_transfer::FileTransferConnection::new(
        stream,
    ))
}

async fn connect_indirect_peer_messages(
    state: &AppState,
    response: &ConnectToPeerResponse,
) -> Result<PeerMessageConnection<TcpStream>, String> {
    let port = u16::try_from(response.port)
        .map_err(|_| format!("connect-to-peer port is out of range: {}", response.port))?;
    let peer_ip = state.config.peer_host_override.unwrap_or(response.ip);
    let stream = connect_soulseek_tcp(
        state,
        SocketAddr::V4(SocketAddrV4::new(peer_ip, port)),
        SoulseekSocketClass::Control,
    )
    .await
    .map_err(|error| format!("indirect peer-message connect failed: {error}"))?;
    let stream = time::timeout(
        state.config.soulseek_connection.timeout_inactivity,
        send_pierce_firewall(stream, response.token),
    )
    .await
    .map_err(|_| "indirect peer-message pierce-firewall timed out".to_owned())?
    .map_err(|error| format!("indirect peer-message pierce-firewall failed: {error}"))?;
    Ok(PeerMessageConnection::new(stream))
}

pub(super) enum PeerTransferNegotiation {
    Allowed { token: u32, size: Option<u64> },
    Rejected { token: u32, reason: String },
    QueuedInbound { token: u32, size: Option<u64> },
}

pub(super) async fn negotiate_peer_transfer(
    state: &AppState,
    address: &PeerAddress,
    transfer: &TransferEntry,
) -> Result<PeerTransferNegotiation, String> {
    let filename_encoding = match transfer.peer_username.as_deref() {
        Some(username) => state
            .remote_path_encodings
            .read()
            .await
            .encoding_for(username, &transfer.filename),
        None => ProtocolTextEncoding::Utf8,
    };
    let message = PeerMessage::TransferRequest(TransferRequest {
        filename_encoding,
        direction: transfer.direction,
        token: transfer.token,
        filename: transfer.filename.clone(),
        size: (transfer.direction == 1).then_some(transfer.size).flatten(),
    });
    let username = outgoing_peer_init_username(state).await?;
    let peer_ip = peer_connect_ip(state, address);
    let regular_port = u16::try_from(address.port).ok().filter(|port| *port != 0);
    let mut last_error = (address.port != 0 && regular_port.is_none())
        .then(|| "peer port is out of range".to_owned());
    for transport in outbound_peer_dial_order(
        &state.config,
        regular_port.is_some(),
        peer_supports_obfuscated_dial(address),
    ) {
        let result = match transport {
            OutboundPeerTransport::Regular => {
                negotiate_plain_peer_transfer(
                    state,
                    SocketAddr::V4(SocketAddrV4::new(
                        peer_ip,
                        regular_port.expect("regular transport requires a port"),
                    )),
                    username.clone(),
                    message.clone(),
                    transfer,
                    state.config.soulseek_connection.timeout_inactivity,
                )
                .await
            }
            OutboundPeerTransport::Obfuscated => {
                negotiate_obfuscated_peer_transfer(
                    state,
                    SocketAddr::V4(SocketAddrV4::new(peer_ip, address.obfuscated_port)),
                    username.clone(),
                    message.clone(),
                    transfer,
                    state.config.soulseek_connection.timeout_inactivity,
                )
                .await
            }
        };
        match result {
            Ok(response) => return Ok(response),
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error.unwrap_or_else(|| "peer did not advertise a peer-message port".to_owned()))
}

pub(super) async fn fetch_peer_browse(
    state: &AppState,
    address: &PeerAddress,
) -> Result<Vec<BrowseEntry>, String> {
    let username = outgoing_peer_init_username(state).await?;
    let peer_ip = peer_connect_ip(state, address);
    let regular_port = u16::try_from(address.port).ok().filter(|port| *port != 0);
    let mut last_error = (address.port != 0 && regular_port.is_none())
        .then(|| "peer port is out of range".to_owned());
    for transport in outbound_peer_dial_order(
        &state.config,
        regular_port.is_some(),
        peer_supports_obfuscated_dial(address),
    ) {
        let result = match transport {
            OutboundPeerTransport::Regular => {
                browse_plain_peer(
                    state,
                    SocketAddr::V4(SocketAddrV4::new(
                        peer_ip,
                        regular_port.expect("regular transport requires a port"),
                    )),
                    username.clone(),
                    state.config.soulseek_connection.timeout_inactivity,
                )
                .await
            }
            OutboundPeerTransport::Obfuscated => {
                browse_obfuscated_peer(
                    state,
                    SocketAddr::V4(SocketAddrV4::new(peer_ip, address.obfuscated_port)),
                    username.clone(),
                    state.config.soulseek_connection.timeout_inactivity,
                )
                .await
            }
        };
        match result {
            Ok(entries) => return Ok(entries),
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error.unwrap_or_else(|| "peer did not advertise a browse port".to_owned()))
}

pub(super) async fn fetch_peer_folder(
    state: &AppState,
    address: &PeerAddress,
    folder: String,
) -> Result<Vec<BrowseEntry>, String> {
    let folder_encoding = state
        .remote_path_encodings
        .read()
        .await
        .encoding_for(&address.username, &folder);
    let response = send_peer_message_request(
        state,
        address,
        PeerMessage::FolderContentsRequest(FolderContentsRequest {
            folder_encoding,
            token: 0,
            folder: folder.clone(),
        }),
    )
    .await?;
    folder_entries_from_peer_message(response, &folder, folder_encoding)
}

pub(super) async fn fetch_indirect_peer_browse(
    state: &AppState,
    response: &ConnectToPeerResponse,
) -> Result<Vec<BrowseEntry>, String> {
    let mut peer = connect_indirect_peer_messages(state, response).await?;
    time::timeout(
        state.config.soulseek_connection.timeout_inactivity,
        peer.send(&PeerMessage::GetShareFileList),
    )
    .await
    .map_err(|_| "indirect browse request timed out".to_owned())?
    .map_err(|error| format!("indirect browse request failed: {error}"))?;
    let message = time::timeout(
        state.config.soulseek_connection.timeout_inactivity,
        peer.receive(),
    )
    .await
    .map_err(|_| "indirect browse response timed out".to_owned())?
    .map_err(|error| format!("indirect browse response failed: {error}"))?;
    browse_entries_from_peer_message(message)
}

pub(super) async fn fetch_indirect_peer_folder(
    state: &AppState,
    response: &ConnectToPeerResponse,
    folder: String,
) -> Result<Vec<BrowseEntry>, String> {
    let folder_encoding = state
        .remote_path_encodings
        .read()
        .await
        .encoding_for(&response.username, &folder);
    let mut peer = connect_indirect_peer_messages(state, response).await?;
    time::timeout(
        state.config.soulseek_connection.timeout_inactivity,
        peer.send(&PeerMessage::FolderContentsRequest(FolderContentsRequest {
            folder_encoding,
            token: 0,
            folder: folder.clone(),
        })),
    )
    .await
    .map_err(|_| "indirect folder browse request timed out".to_owned())?
    .map_err(|error| format!("indirect folder browse request failed: {error}"))?;
    let message = time::timeout(
        state.config.soulseek_connection.timeout_inactivity,
        peer.receive(),
    )
    .await
    .map_err(|_| "indirect folder browse response timed out".to_owned())?
    .map_err(|error| format!("indirect folder browse response failed: {error}"))?;
    folder_entries_from_peer_message(message, &folder, folder_encoding)
}

async fn browse_plain_peer(
    state: &AppState,
    address: SocketAddr,
    username: String,
    timeout: Duration,
) -> Result<Vec<BrowseEntry>, String> {
    let stream = connect_soulseek_tcp(state, address, SoulseekSocketClass::Control)
        .await
        .map_err(|error| format!("plain peer connect failed: {error}"))?;
    let stream = time::timeout(
        timeout,
        send_peer_init(stream, username, ConnectionKind::PeerMessages),
    )
    .await
    .map_err(|_| "plain peer init timed out".to_owned())?
    .map_err(|error| format!("plain peer init failed: {error}"))?;
    let mut peer = PeerMessageConnection::new(stream);
    time::timeout(timeout, peer.send(&PeerMessage::GetShareFileList))
        .await
        .map_err(|_| "plain browse request timed out".to_owned())?
        .map_err(|error| format!("plain browse request failed: {error}"))?;
    let message = time::timeout(timeout, peer.receive())
        .await
        .map_err(|_| "plain browse response timed out".to_owned())?
        .map_err(|error| format!("plain browse response failed: {error}"))?;
    browse_entries_from_peer_message(message)
}

async fn browse_obfuscated_peer(
    state: &AppState,
    address: SocketAddr,
    username: String,
    timeout: Duration,
) -> Result<Vec<BrowseEntry>, String> {
    let stream = connect_soulseek_tcp(state, address, SoulseekSocketClass::Control)
        .await
        .map_err(|error| format!("obfuscated peer connect failed: {error}"))?;
    let stream = time::timeout(
        timeout,
        send_obfuscated_peer_init(stream, username, ConnectionKind::PeerMessages),
    )
    .await
    .map_err(|_| "obfuscated peer init timed out".to_owned())?
    .map_err(|error| format!("obfuscated peer init failed: {error}"))?;
    let mut peer = ObfuscatedPeerMessageConnection::new(stream);
    time::timeout(timeout, peer.send(&PeerMessage::GetShareFileList))
        .await
        .map_err(|_| "obfuscated browse request timed out".to_owned())?
        .map_err(|error| format!("obfuscated browse request failed: {error}"))?;
    let message = time::timeout(timeout, peer.receive())
        .await
        .map_err(|_| "obfuscated browse response timed out".to_owned())?
        .map_err(|error| format!("obfuscated browse response failed: {error}"))?;
    browse_entries_from_peer_message(message)
}

pub(super) async fn send_peer_message_request(
    state: &AppState,
    address: &PeerAddress,
    message: PeerMessage,
) -> Result<PeerMessage, String> {
    let username = outgoing_peer_init_username(state).await?;
    let peer_ip = peer_connect_ip(state, address);
    let regular_port = u16::try_from(address.port).ok().filter(|port| *port != 0);
    let mut last_error = (address.port != 0 && regular_port.is_none())
        .then(|| "peer port is out of range".to_owned());
    for transport in outbound_peer_dial_order(
        &state.config,
        regular_port.is_some(),
        peer_supports_obfuscated_dial(address),
    ) {
        let result = match transport {
            OutboundPeerTransport::Regular => {
                send_plain_peer_message_request(
                    state,
                    SocketAddr::V4(SocketAddrV4::new(
                        peer_ip,
                        regular_port.expect("regular transport requires a port"),
                    )),
                    username.clone(),
                    message.clone(),
                    state.config.soulseek_connection.timeout_inactivity,
                )
                .await
            }
            OutboundPeerTransport::Obfuscated => {
                send_obfuscated_peer_message_request(
                    state,
                    SocketAddr::V4(SocketAddrV4::new(peer_ip, address.obfuscated_port)),
                    username.clone(),
                    message.clone(),
                    state.config.soulseek_connection.timeout_inactivity,
                )
                .await
            }
        };
        match result {
            Ok(response) => return Ok(response),
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error.unwrap_or_else(|| "peer did not advertise a peer-message port".to_owned()))
}

pub(super) async fn send_peer_message_oneway(
    state: &AppState,
    address: &PeerAddress,
    message: PeerMessage,
) -> Result<(), String> {
    let username = outgoing_peer_init_username(state).await?;
    let peer_ip = peer_connect_ip(state, address);
    let regular_port = u16::try_from(address.port).ok().filter(|port| *port != 0);
    let mut last_error = None;
    for transport in outbound_peer_dial_order(
        &state.config,
        regular_port.is_some(),
        peer_supports_obfuscated_dial(address),
    ) {
        let socket = match transport {
            OutboundPeerTransport::Regular => SocketAddr::V4(SocketAddrV4::new(
                peer_ip,
                regular_port.expect("regular transport requires a port"),
            )),
            OutboundPeerTransport::Obfuscated => {
                SocketAddr::V4(SocketAddrV4::new(peer_ip, address.obfuscated_port))
            }
        };
        let result = async {
            let stream = connect_soulseek_tcp(state, socket, SoulseekSocketClass::Control).await?;
            if transport == OutboundPeerTransport::Obfuscated {
                let stream = time::timeout(
                    state.config.soulseek_connection.timeout_inactivity,
                    send_obfuscated_peer_init(
                        stream,
                        username.clone(),
                        ConnectionKind::PeerMessages,
                    ),
                )
                .await
                .map_err(|_| "obfuscated peer init timed out".to_owned())?
                .map_err(|error| format!("obfuscated peer init failed: {error}"))?;
                let mut peer = ObfuscatedPeerMessageConnection::new(stream);
                time::timeout(
                    state.config.soulseek_connection.timeout_inactivity,
                    peer.send(&message),
                )
                .await
                .map_err(|_| "obfuscated peer send timed out".to_owned())?
                .map_err(|error| format!("obfuscated peer send failed: {error}"))
            } else {
                let stream = time::timeout(
                    state.config.soulseek_connection.timeout_inactivity,
                    send_peer_init(stream, username.clone(), ConnectionKind::PeerMessages),
                )
                .await
                .map_err(|_| "peer init timed out".to_owned())?
                .map_err(|error| format!("peer init failed: {error}"))?;
                let mut peer = PeerMessageConnection::new(stream);
                time::timeout(
                    state.config.soulseek_connection.timeout_inactivity,
                    peer.send(&message),
                )
                .await
                .map_err(|_| "peer send timed out".to_owned())?
                .map_err(|error| format!("peer send failed: {error}"))
            }
        }
        .await;
        match result {
            Ok(()) => return Ok(()),
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error.unwrap_or_else(|| "peer did not advertise a response port".to_owned()))
}

const MAX_PEER_ENDPOINT_RECORDS: usize = 1_024;
const PEER_ENDPOINT_TTL_SECONDS: u64 = 300;

pub(super) fn test_user_endpoint_peer_address(
    state: &AppState,
    username: &str,
) -> Option<PeerAddress> {
    let endpoint = state.config.test_user_endpoint_overrides.get(username)?;
    let SocketAddr::V4(endpoint) = endpoint else {
        return None;
    };
    Some(PeerAddress {
        username: username.to_owned(),
        ip: *endpoint.ip(),
        port: u32::from(endpoint.port()),
        obfuscation_type: 0,
        obfuscated_port: 0,
    })
}

pub(super) async fn remember_peer_endpoint(state: &AppState, address: PeerAddress) {
    let key = address.username.clone();
    let mut endpoints = state.peer_endpoints.write().await;
    if endpoints.len() >= MAX_PEER_ENDPOINT_RECORDS && !endpoints.contains_key(&key) {
        if let Some(oldest) = endpoints
            .iter()
            .min_by_key(|(_, (_, updated_at))| *updated_at)
            .map(|(username, _)| username.clone())
        {
            endpoints.remove(&oldest);
        }
    }
    endpoints.insert(key, (address, unix_timestamp()));
}

pub(super) async fn cached_peer_endpoint(state: &AppState, username: &str) -> Option<PeerAddress> {
    if let Some(address) = test_user_endpoint_peer_address(state, username) {
        return Some(address);
    }
    let now = unix_timestamp();
    let mut endpoints = state.peer_endpoints.write().await;
    endpoints
        .retain(|_, (_, updated_at)| now.saturating_sub(*updated_at) <= PEER_ENDPOINT_TTL_SECONDS);
    endpoints.get(username).map(|(address, _)| address.clone())
}

pub(super) async fn request_peer_endpoint(
    state: &AppState,
    username: &str,
) -> Result<PeerAddress, String> {
    if let Some(address) = cached_peer_endpoint(state, username).await {
        return Ok(address);
    }
    try_send_session_command(
        state,
        SessionCommand::RequestPeerEndpoint(username.to_owned()),
    )?;
    let deadline = Instant::now() + state.config.soulseek_connection.timeout_inactivity;
    while Instant::now() < deadline {
        time::sleep(Duration::from_millis(25)).await;
        if let Some(address) = cached_peer_endpoint(state, username).await {
            return Ok(address);
        }
    }
    Err("peer endpoint lookup timed out".to_owned())
}

pub(super) fn peer_connect_ip(state: &AppState, address: &PeerAddress) -> std::net::Ipv4Addr {
    state.config.peer_host_override.unwrap_or(address.ip)
}

pub(super) async fn outgoing_peer_init_username(state: &AppState) -> Result<String, String> {
    if let Some(username) = state.config.username.clone() {
        return Ok(username);
    }
    if let Some(username) = state.session.read().await.username.clone() {
        return Ok(username);
    }
    if let Some(username) = state
        .runtime_credentials
        .read()
        .await
        .as_ref()
        .map(|credentials| credentials.username.clone())
    {
        return Ok(username);
    }
    if let Some(username) = state
        .configured_credentials
        .read()
        .await
        .as_ref()
        .map(|credentials| credentials.username.clone())
    {
        return Ok(username);
    }
    Err("local username is required for peer init".to_owned())
}

async fn send_plain_peer_message_request(
    state: &AppState,
    address: SocketAddr,
    username: String,
    message: PeerMessage,
    timeout: Duration,
) -> Result<PeerMessage, String> {
    let stream = connect_soulseek_tcp(state, address, SoulseekSocketClass::Control)
        .await
        .map_err(|error| format!("plain peer connect failed: {error}"))?;
    let stream = time::timeout(
        timeout,
        send_peer_init(stream, username, ConnectionKind::PeerMessages),
    )
    .await
    .map_err(|_| "plain peer init timed out".to_owned())?
    .map_err(|error| format!("plain peer init failed: {error}"))?;
    let mut peer = PeerMessageConnection::new(stream);
    time::timeout(timeout, peer.send(&message))
        .await
        .map_err(|_| "plain peer request timed out".to_owned())?
        .map_err(|error| format!("plain peer request failed: {error}"))?;
    time::timeout(timeout, peer.receive())
        .await
        .map_err(|_| "plain peer response timed out".to_owned())?
        .map_err(|error| format!("plain peer response failed: {error}"))
}

async fn send_obfuscated_peer_message_request(
    state: &AppState,
    address: SocketAddr,
    username: String,
    message: PeerMessage,
    timeout: Duration,
) -> Result<PeerMessage, String> {
    let stream = connect_soulseek_tcp(state, address, SoulseekSocketClass::Control)
        .await
        .map_err(|error| format!("obfuscated peer connect failed: {error}"))?;
    let stream = time::timeout(
        timeout,
        send_obfuscated_peer_init(stream, username, ConnectionKind::PeerMessages),
    )
    .await
    .map_err(|_| "obfuscated peer init timed out".to_owned())?
    .map_err(|error| format!("obfuscated peer init failed: {error}"))?;
    let mut peer = ObfuscatedPeerMessageConnection::new(stream);
    time::timeout(timeout, peer.send(&message))
        .await
        .map_err(|_| "obfuscated peer request timed out".to_owned())?
        .map_err(|error| format!("obfuscated peer request failed: {error}"))?;
    time::timeout(timeout, peer.receive())
        .await
        .map_err(|_| "obfuscated peer response timed out".to_owned())?
        .map_err(|error| format!("obfuscated peer response failed: {error}"))
}

async fn negotiate_plain_peer_transfer(
    state: &AppState,
    address: SocketAddr,
    username: String,
    message: PeerMessage,
    transfer: &TransferEntry,
    timeout: Duration,
) -> Result<PeerTransferNegotiation, String> {
    let stream = connect_soulseek_tcp(state, address, SoulseekSocketClass::Control)
        .await
        .map_err(|error| format!("plain peer connect failed: {error}"))?;
    let stream = time::timeout(
        timeout,
        send_peer_init(stream, username, ConnectionKind::PeerMessages),
    )
    .await
    .map_err(|_| "plain peer init timed out".to_owned())?
    .map_err(|error| format!("plain peer init failed: {error}"))?;
    let mut peer = PeerMessageConnection::new(stream);
    time::timeout(timeout, peer.send(&message))
        .await
        .map_err(|_| "plain peer transfer request timed out".to_owned())?
        .map_err(|error| format!("plain peer transfer request failed: {error}"))?;
    let response = time::timeout(timeout, peer.receive())
        .await
        .map_err(|_| "plain peer transfer response timed out".to_owned())?
        .map_err(|error| format!("plain peer transfer response failed: {error}"))?;
    handle_peer_transfer_negotiation_response(&mut peer, response, transfer, timeout).await
}

async fn negotiate_obfuscated_peer_transfer(
    state: &AppState,
    address: SocketAddr,
    username: String,
    message: PeerMessage,
    transfer: &TransferEntry,
    timeout: Duration,
) -> Result<PeerTransferNegotiation, String> {
    let stream = connect_soulseek_tcp(state, address, SoulseekSocketClass::Control)
        .await
        .map_err(|error| format!("obfuscated peer connect failed: {error}"))?;
    let stream = time::timeout(
        timeout,
        send_obfuscated_peer_init(stream, username, ConnectionKind::PeerMessages),
    )
    .await
    .map_err(|_| "obfuscated peer init timed out".to_owned())?
    .map_err(|error| format!("obfuscated peer init failed: {error}"))?;
    let mut peer = ObfuscatedPeerMessageConnection::new(stream);
    time::timeout(timeout, peer.send(&message))
        .await
        .map_err(|_| "obfuscated peer transfer request timed out".to_owned())?
        .map_err(|error| format!("obfuscated peer transfer request failed: {error}"))?;
    let response = time::timeout(timeout, peer.receive())
        .await
        .map_err(|_| "obfuscated peer transfer response timed out".to_owned())?
        .map_err(|error| format!("obfuscated peer transfer response failed: {error}"))?;
    handle_peer_transfer_negotiation_response(&mut peer, response, transfer, timeout).await
}

async fn handle_peer_transfer_negotiation_response<C>(
    peer: &mut C,
    response: PeerMessage,
    transfer: &TransferEntry,
    timeout: Duration,
) -> Result<PeerTransferNegotiation, String>
where
    C: PeerMessageSenderReceiver,
{
    let PeerMessage::TransferResponse(response) = response else {
        return Err(format!(
            "expected TransferResponse, got {}",
            peer_message_name(&response)
        ));
    };
    match response {
        TransferResponse::Allowed { token, size } => Ok(PeerTransferNegotiation::Allowed {
            token,
            size: size.or(transfer.size),
        }),
        TransferResponse::Rejected { token, reason }
            if token == transfer.token
                && transfer.direction == 0
                && is_remote_queue_response(&reason) =>
        {
            let queued = time::timeout(timeout, peer.receive_peer_message())
                .await
                .map_err(|_| "queued transfer request timed out".to_owned())?
                .map_err(|error| format!("queued transfer request failed: {error}"))?;
            let PeerMessage::TransferRequest(request) = queued else {
                return Err(format!(
                    "expected queued TransferRequest, got {}",
                    peer_message_name(&queued)
                ));
            };
            if request.direction != 1 || request.filename != transfer.filename {
                return Err("queued transfer request did not match pending download".to_owned());
            }
            peer.send_peer_message(&PeerMessage::TransferResponse(TransferResponse::Allowed {
                token: request.token,
                size: request.size.or(transfer.size),
            }))
            .await
            .map_err(|error| format!("queued transfer response failed: {error}"))?;
            Ok(PeerTransferNegotiation::QueuedInbound {
                token: request.token,
                size: request.size.or(transfer.size),
            })
        }
        TransferResponse::Rejected { token, reason } => {
            Ok(PeerTransferNegotiation::Rejected { token, reason })
        }
    }
}

trait PeerMessageSenderReceiver {
    async fn receive_peer_message(&mut self) -> Result<PeerMessage, String>;
    async fn send_peer_message(&mut self, message: &PeerMessage) -> Result<(), String>;
}

impl PeerMessageSenderReceiver for PeerMessageConnection<TcpStream> {
    async fn receive_peer_message(&mut self) -> Result<PeerMessage, String> {
        self.receive()
            .await
            .map_err(|error| format!("plain peer receive failed: {error}"))
    }

    async fn send_peer_message(&mut self, message: &PeerMessage) -> Result<(), String> {
        self.send(message)
            .await
            .map_err(|error| format!("plain peer send failed: {error}"))
    }
}

impl PeerMessageSenderReceiver for ObfuscatedPeerMessageConnection<TcpStream> {
    async fn receive_peer_message(&mut self) -> Result<PeerMessage, String> {
        self.receive()
            .await
            .map_err(|error| format!("obfuscated peer receive failed: {error}"))
    }

    async fn send_peer_message(&mut self, message: &PeerMessage) -> Result<(), String> {
        self.send(message)
            .await
            .map_err(|error| format!("obfuscated peer send failed: {error}"))
    }
}

fn browse_entries_from_peer_message(message: PeerMessage) -> Result<Vec<BrowseEntry>, String> {
    match message {
        PeerMessage::SharedFileListResponse(payload) => parse_shared_file_list_payload(&payload),
        other => Err(format!(
            "expected SharedFileListResponse, got {}",
            peer_message_name(&other)
        )),
    }
}

pub(super) fn folder_entries_from_peer_message(
    message: PeerMessage,
    folder: &str,
    folder_encoding: ProtocolTextEncoding,
) -> Result<Vec<BrowseEntry>, String> {
    match message {
        PeerMessage::FolderContentsResponse(payload) => {
            parse_folder_contents_response_payload(&payload, folder, folder_encoding)
                .or_else(|_| parse_shared_file_list_payload(&payload))
                .or_else(|_| parse_folder_file_list_payload(&payload, folder, folder_encoding))
        }
        other => Err(format!(
            "expected FolderContentsResponse, got {}",
            peer_message_name(&other)
        )),
    }
}

pub(super) fn parse_folder_contents_response_payload(
    payload: &[u8],
    folder: &str,
    folder_encoding: ProtocolTextEncoding,
) -> Result<Vec<BrowseEntry>, String> {
    let decompressed = decompress_zlib_payload(payload).map_err(|error| error.to_string())?;
    let mut reader = Reader::new(&decompressed);
    let _token = reader.read_u32_le().map_err(|error| error.to_string())?;
    let (_root, root_encoding) = reader
        .read_string_with_encoding()
        .map_err(|error| error.to_string())?;
    let directory_count = reader
        .read_bounded_count("folder directories", 8)
        .map_err(|error| error.to_string())?;
    let mut wire_folder_count = 0;
    add_browse_wire_count(
        &mut wire_folder_count,
        directory_count,
        "folder directories",
        MAX_BROWSE_WIRE_FOLDERS_PER_RESPONSE,
    )?;
    let mut wire_file_count = 0;
    let mut entries = Vec::new();
    for _ in 0..directory_count {
        let (directory, directory_encoding) = reader
            .read_string_with_encoding()
            .map_err(|error| error.to_string())?;
        let file_count = reader
            .read_bounded_count("folder files", 21)
            .map_err(|error| error.to_string())?;
        add_browse_wire_count(
            &mut wire_file_count,
            file_count,
            "folder files",
            MAX_BROWSE_WIRE_FILES_PER_RESPONSE,
        )?;
        for _ in 0..file_count {
            let code = reader.read_u8().map_err(|error| error.to_string())?;
            let (filename, filename_encoding) = reader
                .read_string_with_encoding()
                .map_err(|error| error.to_string())?;
            let size = reader.read_u64_le().map_err(|error| error.to_string())?;
            let (extension, _) = reader
                .read_string_with_encoding()
                .map_err(|error| error.to_string())?;
            let attribute_count = reader
                .read_bounded_count("folder file attributes", 8)
                .map_err(|error| error.to_string())?;
            for _ in 0..attribute_count {
                let _code = reader.read_u32_le().map_err(|error| error.to_string())?;
                let _value = reader.read_u32_le().map_err(|error| error.to_string())?;
            }
            if code == 1 {
                if entries.len() >= MAX_BROWSE_ENTRIES_PER_USER {
                    return Err(format!(
                        "folder contents response exceeds {MAX_BROWSE_ENTRIES_PER_USER} entries"
                    ));
                }
                let directory = if directory.is_empty() {
                    folder
                } else {
                    directory.as_str()
                };
                entries.push(bounded_browse_entry(BrowseEntry {
                    filename: join_virtual_path(directory, &filename),
                    size,
                    extension,
                    path_encoding: if root_encoding == ProtocolTextEncoding::Utf8
                        && directory_encoding == ProtocolTextEncoding::Utf8
                    {
                        filename_encoding
                    } else if directory_encoding != ProtocolTextEncoding::Utf8 {
                        directory_encoding
                    } else {
                        folder_encoding
                    },
                }));
            }
        }
    }
    reader.finish().map_err(|error| error.to_string())?;
    Ok(entries)
}
