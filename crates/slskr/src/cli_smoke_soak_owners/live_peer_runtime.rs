use super::*;

pub(super) async fn run_listener(listener: Listener, duration: Duration) -> Result<(), String> {
    let deadline = Instant::now() + duration;
    let mut accepted = 0usize;
    let mut workers = SoakTaskSet::default();

    while Instant::now() < deadline {
        match time::timeout(
            deadline.saturating_duration_since(Instant::now()),
            listener.accept(),
        )
        .await
        {
            Ok(Ok((incoming, address))) => {
                accepted += 1;
                let name = incoming_connection_name(&incoming);
                let address = scrub_socket_addr(address);
                // A public listener receives unrelated peer attempts. Handle
                // each accepted stream independently so a slow or malformed
                // peer cannot block the next valid direct/indirect handshake.
                if !workers.try_spawn(async move {
                    let response_result = handle_plain_soak_incoming(incoming).await;
                    println!("listener event: {name} from {address}");
                    if let Err(error) = response_result {
                        println!(
                            "listener isolated failed peer connection: {}",
                            peer_close_reason(&error)
                        );
                    }
                }) {
                    println!("listener rejected peer work at live-soak capacity");
                }
            }
            Ok(Err(error)) => println!(
                "listener rejected invalid peer initialization: {}",
                peer_close_reason(&error.to_string())
            ),
            Err(_) => break,
        }
    }

    workers.shutdown().await;
    println!("listener observed {accepted} inbound connection(s)");
    Ok(())
}

pub(super) async fn run_obfuscated_listener(
    listener: Listener,
    duration: Duration,
) -> Result<(), String> {
    let deadline = Instant::now() + duration;
    let mut accepted = 0usize;
    let mut workers = SoakTaskSet::default();

    while Instant::now() < deadline {
        match time::timeout(
            deadline.saturating_duration_since(Instant::now()),
            listener.accept_obfuscated(),
        )
        .await
        {
            Ok(Ok((incoming, address))) => {
                accepted += 1;
                let name = incoming_connection_name(&incoming);
                let address = scrub_socket_addr(address);
                if !workers.try_spawn(async move {
                    let response_result = handle_obfuscated_soak_incoming(incoming).await;
                    println!("obfuscated listener event: {name} from {address}");
                    if let Err(error) = response_result {
                        println!(
                            "obfuscated listener isolated failed peer connection: {}",
                            peer_close_reason(&error)
                        );
                    }
                }) {
                    println!("listener rejected peer work at live-soak capacity");
                }
            }
            Ok(Err(error)) => println!(
                "obfuscated listener rejected invalid peer initialization: {}",
                peer_close_reason(&error.to_string())
            ),
            Err(_) => break,
        }
    }

    workers.shutdown().await;
    println!("obfuscated listener observed {accepted} inbound connection(s)");
    Ok(())
}

pub(super) async fn handle_plain_soak_incoming(
    incoming: IncomingConnection<TcpStream>,
) -> Result<(), String> {
    match incoming {
        IncomingConnection::PeerInit {
            kind: ConnectionKind::PeerMessages,
            stream,
            ..
        } => {
            let mut peer = PeerMessageConnection::new(stream);
            match time::timeout(Duration::from_secs(5), peer.receive()).await {
                Ok(Ok(PeerMessage::UserInfoRequest)) => {
                    peer.send(&PeerMessage::UserInfoResponse(UserInfo {
                        description: "slskr live soak".to_owned(),
                        picture: None,
                        total_uploads: 0,
                        queue_size: 0,
                        slots_free: true,
                        upload_permissions: None,
                    }))
                    .await
                    .map_err(|error| format!("peer response send failed: {error}"))?;
                    println!("listener proof: peer user-info request answered");
                }
                Ok(Ok(PeerMessage::FileSearchResponse(response))) => {
                    println!(
                        "listener proof: search response username={} token={} results={} private_results={} slots_free={} queue_length={} speed={}",
                        redact_username(&response.username),
                        response.token,
                        response.results.len(),
                        response.private_results.len(),
                        response.slot_free,
                        response.queue_length,
                        response.average_speed
                    );
                }
                Ok(Ok(PeerMessage::TransferRequest(request))) => {
                    println!(
                        "listener proof: transfer request direction={} token={} filename={} size={}",
                        request.direction,
                        request.token,
                        redact_path(&request.filename),
                        request
                            .size
                            .map(|size| size.to_string())
                            .unwrap_or_else(|| "unknown".to_owned())
                    );
                }
                Ok(Ok(other)) => {
                    println!("listener proof: peer message {}", peer_message_name(&other));
                }
                Ok(Err(error)) => return Err(format!("peer receive failed: {error}")),
                Err(_) => println!("listener proof: peer connection accepted without message"),
            }
        }
        IncomingConnection::PeerInit {
            kind: ConnectionKind::Distributed,
            stream,
            ..
        } => {
            let mut distributed = DistributedConnection::new(stream);
            let message = time::timeout(Duration::from_secs(5), distributed.receive())
                .await
                .map_err(|_| "distributed receive timed out".to_owned())?
                .map_err(|error| format!("distributed receive failed: {error}"))?;
            if message == DistributedMessage::Ping {
                distributed
                    .send(&DistributedMessage::PingResponse { token: 1 })
                    .await
                    .map_err(|error| format!("distributed ping response failed: {error}"))?;
            }
        }
        IncomingConnection::PeerInit {
            kind: ConnectionKind::FileTransfer,
            stream,
            ..
        } => {
            let mut transfer = FileTransferConnection::new(stream);
            let token = time::timeout(Duration::from_secs(5), transfer.receive_token())
                .await
                .map_err(|_| "file-transfer token receive timed out".to_owned())?
                .map_err(|error| format!("file-transfer token receive failed: {error}"))?;
            transfer
                .send_token(token)
                .await
                .map_err(|error| format!("file-transfer token echo failed: {error}"))?;
        }
        _ => {}
    }

    Ok(())
}

pub(super) async fn handle_obfuscated_soak_incoming(
    incoming: IncomingConnection<TcpStream>,
) -> Result<(), String> {
    if let IncomingConnection::ObfuscatedPeerMessages(mut peer) = incoming {
        match time::timeout(Duration::from_secs(5), peer.receive()).await {
            Ok(Ok(PeerMessage::UserInfoRequest)) => {
                peer.send(&PeerMessage::UserInfoResponse(UserInfo {
                    description: "slskr obfuscated live soak".to_owned(),
                    picture: None,
                    total_uploads: 0,
                    queue_size: 0,
                    slots_free: true,
                    upload_permissions: None,
                }))
                .await
                .map_err(|error| format!("obfuscated peer response send failed: {error}"))?;
                println!("obfuscated listener proof: peer user-info request answered");
            }
            Ok(Ok(PeerMessage::FileSearchResponse(response))) => {
                println!(
                    "obfuscated listener proof: search response username={} token={} results={} private_results={} slots_free={} queue_length={} speed={}",
                    redact_username(&response.username),
                    response.token,
                    response.results.len(),
                    response.private_results.len(),
                    response.slot_free,
                    response.queue_length,
                    response.average_speed
                );
            }
            Ok(Ok(other)) => {
                println!(
                    "obfuscated listener proof: peer message {}",
                    peer_message_name(&other)
                );
            }
            Ok(Err(error)) => return Err(format!("obfuscated peer receive failed: {error}")),
            Err(_) => {
                println!("obfuscated listener proof: peer connection accepted without message")
            }
        }
    }

    Ok(())
}

pub(in crate::cli) async fn respond_to_user_info_request<C>(
    peer: &mut C,
    description: &str,
) -> Result<(), String>
where
    C: PeerUserInfoResponder,
{
    match time::timeout(Duration::from_secs(5), peer.receive_user_info_request()).await {
        Ok(Ok(true)) => {
            peer.send_user_info_response(UserInfo {
                description: description.to_owned(),
                picture: None,
                total_uploads: 0,
                queue_size: 0,
                slots_free: true,
                upload_permissions: None,
            })
            .await
        }
        Ok(Ok(false)) => Ok(()),
        Ok(Err(error)) => Err(error),
        Err(_) => Ok(()),
    }
}

pub(in crate::cli) trait PeerUserInfoResponder {
    async fn receive_user_info_request(&mut self) -> Result<bool, String>;
    async fn send_user_info_response(&mut self, info: UserInfo) -> Result<(), String>;
}

impl<S> PeerUserInfoResponder for PeerMessageConnection<S>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    async fn receive_user_info_request(&mut self) -> Result<bool, String> {
        Ok(self
            .receive()
            .await
            .map_err(|error| format!("peer receive failed: {error}"))?
            == PeerMessage::UserInfoRequest)
    }

    async fn send_user_info_response(&mut self, info: UserInfo) -> Result<(), String> {
        self.send(&PeerMessage::UserInfoResponse(info))
            .await
            .map_err(|error| format!("peer response send failed: {error}"))
    }
}

impl<S> PeerUserInfoResponder for ObfuscatedPeerMessageConnection<S>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    async fn receive_user_info_request(&mut self) -> Result<bool, String> {
        Ok(self
            .receive()
            .await
            .map_err(|error| format!("obfuscated peer receive failed: {error}"))?
            == PeerMessage::UserInfoRequest)
    }

    async fn send_user_info_response(&mut self, info: UserInfo) -> Result<(), String> {
        self.send(&PeerMessage::UserInfoResponse(info))
            .await
            .map_err(|error| format!("obfuscated peer response send failed: {error}"))
    }
}

pub(in crate::cli) fn peer_probe_messages(peer: &str) -> [ServerMessage; 4] {
    [
        ServerMessage::WatchUserRequest {
            username: peer.to_owned(),
        },
        ServerMessage::GetUserStatusRequest {
            username: peer.to_owned(),
        },
        ServerMessage::GetUserStatsRequest {
            username: peer.to_owned(),
        },
        ServerMessage::GetPeerAddressRequest {
            username: peer.to_owned(),
        },
    ]
}
